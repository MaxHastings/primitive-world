use super::*;

#[test]
fn gru_contract_memory_and_checkpoint() {
    use crate::neural::{ACTIONS, HIDDEN, NeuralPolicy, NeuralState, NeuralWeights};
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 71);
    let mut flat = NeuralWeights::baseline().flat();
    for (i, w) in flat.iter_mut().enumerate() {
        *w = ((i as f32 * 0.713).sin()) * 0.12;
    }
    let weights = NeuralWeights::from_flat(&flat).unwrap();
    sim.set_neural_weights(&queue, &weights).unwrap();
    sim.neural_world(
        &queue, 4, 71, true, false, false, true, None, false, false, false,
    )
    .unwrap();
    let mut expected = vec![NeuralPolicy::default(); 4];
    for t in 0..3 {
        sim.neural_frame(&device, &queue, 4).unwrap();
        let traces = read::<NeuralState>(&device, &queue, &sim.neural_state_buffer, 4);
        for (i, st) in traces.iter().enumerate() {
            assert_eq!(st.tick, t * 8);
            assert_eq!(st.generation, 1);
            for h in 0..HIDDEN {
                assert!((st.before[h] - expected[i].hidden[h]).abs() < 2e-5);
            }
            let logits = expected[i].step(&weights, st.observation);
            for a in 0..ACTIONS {
                assert!((logits[a] - st.logits[a]).abs() < 2e-5);
                if st.mask[a] == 0. {
                    assert_eq!(st.probabilities[a], 0.);
                }
            }
            assert_eq!(st.mask[st.choice as usize], 1.);
            assert!((st.probabilities.iter().sum::<f32>() - 1.).abs() < 1e-5);
            for h in 0..HIDDEN {
                assert!((st.after[h] - expected[i].hidden[h]).abs() < 2e-5);
            }
        }
    }
    let path = std::env::temp_dir().join(format!("gru-replay-{}.checkpoint", std::process::id()));
    sim.save_checkpoint(&device, &queue, &path).unwrap();
    let next = sim.neural_frame(&device, &queue, 4).unwrap();
    sim.set_neural_weights(&queue, &NeuralWeights::baseline())
        .unwrap();
    sim.reset_neural_memory(&queue);
    sim.load_checkpoint(&queue, &path).unwrap();
    assert_eq!(
        next,
        sim.neural_frame(&device, &queue, 4).unwrap(),
        "checkpoint includes model, RNG and private state"
    );
    // A dying body must clear working memory without losing its final decision trace.
    let mut a = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
    a.energy = 0.001;
    a.food = 0.;
    write_agents(&sim, &queue, &[a]);
    sim.neural_frame(&device, &queue, 4).unwrap();
    assert!(
        sim.neural_inspect(&device, &queue, 0)
            .unwrap()
            .hidden
            .iter()
            .all(|h| *h == 0.)
    );
    // A reused identity starts immediately, then keeps its own eight-tick cadence.
    step(&mut sim, &device, &queue, 3);
    let born_at = sim.tick;
    a.alive = 1;
    a.energy = 60.;
    a.generation += 1;
    write_agents(&sim, &queue, &[a]);
    sim.neural_frame(&device, &queue, 4).unwrap();
    let st = sim.neural_inspect(&device, &queue, 0).unwrap();
    assert_eq!(st.generation, 2);
    assert_eq!(st.tick, born_at);
    assert!(st.before.iter().all(|h| *h == 0.));
    std::fs::remove_file(path).unwrap();
}

fn gpu() -> (wgpu::Device, wgpu::Queue) {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .expect("GPU required for simulation validation");
        eprintln!("GPU: {:?}", adapter.get_info());
        adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("simulation test device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: adapter.limits(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .unwrap()
    })
}

fn read<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    count: usize,
) -> Vec<T> {
    let size = (count * std::mem::size_of::<T>()) as u64;
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("test readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let result = bytemuck::cast_slice(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    result
}

fn step(sim: &mut Simulation, device: &wgpu::Device, queue: &wgpu::Queue, ticks: u32) {
    let mut remaining = ticks;
    while remaining > 0 {
        let count = remaining.min(32);
        let mut encoder = device.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut encoder, device, queue, count);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
        remaining -= count;
    }
}

fn dispatch(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::ComputePipeline,
    group: &wgpu::BindGroup,
    count: u32,
) {
    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.dispatch_workgroups(count.div_ceil(64), 1, 1);
    }
    queue.submit(Some(encoder.finish()));
    device.poll(wgpu::Maintain::Wait);
}

fn write_agents(sim: &Simulation, queue: &wgpu::Queue, agents: &[AgentGpu]) {
    for buffer in &sim.agent_buffers {
        queue.write_buffer(buffer, 0, bytemuck::cast_slice(agents));
    }
}

fn migration_episode(
    sim: &mut Simulation,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    destination: [f32; 2],
    ticks: u32,
) -> (Vec<bool>, u32) {
    // A controlled barren arena with explicitly supplied food. Procedural
    // capacity must not clip the test patch or change between map revisions.
    queue.write_buffer(
        &sim.ground_buffer,
        0,
        &vec![0u8; (RESOURCE_GRID * RESOURCE_GRID * 32) as usize],
    );
    let mut food = vec![0u32; (RESOURCE_GRID * RESOURCE_GRID) as usize];
    for y in 0..RESOURCE_GRID {
        for x in 0..RESOURCE_GRID {
            if (x as f32 * 4.0 + 2.0 - destination[0]).hypot(y as f32 * 4.0 + 2.0 - destination[1])
                < 16.0
            {
                food[(y * RESOURCE_GRID + x) as usize] = 1000;
            }
        }
    }
    queue.write_buffer(&sim.resource_buffer, 0, bytemuck::cast_slice(&food));
    let mut arrived = vec![false; 3];
    let mut contact = 0;
    for _ in 0..ticks {
        step(sim, device, queue, 1);
        let a = read::<AgentGpu>(device, queue, &sim.agent_buffers[sim.current_buffer], 3);
        for i in 0..3 {
            arrived[i] |= a[i].alive != 0
                && (a[i].position[0] - destination[0]).hypot(a[i].position[1] - destination[1])
                    < 16.0;
            if i > 0
                && a[i].alive != 0
                && a[0].alive != 0
                && (a[i].position[0] - a[0].position[0]).hypot(a[i].position[1] - a[0].position[1])
                    < 24.0
            {
                contact += 1;
            }
        }
    }
    (arrived, contact)
}

#[test]
fn raw_local_perception_does_not_create_a_shared_information_channel() {
    let (device, queue) = gpu();
    let mut report = Vec::new();
    for mode in [
        "full",
        "no_reports",
        "forget_experience",
        "no_companion_access",
    ] {
        let mut sim = Simulation::new(&device, &queue, 61);
        sim.settings.population = 0;
        sim.settings.resource_regeneration = 0.0;
        sim.settings.evolving_landscape = false;
        sim.settings.force_enabled = false;
        sim.settings.communication_enabled = mode != "no_reports";
        sim.reset(&queue);
        sim.tick = 100;
        let mut agents: Vec<_> = (0..3)
            .map(|i| AgentGpu {
                position: [100.0 + i as f32, 100.0],
                energy: 65.0,
                age: 400.0,
                max_speed: 1.2,
                sensor_radius: 24.0,
                alive: 1,
                generation: 1,
                rng: 7919 + i * 131,
                max_age: 10000.0,
                next_birth: 10000,
                target: MAX_AGENTS,
                event_actor: MAX_AGENTS,
                guide_id: MAX_AGENTS,
                ..Default::default()
            })
            .collect();
        agents[0].places[0] = PlaceGpu {
            position: [220.0, 100.0],
            food: 1.0,
            observed: 100,
            source_id: 0,
            source_generation: 1,
            confidence: 1.0,
            ..Default::default()
        };
        write_agents(&sim, &queue, &agents);
        let mut relations = build_social_memory();
        for i in 0..3 {
            for j in 0..3 {
                if i != j {
                    relations[i * 8 + j] = SocialRelationGpu {
                        target_slot: j as u32,
                        target_generation: 1,
                        familiarity: 0.8,
                        last_seen_tick: 100,
                        ..Default::default()
                    };
                }
            }
        }
        queue.write_buffer(
            &sim.social_memory_buffer,
            0,
            bytemuck::cast_slice(&relations),
        );
        let (first, first_contact) =
            migration_episode(&mut sim, &device, &queue, [220.0, 100.0], 240);
        let learned = read::<SocialRelationGpu>(&device, &queue, &sim.social_memory_buffer, 24);
        let guidance: Vec<_> = (1..3)
            .map(|i| {
                learned[i * 8..i * 8 + 8]
                    .iter()
                    .filter(|r| r.target_slot == 0)
                    .map(|r| r.navigation)
                    .fold(0.0f32, f32::max)
            })
            .collect();
        // A second departure from a gathered camp isolates experience. No reports
        // are available; only the guide has new food information. Physical state
        // is standardized while relationships retain the first episode's outcomes.
        for (i, a) in agents.iter_mut().enumerate() {
            a.position = [220.0 + i as f32, 100.0];
            a.goal = a.position;
            a.energy = 15.0;
            a.places = [PlaceGpu::default(); 4];
        }
        agents[0].places[0] = PlaceGpu {
            position: [340.0, 160.0],
            food: 1.0,
            observed: sim.tick,
            source_id: 0,
            source_generation: 1,
            confidence: 1.0,
            ..Default::default()
        };
        write_agents(&sim, &queue, &agents);
        sim.settings.communication_enabled = false;
        if mode == "no_companion_access" {
            sim.settings.social_access = 0.0;
        }
        if mode == "forget_experience" {
            let mut memory = learned.clone();
            for r in &mut memory {
                r.navigation = 0.0;
                r.navigation_evidence = 0.0;
                r.benefit = 0.0;
                r.benefit_evidence = 0.0;
            }
            queue.write_buffer(&sim.social_memory_buffer, 0, bytemuck::cast_slice(&memory));
        }
        let (second, second_contact) =
            migration_episode(&mut sim, &device, &queue, [340.0, 160.0], 600);
        let alive = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 3)
            .iter()
            .filter(|a| a.alive != 0)
            .count();
        let row = serde_json::json!({"mode":mode,"first_arrivals":first,"first_contact_ticks":first_contact,"learned_navigation":guidance,
            "second_arrivals":second,"second_contact_ticks":second_contact,"survivors":alive,"signals":sim.metrics(&device,&queue).unwrap().signals});
        eprintln!("Migration: {row}");
        report.push(row);
    }
    // A local signal is not a structured map update and must not become a
    // hidden population-level information channel.
    assert_eq!(report[0]["signals"].as_u64().unwrap(), 0);
    assert_eq!(
        report[0]["learned_navigation"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v.as_f64().unwrap() > 0.0)
            .count(),
        0
    );
}

#[test]
fn habitat_is_patchy_seeded_and_keeps_barren_gaps() {
    let habitat = build_habitat(1);
    assert_eq!(habitat, build_habitat(1));
    assert_ne!(habitat, build_habitat(2));
    let coverage = habitat.iter().filter(|h| **h > 0.0).count() as f32 / habitat.len() as f32;
    assert!(
        (0.08..0.4).contains(&coverage),
        "patch coverage: {coverage}"
    );
    let ground = build_ground(&habitat);
    let productivity = ground
        .iter()
        .map(|g| f32::from_bits(g[7]) as f64)
        .sum::<f64>()
        / ground.len() as f64;
    assert!(
        (productivity - 1.0).abs() < 0.001,
        "concentrate the growth budget into patches"
    );

    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 1);
    sim.settings.population = 0;
    sim.settings.evolving_landscape = false;
    sim.reset(&queue);
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        bytemuck::cast_slice(&vec![0u32; habitat.len()]),
    );
    step(&mut sim, &device, &queue, 640);
    let grown = read::<u32>(&device, &queue, &sim.resource_buffer, habitat.len());
    assert!(grown.iter().sum::<u32>() > 0);
    assert!(
        grown
            .iter()
            .zip(&habitat)
            .all(|(food, h)| *h > 0.0 || *food == 0),
        "rain and soil recovery must not fill the barren gaps"
    );
    let barren = habitat.iter().position(|h| *h == 0.0).unwrap();
    queue.write_buffer(
        &sim.resource_buffer,
        barren as u64 * 4,
        bytemuck::bytes_of(&333u32),
    );
    step(&mut sim, &device, &queue, 16);
    let painted = read::<u32>(&device, &queue, &sim.resource_buffer, barren + 1);
    assert_eq!(
        painted[barren], 333,
        "manually supplied food remains harvestable in barren space"
    );
    eprintln!(
        "Habitat coverage {:.1}%; normalized productivity {:.4}",
        coverage * 100.0,
        productivity
    );
}

#[test]
fn landscape_evolves_smoothly_and_crosses_keyframes_in_order() {
    let first = build_habitat_at(1, 0);
    let later = build_habitat_at(1, 3);
    assert!(
        first.iter().zip(&later).any(|(a, b)| *a > 0.2 && *b < 0.01),
        "old peaks can fade"
    );
    assert!(
        first.iter().zip(&later).any(|(a, b)| *a < 0.01 && *b > 0.2),
        "new peaks can appear elsewhere"
    );
    let pair = build_terrain_pair(1, 0);
    let next = build_terrain_pair(1, 1);
    assert!(
        pair.iter()
            .zip(&next)
            .all(|(a, b)| a[1] == b[0] && a[3] == b[2]),
        "keyframe endpoints must agree"
    );
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 1);
    sim.settings.population = 0;
    sim.reset(&queue);
    sim.tick = 4096;
    step(&mut sim, &device, &queue, 1);
    let midpoint = read::<u32>(&device, &queue, &sim.ground_buffer, pair.len() * 8);
    for (i, p) in pair.iter().enumerate() {
        assert!((f32::from_bits(midpoint[i * 8 + 6]) - (p[0] + p[1]) * 0.5).abs() < 0.00001);
    }
    sim.tick = 8190;
    let path = std::env::temp_dir().join(format!(
        "primitive-terrain-{}.checkpoint",
        std::process::id()
    ));
    sim.save_checkpoint(&device, &queue, &path).unwrap();
    step(&mut sim, &device, &queue, 4);
    let batched = read::<u32>(&device, &queue, &sim.ground_buffer, pair.len() * 8);
    sim.load_checkpoint(&queue, &path).unwrap();
    for _ in 0..4 {
        step(&mut sim, &device, &queue, 1);
    }
    let single = read::<u32>(&device, &queue, &sim.ground_buffer, pair.len() * 8);
    assert_eq!(
        batched, single,
        "terrain copies must occur at the correct tick inside a batch"
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn an_empty_world_search_reaches_its_chosen_destination() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 53);
    sim.settings.population = 0;
    sim.settings.resource_regeneration = 0.0;
    sim.reset(&queue);
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        bytemuck::cast_slice(&vec![0u32; (RESOURCE_GRID * RESOURCE_GRID) as usize]),
    );
    let a = AgentGpu {
        position: [1024.0, 1024.0],
        energy: 65.0,
        age: 400.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        rng: 7919,
        max_age: 10000.0,
        target: MAX_AGENTS,
        event_actor: MAX_AGENTS,
        guide_id: MAX_AGENTS,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a]);
    step(&mut sim, &device, &queue, 1);
    let first = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
    assert_eq!(first.action, 1);
    let mut reached = false;
    for _ in 0..48 {
        let now = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
        let distance = (now.position[0] - first.goal[0]).hypot(now.position[1] - first.goal[1]);
        if distance <= 2.0 {
            reached = true;
            break;
        }
        assert_eq!(
            now.goal, first.goal,
            "empty-space exploration should not choose a new heading before arriving"
        );
        step(&mut sim, &device, &queue, 1);
    }
    assert!(reached, "the selected search destination must be reached");
}

#[test]
#[ignore = "population trajectory diagnostic; run explicitly"]
fn motion_diagnostic() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 1);
    step(&mut sim, &device, &queue, 4000);
    let mut frames = Vec::new();
    for _ in 0..257 {
        frames.push(read::<AgentGpu>(
            &device,
            &queue,
            &sim.agent_buffers[sim.current_buffer],
            512,
        ));
        step(&mut sim, &device, &queue, 1);
    }
    let mut rows = Vec::new();
    for i in 0..512 {
        let first = frames[0][i];
        let last = frames[256][i];
        if first.alive == 0 || last.alive == 0 || first.generation != last.generation {
            continue;
        }
        let mut path = 0.0f32;
        let mut reversals = 0;
        let mut actions = [0; 7];
        for t in 1..frames.len() {
            let a = frames[t][i];
            let b = frames[t - 1][i];
            path += ((a.position[0] - b.position[0]).powi(2)
                + (a.position[1] - b.position[1]).powi(2))
            .sqrt();
            actions[a.action as usize] += 1;
            if a.velocity[0] * b.velocity[0] + a.velocity[1] * b.velocity[1] < -0.1 {
                reversals += 1;
            }
        }
        let distance = ((last.position[0] - first.position[0]).powi(2)
            + (last.position[1] - first.position[1]).powi(2))
        .sqrt();
        rows.push(serde_json::json!({"slot":i,"path":path,"distance":distance,"reversals":reversals,"actions":actions,"start_energy":first.energy,"end_energy":last.energy}));
    }
    rows.sort_by(|a, b| b["reversals"].as_u64().cmp(&a["reversals"].as_u64()));
    eprintln!(
        "Worst motion: {}",
        serde_json::to_string_pretty(&rows[..rows.len().min(5)]).unwrap()
    );
    if let Some(row) = rows.first() {
        let i = row["slot"].as_u64().unwrap() as usize;
        let reversal = (1..frames.len())
            .find(|&t| {
                let a = frames[t][i];
                let b = frames[t - 1][i];
                a.velocity[0] * b.velocity[0] + a.velocity[1] * b.velocity[1] < -0.1
            })
            .unwrap_or(0);
        for frame in frames.iter().skip(reversal.saturating_sub(3)).take(16) {
            let a = frame[i];
            eprintln!(
                "slot {i}: pos {:?} goal {:?} action {} food {:.3} energy {:.3} commit {}",
                a.position, a.goal, a.action, a.food, a.energy, a.commit_until
            );
        }
    }
}

#[test]
fn physical_actions_and_births_conserve_reserves() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 7);
    sim.settings.population = 0;
    sim.settings.metabolic_cost = 0.0;
    sim.settings.movement_energy_cost = 0.0;
    sim.settings.resource_regeneration = 0.0;
    sim.reset(&queue);
    sim.update_params(&queue);
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        energy: 50.0,
        max_speed: 1.0,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a]);
    let ci = 25 * 512 + 25;
    let before = read::<u32>(&device, &queue, &sim.resource_buffer, ci + 1)[ci];
    let d = DecisionGpu {
        selected_action: 2,
        target: MAX_AGENTS,
        ..Default::default()
    };
    queue.write_buffer(&sim._decision_buffer, 0, bytemuck::bytes_of(&d));
    dispatch(
        &device,
        &queue,
        &sim.consume_pipeline,
        &sim.consume_bind_groups[0],
        1,
    );
    dispatch(
        &device,
        &queue,
        &sim.update_pipeline,
        &sim.update_bind_groups[0][1],
        1,
    );
    let harvested = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[1], 1)[0];
    let after = read::<u32>(&device, &queue, &sim.resource_buffer, ci + 1)[ci];
    assert!((harvested.food * 1000.0 - (before - after) as f32).abs() < 0.001);
    assert_eq!(harvested.position, a.position, "harvest must stop movement");
    a.food = 1.0;
    write_agents(&sim, &queue, &[a]);
    queue.write_buffer(
        &sim._decision_buffer,
        0,
        bytemuck::bytes_of(&DecisionGpu {
            selected_action: 3,
            ..d
        }),
    );
    queue.write_buffer(&sim._request_buffer, 0, bytemuck::bytes_of(&0u32));
    dispatch(
        &device,
        &queue,
        &sim.update_pipeline,
        &sim.update_bind_groups[0][1],
        1,
    );
    let eaten = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[1], 1)[0];
    assert!((eaten.energy + eaten.food * 8.0 - (a.energy + a.food * 8.0)).abs() < 0.0001);
    a.alive = 0;
    a.food = 1.25;
    write_agents(&sim, &queue, &[a]);
    dispatch(
        &device,
        &queue,
        &sim.death_pipeline,
        &sim.death_bind_groups[0],
        MAX_AGENTS,
    );
    let ground = read::<u32>(&device, &queue, &sim.ground_buffer, (ci + 1) * 8);
    assert_eq!(ground[ci * 8], 1250);
    assert_eq!(
        ground.iter().step_by(8).sum::<u32>(),
        1250,
        "unused slots must not create supplies"
    );
    dispatch(
        &device,
        &queue,
        &sim.death_pipeline,
        &sim.death_bind_groups[0],
        MAX_AGENTS,
    );
    assert_eq!(
        read::<u32>(&device, &queue, &sim.ground_buffer, (ci + 1) * 8)[ci * 8],
        1250,
        "release once"
    );
    // Isolate the birth allocator: exactly one mature parent and one free child slot.
    a.alive = 1;
    a.energy = 90.0;
    a.food = 4.0;
    a.age = 500.0;
    a.genome = [0.4; crate::simulation::GENOME_SIZE];
    write_agents(&sim, &queue, &[a, AgentGpu::default()]);
    let old_memory = crate::neural::NeuralState {
        generation: 99,
        valid: 1,
        hidden: [0.75; crate::neural::HIDDEN],
        ..Default::default()
    };
    queue.write_buffer(
        &sim.neural_state_buffer,
        std::mem::size_of_val(&old_memory) as u64,
        bytemuck::bytes_of(&old_memory),
    );
    queue.write_buffer(&sim.birth_parents, 0, bytemuck::bytes_of(&0u32));
    queue.write_buffer(&sim.free_indices, 0, bytemuck::bytes_of(&1u32));
    for buffer in [&sim.birth_prefix[1], &sim.free_prefix[1]] {
        queue.write_buffer(
            buffer,
            (MAX_AGENTS as u64 - 1) * 4,
            bytemuck::bytes_of(&1u32),
        );
    }
    // Both prefix scans have 17 steps, so birth groups bind the final index 1.
    dispatch(
        &device,
        &queue,
        &sim.birth_pipeline,
        &sim.birth_bind_groups[0],
        1,
    );
    let pair = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 2);
    assert_eq!(pair[1].alive, 1);
    let child_memory = sim.neural_inspect(&device, &queue, 1).unwrap();
    assert_eq!(child_memory.valid, 0);
    assert!(child_memory.hidden.iter().all(|h| *h == 0.));
    assert_eq!(pair[0].food + pair[1].food, a.food);
    assert!(
        (pair[0].energy + pair[1].energy - (a.energy - sim.settings.reproduction_cost * 0.2)).abs()
            < 0.0001
    );
    assert_eq!(pair[0].next_birth, sim.settings.birth_cooldown);
    assert_eq!(pair[1].parent_lineage, pair[0].lineage_id);
    assert_ne!(pair[1].lineage_id, pair[0].lineage_id);
    assert_eq!(pair[1].birth_parent_slot, 0);
    assert!(
        pair[1]
            .genome
            .iter()
            .all(|gene| *gene >= -1.0 && *gene <= 1.0)
    );
    assert_ne!(pair[0].genome, pair[1].genome);
}

#[test]
fn famine_survivors_recover_and_fractional_growth_accumulates() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 13);
    sim.settings.population = 0;
    sim.settings.resource_regeneration = 0.01;
    sim.reset(&queue);
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        &vec![0; (RESOURCE_GRID * RESOURCE_GRID * 4) as usize],
    );
    step(&mut sim, &device, &queue, 32);
    let growth = read::<u32>(
        &device,
        &queue,
        &sim.resource_buffer,
        (RESOURCE_GRID * RESOURCE_GRID) as usize,
    );
    assert!(
        growth.iter().sum::<u32>() > 0,
        "sub-unit growth must accumulate"
    );
    let ground = read::<u32>(
        &device,
        &queue,
        &sim.ground_buffer,
        (RESOURCE_GRID * RESOURCE_GRID * 8) as usize,
    );
    assert_eq!(
        ground.iter().step_by(8).sum::<u32>(),
        0,
        "no spontaneous food from unused agents"
    );
    // Start famine with low-reserve and high-reserve adults; no food and no regeneration.
    sim.settings.resource_regeneration = 0.0;
    sim.settings.maturity_age = 80.0;
    sim.settings.birth_cooldown = 80;
    sim.settings.force_enabled = false;
    sim.reset(&queue);
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        &vec![0; (RESOURCE_GRID * RESOURCE_GRID * 4) as usize],
    );
    let mut agents = Vec::new();
    for i in 0..16 {
        agents.push(AgentGpu {
            position: [200.0 + i as f32 * 40.0, 500.0],
            energy: if i < 8 { 0.1 } else { 65.0 },
            age: 200.0,
            max_speed: 1.2,
            sensor_radius: 24.0,
            alive: 1,
            generation: 1,
            max_age: 10000.0,
            rng: i * 7919 + 1,
            target: MAX_AGENTS,
            event_actor: MAX_AGENTS,
            ..Default::default()
        });
    }
    write_agents(&sim, &queue, &agents);
    step(&mut sim, &device, &queue, 16);
    let famine = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 16);
    assert_eq!(famine.iter().filter(|a| a.alive != 0).count(), 8);
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        bytemuck::cast_slice(&vec![1000u32; (RESOURCE_GRID * RESOURCE_GRID) as usize]),
    );
    sim.settings.resource_regeneration = 1.0;
    step(&mut sim, &device, &queue, 1200);
    let recovered = read::<AgentGpu>(
        &device,
        &queue,
        &sim.agent_buffers[sim.current_buffer],
        MAX_AGENTS as usize,
    );
    let living = recovered.iter().filter(|a| a.alive != 0).count();
    eprintln!("Famine recovery: 16 -> 8 -> {living}");
    assert!(
        living > 8,
        "survivors must be able to reproduce after restored food"
    );
    assert!(
        recovered
            .iter()
            .filter(|a| a.alive != 0)
            .all(|a| a.energy.is_finite() && a.food >= 0.0 && a.food <= 8.001)
    );
}

#[test]
fn attainable_food_and_better_destinations_drive_decisions() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 41);
    sim.settings.population = 0;
    sim.settings.exploration_noise = 0.0;
    sim.reset(&queue);
    sim.update_params(&queue);
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        energy: 50.0,
        food: 0.005,
        max_speed: 1.2,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        target: MAX_AGENTS,
        event_actor: MAX_AGENTS,
        guide_id: MAX_AGENTS,
        ..Default::default()
    };
    let p = PerceptionGpu {
        resource_here: 0.025,
        projected_food: 0.025,
        ..Default::default()
    };
    queue.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&p));
    write_agents(&sim, &queue, &[a]);
    dispatch(
        &device,
        &queue,
        &sim.decision_pipeline,
        &sim.decision_bind_groups[0],
        1,
    );
    assert_eq!(
        read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0].selected_action,
        2,
        "collect a useful harvest rather than spend a tick eating a crumb"
    );

    a.food = 0.0;
    a.energy = 70.0;
    a.action = 1;
    a.commit_until = 24;
    a.goal = [76.0, 100.0];
    let p = PerceptionGpu {
        resource_east: 1.0,
        ..Default::default()
    };
    queue.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&p));
    write_agents(&sim, &queue, &[a]);
    dispatch(
        &device,
        &queue,
        &sim.decision_pipeline,
        &sim.decision_bind_groups[0],
        1,
    );
    let d = read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0];
    assert_eq!(d.selected_action, 1);
    assert_eq!(
        d.goal,
        [124.0, 100.0],
        "a better observed patch must override the old trip"
    );
}

#[test]
fn a_trip_survives_small_preference_changes_and_an_eating_pause() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 47);
    sim.settings.population = 0;
    sim.settings.exploration_noise = 0.0;
    sim.reset(&queue);
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        energy: 10.0,
        age: 400.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        target: MAX_AGENTS,
        event_actor: MAX_AGENTS,
        guide_id: MAX_AGENTS,
        ..Default::default()
    };
    for tick in 0..16 {
        sim.tick = tick;
        sim.update_params(&queue);
        if tick == 4 {
            a.food = 0.1;
        }
        let p = PerceptionGpu {
            resource_east: if tick % 2 == 0 { 0.7 } else { 0.69 },
            resource_west: if tick % 2 == 0 { 0.69 } else { 0.7 },
            ..Default::default()
        };
        queue.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&p));
        write_agents(&sim, &queue, &[a]);
        dispatch(
            &device,
            &queue,
            &sim.decision_pipeline,
            &sim.decision_bind_groups[0],
            1,
        );
        let decision = read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0];
        assert_eq!(decision.selected_action, if tick == 4 { 3 } else { 1 });
        dispatch(
            &device,
            &queue,
            &sim.update_pipeline,
            &sim.update_bind_groups[0][1],
            1,
        );
        let next = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[1], 1)[0];
        assert_eq!(
            next.goal,
            [124.0, 100.0],
            "tick {tick}: do not chase a slightly better sample in the opposite direction"
        );
        assert_eq!(
            next.commit_until, 40,
            "pausing must not extend commitment indefinitely"
        );
        assert!(next.position[0] >= a.position[0]);
        a = next;
    }
    assert!(a.position[0] > 117.0, "movement must make actual progress");

    // Visible danger still permits abandoning the committed destination.
    a.places = [PlaceGpu::default(); 4];
    write_agents(&sim, &queue, &[a]);
    let p = PerceptionGpu {
        resource_west: 0.7,
        ..Default::default()
    };
    queue.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&p));
    let social = SocialPerceptionGpu {
        danger: 1.0,
        give_target: MAX_AGENTS,
        force_target: MAX_AGENTS,
        report_target: MAX_AGENTS,
        ..Default::default()
    };
    queue.write_buffer(
        &sim._social_perception_buffer,
        0,
        bytemuck::bytes_of(&social),
    );
    dispatch(
        &device,
        &queue,
        &sim.decision_pipeline,
        &sim.decision_bind_groups[0],
        1,
    );
    let decision = read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0];
    assert!(decision.goal[0] < a.position[0]);
}

#[test]
fn surplus_can_produce_a_physical_transfer() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 43);
    sim.settings.population = 0;
    sim.settings.exploration_noise = 0.0;
    sim.reset(&queue);
    let donor = AgentGpu {
        position: [100.0, 100.0],
        energy: 90.0,
        food: 4.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        next_birth: 10000,
        target: MAX_AGENTS,
        event_actor: MAX_AGENTS,
        guide_id: MAX_AGENTS,
        ..Default::default()
    };
    let recipient = AgentGpu {
        position: [102.0, 100.0],
        food: 0.0,
        energy: 40.0,
        ..donor
    };
    write_agents(&sim, &queue, &[donor, recipient]);
    let relation = SocialRelationGpu {
        target_slot: 1,
        target_generation: 1,
        familiarity: 1.0,
        ..Default::default()
    };
    queue.write_buffer(&sim.social_memory_buffer, 0, bytemuck::bytes_of(&relation));
    step(&mut sim, &device, &queue, 1);
    let result = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 2);
    assert_eq!(
        result[0].action, 4,
        "the normal decision pipeline should choose the transfer"
    );
    assert!((result[0].food - 3.5).abs() < 0.001);
    assert!(result[1].food >= 0.5);
    assert_eq!(sim.metrics(&device, &queue).unwrap().events[4], 1);
}

#[test]
fn place_memory_guides_travel_and_urgent_eating_interrupts_it() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 17);
    sim.settings.population = 0;
    sim.settings.resource_regeneration = 0.0;
    sim.reset(&queue);
    sim.update_params(&queue);
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        energy: 70.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        rng: 1,
        ..Default::default()
    };
    a.places[0] = PlaceGpu {
        position: [160.0, 100.0],
        food: 1.0,
        observed: 0,
        confidence: 1.0,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a]);
    dispatch(
        &device,
        &queue,
        &sim.decision_pipeline,
        &sim.decision_bind_groups[0],
        1,
    );
    let decision = read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0];
    assert_eq!(decision.selected_action, 1);
    assert_eq!(decision.goal, [160.0, 100.0]);
    a.energy = 0.04;
    a.food = 0.5;
    a.action = 1;
    a.goal = [160.0, 100.0];
    a.commit_until = 24;
    write_agents(&sim, &queue, &[a]);
    dispatch(
        &device,
        &queue,
        &sim.decision_pipeline,
        &sim.decision_bind_groups[0],
        1,
    );
    assert_eq!(
        read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0].selected_action,
        3
    );
}

#[test]
fn completed_transfers_conserve_matter_and_remote_people_are_not_tracked() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 19);
    sim.settings.population = 0;
    sim.settings.social_access = 0.0;
    sim.settings.social_concern = 0.0;
    sim.settings.reciprocity = 0.0;
    sim.reset(&queue);
    sim.update_params(&queue);
    let mut donor = AgentGpu {
        position: [100.0, 100.0],
        energy: 80.0,
        food: 4.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        sensor_radius: 24.0,
        ..Default::default()
    };
    let recipient = AgentGpu {
        position: [104.0, 100.0],
        food: 0.0,
        ..donor
    };
    write_agents(&sim, &queue, &[donor, recipient]);
    let ds = [
        DecisionGpu {
            selected_action: 4,
            target: 1,
            amount: 0.5,
            ..Default::default()
        },
        DecisionGpu {
            target: MAX_AGENTS,
            ..Default::default()
        },
    ];
    queue.write_buffer(&sim._decision_buffer, 0, bytemuck::cast_slice(&ds));
    for pipeline in &sim.interaction_pipelines {
        dispatch(
            &device,
            &queue,
            pipeline,
            &sim.interaction_bind_groups[0],
            2,
        );
    }
    let after = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 2);
    assert_eq!(after[0].food + after[1].food, 4.0);
    assert_eq!(after[1].food, 0.5);
    sim.tick = 1;
    sim.update_params(&queue);
    dispatch(
        &device,
        &queue,
        &sim.social_pipeline,
        &sim.social_bind_groups[0],
        2,
    );
    // The transfer is a physical consequence, not a reputation update.
    assert!(
        read::<SocialRelationGpu>(&device, &queue, &sim.social_memory_buffer, 16)
            .iter()
            .all(|r| r.target_slot >= MAX_AGENTS)
    );
    // Remote bodies do not enter the local candidate set.
    donor = after[0];
    donor.position = [1000.0, 1000.0];
    queue.write_buffer(&sim.agent_buffers[0], 0, bytemuck::bytes_of(&donor));
    sim.tick = 2;
    sim.update_params(&queue);
    dispatch(
        &device,
        &queue,
        &sim.social_pipeline,
        &sim.social_bind_groups[0],
        2,
    );
    let social = read::<SocialPerceptionGpu>(&device, &queue, &sim._social_perception_buffer, 2);
    assert_eq!(social[1].avoidance, [0.0, 0.0]);
    assert_eq!(
        social[1].companion_value, 0.0,
        "remote helpers must not influence movement"
    );
    assert!(
        read::<SocialRelationGpu>(&device, &queue, &sim.social_memory_buffer, 16)
            .iter()
            .all(|r| r.target_slot >= MAX_AGENTS)
    );
}

#[test]
fn emitted_signal_is_local_and_does_not_create_a_shared_map() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 21);
    sim.settings.population = 0;
    sim.settings.communication_enabled = true;
    sim.reset(&queue);
    sim.tick = 100;
    sim.update_params(&queue);
    let a = AgentGpu {
        position: [100.0, 100.0],
        energy: 80.0,
        alive: 1,
        generation: 4,
        max_age: 10000.0,
        sensor_radius: 24.0,
        rng: 1,
        ..Default::default()
    };
    let b = AgentGpu {
        position: [103.0, 100.0],
        generation: 2,
        ..a
    };
    write_agents(&sim, &queue, &[a, b]);
    let d = DecisionGpu {
        selected_action: 6,
        target: 1,
        amount: 0.75,
        ..Default::default()
    };
    queue.write_buffer(
        &sim._decision_buffer,
        0,
        bytemuck::cast_slice(&[
            d,
            DecisionGpu {
                target: MAX_AGENTS,
                ..Default::default()
            },
        ]),
    );
    for pipeline in &sim.interaction_pipelines {
        dispatch(
            &device,
            &queue,
            pipeline,
            &sim.interaction_bind_groups[0],
            2,
        );
    }
    let before = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 2);
    for pipeline in &sim.interaction_pipelines {
        dispatch(
            &device,
            &queue,
            pipeline,
            &sim.interaction_bind_groups[0],
            2,
        );
    }
    let after = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 2);
    assert_eq!(after[1].event_actor, 0);
    assert!((after[1].event_amount - 0.75).abs() < 0.001);
    assert_eq!(after[1].places, before[1].places);
}

#[test]
fn apply_force_spills_matter_without_direct_transfer() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 22);
    sim.settings.population = 0;
    sim.settings.force_enabled = true;
    sim.reset(&queue);
    let a = AgentGpu {
        position: [100.0, 100.0],
        energy: 100.0,
        food: 0.0,
        alive: 1,
        generation: 1,
        sensor_radius: 24.0,
        ..Default::default()
    };
    let b = AgentGpu {
        position: [102.0, 100.0],
        energy: 0.01,
        food: 2.0,
        alive: 1,
        generation: 1,
        sensor_radius: 24.0,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a, b]);
    queue.write_buffer(
        &sim._decision_buffer,
        0,
        bytemuck::bytes_of(&DecisionGpu {
            selected_action: 5,
            target: 1,
            amount: 1.0,
            ..Default::default()
        }),
    );
    for pipeline in &sim.interaction_pipelines {
        dispatch(
            &device,
            &queue,
            pipeline,
            &sim.interaction_bind_groups[0],
            2,
        );
    }
    let pair = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 2);
    let ci = 25 * 512 + 25;
    let dropped =
        read::<u32>(&device, &queue, &sim.ground_buffer, (ci + 1) * 8)[ci * 8] as f32 / 1000.0;
    assert_eq!(pair[0].food, 0.0);
    assert!((pair[0].food + pair[1].food + dropped - 2.0).abs() < 0.001);
    assert!(pair[1].event_amount < 0.0);
    assert!(pair[0].energy < 100.0);
}

#[test]
fn generic_force_and_contended_transfers_conserve_matter() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 23);
    sim.settings.population = 0;
    sim.settings.force_enabled = true;
    sim.reset(&queue);
    let a = AgentGpu {
        position: [100.0, 100.0],
        energy: 90.0,
        food: 4.0,
        alive: 1,
        generation: 1,
        max_age: 10000.0,
        sensor_radius: 24.0,
        ..Default::default()
    };
    // Two donors contend over the same recipient; accepted pairs may not overlap.
    let agents = [a, a, AgentGpu { food: 0.0, ..a }];
    write_agents(&sim, &queue, &agents);
    let ds = [
        DecisionGpu {
            selected_action: 4,
            target: 2,
            amount: 0.5,
            ..Default::default()
        },
        DecisionGpu {
            selected_action: 4,
            target: 2,
            amount: 0.5,
            ..Default::default()
        },
        DecisionGpu {
            target: MAX_AGENTS,
            ..Default::default()
        },
    ];
    queue.write_buffer(&sim._decision_buffer, 0, bytemuck::cast_slice(&ds));
    for pipeline in &sim.interaction_pipelines {
        dispatch(
            &device,
            &queue,
            pipeline,
            &sim.interaction_bind_groups[0],
            3,
        );
    }
    let pair = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 3);
    assert_eq!(pair.iter().map(|a| a.food).sum::<f32>(), 8.0);
    assert_eq!(
        pair[2].food, 0.5,
        "exactly one disjoint transfer may resolve"
    );
}

#[test]
fn gpu_pipeline_and_clock_validation() {
    assert_eq!(std::mem::size_of::<crate::neural::NeuralState>(), 672);
    assert_eq!(std::mem::size_of::<AgentGpu>(), 304);
    assert_eq!(std::mem::size_of::<SocialRelationGpu>(), 48);
    assert_eq!(std::mem::size_of::<SocialPerceptionGpu>(), 640);
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 9);
    // Validate render shaders/layouts as well as every compute pipeline, without a window.
    let _renderer = crate::renderer::Renderer::new(
        &device,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        &sim,
        800,
        600,
    );
    sim.settings.population = 1;
    sim.reset(&queue);
    step(&mut sim, &device, &queue, 8);
    let batched = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1);
    let resources = read::<u32>(
        &device,
        &queue,
        &sim.resource_buffer,
        (RESOURCE_GRID * RESOURCE_GRID) as usize,
    );
    sim.reset(&queue);
    for _ in 0..8 {
        step(&mut sim, &device, &queue, 1);
    }
    let single = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1);
    assert_eq!(
        bytemuck::bytes_of(&batched[0]),
        bytemuck::bytes_of(&single[0])
    );
    assert_eq!(
        resources,
        read::<u32>(&device, &queue, &sim.resource_buffer, resources.len())
    );
    assert_eq!(sim.tick, 8);
    assert!(single[0].energy.is_finite());
    assert!((0.0..=8.0).contains(&single[0].food));
    let selected = sim
        .select_agent(&device, &queue, single[0].position, 10.0)
        .expect("selection works with new layouts");
    assert_eq!(selected.agent.generation, single[0].generation);
    let metrics = sim.metrics(&device, &queue).unwrap();
    assert_eq!(metrics.living, 1);
    let evolution = sim.evolution_snapshot(&device, &queue).unwrap();
    assert_eq!(evolution.living, 1);
    assert_eq!(evolution.unique_lineages, 1);
    assert!(evolution.mean_copy_fidelity >= 0.0 && evolution.mean_copy_fidelity <= 1.0);
    let checkpoint = std::env::temp_dir().join(format!(
        "primitive-world-{}-test.checkpoint",
        std::process::id()
    ));
    sim.save_checkpoint(&device, &queue, &checkpoint).unwrap();
    step(&mut sim, &device, &queue, 4);
    let expected = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1);
    sim.load_checkpoint(&queue, &checkpoint).unwrap();
    step(&mut sim, &device, &queue, 4);
    assert_eq!(
        bytemuck::bytes_of(&expected[0]),
        bytemuck::bytes_of(
            &read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0]
        )
    );
    let header = std::fs::read(&checkpoint).unwrap();
    assert_eq!(&header[..12], b"PRIMWORLD010");
    std::fs::remove_file(checkpoint).unwrap();
}
