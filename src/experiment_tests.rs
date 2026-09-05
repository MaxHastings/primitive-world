use super::{body, fixed, gpu, put, read, scene, step};
use crate::{experiments, simulation::*, survivor_observer, visible_trial::VisibleTrial};

#[test]
fn experiment_preserves_memory_archive_and_world_number_across_resume() {
    let (d, q) = gpu();
    let mut sim = Simulation::new(&d, &q, 1);
    sim.settings.population = 4;
    sim.settings.metabolic_cost = 0.08;
    sim.reset(&q);
    step(&mut sim, &d, &q, 1);
    assert_eq!(sim.metrics(&d, &q).unwrap().living, 4);
    let original = sim.agent_snapshot(&d, &q).unwrap();
    let mut latest = None;
    survivor_observer::observe(&mut latest, &sim, &d, &q).unwrap();
    let snapshot = crate::visible_trial::LoopSnapshot {
        world_number: 7,
        latest: latest.unwrap(),
    };
    let root = std::env::temp_dir().join(format!(
        "primitive-resume-test-{}",
        experiments::stamp().unwrap()
    ));
    let directory = root.join("experiment");
    std::fs::create_dir_all(&directory).unwrap();
    let experiment = experiments::Experiment {
        directory,
        name: "A continuing line".into(),
        origin: "Random V4 fixture".into(),
        total_ticks: 8000,
    };
    experiment
        .save(&sim, &d, &q, Some(snapshot.clone()))
        .unwrap();
    let (saved, invalid) = experiments::list(&root).unwrap();
    assert_eq!(invalid, 0);
    assert_eq!(saved[0].world_number(), 7);
    assert_eq!(saved[0].record.total_ticks, 8000);
    step(&mut sim, &d, &q, 8);
    sim.load_checkpoint(&q, &saved[0].checkpoint()).unwrap();
    let restored = sim.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&original),
        bytemuck::cast_slice::<AgentGpu, u8>(&restored)
    );
    let trial = VisibleTrial::resume_loop(
        &experiment.next_session().unwrap(),
        saved[0].record.evolution.clone().unwrap(),
        &sim,
    )
    .unwrap();
    assert_eq!(trial.world_number, 7);
    // Save at extinction: bodies alone cannot reconstruct the survivors.
    sim.kill_agents_in_region(&d, &q, [WORLD_SIZE * 0.5; 2], WORLD_SIZE);
    assert_eq!(sim.metrics(&d, &q).unwrap().living, 0);
    let dead_checkpoint = experiment
        .save(&sim, &d, &q, Some(trial.snapshot().unwrap()))
        .unwrap();
    sim.load_checkpoint(&q, &dead_checkpoint).unwrap();
    let (saved, _) = experiments::list(&root).unwrap();
    let mut resumed = VisibleTrial::resume_loop(
        &experiment.next_session().unwrap(),
        saved[0].record.evolution.clone().unwrap(),
        &sim,
    )
    .unwrap();
    resumed.advance(&mut sim, &d, &q).unwrap();
    assert_eq!(resumed.world_number, 8);
    assert_eq!(sim.tick, 0);
    assert_eq!(sim.metrics(&d, &q).unwrap().living, 4);
    assert_eq!(
        sim.settings.founder_genomes[..snapshot.latest.bank.genomes.len()],
        snapshot.latest.bank.genomes
    );
    assert_eq!(sim.settings.metabolic_cost, 0.08);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn paused_food_brush_changes_food_inside_the_visible_circle() {
    let (d, q) = gpu();
    let sim = scene(&d, &q);
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 56.0), egui::vec2(920.0, 734.0));
    let point = rect.center() + egui::vec2(90.0, -70.0);
    let world = crate::controls::world_position(rect, [WORLD_SIZE * 0.5; 2], 2.0, point);
    let before = sim.metrics(&d, &q).unwrap();
    sim.apply_resource_shock(&d, &q, world, 45.0 / 2.0, 0.45);
    let added = sim.metrics(&d, &q).unwrap();
    assert!(added.dropped_food > before.dropped_food);
    assert_eq!(sim.tick, 0, "Painting must work without unpausing");
    sim.apply_resource_shock(&d, &q, world, 45.0 / 2.0, -0.65);
    let removed = sim.metrics(&d, &q).unwrap();
    assert!(removed.dropped_food < added.dropped_food);
    assert_eq!(sim.tick, 0);
}

#[test]
fn rolling_survivors_retain_diversity_refresh_controls_and_follow_recovery() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    sim.settings.population = 64;
    let genes = fixed(0, [0.0; 2]);
    for slot in 0..64 {
        let mut a = body([602.0, 902.0]);
        a.lineage_id = slot + 1;
        a.mutation_probability = 0.25;
        a.mutation_magnitude = 0.1;
        put(&sim, &q, slot as usize, a, &genes);
    }
    let root = super::temp("rolling-archive");
    let mut trial = VisibleTrial::new_loop(&root, &sim, &d, &q).unwrap();
    let before = trial.snapshot().unwrap();
    assert_eq!(before.latest.bodies.len(), 64);
    let mut agents = sim.agent_snapshot(&d, &q).unwrap();
    for a in agents.iter_mut().skip(1) {
        a.alive = 0;
    }
    agents[0].mutation_probability = 1.0;
    agents[0].mutation_magnitude = 8.0;
    q.write_buffer(
        &sim.agent_buffers[sim.current_buffer],
        0,
        bytemuck::cast_slice(&agents),
    );
    sim.tick = 128;
    trial.observe(&sim, &d, &q).unwrap();
    let saved = trial.snapshot().unwrap();
    assert_eq!(saved.latest.bodies.len(), 64);
    assert_eq!(saved.latest.bodies[0].lineage_id, 1);
    assert_eq!(saved.latest.bodies[0].mutation_magnitude, 8.0);
    assert_eq!(saved.latest.bodies[0].observed_tick, Some(128));
    assert_eq!(
        saved
            .latest
            .bodies
            .iter()
            .filter(|b| b.lineage_id == 1)
            .count(),
        1
    );
    assert!(
        saved
            .latest
            .bodies
            .iter()
            .skip(1)
            .all(|b| b.observed_tick == Some(0))
    );

    // The actual serialized archive must survive restart, not just in-memory state.
    let snapshot = serde_json::from_slice(&serde_json::to_vec(&saved).unwrap()).unwrap();
    let resumed_root = super::temp("rolling-resume");
    let mut resumed = VisibleTrial::resume_loop(&resumed_root, snapshot, &sim).unwrap();
    for slot in 0..64 {
        let mut a = body([602.0, 902.0]);
        a.lineage_id = slot + 100;
        put(&sim, &q, slot as usize, a, &genes);
    }
    sim.tick = 256;
    resumed.observe(&sim, &d, &q).unwrap();
    let recovered = resumed.snapshot().unwrap();
    assert_eq!(recovered.latest.bodies.len(), 64);
    assert!(recovered.latest.bodies.iter().all(|b| b.lineage_id >= 100));
    let expected = serde_json::to_vec(&recovered.latest).unwrap();
    sim.kill_agents_in_region(&d, &q, [WORLD_SIZE * 0.5; 2], WORLD_SIZE);
    resumed.observe(&sim, &d, &q).unwrap();
    assert_eq!(
        serde_json::to_vec(&resumed.snapshot().unwrap().latest).unwrap(),
        expected
    );
    resumed.advance(&mut sim, &d, &q).unwrap();
    assert_eq!(sim.settings.founder_genomes.len(), 256);
    let transfer: serde_json::Value = serde_json::from_slice(
        &std::fs::read(resumed_root.join("world-000001/transfer.json")).unwrap(),
    )
    .unwrap();
    let mut contributions = [0; 64];
    for entry in transfer["provenance"].as_array().unwrap() {
        contributions[entry["parent"].as_u64().unwrap() as usize] += 1;
    }
    assert_eq!(contributions, [4; 64]);
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(resumed_root).unwrap();
}

#[test]
fn mismatched_experiment_receipt_does_not_replace_live_world() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    put(&sim, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut sim, &d, &q, 1);
    let path = super::temp("receipt-mismatch.checkpoint");
    sim.save_checkpoint(&d, &q, &path).unwrap();
    let saved_seed = sim.seed;
    let saved_tick = sim.tick;
    step(&mut sim, &d, &q, 2);
    let before = read::<AgentGpu>(
        &d,
        &q,
        &sim.agent_buffers[sim.current_buffer],
        MAX_AGENTS as usize,
    );
    for expected in [
        (saved_seed + 1, saved_tick, 1),
        (saved_seed, saved_tick + 1, 1),
        (saved_seed, saved_tick, 2),
    ] {
        assert!(
            sim.load_checkpoint_checked(&q, std::fs::File::open(&path).unwrap(), Some(expected))
                .is_err()
        );
        assert_eq!(sim.tick, saved_tick + 2);
        let after = sim.agent_snapshot(&d, &q).unwrap();
        assert_eq!(
            bytemuck::cast_slice::<AgentGpu, u8>(&before),
            bytemuck::cast_slice::<AgentGpu, u8>(&after)
        );
    }
    sim.load_checkpoint_checked(
        &q,
        std::fs::File::open(&path).unwrap(),
        Some((saved_seed, saved_tick, 1)),
    )
    .unwrap();
    assert_eq!(sim.tick, saved_tick);
    std::fs::remove_file(path).unwrap();
}
