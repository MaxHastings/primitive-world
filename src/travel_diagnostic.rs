//! Isolated observer experiment, never called by the ordinary simulation.
//! Patch labels, visits and trip scores are not policy inputs or rewards.
use super::*;
use serde::Serialize;
use serde_json::json;
use std::io::Write;

#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
struct Frame {
    agent: AgentGpu,
    perception: PerceptionGpu,
    decision: DecisionGpu,
    harvested: u32,
}

#[derive(Clone, Serialize)]
struct Visit {
    tick: u32,
    patch: usize,
    reserves: f32,
}

#[derive(Serialize)]
struct Trip {
    from: usize,
    to: Option<usize>,
    start_tick: u32,
    end_tick: u32,
    path: f32,
    displacement: f32,
    reserve_change: f32,
    collected_food: f32,
    goal_changes: u32,
    ended_by: &'static str,
}

struct Journey {
    from: usize,
    tick: u32,
    position: [f32; 2],
    reserves: f32,
    food_collected: f32,
    path: f32,
    goal_changes: u32,
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn patch_at(position: [f32; 2], centers: &[[f32; 2]; 2], radius: f32) -> Option<usize> {
    centers
        .iter()
        .position(|c| distance(position, *c) <= radius)
}
fn reserves(a: &AgentGpu, conversion: f32) -> f32 {
    a.energy + a.food * conversion
}

// Observer mirror of common.wgsl; GPU tests check each sweep phase against
// uniquely painted cells. Keep coordinates attached to their actual readings.
pub(super) fn sample_offset(slot: usize, radius: f32, tick: u32, flags: u32) -> [f32; 2] {
    if slot == 0 {
        return [0.0; 2];
    }
    let mut dir = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][slot - 1];
    let mut range = if flags & 8 != 0 { radius / 6.0 } else { radius };
    if flags & 16 != 0 {
        let phase = tick % 6;
        range = [radius / 6.0, radius / 2.0, radius][(phase % 3) as usize];
        if phase >= 3 {
            dir = [
                (dir[0] - dir[1]) * std::f32::consts::FRAC_1_SQRT_2,
                (dir[0] + dir[1]) * std::f32::consts::FRAC_1_SQRT_2,
            ];
        }
    }
    [dir[0] * range, dir[1] * range]
}

fn finish_trip(
    j: Journey,
    a: &AgentGpu,
    tick: u32,
    to: Option<usize>,
    collected: f32,
    conversion: f32,
    ended_by: &'static str,
) -> Trip {
    Trip {
        from: j.from,
        to,
        start_tick: j.tick,
        end_tick: tick,
        path: j.path,
        displacement: distance(j.position, a.position),
        reserve_change: reserves(a, conversion) - j.reserves,
        collected_food: collected - j.food_collected,
        goal_changes: j.goal_changes,
        ended_by,
    }
}

impl Simulation {
    pub fn travel_diagnostic(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        args: &[String],
    ) -> Result<(), String> {
        let supported = [
            "--headless",
            "--travel-diagnostic",
            "--seed",
            "--ticks",
            "--travel-distance",
            "--regeneration",
            "--travel-genome",
            "--travel-mode",
            "--travel-food",
            "--travel-radius",
            "--output",
            "--erase-place-memory",
            "--bootstrap",
            "--travel-sensing",
        ];
        if let Some(flag) = args
            .iter()
            .find(|a| a.starts_with("--") && !supported.contains(&a.as_str()))
        {
            return Err(format!("Unsupported travel diagnostic option: {flag}"));
        }
        let arg = |name: &str, default: &str| -> Result<String, String> {
            match args.iter().position(|a| a == name) {
                Some(i) => args
                    .get(i + 1)
                    .filter(|v| !v.starts_with("--"))
                    .cloned()
                    .ok_or_else(|| format!("Missing {name}")),
                None => Ok(default.into()),
            }
        };
        let ticks: u32 = arg("--ticks", "3000")?
            .parse()
            .map_err(|_| "Invalid ticks")?;
        let separation: f32 = arg("--travel-distance", "300")?
            .parse()
            .map_err(|_| "Invalid distance")?;
        let regeneration: f32 = arg("--regeneration", "0.01")?
            .parse()
            .map_err(|_| "Invalid regeneration")?;
        let initial_food: f32 = arg("--travel-food", "0.3")?
            .parse()
            .map_err(|_| "Invalid food")?;
        let radius: f32 = arg("--travel-radius", "16")?
            .parse()
            .map_err(|_| "Invalid radius")?;
        let genome_index: usize = arg("--travel-genome", "0")?
            .parse()
            .map_err(|_| "Invalid genome index")?;
        let mode = arg("--travel-mode", "discovery")?;
        let sensing = arg("--travel-sensing", "baseline")?;
        let sensing_flags = match sensing.as_str() {
            "baseline" => 0,
            "near" => 8,
            "sweep" => 16,
            _ => return Err("Travel sensing must be baseline|near|sweep".into()),
        };
        let output = arg("--output", "reports/travel-diagnostic.json")?;
        let erase = args.iter().any(|a| a == "--erase-place-memory");
        let bootstrap = args.iter().any(|a| a == "--bootstrap");
        if !(1..=10000).contains(&ticks)
            || !separation.is_finite()
            || !(80.0..=800.0).contains(&separation)
            || !regeneration.is_finite()
            || !(0.0..=0.1).contains(&regeneration)
            || !initial_food.is_finite()
            || !(0.01..=1.0).contains(&initial_food)
            || !radius.is_finite()
            || !(8.0..=96.0).contains(&radius)
            || radius * 2.0 >= separation
            || !["discovery", "known-target"].contains(&mode.as_str())
        {
            return Err("Travel experiment requires ticks 1..10000, distance 80..800, regeneration 0..0.1, food 0.01..1, nonoverlapping radius 8..96, and mode discovery|known-target".into());
        }
        if genome_index >= self.settings.founder_genomes.len() {
            return Err("Invalid bundled genome index".into());
        }
        // Reserve the report before allocating/running an experiment; never overwrite evidence.
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
            .map_err(|e| format!("{output}: {e}"))?;
        let centers = [[600.0, 900.0], [600.0 + separation, 900.0]];
        self.settings.population = 1;
        self.settings.resource_regeneration = regeneration;
        self.settings.evolving_landscape = false;
        self.settings.neural_flags = sensing_flags;
        self.reset(queue);
        let mut a: AgentGpu =
            neural_bridge::read_items(device, queue, &self.agent_buffers[0], 0, 1)?[0];
        a.position = centers[0];
        a.goal = a.position;
        a.velocity = [0.0; 2];
        a.energy = 65.0;
        a.food = 2.0;
        a.age = 500.0;
        a.next_birth = u32::MAX;
        a.max_age = 11000.0;
        a.max_speed = 1.2;
        a.places = [PlaceGpu::default(); PLACE_SLOTS];
        a.commit_until = 0;
        a.genome = if bootstrap {
            bootstrap_genome()
        } else {
            self.settings.founder_genomes[genome_index]
                .as_slice()
                .try_into()
                .unwrap()
        };
        if mode == "known-target" {
            a.places[0] = PlaceGpu {
                position: centers[1],
                food: initial_food,
                observed: 0,
                confidence: 1.0,
                source_id: 0,
                source_generation: a.generation,
                ..Default::default()
            };
        }
        if erase {
            a.places = [PlaceGpu::default(); PLACE_SLOTS];
        }
        for buffer in &self.agent_buffers {
            queue.write_buffer(buffer, 0, bytemuck::bytes_of(&a));
        }
        let cells = (RESOURCE_GRID * RESOURCE_GRID) as usize;
        let mut resource = vec![0u32; cells];
        let mut ground = vec![[0u32; 8]; cells];
        let mut patch_cells = [0u32; 2];
        for y in 0..RESOURCE_GRID {
            for x in 0..RESOURCE_GRID {
                let index = (y * RESOURCE_GRID + x) as usize;
                if let Some(patch) = patch_at(
                    [x as f32 * 4.0 + 2.0, y as f32 * 4.0 + 2.0],
                    &centers,
                    radius,
                ) {
                    patch_cells[patch] += 1;
                    resource[index] = if mode == "known-target" && patch == 0 {
                        0
                    } else {
                        (initial_food * 1000.0).round() as u32
                    };
                    ground[index][6] = 1.0f32.to_bits(); // habitat
                    ground[index][7] = 1.0f32.to_bits(); // explicit productivity, no global normalization
                }
            }
        }
        queue.write_buffer(&self.resource_buffer, 0, bytemuck::cast_slice(&resource));
        queue.write_buffer(&self.ground_buffer, 0, bytemuck::cast_slice(&ground));
        queue.write_buffer(
            &self.fertility_buffer,
            0,
            bytemuck::cast_slice(&vec![0.65f32; cells]),
        );
        let initial = a;
        let conversion = self.settings.conversion_efficiency;
        let mut visits = vec![Visit {
            tick: 0,
            patch: 0,
            reserves: reserves(&a, conversion),
        }];
        let mut trips = Vec::new();
        let mut journey: Option<Journey> = None;
        let mut observed = [None; 2];
        let mut entry = [Some(0u32), None];
        let mut actions = [0u32; 7];
        let mut path = 0.0f32;
        let mut collected = 0.0f32;
        let mut collected_per_patch = [0.0f32; 2];
        let mut switches = 0u32;
        let mut expired_switches = 0u32;
        let mut trace = Vec::new();
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("travel frame"),
            size: std::mem::size_of::<Frame>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for _ in 0..ticks {
            let before = a;
            if erase {
                a.places = [PlaceGpu::default(); PLACE_SLOTS];
                queue.write_buffer(
                    &self.agent_buffers[self.current_buffer],
                    0,
                    bytemuck::bytes_of(&a),
                );
            }
            let mut encoder = device.create_command_encoder(&Default::default());
            self.encode_ticks(&mut encoder, device, queue, 1);
            let mut offset = 0u64;
            for (buffer, size) in [
                (
                    &self.agent_buffers[self.current_buffer],
                    std::mem::size_of::<AgentGpu>(),
                ),
                (
                    &self.perception_buffer,
                    std::mem::size_of::<PerceptionGpu>(),
                ),
                (&self._decision_buffer, std::mem::size_of::<DecisionGpu>()),
                (&self._request_buffer, 4),
            ] {
                encoder.copy_buffer_to_buffer(buffer, 0, &staging, offset, size as u64);
                offset += size as u64;
            }
            queue.submit(Some(encoder.finish()));
            let (tx, rx) = mpsc::channel();
            staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                let _ = tx.send(r);
            });
            device.poll(wgpu::Maintain::Wait);
            rx.recv()
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;
            let frame: Frame = bytemuck::pod_read_unaligned(&staging.slice(..).get_mapped_range());
            staging.unmap();
            a = frame.agent;
            let p = frame.perception;
            let samples = [
                p.resource_here,
                p.resource_north,
                p.resource_east,
                p.resource_south,
                p.resource_west,
            ];
            for k in 0..5 {
                let offset = sample_offset(k, before.sensor_radius, self.tick - 1, sensing_flags);
                if samples[k] > 0.001 {
                    if let Some(patch) = patch_at(
                        [
                            (before.position[0] + offset[0]).clamp(0.0, WORLD_SIZE),
                            (before.position[1] + offset[1]).clamp(0.0, WORLD_SIZE),
                        ],
                        &centers,
                        radius,
                    ) {
                        observed[patch].get_or_insert(self.tick - 1);
                    }
                }
            }
            let moved = distance(a.position, before.position);
            path += moved;
            collected += frame.harvested as f32 / 1000.0;
            if let Some(patch) = patch_at(before.position, &centers, radius) {
                collected_per_patch[patch] += frame.harvested as f32 / 1000.0;
            }
            actions[a.action.min(6) as usize] += 1;
            let switched = a.action == 1 && distance(a.goal, before.goal) > 2.0;
            if switched {
                switches += 1;
                if self.tick - 1 >= before.commit_until {
                    expired_switches += 1;
                }
            }
            let old_patch = patch_at(before.position, &centers, radius);
            let new_patch = patch_at(a.position, &centers, radius);
            if journey.is_none() && old_patch.is_some() && new_patch.is_none() {
                journey = Some(Journey {
                    from: old_patch.unwrap(),
                    tick: self.tick - 1,
                    position: before.position,
                    reserves: reserves(&before, conversion),
                    food_collected: collected - frame.harvested as f32 / 1000.0,
                    path: 0.0,
                    goal_changes: 0,
                });
            }
            if let Some(j) = &mut journey {
                j.path += moved;
                j.goal_changes += u32::from(switched);
            }
            if a.alive != 0 {
                if let Some(patch) = new_patch {
                    entry[patch].get_or_insert(self.tick);
                    if visits.last().unwrap().patch != patch {
                        visits.push(Visit {
                            tick: self.tick,
                            patch,
                            reserves: reserves(&a, conversion),
                        });
                    }
                    if let Some(j) = journey.take() {
                        trips.push(finish_trip(
                            j,
                            &a,
                            self.tick,
                            Some(patch),
                            collected,
                            conversion,
                            "arrival",
                        ));
                    }
                }
            }
            if self.tick % 16 == 0 || switched || new_patch != old_patch || a.alive == 0 {
                trace.push(json!({"tick":self.tick,"position":a.position,"goal":a.goal,"action":a.action,"energy":a.energy,"food":a.food,"commit_until":a.commit_until,
                    "place_count":a.places.iter().filter(|p|p.confidence>0.0).count(),"selected_score":frame.decision.scores[a.action.min(6)as usize]}));
            }
            if a.alive == 0 {
                break;
            }
        }
        if let Some(j) = journey {
            trips.push(finish_trip(
                j,
                &a,
                self.tick,
                None,
                collected,
                conversion,
                if a.alive == 0 { "death" } else { "censored" },
            ));
        }
        let completed = visits.len().saturating_sub(1);
        // Final resource totals are observer readbacks, never fed to the agent.
        let resource_bytes = observability::read_buffer(device, queue, &self.resource_buffer)?;
        let ground_bytes = observability::read_buffer(device, queue, &self.ground_buffer)?;
        let resource_final: &[u32] = bytemuck::cast_slice(&resource_bytes);
        let ground_final: &[[u32; 8]] = bytemuck::cast_slice(&ground_bytes);
        let mut remaining_food = [0.0f64; 2];
        let mut regenerated_food = [0.0f64; 2];
        let mut dropped_food = 0.0f64;
        for y in 0..RESOURCE_GRID {
            for x in 0..RESOURCE_GRID {
                let cell = (y * RESOURCE_GRID + x) as usize;
                dropped_food += ground_final[cell][0] as f64 / 1000.0;
                if let Some(patch) = patch_at(
                    [x as f32 * 4.0 + 2.0, y as f32 * 4.0 + 2.0],
                    &centers,
                    radius,
                ) {
                    remaining_food[patch] += resource_final[cell] as f64 / 1000.0;
                    regenerated_food[patch] += ground_final[cell][3] as f64 / 1000.0;
                }
            }
        }
        let report = json!({"schema":1,"experiment":"isolated-two-patch-travel","seed":self.seed,"requested_ticks":ticks,"elapsed_ticks":self.tick,
            "configuration":{"mode":mode,"sensing":sensing,"sensing_flags":sensing_flags,"erase_place_memory":erase,"distance":separation,"patch_radius":radius,"patch_cells":patch_cells,"regeneration":regeneration,"initial_food_per_cell":initial_food,
                "centers":centers,"founder_name":if bootstrap {"candidate-v1-bootstrap"}else{&self.settings.founder_name},"genome_index":genome_index,"genome":initial.genome.to_vec(),
                "births_suppressed":true,"geography_static":true,"weather_active":true,"patch_productivity":1.0,"initial_energy":initial.energy,"initial_inventory":initial.food},
            "outcome":{"alive":a.alive!=0,"observed_patch_tick":observed,"entered_patch_tick":entry,"inter_patch_transitions":completed,"returns_after_other_patch":completed.saturating_sub(1),
                "path_length":path,"food_collected":collected,"food_collected_per_patch":collected_per_patch,"remaining_food_per_patch":remaining_food,"regenerated_food_per_patch":regenerated_food,"dropped_food_at_end":dropped_food,"initial_reserves":reserves(&initial,conversion),"final_reserves":reserves(&a,conversion),
                "metabolic_cost":self.tick as f32*self.settings.metabolic_cost,"movement_cost":path*self.settings.movement_energy_cost,
                "energy_accounting_residual":reserves(&a,conversion)-(reserves(&initial,conversion)+collected*conversion-self.tick as f32*self.settings.metabolic_cost-path*self.settings.movement_energy_cost),
                "energy_accounting_residual_including_death_drops":reserves(&a,conversion)as f64+dropped_food*conversion as f64-(reserves(&initial,conversion)+collected*conversion-self.tick as f32*self.settings.metabolic_cost-path*self.settings.movement_energy_cost)as f64,
                "action_ticks":actions,"goal_changes":switches,"goal_changes_after_deadline":expired_switches},"visits":visits,"trips":trips,"trace":trace,
            "scope":"Controlled local navigation, not ordinary-world population fitness or evidence of social routes. Known-target mode supplies one remembered coordinate as an explicit treatment. Memory erasure preserves current journey commitment."});
        file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        eprintln!(
            "Travel {} seed {} distance {} erase {}: {}",
            mode, self.seed, separation, erase, report["outcome"]
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patch_labels_and_trip_accounting_are_observer_only() {
        let centers = [[0.0, 0.0], [100.0, 0.0]];
        assert_eq!(patch_at([0.0, 0.0], &centers, 16.0), Some(0));
        assert_eq!(patch_at([50.0, 0.0], &centers, 16.0), None);
        assert_eq!(patch_at([100.0, 0.0], &centers, 16.0), Some(1));
        let a = AgentGpu {
            position: [100.0, 0.0],
            energy: 40.0,
            food: 2.0,
            ..Default::default()
        };
        let j = Journey {
            from: 0,
            tick: 0,
            position: [0.0, 0.0],
            reserves: 60.0,
            food_collected: 1.0,
            path: 120.0,
            goal_changes: 2,
        };
        let trip = finish_trip(j, &a, 100, Some(1), 3.0, 8.0, "arrival");
        assert_eq!(trip.reserve_change, -4.0);
        assert_eq!(trip.collected_food, 2.0);
        assert_eq!(trip.displacement, 100.0);
    }
}
