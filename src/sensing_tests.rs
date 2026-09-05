//! Sensory/retention wiring checks, not evidence of evolved competence.
use super::*;

fn sense(s: &Simulation, d: &wgpu::Device, q: &wgpu::Queue) -> (PerceptionGpu, DecisionGpu) {
    s.update_params(q);
    let mut e = d.create_command_encoder(&Default::default());
    let groups = MAX_AGENTS.div_ceil(64);
    s.dispatch(&mut e, "clear", 0, 32, 32);
    s.dispatch(&mut e, "count", s.current_buffer, groups, 1);
    s.dispatch(&mut e, "spatial_init", 0, 1024, 1);
    for n in 0..16 {
        s.dispatch(&mut e, &format!("spatial_{n}"), n % 2, 1024, 1);
    }
    s.dispatch(&mut e, "cursors", 0, 1024, 1);
    s.dispatch(&mut e, "scatter", s.current_buffer, groups, 1);
    s.dispatch(&mut e, "perceive", s.current_buffer, groups, 1);
    s.dispatch(&mut e, "decide", s.current_buffer, groups, 1);
    q.submit(Some(e.finish()));
    (
        read::<PerceptionGpu>(d, q, &s.perception_buffer, 1)[0],
        read::<DecisionGpu>(d, q, &s.decision_buffer, 1)[0],
    )
}

fn sector(dx: f32, dy: f32) -> usize {
    if dx == 0.0 && dy == 0.0 {
        return 0;
    }
    ((dy.atan2(dx) + std::f32::consts::TAU + std::f32::consts::FRAC_PI_8)
        / std::f32::consts::FRAC_PI_4)
        .floor() as usize
        % SECTORS
}

#[test]
fn regional_food_matches_full_grid_reference_at_edges_and_all_radii() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let food: Vec<u32> = (0..512 * 512).map(|i| (i * 17 % 1000) as u32).collect();
    q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
    for (position, radius) in [
        ([602.0, 902.0], 24.0),
        ([601.3, 901.7], 4.0),
        ([0.0, 0.0], 48.0),
        ([2048.0, 2048.0], 24.0),
    ] {
        let mut a = body(position);
        a.sensor_radius = radius;
        put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
        let (p, decision) = sense(&s, &d, &q);
        let mut sums = [0.0; REGIONS];
        let mut counts = [0u32; REGIONS];
        for y in 0..512 {
            for x in 0..512 {
                let dx = x as f32 * 4.0 + 2.0 - position[0];
                let dy = y as f32 * 4.0 + 2.0 - position[1];
                let distance2 = dx * dx + dy * dy;
                if distance2 > radius * radius {
                    continue;
                }
                let region = sector(dx, dy)
                    + if distance2 > radius * radius * 0.25 {
                        8
                    } else {
                        0
                    };
                sums[region] += food[y * 512 + x] as f32 / 1000.0;
                counts[region] += 1;
            }
        }
        for k in 0..REGIONS {
            near(p.regions[k].food, sums[k] / counts[k].max(1) as f32);
            near(decision.inputs[20 + k * 2], p.regions[k].food);
            assert_eq!(p.regions[k].bodies, 0.0);
        }
        assert_eq!(p.nearby_count, 0.0);
        assert_eq!(decision.invalid, 0);
    }
}

#[test]
fn diagonal_and_between_probe_food_is_visible_without_distant_leakage() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    // SE diagonal near food, E food between old 4/24 probes, and remote food.
    for (x, y) in [(151, 226), (154, 225), (250, 250)] {
        q.write_buffer(
            &s.resource_buffer,
            ((y * 512 + x) * 4) as u64,
            bytemuck::bytes_of(&1000u32),
        );
    }
    let (p, _) = sense(&s, &d, &q);
    assert!(p.regions[1].food > 0.0);
    assert!(p.regions[8].food > 0.0);
    assert!(
        p.regions
            .iter()
            .enumerate()
            .all(|(i, r)| i == 1 || i == 8 || r.food == 0.0)
    );
    assert_eq!(p.resource_here, 0.0);
}

#[test]
fn sector_neighbors_count_crowds_choose_nearest_and_do_not_shuffle_or_expose_inventory() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let origin = [602.0, 902.0];
    let mut rng = 918u32;
    let genes = random_genome(&mut rng);
    let mut a = body(origin);
    put(&s, &q, 0, a, &genes);
    let directions = [
        [4.0, 0.0],
        [4.0, 4.0],
        [0.0, 4.0],
        [-4.0, 4.0],
        [-4.0, 0.0],
        [-4.0, -4.0],
        [0.0, -4.0],
        [4.0, -4.0],
    ];
    for (k, delta) in directions.iter().enumerate() {
        let mut b = body([origin[0] + delta[0], origin[1] + delta[1]]);
        b.generation = k as u32 + 10;
        put(&s, &q, k + 1, b, &genes);
    }
    // Many candidates in one spatial cell; nearest occurs at a high slot.
    for slot in 9..73 {
        put(&s, &q, slot, body([607.0, 902.0]), &genes);
    }
    put(&s, &q, 73, body([603.0, 902.0]), &genes);
    put(&s, &q, 74, body([603.0, 902.0]), &genes); // exact tie -> 73
    put(&s, &q, 75, body([626.0, 902.0]), &genes); // radius equality
    put(&s, &q, 76, body([626.01, 902.0]), &genes); // outside
    let (reference, decision) = sense(&s, &d, &q);
    assert_eq!(reference.nearby_count, 75.0);
    assert_eq!(
        reference.regions.iter().map(|r| r.bodies).sum::<f32>(),
        75.0
    );
    assert_eq!(reference.regions[0].bodies, 67.0);
    assert_eq!(reference.regions[8].bodies, 1.0);
    assert_eq!(reference.bodies[0].slot, 73);
    for k in 1..8 {
        assert_eq!(reference.bodies[k].slot, k as u32 + 1);
    }
    for tick in [1, 19, 128] {
        s.tick = tick;
        a.rng = tick * 1234;
        put(&s, &q, 0, a, &genes);
        // Change private inventory/energy, keeping externally observable facts fixed.
        let mut b = body([603.0, 902.0]);
        b.food = 7.0;
        b.energy = 1.0;
        put(&s, &q, 73, b, &genes);
        let (p, actual) = sense(&s, &d, &q);
        assert_eq!(bytemuck::bytes_of(&p), bytemuck::bytes_of(&reference));
        assert_eq!(actual.inputs, decision.inputs);
        assert_eq!(actual.hidden, decision.hidden);
    }
}

#[test]
fn each_sector_logit_targets_the_observed_incarnation_and_contact_uses_it() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let origin = [602.0, 902.0];
    for k in 0..8 {
        let theta = k as f32 * std::f32::consts::FRAC_PI_4;
        let mut b = body([origin[0] + 4.0 * theta.cos(), origin[1] + 4.0 * theta.sin()]);
        b.generation = k + 11;
        b.lineage_id = k + 2;
        put(&s, &q, k as usize + 1, b, &fixed(0, [0.0; 2]));
    }
    for k in 0..8 {
        let mut genes = fixed(3, [0.0; 2]);
        genes[OUTPUT_BASE + (10 + k) * 17 + 16] = 4.0;
        put(&s, &q, 0, body(origin), &genes);
        let (p, decision) = sense(&s, &d, &q);
        assert_eq!(decision.target, p.bodies[k].slot);
        assert_eq!(decision.target, k as u32 + 1);
        assert_eq!(decision.target_generation, k as u32 + 11);
    }
    let before = read::<AgentGpu>(&d, &q, &s.agent_buffers[0], 9);
    step(&mut s, &d, &q, 1); // last selected sector = NE
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 9);
    for k in 1..8 {
        assert_eq!(before[k].position, after[k].position);
    }
    assert!(after[8].position[0] > before[8].position[0]);
}

#[test]
fn evolved_gates_can_store_a_cue_retain_through_distraction_and_replace_it() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let mut genes = fixed(0, [0.0; 2]);
    genes[20] = 1.0; // candidate 0 reads regional food
    genes[RECURRENT_ROW + 2] = 1.0; // candidate 1 reads underfoot cue
    genes[GATE_BASE + 1] = 4.0; // gate 0 opens when cue is present
    genes[OUTPUT_BASE + 6 * 17] = 1.0; // retained value can influence action
    put(&s, &q, 0, body([602.0, 902.0]), &genes);
    s.update_params(&q);
    let mut p = PerceptionGpu::default();
    for b in &mut p.bodies {
        b.slot = MAX_AGENTS;
    }
    p.resource_here = 1.0;
    p.regions[0].food = 1.0;
    let run = |p: &PerceptionGpu, n: usize| {
        q.write_buffer(&s.perception_buffer, 0, bytemuck::bytes_of(p));
        let mut e = d.create_command_encoder(&Default::default());
        for _ in 0..n {
            s.dispatch(&mut e, "decide", 0, 1, 1);
            e.copy_buffer_to_buffer(
                &s.decision_buffer,
                std::mem::offset_of!(DecisionGpu, hidden) as u64,
                &s.agent_buffers[0],
                std::mem::offset_of!(AgentGpu, hidden) as u64,
                (HIDDEN * 4) as u64,
            );
        }
        q.submit(Some(e.finish()));
        read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0]
    };
    let stored = run(&p, 1);
    near(stored.hidden[0], 1.0f32.tanh());
    assert_eq!(stored.update_gates[0], 1.0);
    p.resource_here = 0.0;
    p.regions[0].food = 7.0; // different sensory input must not erase memory
    let held = run(&p, 1024);
    assert_eq!(held.hidden[0], stored.hidden[0]);
    assert_eq!(held.update_gates[0], 0.0);
    assert_eq!(held.movement, stored.movement);
    p.resource_here = 1.0;
    p.regions[0].food = 0.0;
    let replaced = run(&p, 1);
    assert_eq!(replaced.hidden[0], 0.0);
    assert_eq!(replaced.update_gates[0], 1.0);
    assert_eq!(read::<f32>(&d, &q, &s.genome_buffer, GENOME_SIZE), genes);
}

#[test]
fn previous_model_checkpoint_is_rejected_before_any_world_change() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let err = s
        .load_checkpoint_reader(&q, std::io::Cursor::new(b"PRIMWORLD016"))
        .unwrap_err();
    assert!(err.contains("format 17"));
    assert_eq!(s.tick, 0);
    assert_eq!(
        bytemuck::bytes_of(&read::<AgentGpu>(&d, &q, &s.agent_buffers[0], 1)[0]),
        bytemuck::bytes_of(&a)
    );
}

#[test]
#[ignore = "manual throughput measurement; wall time depends on GPU load"]
fn sensory_rewrite_throughput_probe() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for (population, crowded) in [(1000, false), (MAX_AGENTS, false), (512, true)] {
        s.settings.population = population;
        s.settings.founder_genomes = vec![fixed(0, [0.0; 2]).to_vec()];
        s.reset(&q);
        if crowded {
            let mut bodies = build_agents(s.seed, &s.settings);
            for a in bodies.iter_mut().filter(|a| a.alive != 0) {
                a.position = [1024.0, 1024.0];
            }
            for b in &s.agent_buffers {
                q.write_buffer(b, 0, bytemuck::cast_slice(&bodies));
            }
        }
        step(&mut s, &d, &q, 1);
        let start = std::time::Instant::now();
        step(&mut s, &d, &q, 32);
        eprintln!(
            "V5 population={population} coincident={crowded}: {:.3} ms/tick (32 ticks, full simulation, no rendering)",
            start.elapsed().as_secs_f64() * 1000.0 / 32.0
        );
        assert_eq!(s.metrics(&d, &q).unwrap().invalid_outputs, 0);
    }
}
