use super::*;
use std::collections::HashSet;
use std::io::{Read, Write};
pub(crate) const METRICS_SUMMARY_SIZE: u64 = 4096 * 64;
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct CheckpointMetadata {
    settings: SimSettings,
    progress: crate::evolution::Progress,
    environment_start_age: u32,
    #[serde(default)]
    assisted: bool,
}

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct WorldMetrics {
    pub food_ingested: f64,
    pub tick: u32,
    pub living: u64,
    pub packets: u64,
    pub painted_agents: u32,
    pub packet_energy: f64,
    pub mean_packet_size: f64,
    pub failed_fusions: u32,
    pub capacity_blocked_packets: u32,
    pub juveniles: u64,
    pub carried_food: f64,
    pub energy: f64,
    pub vegetation: f64,
    pub dropped_food: f64,
    pub regenerated: f64,
    pub weather_loss: f64,
    pub events: [u32; 8],
    pub signals: u32,
    pub exact_copy_births: u32,
    pub stocked_agents: u64,
    pub hungry_agents: u64,
    pub moving_agents: u64,
    pub eating_agents: u64,
    pub harvested: f64,
    /// Packet production: immature, budget shortfall, produced, requested, eligible; then births.
    pub birth_gates: [u32; 6],
    pub action_ticks: [u32; 6],
    pub invalid_outputs: u32,
    pub force_attempts: u32,
    pub force_energy_spent: f64,
    pub forced_distance: f64,
    /// Births that expanded/retired an expressed unit, respectively.
    pub topology_activations: u32,
    pub topology_retirals: u32,
    pub cognitive_write_energy: f64,
    pub cognitive_upkeep_energy: f64,
}

/// A read-only population snapshot for studying evolution. Nothing in this
/// structure is uploaded to the GPU or consulted by the decision pipeline.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct EvolutionSnapshot {
    pub tick: u32,
    pub living: u64,
    pub individual_identities: u64,
    pub parent_lineages_present: u64,
    pub maximum_ancestry_depth: u32,
    pub mean_ancestry_depth: f64,
    /// Mean last-tick actual velocity for living agents.
    pub mean_velocity_x: f64,
    pub mean_velocity_y: f64,
    /// Direction counts use an intentionally tiny horizontal dead zone.
    pub leftward_agents: u64,
    pub rightward_agents: u64,
    pub nonhorizontal_agents: u64,
    /// -1 means all horizontally moving agents went left; +1 means all went
    /// right; 0 means balanced or no horizontal movement.
    pub horizontal_directional_bias: f64,
    /// Mean normalized local food-gradient direction seen under living bodies.
    pub mean_food_gradient_x: f64,
    pub mean_food_gradient_y: f64,
    /// Mean alignment of actual velocity with the local food gradient.
    /// Positive values move up-gradient; negative values move away.
    pub food_gradient_alignment: f64,
    pub food_gradient_samples: u64,
    /// Distribution over capacities 1..16; index zero is intentionally unused.
    pub active_capacity_distribution: Vec<u64>,
    /// Body packet-size traits in bins [1,6), [6,12), ... [42,48].
    pub packet_size_distribution: [u64; 8],
    pub mean_active_capacity: f64,
    pub mean_hidden_memory_magnitude: f64,
    pub mean_learned_weight_magnitude: f64,
}

fn local_food_gradient(food: &[u32], position: [f32; 2], world_size: [f32; 2]) -> [f64; 2] {
    let grid = RESOURCE_GRID as usize;
    let cell_x = (position[0] / (world_size[0] / RESOURCE_GRID as f32))
        .floor()
        .clamp(0.0, (grid - 1) as f32) as i32;
    let cell_y = (position[1] / (world_size[1] / RESOURCE_GRID as f32))
        .floor()
        .clamp(0.0, (grid - 1) as f32) as i32;
    let sample = |x: i32, y: i32| -> f64 {
        let x = x.rem_euclid(grid as i32) as usize;
        let y = y.rem_euclid(grid as i32) as usize;
        food[y * grid + x] as f64
    };
    [
        (sample(cell_x + 1, cell_y) - sample(cell_x - 1, cell_y)) / world_size[0] as f64,
        (sample(cell_x, cell_y + 1) - sample(cell_x, cell_y - 1)) / world_size[1] as f64,
    ]
}

pub fn read_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, String> {
    let size = buffer.size();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("state readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let bytes = staging.slice(..).get_mapped_range().to_vec();
    staging.unmap();
    Ok(bytes)
}

impl Simulation {
    pub fn event_sequence(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<u32, String> {
        let stats = read_buffer(device, queue, &self.death_stats_buffer)?;
        Ok(bytemuck::cast_slice::<u8, u32>(&stats)[8])
    }

    /// Vegetation only, for optional read-only journey diagnostics.
    pub fn vegetation_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<u32>, String> {
        let bytes = read_buffer(device, queue, &self.resource_buffer)?;
        Ok(bytes
            .chunks_exact(4)
            .map(bytemuck::pod_read_unaligned::<u32>)
            .collect())
    }

    /// Read the current body buffer without changing simulation state.
    pub fn agent_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<AgentGpu>, String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        Ok(bytes
            .chunks_exact(std::mem::size_of::<AgentGpu>())
            .map(bytemuck::pod_read_unaligned::<AgentGpu>)
            .collect())
    }

    pub fn evolution_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<EvolutionSnapshot, String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let agents = bytemuck::cast_slice::<u8, AgentGpu>(&bytes);
        let food_bytes = read_buffer(device, queue, &self.resource_buffer)?;
        let food = bytemuck::cast_slice::<u8, u32>(&food_bytes);
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(
            &mut encoder,
            "summarize_learning",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        queue.submit(Some(encoder.finish()));
        let learned_bytes = read_buffer(device, queue, &self.learned_summary_buffer)?;
        let learned: &[f32] = bytemuck::cast_slice(&learned_bytes);
        let mut lineages = HashSet::new();
        let mut parent_lineages = HashSet::new();
        let mut moving_gradient_samples = 0u64;
        let mut snapshot = EvolutionSnapshot {
            tick: self.tick,
            active_capacity_distribution: vec![0; HIDDEN + 1],
            ..Default::default()
        };
        for (slot, agent) in agents.iter().enumerate().filter(|(_, a)| a.alive == 1) {
            snapshot.living += 1;
            snapshot.packet_size_distribution[((agent.packet_size / 6.0) as usize).min(7)] += 1;
            let capacity = agent.active_mask.count_ones() as usize;
            snapshot.active_capacity_distribution[capacity] += 1;
            snapshot.mean_active_capacity += capacity as f64;
            snapshot.mean_hidden_memory_magnitude += agent
                .hidden
                .iter()
                .enumerate()
                .filter(|(h, _)| agent.active_mask & (1u32 << h) != 0)
                .map(|(_, v)| v.abs() as f64)
                .sum::<f64>();
            snapshot.mean_learned_weight_magnitude += learned[slot] as f64;
            lineages.insert(agent.lineage_id);
            if agent.parent_lineage != 0 {
                parent_lineages.insert(agent.parent_lineage);
            }
            snapshot.maximum_ancestry_depth =
                snapshot.maximum_ancestry_depth.max(agent.ancestry_depth);
            snapshot.mean_ancestry_depth += agent.ancestry_depth as f64;
            snapshot.mean_velocity_x += agent.velocity[0] as f64;
            snapshot.mean_velocity_y += agent.velocity[1] as f64;
            let gradient = local_food_gradient(
                food,
                agent.position,
                [self.settings.habitat_width, self.settings.habitat_height],
            );
            let gradient_length = gradient[0].hypot(gradient[1]);
            if gradient_length > 0.0 {
                snapshot.mean_food_gradient_x += gradient[0] / gradient_length;
                snapshot.mean_food_gradient_y += gradient[1] / gradient_length;
                snapshot.food_gradient_samples += 1;
                let speed = f64::from(agent.velocity[0]).hypot(f64::from(agent.velocity[1]));
                if speed > 0.0001 {
                    moving_gradient_samples += 1;
                    snapshot.food_gradient_alignment += (f64::from(agent.velocity[0])
                        * gradient[0]
                        + f64::from(agent.velocity[1]) * gradient[1])
                        / (speed * gradient_length);
                }
            }
            if agent.velocity[0] < -0.0001 {
                snapshot.leftward_agents += 1;
            } else if agent.velocity[0] > 0.0001 {
                snapshot.rightward_agents += 1;
            } else {
                snapshot.nonhorizontal_agents += 1;
            }
        }
        snapshot.individual_identities = lineages.len() as u64;
        snapshot.parent_lineages_present = parent_lineages.len() as u64;
        if snapshot.living > 0 {
            let count = snapshot.living as f64;
            snapshot.mean_ancestry_depth /= count;
            snapshot.mean_active_capacity /= count;
            snapshot.mean_hidden_memory_magnitude /= count;
            snapshot.mean_learned_weight_magnitude /= count;
            snapshot.mean_velocity_x /= count;
            snapshot.mean_velocity_y /= count;
            let horizontal = snapshot.leftward_agents + snapshot.rightward_agents;
            if horizontal > 0 {
                snapshot.horizontal_directional_bias = (snapshot.rightward_agents as f64
                    - snapshot.leftward_agents as f64)
                    / horizontal as f64;
            }
        }
        if snapshot.food_gradient_samples > 0 {
            let count = snapshot.food_gradient_samples as f64;
            snapshot.mean_food_gradient_x /= count;
            snapshot.mean_food_gradient_y /= count;
        }
        if moving_gradient_samples > 0 {
            snapshot.food_gradient_alignment /= moving_gradient_samples as f64;
        } else {
            snapshot.food_gradient_alignment = 0.0;
        }
        Ok(snapshot)
    }

    pub fn metrics(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<WorldMetrics, String> {
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, "summary", self.current_buffer, 64, 1);
        queue.submit(Some(encoder.finish()));
        let bytes = read_buffer(device, queue, &self.summary_buffer)?;
        let events = read_buffer(device, queue, &self.death_stats_buffer)?;
        self.decode_metrics(&bytes, &events)
    }

    pub(crate) fn encode_metrics(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        output: &wgpu::Buffer,
        offset: u64,
    ) {
        self.dispatch(encoder, "summary", self.current_buffer, 64, 1);
        encoder.copy_buffer_to_buffer(
            &self.summary_buffer,
            0,
            output,
            offset,
            METRICS_SUMMARY_SIZE,
        );
    }

    pub(crate) fn decode_metrics(
        &self,
        bytes: &[u8],
        events: &[u8],
    ) -> Result<WorldMetrics, String> {
        let mut total = [0u64; 16];
        for chunk in bytemuck::cast_slice::<u8, u32>(bytes).chunks_exact(16) {
            for i in 0..16 {
                total[i] += chunk[i] as u64;
            }
        }
        let counters: &[u32] = bytemuck::cast_slice(events);
        Ok(WorldMetrics {
            food_ingested: (u64::from(counters[0]) + (u64::from(counters[14]) << 32)) as f64
                / 1000.0,
            tick: self.tick,
            living: total[0],
            packets: total[8],
            painted_agents: counters[11],
            packet_energy: total[9] as f64 / 1000.0,
            mean_packet_size: total[10] as f64 / (1000.0 * (total[0] - total[8]).max(1) as f64),
            failed_fusions: counters[38],
            capacity_blocked_packets: counters[39],
            juveniles: total[1],
            carried_food: total[2] as f64 / 1000.0,
            energy: total[3] as f64 / 1000.0,
            vegetation: total[4] as f64 / 1000.0,
            dropped_food: total[5] as f64 / 1000.0,
            regenerated: total[6] as f64 / 1000.0,
            weather_loss: total[7] as f64 / 1000.0,
            events: counters[..8]
                .try_into()
                .map_err(|_| "invalid event buffer")?,
            signals: counters[9],
            stocked_agents: total[11],
            hungry_agents: total[12],
            moving_agents: total[13],
            harvested: total[14] as f64 / 1000.0,
            eating_agents: total[15],
            birth_gates: [
                counters[16],
                counters[17],
                counters[19],
                counters[20],
                counters[21],
                counters[22],
            ],
            action_ticks: counters[24..30]
                .try_into()
                .map_err(|_| "Invalid action counters")?,
            invalid_outputs: counters[31],
            force_attempts: counters[12],
            force_energy_spent: counters[13] as f64 / 1000.0,
            forced_distance: counters[15] as f64 / 1000.0,
            exact_copy_births: counters[37],
            topology_activations: counters[32],
            topology_retirals: counters[33],
            cognitive_write_energy: counters[34] as f64 / 1000.0,
            cognitive_upkeep_energy: counters[35] as f64 / 1000.0,
        })
    }

    pub fn checkpoint_metadata(&self) -> Result<Vec<u8>, String> {
        self.settings.validate()?;
        self.progress
            .validate(self.settings.population, self.seed, self.tick)?;
        serde_json::to_vec(&CheckpointMetadata {
            settings: self.settings.clone(),
            progress: self.progress.clone(),
            environment_start_age: self.environment_start_age,
            assisted: self.assisted,
        })
        .map_err(|e| e.to_string())
    }
    pub fn save_checkpoint(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), String> {
        // Capture before opening the destination, so a GPU read failure cannot truncate a save.
        let settings = self.checkpoint_metadata()?;
        let buffers = [
            &self.agent_buffers[self.current_buffer],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
            &self.perception_buffer,
            &self.decision_buffer,
            &self.genome_buffers[0],
            &self.genome_buffers[1],
            &self.fast_weight_buffers[0],
            &self.fast_weight_buffers[1],
            &self.trace_buffer,
            &self.reservoir_genome_buffers[0],
            &self.reservoir_genome_buffers[1],
            &self.reservoir_traits_buffer,
            &self.reservoir_rng_buffer,
            &self.ecology_buffer,
        ];
        let data: Vec<_> = buffers
            .iter()
            .map(|b| read_buffer(device, queue, b))
            .collect::<Result<_, _>>()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                format!(
                    "{}: {e}; preserve or rename the previous save first",
                    path.display()
                )
            })?;
        file.write_all(CHECKPOINT_MAGIC)
            .map_err(|e| e.to_string())?;
        for n in [self.seed, self.tick, settings.len() as u32] {
            file.write_all(&n.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        file.write_all(&settings).map_err(|e| e.to_string())?;
        for bytes in data {
            file.write_all(&(bytes.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_checkpoint(
        &mut self,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        self.load_checkpoint_reader(queue, file)
    }

    pub fn load_checkpoint_reader(
        &mut self,
        queue: &wgpu::Queue,
        file: impl Read,
    ) -> Result<(), String> {
        self.load_checkpoint_checked(queue, file, None)
    }

    pub fn load_checkpoint_checked(
        &mut self,
        queue: &wgpu::Queue,
        file: impl Read,
        expected: Option<(u32, u32, u32)>,
    ) -> Result<(), String> {
        self.load_checkpoint_data(queue, file, expected, None)
    }
    pub fn load_game_checkpoint(
        &mut self,
        queue: &wgpu::Queue,
        file: impl Read,
        expected: (u32, u32, u32),
        world: u64,
    ) -> Result<(), String> {
        self.load_checkpoint_data(queue, file, Some(expected), Some(world))
    }
    fn load_checkpoint_data(
        &mut self,
        queue: &wgpu::Queue,
        mut file: impl Read,
        expected: Option<(u32, u32, u32)>,
        expected_world: Option<u64>,
    ) -> Result<(), String> {
        let mut magic = [0; 12];
        file.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != CHECKPOINT_MAGIC {
            return Err(format!(
                "Unsupported checkpoint: expected {} format {}. Only current-format data can be loaded.",
                MODEL_ID, CHECKPOINT_VERSION
            ));
        }
        let mut fields = [0; 12];
        file.read_exact(&mut fields).map_err(|e| e.to_string())?;
        let seed = u32::from_le_bytes(fields[0..4].try_into().unwrap());
        let tick = u32::from_le_bytes(fields[4..8].try_into().unwrap());
        if tick > MAX_WORLD_TICKS {
            return Err("Checkpoint exceeds world tick capacity".into());
        }
        let settings_len = u32::from_le_bytes(fields[8..12].try_into().unwrap()) as usize;
        if settings_len > 16_777_216 {
            return Err("Invalid settings length".into());
        }
        let mut json = vec![0; settings_len];
        file.read_exact(&mut json).map_err(|e| e.to_string())?;
        let metadata: CheckpointMetadata =
            serde_json::from_slice(&json).map_err(|e| e.to_string())?;
        let settings = metadata.settings;
        if metadata.environment_start_age > MAX_WORLD_TICKS {
            return Err("Checkpoint environment age exceeds tick capacity".into());
        }
        metadata
            .progress
            .validate(settings.population, seed, tick)?;
        if expected_world.is_some_and(|world| world != metadata.progress.world) {
            return Err("The game receipt and checkpoint world do not match".into());
        }
        settings.validate()?;
        let mut buffers = vec![
            &self.agent_buffers[0],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
        ];
        buffers.extend([
            &self.perception_buffer,
            &self.decision_buffer,
            &self.genome_buffers[0],
            &self.genome_buffers[1],
        ]);
        let mut data = Vec::new();
        buffers.extend([
            &self.fast_weight_buffers[0],
            &self.fast_weight_buffers[1],
            &self.trace_buffer,
            &self.reservoir_genome_buffers[0],
            &self.reservoir_genome_buffers[1],
            &self.reservoir_traits_buffer,
            &self.reservoir_rng_buffer,
            &self.ecology_buffer,
        ]);
        for buffer in buffers.iter() {
            let mut length = [0; 8];
            file.read_exact(&mut length).map_err(|e| e.to_string())?;
            let stored = u64::from_le_bytes(length);
            let expected = buffer.size();
            if stored != expected {
                return Err("Checkpoint layout mismatch".into());
            }
            let mut bytes = vec![0; expected as usize];
            file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
            data.push(bytes);
        }
        for a in data[0]
            .chunks_exact(std::mem::size_of::<AgentGpu>())
            .map(bytemuck::pod_read_unaligned::<AgentGpu>)
        {
            if a.lived_ticks > tick
                || a.alive > 2
                || a.action > 5
                || !a.position[0].is_finite()
                || !a.position[1].is_finite()
                || !(0.0..=settings.habitat_width).contains(&a.position[0])
                || !(0.0..=settings.habitat_height).contains(&a.position[1])
                || [
                    a.energy,
                    a.age,
                    a.food,
                    a.max_speed,
                    a.sensor_radius,
                    a.max_age,
                    a.heading,
                    a.angular_velocity,
                    a.signal_payload,
                    a.collected,
                    a.ingested,
                    a.spent,
                    a.received,
                ]
                .iter()
                .chain(&a.hidden)
                .chain(&a.velocity)
                .chain(&a.moved)
                .any(|v| !v.is_finite())
                || a.physical_previous
                    .iter()
                    .any(|bits| !f32::from_bits(*bits).is_finite())
                || a.food < 0.0
                || a.food > 8.001
                || a.energy < 0.0
                || a.energy > 100.001
                || !a.packet_size.is_finite()
                || !(1.0..=48.0).contains(&a.packet_size)
                || (a.alive == 2
                    && (a.energy > a.packet_size + 0.001
                        || a.food != 0.0
                        || a.hidden != [0.0; HIDDEN]))
                || a.age < 0.0
                || a.max_age < 1.0
                || a.max_age > 11000.0
                || a.max_speed < 0.0
                || a.max_speed > 1.2
                || a.hidden.iter().any(|v| v.abs() > 1.0)
                || a.active_mask == 0
                || a.active_mask & !ACTIVE_MASK_ALL != 0
                || !a.trace_retention.is_finite()
                || !a.learned_weight_retention.is_finite()
                || !(0.0..=0.9999).contains(&a.trace_retention)
                || !(0.0..=0.9999).contains(&a.learned_weight_retention)
                || !a.parameter_mutation_rate.is_finite()
                || !(0.25..=4.0).contains(&a.parameter_mutation_rate)
                || !a.parameter_mutation_step.is_finite()
                || !(0.25..=4.0).contains(&a.parameter_mutation_step)
                || !a.topology_mutation_rate.is_finite()
                || !(0.25..=4.0).contains(&a.topology_mutation_rate)
                || a.plasticity_rate
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 0.2)
                || a.sensor_radius < 4.0
                || a.sensor_radius > 48.0
            {
                return Err("Invalid primitive-world body checkpoint".into());
            }
        }
        let bank0: &[f32] = bytemuck::cast_slice(&data[8]);
        let bank1: &[f32] = bytemuck::cast_slice(&data[9]);
        for slot in 0..MAX_AGENTS as usize {
            let mut genome = [0.0; GENOME_SIZE];
            genome[..GENOME_BANK_STRIDE].copy_from_slice(
                &bank0[slot * GENOME_BANK_STRIDE..(slot + 1) * GENOME_BANK_STRIDE],
            );
            genome[GENOME_BANK_STRIDE..].copy_from_slice(
                &bank1[slot * GENOME_BANK_STRIDE
                    ..slot * GENOME_BANK_STRIDE + GENOME_SIZE - GENOME_BANK_STRIDE],
            );
            crate::brain::validate(&genome)?;
        }
        if data[10..13].iter().any(|bytes| {
            bytes
                .chunks_exact(4)
                .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
                .any(|v| !v.is_finite() || v.abs() > 1.0)
        }) {
            return Err("Invalid checkpoint learned memory state".into());
        }
        if data[2]
            .chunks_exact(4)
            .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err("Invalid checkpoint soil".into());
        }
        for cell in data[3].chunks_exact(32) {
            let value = |i| f32::from_le_bytes(cell[i..i + 4].try_into().unwrap());
            // Mean-preserving fragmentation can raise a local habitat multiplier
            // slightly above one. It is finite, nonnegative model state rather
            // than corruption; rendering clamps it while capacity uses it as a
            // local geography multiplier.
            if [value(8), value(24), value(28)]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
                || value(8) >= 1.0
                || value(28) > 1000.0
            {
                return Err("Invalid checkpoint ground".into());
            }
        }
        for p in data[6]
            .chunks_exact(std::mem::size_of::<PerceptionGpu>())
            .map(bytemuck::pod_read_unaligned::<PerceptionGpu>)
        {
            if !p.resource_here.is_finite()
                || !p.nearby_count.is_finite()
                || p.regions.iter().any(|s| {
                    !s.food.is_finite() || !s.bodies.is_finite() || s.food < 0.0 || s.bodies < 0.0
                })
                || p.regions.iter().any(|b| {
                    !b.signal.is_finite()
                        || !b.pressure.is_finite()
                        || b.velocity.iter().any(|v| !v.is_finite())
                })
            {
                return Err("Invalid checkpoint perception".into());
            }
        }
        for d in data[7]
            .chunks_exact(std::mem::size_of::<DecisionGpu>())
            .map(bytemuck::pod_read_unaligned::<DecisionGpu>)
        {
            if d.evaluated > 1
                || d.selected_action > 5
                || d.invalid > 1
                || d.update_gates
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1.0).contains(v))
                || [d.amount, d.payload]
                    .iter()
                    .chain(&d.movement)
                    .chain(&d.force)
                    .chain(&d.placement)
                    .chain(&d.scores)
                    .chain(&d.hidden)
                    .chain(&d.inputs)
                    .chain(&d.candidate)
                    .chain(&d.outputs)
                    .any(|v| !v.is_finite())
            {
                return Err("Invalid checkpoint decision".into());
            }
        }
        let reservoir0: &[f32] = bytemuck::cast_slice(&data[13]);
        let reservoir1: &[f32] = bytemuck::cast_slice(&data[14]);
        let reservoir_traits: &[CognitiveTraits] = bytemuck::cast_slice(&data[15]);
        for slot in 0..HEREDITARY_RESERVOIR_SIZE as usize {
            let mut genome = [0.0; GENOME_SIZE];
            genome[..GENOME_BANK_STRIDE].copy_from_slice(
                &reservoir0[slot * GENOME_BANK_STRIDE..(slot + 1) * GENOME_BANK_STRIDE],
            );
            genome[GENOME_BANK_STRIDE..].copy_from_slice(
                &reservoir1[slot * GENOME_BANK_STRIDE
                    ..slot * GENOME_BANK_STRIDE + GENOME_SIZE - GENOME_BANK_STRIDE],
            );
            crate::brain::validate(&genome)?;
            if !reservoir_traits[slot].validate()
                || reservoir_traits[slot].padding[0] > MAX_WORLD_TICKS
                || reservoir_traits[slot].padding[1] != 0
            {
                return Err("Invalid hereditary reservoir traits".into());
            }
        }
        for cell in data[17].chunks_exact(16) {
            let pools: [f32; 4] = bytemuck::pod_read_unaligned(cell);
            if pools.iter().any(|x| !x.is_finite() || *x < 0.0) || pools[0] > 2.0 {
                return Err("Invalid ecological water or material pools".into());
            }
        }
        let counters: &[u32] = bytemuck::cast_slice(&data[4]);
        if counters[18] > tick || counters[30] > 1 {
            return Err("Invalid world completion counters".into());
        }
        if let Some(o) = &metadata.progress.completed {
            let alive = bytemuck::cast_slice::<u8, AgentGpu>(&data[0])
                .iter()
                .any(|a| a.alive != 0);
            if alive
                || o.duration != counters[18]
                || o.births != counters[3]
                || o.maximum_generation != counters[23]
                || o.food_ingested
                    != (u64::from(counters[0]) + (u64::from(counters[14]) << 32)) as f64 / 1000.0
                || o.assisted != (counters[30] != 0)
            {
                return Err("Completed outcome does not match physical checkpoint".into());
            }
        }
        // Reject trailing payloads before mutating live state.
        let mut trailing = [0u8; 1];
        if file.read(&mut trailing).map_err(|e| e.to_string())? != 0 {
            return Err("Trailing checkpoint data".into());
        }
        // Apply only after the entire checkpoint has been read and checked.
        if let Some((expected_seed, expected_tick, expected_living)) = expected {
            let living = data[0]
                .chunks_exact(std::mem::size_of::<AgentGpu>())
                .map(bytemuck::pod_read_unaligned::<AgentGpu>)
                .filter(|a| a.alive != 0)
                .count() as u32;
            if (seed, tick, living) != (expected_seed, expected_tick, expected_living) {
                return Err("The experiment receipt and checkpoint do not match".into());
            }
        }
        for (buffer, bytes) in buffers.iter().zip(&data) {
            queue.write_buffer(buffer, 0, bytes);
        }
        queue.write_buffer(&self.agent_buffers[1], 0, &data[0]);
        queue.write_buffer(&self.resource_display_buffer, 0, &data[1]);
        self.progress = metadata.progress;
        self.progress.engine_saturated |= counters[36] != 0 || tick >= MAX_WORLD_TICKS;
        self.settings = settings;
        self.seed = seed;
        self.tick = tick;
        self.environment_start_age = metadata.environment_start_age;
        self.assisted = counters[30] != 0 || metadata.assisted;
        self.current_buffer = 0;
        self.terrain_epoch = u32::MAX;
        self.update_params(queue);
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, serde::Serialize)]
pub struct InteractionEvent {
    pub tick: u32,
    pub actor: u32,
    pub other: u32,
    pub action: u32,
    pub amount: f32,
    pub sequence: u32,
    pub actor_lineage: u32,
    pub other_lineage: u32,
    pub position: [f32; 2],
    /// Diagnostic context for memory samples: carried food and local resource.
    pub context: [f32; 2],
    /// For memory samples, the action selected with memory retained.
    /// Other event kinds leave this zero.
    pub actual_action: u32,
    pub padding: u32,
}
impl Simulation {
    pub fn recent_events(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<InteractionEvent>, String> {
        let stats = read_buffer(device, queue, &self.death_stats_buffer)?;
        let total = bytemuck::cast_slice::<u8, u32>(&stats)[8];
        let data = read_buffer(device, queue, &self.event_buffer)?;
        let ring = bytemuck::cast_slice::<u8, InteractionEvent>(&data);
        let start = total.saturating_sub(crate::simulation::EVENT_RING_SIZE);
        Ok((start..total)
            .map(|n| ring[n as usize % crate::simulation::EVENT_RING_SIZE as usize])
            .collect())
    }
}
