use super::*;

#[test]
fn journey_observation_does_not_modify_physical_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 32);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    s.reset(&q);
    put(&s, &q, 0, a, &g);
    let mut observer = crate::journey_observer::JourneyObserver::default();
    for _ in 0..4 {
        step(&mut s, &d, &q, 8);
        observer
            .observe(
                s.tick,
                &s.agent_snapshot(&d, &q).unwrap(),
                &s.vegetation_snapshot(&d, &q).unwrap(),
            )
            .unwrap();
    }
    let actual = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&actual)
    );
}

/// Diagnostic of the released bank, not a required behavior for future models.
/// Paired gradients cancel unconditional drift; only the food field changes.
#[test]
fn released_bank_sensor_frame_diagnostic() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let bank = crate::founders::bundled();
    let mut summary = Vec::new();
    for angle in [
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -std::f32::consts::FRAC_PI_2,
    ] {
        for (slot, genome) in bank.genomes.iter().enumerate() {
            let mut a = body([1026.0, 1026.0]);
            a.energy = 50.0;
            a.food = 1.0;
            a.attention = angle;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, genome.as_slice().try_into().unwrap());
        }
        let mut paired = Vec::new();
        for sign in [1.0f32, -1.0] {
            let resources: Vec<u32> = (0..512 * 512)
                .map(|i| {
                    let x = ((i % 512) as f32 + 0.5) * 4.0;
                    ((0.4 + sign * (x - 1026.0) * 0.01).clamp(0.0, 1.0) * 1000.0).round() as u32
                })
                .collect();
            q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&resources));
            let mut e = d.create_command_encoder(&Default::default());
            s.dispatch(&mut e, "perceive", s.current_buffer, 2, 1);
            s.dispatch(&mut e, "decide", s.current_buffer, 2, 1);
            q.submit(Some(e.finish()));
            let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, bank.genomes.len());
            assert!(decisions.iter().all(|v| v.invalid == 0));
            // The sensory offsets really rotate; the cue is not a fixed slot.
            let east_slot = [0usize, 1, 2, 3]
                .into_iter()
                .max_by(|&a, &b| {
                    decisions[0].inputs[17 + 3 * a].total_cmp(&decisions[0].inputs[17 + 3 * b])
                })
                .unwrap();
            assert!(decisions[0].inputs[17 + 3 * east_slot] > 0.16);
            paired.push(decisions);
        }
        let deltas: Vec<[f32; 2]> = paired[0]
            .iter()
            .zip(&paired[1])
            .map(|(east, west)| {
                [
                    (east.movement[0] - west.movement[0]) * 0.5,
                    (east.movement[1] - west.movement[1]) * 0.5,
                ]
            })
            .collect();
        let mean = |axis: usize| deltas.iter().map(|v| v[axis]).sum::<f32>() / deltas.len() as f32;
        let toward = deltas.iter().filter(|v| v[0] > 0.0).count();
        println!(
            "attention={angle:.6} paired_food_response=({:.6},{:.6}) toward_east={toward}/{}",
            mean(0),
            mean(1),
            deltas.len()
        );
        summary.push((mean(0), toward));
    }
    // Reproduce the suspected failure in the frozen historical bank on GPU.
    assert!(summary[0].0 > 0.02 && summary[0].1 == bank.genomes.len());
    assert!(summary[2].0 < -0.02 && summary[2].1 == 0);
}

#[test]
fn reproduction_rechecks_reserves_after_force() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    let amount = 1.0 / (1.0 + (-3.0f32).exp());
    parent.energy = 50.0 * (0.2 + 0.8 * amount) + 0.2;
    put(&s, &q, 0, parent, &fixed(6, [0.0; 2]));
    put(&s, &q, 1, body([604.0, 902.0]), &fixed(4, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.birth_gates[5], 1);
    assert_eq!(m.birth_gates[6], 0);
    assert_eq!(m.events[5], 1);
}
#[test]
fn dead_slot_reuse_resets_experience_and_advances_incarnation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut dead = body([500.0, 500.0]);
    dead.generation = 8;
    dead.hidden = [0.9; 16];
    put(&s, &q, 0, dead, &fixed(0, [0.0; 2]));
    s.kill_agents_in_region(&d, &q, [500.0, 500.0], 2.0);
    put(&s, &q, 1, body([602.0, 902.0]), &fixed(6, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    assert_eq!(bodies[0].generation, 9);
    assert_eq!(bodies[0].hidden, [0.0; 16]);
    assert_eq!(bodies[0].ancestry_depth, 1);
    assert_eq!(bodies[0].event_actor, MAX_AGENTS);
    near(s.metrics(&d, &q).unwrap().dropped_food as f32, 2.0);
}
#[test]
fn fresh_world_defaults_keep_original_energy_costs() {
    let settings = SimSettings::default();
    assert_eq!(settings.metabolic_cost, 0.06);
    assert_eq!(settings.movement_energy_cost, 0.01);
    assert_eq!(settings.motor_response_gain, 4.0);
    assert_eq!(settings.resource_regeneration, 0.01);
    assert_eq!(settings.population, 1000);
    assert!(settings.evolving_landscape);
    settings.validate().unwrap();
}

#[test]
fn legacy_settings_keep_historical_motor_response_and_reject_bad_gains() {
    let mut value = serde_json::to_value(SimSettings::default()).unwrap();
    value.as_object_mut().unwrap().remove("motor_response_gain");
    let settings: SimSettings = serde_json::from_value(value).unwrap();
    assert_eq!(settings.motor_response_gain, 1.0);
    for gain in [0.0, -1.0, 33.0, f32::NAN, f32::INFINITY] {
        let settings = SimSettings {
            motor_response_gain: gain,
            ..settings.clone()
        };
        assert!(settings.validate().is_err());
    }
}

#[test]
fn motor_response_is_continuous_optional_reversible_and_bounded() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.motor_response_gain = 8.0;
    for effort in [0.0f32, 0.01, -0.01, 4.0] {
        let mut g = fixed(0, [0.0; 2]);
        g[1296 + 7 * 17 + 16] = effort;
        put(&s, &q, 0, body([602.0, 902.0]), &g);
        step(&mut s, &d, &q, 1);
        let a = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
        near(a.velocity[0], (effort * 8.0).tanh() * 1.2);
        near(a.velocity[1], 0.0);
        assert!(a.velocity[0].abs() <= 1.2001);
        near(a.spent, 0.06 + a.velocity[0].abs() * 0.01);
    }
}

#[test]
fn physical_cli_overrides_validate_and_cannot_override_checkpoints() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let args: Vec<String> = [
        "world",
        "--motor-gain",
        "8",
        "--metabolic-cost",
        "0.05",
        "--movement-cost",
        "0.02",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    crate::headless::configure(&mut s, &args).unwrap();
    assert_eq!(s.settings.motor_response_gain, 8.0);
    assert_eq!(s.settings.metabolic_cost, 0.05);
    assert_eq!(s.settings.movement_energy_cost, 0.02);
    for flag in ["--motor-gain", "--metabolic-cost", "--movement-cost"] {
        let args: Vec<String> = ["world", "--checkpoint", "unused.checkpoint", flag, "1"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(
            crate::headless::configure(&mut s, &args)
                .unwrap_err()
                .contains("Checkpoint restores settings")
        );
    }
    let args: Vec<String> = ["world", "--motor-gain", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    assert!(crate::headless::configure(&mut s, &args).is_err());
}

#[test]
fn prepared_default_is_the_declared_bank_not_a_silent_fallback() {
    let settings = SimSettings::default();
    let bank = crate::founders::bundled();
    assert_eq!(bank.genomes.len(), 128);
    assert_eq!(settings.founder_genomes, bank.genomes);
    assert_eq!(settings.founder_name, bank.name);
    assert!(bank.name.contains("descendants"));
}

#[test]
fn concurrent_collection_and_pair_resolution_do_not_double_spend() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let pos = [602.0, 902.0];
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&17u32));
    for i in 0..8 {
        let mut a = body(pos);
        a.food = 0.0;
        a.lineage_id = i as u32 + 1;
        put(&s, &q, i, a, &fixed(1, [0.0; 2]));
    }
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    near((m.carried_food + m.vegetation) as f32, 0.017);
    assert_eq!(m.living, 8);
    // Every body requests transfer to the same target; accepted pairs must be disjoint.
    for i in 0..8 {
        let mut a = body(pos);
        a.lineage_id = i as u32 + 1;
        a.food = if i == 7 { 0.0 } else { 1.0 };
        put(&s, &q, i, a, &fixed(0, [0.0; 2]));
    }
    let mut decisions = vec![DecisionGpu::default(); 8];
    for item in &mut decisions[..7] {
        item.selected_action = 3;
        item.target = 7;
        item.target_generation = 1;
        item.amount = 1.0;
    }
    q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(&decisions));
    let mut e = d.create_command_encoder(&Default::default());
    for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
        s.dispatch(&mut e, pass, s.current_buffer, MAX_AGENTS / 64, 1);
    }
    q.submit(Some(e.finish()));
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 8);
    near(bodies.iter().map(|a| a.food).sum(), 7.0);
    near(bodies[7].food, 1.0);
}
#[test]
fn stale_targets_out_of_range_and_disabled_actions_cannot_claim() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.force_enabled = false;
    s.update_params(&q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    let run = |s: &Simulation, decisions: &[DecisionGpu]| {
        q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(decisions));
        let mut e = d.create_command_encoder(&Default::default());
        for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
            s.dispatch(&mut e, pass, 0, MAX_AGENTS / 64, 1);
        }
        q.submit(Some(e.finish()));
        read::<AgentGpu>(&d, &q, &s.agent_buffers[0], 2)
    };
    let mut intent = DecisionGpu {
        selected_action: 3,
        target: 1,
        target_generation: 2,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent])[1].food, 0.0);
    intent.target_generation = 1;
    b.position = [620.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    near(run(&s, &[intent])[1].food, 0.0);
    b.position = [604.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    // Disabled force from receiver must not defeat a valid transfer in arbitration.
    let force = DecisionGpu {
        selected_action: 4,
        target: 0,
        target_generation: 1,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent, force])[1].food, 1.0);
}
#[test]
fn attention_amount_and_failed_actions_remain_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(3, [0.2, 0.0]); // no target exists
    g[OUTPUT_BASE + 9 * 17 + 16] = 1.0;
    g[OUTPUT_BASE + 10 * 17 + 16] = -2.0;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let intent = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(b.action, 3);
    near(b.food, a.food);
    assert!(b.position[0] > a.position[0]);
    near(b.attention, 1.0f32.tanh() * 0.25);
    near(intent.amount, 1.0 / (1.0 + 2.0f32.exp()));
}
#[test]
fn founder_export_requires_descendants_and_preserves_existing_files() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    let path = temp("founders.json");
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert!(!path.exists());
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(6, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    s.export_founders(&d, &q, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    s.load_founders(&path).unwrap();
    assert_eq!(s.settings.founder_genomes.len(), 1);
    assert_eq!(s.settings.founder_genomes[0].len(), GENOME_SIZE);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn checkpoint_rejects_corrupt_trace_without_mutating_live_world() {
    use std::io::{Seek, SeekFrom, Write};
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let path = temp("corrupt.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    assert!(s.save_checkpoint(&d, &q, &path).is_err());
    let lengths = [
        s.agent_buffers[0].size(),
        s.resource_buffer.size(),
        s.fertility_buffer.size(),
        s.ground_buffer.size(),
        s.death_stats_buffer.size(),
        s.event_buffer.size(),
    ];
    let pos = 24
        + serde_json::to_vec(&s.settings).unwrap().len() as u64
        + lengths.iter().map(|v| v + 8).sum::<u64>()
        + 8;
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(pos)).unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    drop(file);
    let before = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert!(
        s.load_checkpoint(&q, &path)
            .unwrap_err()
            .contains("perception")
    );
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&before),
        bytemuck::cast_slice::<AgentGpu, u8>(&after)
    );
    std::fs::remove_file(path).unwrap();
}
fn gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("GPU");
    pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("recurrent tests"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap()
}
fn read<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    b: &wgpu::Buffer,
    count: usize,
) -> Vec<T> {
    let staging = readback(device, (count * std::mem::size_of::<T>()) as u64);
    let mut e = device.create_command_encoder(&Default::default());
    e.copy_buffer_to_buffer(b, 0, &staging, 0, staging.size());
    queue.submit(Some(e.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let out = bytemuck::cast_slice(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    out
}
fn step(s: &mut Simulation, d: &wgpu::Device, q: &wgpu::Queue, n: u32) {
    let mut e = d.create_command_encoder(&Default::default());
    s.encode_ticks(&mut e, d, q, n);
    q.submit(Some(e.finish()));
    d.poll(wgpu::Maintain::Wait);
}
fn scene(d: &wgpu::Device, q: &wgpu::Queue) -> Simulation {
    let mut s = Simulation::new(d, q, 91);
    s.settings.population = 0;
    s.settings.resource_regeneration = 0.0;
    s.settings.evolving_landscape = false;
    s.reset(q);
    q.write_buffer(&s.resource_buffer, 0, &vec![0; 512 * 512 * 4]);
    q.write_buffer(&s.ground_buffer, 0, &vec![0; 512 * 512 * 32]);
    s
}
fn body(pos: [f32; 2]) -> AgentGpu {
    AgentGpu {
        position: pos,
        energy: 80.0,
        food: 2.0,
        age: 500.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        max_age: 11000.0,
        alive: 1,
        generation: 1,
        event_actor: MAX_AGENTS,
        target: MAX_AGENTS,
        lineage_id: 1,
        ..Default::default()
    }
}
fn fixed(action: usize, motion: [f32; 2]) -> [f32; GENOME_SIZE] {
    let mut g = [0.0; GENOME_SIZE];
    g[OUTPUT_BASE + action * 17 + 16] = 2.0;
    g[OUTPUT_BASE + 7 * 17 + 16] = motion[0];
    g[OUTPUT_BASE + 8 * 17 + 16] = motion[1];
    g[OUTPUT_BASE + 10 * 17 + 16] = 3.0;
    g
}
fn put(s: &Simulation, q: &wgpu::Queue, slot: usize, a: AgentGpu, g: &[f32; GENOME_SIZE]) {
    for b in &s.agent_buffers {
        q.write_buffer(
            b,
            (slot * std::mem::size_of::<AgentGpu>()) as u64,
            bytemuck::bytes_of(&a),
        );
    }
    q.write_buffer(
        &s.genome_buffer,
        (slot * GENOME_SIZE * 4) as u64,
        bytemuck::cast_slice(g),
    );
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.002, "{a} != {b}");
}
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("recurrent-{}-{name}", std::process::id()))
}

#[test]
fn layout_and_cli_contract() {
    assert_eq!(GENOME_SIZE, 1568);
    assert_eq!(std::mem::size_of::<AgentGpu>(), 208);
    assert_eq!(std::mem::size_of::<PerceptionGpu>(), 272);
    assert_eq!(std::mem::size_of::<DecisionGpu>(), 384);
    assert_eq!(std::mem::size_of::<SelectionOutput>(), 872);
    assert_eq!(std::mem::size_of::<SimParams>(), 96);
    assert!(MAX_AGENTS as usize * GENOME_SIZE * 4 <= 128 * 1024 * 1024);
    for flag in ["--neural", "--legacy-controller", "--travel-diagnostic"] {
        assert!(crate::headless::arguments(&["world".into(), flag.into()]).is_err());
    }
    let settings = SimSettings {
        sensor_radius: f32::NAN,
        ..Default::default()
    };
    assert!(settings.validate().is_err());
    assert!(crate::founders::validate_genomes(&[vec![0.0; 128]]).is_err());
}
#[test]
fn recurrent_cpu_gpu_parity_and_observer_isolation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.update_params(&q);
    let mut a = body([602.0, 902.0]);
    a.hidden = [0.1; 16];
    let mut g = [0.0; GENOME_SIZE];
    for (i, x) in g.iter_mut().enumerate() {
        *x = ((i % 17) as f32 - 8.0) * 0.015;
    }
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let expected = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    let mut hidden = [0.0; 16];
    for (h, value) in hidden.iter_mut().enumerate() {
        let row = h * 81;
        let mut v = g[row + 80];
        for k in 0..64 {
            v += g[row + k] * expected.inputs[k];
        }
        for k in 0..16 {
            v += g[row + 64 + k] * a.hidden[k];
        }
        *value = v.tanh();
        near(*value, expected.hidden[h]);
    }
    for o in 0..7 {
        let row = 1296 + o * 17;
        let mut v = g[row + 16];
        for h in 0..16 {
            v += g[row + h] * hidden[h];
        }
        near(v, expected.scores[o]);
    }
    a.lineage_id = 123456;
    a.ancestry_depth = 100;
    a.lifetime_births = 1000;
    a.distance_travelled = 30000.0;
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let actual = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(bytemuck::bytes_of(&actual), bytemuck::bytes_of(&expected));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_ne!(after.hidden, a.hidden);
    assert_eq!(read::<f32>(&d, &q, &s.genome_buffer, GENOME_SIZE), g);
}
#[test]
fn perception_is_local_and_attention_uses_true_coordinates() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.attention = std::f32::consts::FRAC_PI_2;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let mut food = vec![0u32; 512 * 512];
    food[225 * 512 + 151] = 700;
    q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
    let mut far = body([1000.0, 1000.0]);
    far.lineage_id = 2;
    put(&s, &q, 1, far, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let p = read::<PerceptionGpu>(&d, &q, &s.perception_buffer, 1)[0];
    near(p.samples[0].offset[0], 4.0);
    near(p.samples[0].offset[1], 0.0);
    near(p.samples[0].food, 0.7);
    assert!(
        p.samples
            .iter()
            .all(|p| p.offset[0].hypot(p.offset[1]) <= 24.001)
    );
    assert!(p.bodies.iter().all(|b| b.slot == MAX_AGENTS));
    let renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8UnormSrgb, &s, 800, 600);
    drop(renderer);
}
#[test]
fn physical_collection_ingestion_and_movement_conserve_reserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.food = 0.0;
    a.energy = 50.0;
    put(&s, &q, 0, a, &fixed(1, [0.5, 0.0]));
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&1000u32));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
    near(food[idx as usize] as f32 / 1000.0 + after.food, 1.0);
    near(after.food, after.collected);
    near(after.energy + after.spent, 50.0);
    assert!(after.velocity[0] > 0.0);
    assert!(after.collected > 0.0);
    let mut a = after;
    a.energy = 50.0;
    a.food = 1.0;
    put(&s, &q, 0, a, &fixed(2, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    near(after.energy + after.food * 8.0 + after.spent, 58.0);
    assert!(after.ingested > 0.0);
}
#[test]
fn reproduction_is_requested_can_coexist_with_motion_and_conserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    a.hidden = [0.5; 16];
    let g = fixed(6, [0.5, 0.0]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let p = agents[0];
    let c = agents[1];
    assert_eq!(c.alive, 1);
    assert_eq!(c.ancestry_depth, 1);
    assert_eq!(c.parent_lineage, p.lineage_id);
    assert_eq!(c.hidden, [0.0; 16]);
    assert!(p.velocity[0] > 0.0);
    near(p.food + c.food, a.food);
    near(
        p.energy
            + c.energy
            + s.settings.metabolic_cost
            + p.velocity[0].abs() * s.settings.movement_energy_cost
            + 10.0,
        90.0,
    );
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    let genes = read::<f32>(&d, &q, &s.genome_buffer, 2 * GENOME_SIZE);
    assert_eq!(&genes[..GENOME_SIZE], &g);
    assert!(genes.iter().all(|x| x.is_finite() && x.abs() <= 4.0));
}
#[test]
fn abundant_reserves_do_not_trigger_automatic_birth() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 100.0;
    a.food = 8.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 32);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
}
#[test]
fn transfer_and_signal_are_local_and_payload_is_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(3, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(bodies[0].food + bodies[1].food, 2.0);
    assert!(bodies[1].received > 0.0);
    let mut g = fixed(5, [0.0; 2]);
    g[1296 + 11 * 17 + 16] = -0.7;
    let mut a = bodies[0];
    a.last_communication = 0;
    s.tick = 10;
    put(&s, &q, 0, a, &g);
    put(&s, &q, 1, bodies[1], &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(bodies[1].event_amount, (-0.7f32).tanh());
    assert_eq!(bodies[1].event_actor, 0);
}
#[test]
fn force_spills_instead_of_directly_stealing_and_costs_energy() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    let mut b = body([604.0, 902.0]);
    b.energy = 10.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(4, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let m = s.metrics(&d, &q).unwrap();
    near(agents[0].food, a.food);
    near((m.carried_food + m.dropped_food) as f32, 4.0);
    assert_eq!(m.events[5], 1);
    near(
        (m.energy + m.force_energy_spent) as f32,
        100.0 - 2.0 * s.settings.metabolic_cost,
    );
}
#[test]
fn nonfinite_controller_output_is_contained() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(1, [1.0, 1.0]);
    g[0] = f32::NAN;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(after.action, 0);
    assert_eq!(after.position, a.position);
    assert!(after.hidden.iter().all(|x| x.is_finite()));
    assert_eq!(s.metrics(&d, &q).unwrap().invalid_outputs, 1);
}
#[test]
fn batching_checkpoint_and_selection_preserve_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // A compatible checkpoint retains historical costs, not new defaults.
    s.settings.metabolic_cost = 0.06;
    s.settings.movement_energy_cost = 0.01;
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 12);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.reset(&q);
    put(&s, &q, 0, a, &g);
    for _ in 0..12 {
        step(&mut s, &d, &q, 1);
    }
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    let selection = s.select_agent(&d, &q, actual.position, 2.0).unwrap();
    assert_eq!(selection.selected, 1);
    let path = temp("state.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 8);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.settings.metabolic_cost = 0.005;
    s.settings.movement_energy_cost = 0.002;
    let saved_motor_gain = s.settings.motor_response_gain;
    s.settings.motor_response_gain = 16.0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.settings.metabolic_cost, 0.06);
    assert_eq!(s.settings.movement_energy_cost, 0.01);
    assert_eq!(s.settings.motor_response_gain, saved_motor_gain);
    step(&mut s, &d, &q, 8);
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    std::fs::remove_file(path).unwrap();
    let path = temp("old.checkpoint");
    std::fs::write(&path, b"PRIMWORLD011").unwrap();
    let tick = s.tick;
    assert!(s.load_checkpoint(&q, &path).is_err());
    assert_eq!(s.tick, tick);
    assert!(path.exists());
    std::fs::remove_file(path).unwrap();
}
