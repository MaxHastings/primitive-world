#[test]
fn signed_controller_weights_can_choose_or_reject_force_and_read_events() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 91);
    sim.settings.population = 0;
    sim.settings.exploration_noise = 0.0;
    sim.reset(&queue);
    sim.update_params(&queue);
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        energy: 55.0,
        food: 2.0,
        age: 500.0,
        alive: 1,
        generation: 1,
        max_speed: 1.2,
        sensor_radius: 24.0,
        max_age: 10000.0,
        genome: bootstrap_genome(),
        event_actor: MAX_AGENTS,
        ..Default::default()
    };
    let p = PerceptionGpu {
        resource_here: 0.8,
        projected_food: 0.8,
        ..Default::default()
    };
    let mut s = SocialPerceptionGpu::default();
    s.candidates[0] = SocialCandidateGpu {
        target_slot: 1,
        target_generation: 1,
        distance: 1.0,
        food: 8.0,
        event_actor: MAX_AGENTS,
        ..Default::default()
    };
    queue.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&p));
    queue.write_buffer(&sim._social_perception_buffer, 0, bytemuck::bytes_of(&s));
    let decide = |a: AgentGpu| {
        write_agents(&sim, &queue, &[a]);
        dispatch(
            &device,
            &queue,
            &sim.decision_pipeline,
            &sim.decision_bind_groups[0],
            1,
        );
        read::<DecisionGpu>(&device, &queue, &sim._decision_buffer, 1)[0]
    };
    assert_ne!(
        decide(a).selected_action,
        5,
        "rich neighbors do not force an attack through an authored score"
    );
    a.genome[96] = 2.0;
    assert_eq!(
        decide(a).selected_action,
        5,
        "force remains reachable through inherited weights"
    );
    a.genome[96] = -2.0;
    assert_ne!(
        decide(a).selected_action,
        5,
        "signed weights can reverse the preference"
    );
    a.genome = bootstrap_genome();
    a.genome[16 + 11] = 2.0;
    a.event_actor = 1;
    a.event_tick = 0;
    a.event_amount = 0.75;
    assert_eq!(
        decide(a).selected_action,
        0,
        "received raw signal can causally change an intent"
    );
    a.event_amount = 0.0;
    assert_ne!(decide(a).selected_action, 0);
    let original = decide(a);
    a.ancestry_depth = 900;
    a.lineage_id = 98765;
    a.lifetime_births = 1000;
    a.distance_travelled = 1_000_000.0;
    assert_eq!(
        bytemuck::bytes_of(&original),
        bytemuck::bytes_of(&decide(a)),
        "observer measurements cannot influence intents"
    );
}

#[test]
fn remembered_coordinates_do_not_drift_and_candidate_checkpoint_replays() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 92);
    sim.settings.population = 1;
    sim.reset(&queue);
    sim.tick = 100;
    let mut a = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 1)[0];
    a.position = [100.0, 100.0];
    a.goal = [300.0, 100.0];
    a.energy = 70.0;
    a.age = 500.0;
    a.places = [PlaceGpu::default(); PLACE_SLOTS];
    a.places[0] = PlaceGpu {
        position: [95.0, 100.0],
        food: 0.9,
        observed: 90,
        confidence: 1.0,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a]);
    step(&mut sim, &device, &queue, 8);
    let now = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
    assert_eq!(now.places[0].position, [95.0, 100.0]);
    let path = std::env::temp_dir().join(format!(
        "candidate-replay-{}.checkpoint",
        std::process::id()
    ));
    sim.save_checkpoint(&device, &queue, &path).unwrap();
    step(&mut sim, &device, &queue, 16);
    let expected = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
    sim.load_checkpoint(&queue, &path).unwrap();
    step(&mut sim, &device, &queue, 16);
    let actual = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    std::fs::remove_file(path).unwrap();
    let metrics = sim.metrics(&device, &queue).unwrap();
    assert_eq!(metrics.action_ticks.iter().sum::<u32>(), 24);
}

#[test]
fn founder_contract_rejects_invalid_weights() {
    use crate::founders::validate_genomes;
    assert!(validate_genomes(&[bootstrap_genome().to_vec()]).is_ok());
    assert!(validate_genomes(&[vec![0.0; 8]]).is_err());
    let mut g = bootstrap_genome().to_vec();
    g[96] = f32::NAN;
    assert!(validate_genomes(&[g]).is_err());
    let bank = crate::founders::bundled();
    assert_eq!(bank.source_seed, 22);
    assert_eq!(bank.source_tick, 30000);
    assert_eq!(bank.genomes.len(), 128);
    let settings = SimSettings::default();
    assert_eq!(settings.founder_genomes, bank.genomes);
    assert_eq!(settings.founder_name, bank.name);
}

#[test]
fn founder_export_load_and_bootstrap_are_explicit() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 95);
    sim.settings.population = 1;
    sim.reset(&queue);
    let mut a = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 1)[0];
    assert!(sim
        .settings
        .founder_genomes
        .iter()
        .any(|g| g.as_slice() == a.genome));
    let path = std::env::temp_dir().join(format!("founder-contract-{}.json", std::process::id()));
    assert!(
        sim.export_founders(&device, &queue, &path).is_err(),
        "founders alone are not descendant preparation"
    );
    a.ancestry_depth = 1;
    write_agents(&sim, &queue, &[a]);
    sim.export_founders(&device, &queue, &path).unwrap();
    sim.load_founders(&path).unwrap();
    sim.reset(&queue);
    let child = read::<AgentGpu>(&device, &queue, &sim.agent_buffers[0], 1)[0];
    assert_eq!(child.genome, a.genome);
    assert_eq!(
        child.ancestry_depth, 0,
        "a reseeded world starts its own ancestry accounting"
    );
    sim.use_bootstrap_founders();
    assert!(sim.settings.founder_genomes.is_empty());
    assert_eq!(sim.settings.founder_name, "candidate-v1-bootstrap");
    std::fs::remove_file(path).unwrap();
}

#[test]
fn candidate_follows_a_known_distant_patch_across_empty_space() {
    let (device, queue) = gpu();
    let mut sim = Simulation::new(&device, &queue, 94);
    sim.settings.population = 0;
    sim.settings.resource_regeneration = 0.0;
    sim.settings.evolving_landscape = false;
    sim.settings.exploration_noise = 0.0;
    sim.reset(&queue);
    sim.tick = 100;
    queue.write_buffer(
        &sim.resource_buffer,
        0,
        bytemuck::cast_slice(&vec![0u32; (RESOURCE_GRID * RESOURCE_GRID) as usize]),
    );
    queue.write_buffer(
        &sim.ground_buffer,
        0,
        &vec![0u8; (RESOURCE_GRID * RESOURCE_GRID * 32) as usize],
    );
    let mut a = AgentGpu {
        position: [100.0, 100.0],
        goal: [100.0, 100.0],
        energy: 75.0,
        food: 2.0,
        age: 500.0,
        alive: 1,
        generation: 1,
        max_speed: 1.2,
        sensor_radius: 24.0,
        max_age: 10000.0,
        genome: bootstrap_genome(),
        event_actor: MAX_AGENTS,
        ..Default::default()
    };
    a.places[0] = PlaceGpu {
        position: [400.0, 100.0],
        food: 1.0,
        observed: 100,
        confidence: 1.0,
        ..Default::default()
    };
    write_agents(&sim, &queue, &[a]);
    let mut arrived = false;
    for _ in 0..40 {
        step(&mut sim, &device, &queue, 8);
        let current =
            read::<AgentGpu>(&device, &queue, &sim.agent_buffers[sim.current_buffer], 1)[0];
        if (current.position[0] - 400.0).hypot(current.position[1] - 100.0) < 24.0 {
            arrived = true;
            break;
        }
    }
    assert!(
        arrived,
        "a remembered patch beyond sensory range must be reachable through private navigation"
    );
}
