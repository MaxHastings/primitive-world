use super::{body, fixed, gpu, put, read, scene, step};
use crate::{experiments, simulation::*};

#[test]
fn viewer_rounds_save_load_and_extinction_resume_preserve_learning_and_pool() {
    use crate::live_rounds::{Config, Viewer};
    let (d, q) = gpu();
    let mut sim = Simulation::new(&d, &q, 42);
    sim.settings.population = 4;
    sim.settings.metabolic_cost = 10.0;
    sim.settings.founder_genomes = vec![fixed(0, [0.0; 2]).to_vec()];
    sim.reset(&q);
    let mut rounds = Viewer::new(
        &mut sim,
        &d,
        &q,
        &Config {
            batches: 2,
            compositions: 2,
            environments: 2,
            retention: 2,
        },
    )
    .unwrap();
    let root = super::temp("viewer-round-save");
    let directory = root.join("experiment");
    std::fs::create_dir_all(&directory).unwrap();
    let experiment = experiments::Experiment {
        directory,
        name: "Round test".into(),
        origin: "Test".into(),
        total_ticks: 0,
    };
    // Save one live tick: no reward, no proposal and no hidden reset on load.
    step(&mut sim, &d, &q, 1);
    let original_bodies = sim.agent_snapshot(&d, &q).unwrap();
    let snapshot = rounds.snapshot(&sim, &d, &q).unwrap();
    experiment.save(&sim, &d, &q, snapshot.clone()).unwrap();
    let (saves, skipped) = experiments::list(&root).unwrap();
    assert_eq!(skipped, 0);
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].record.version, 2);
    assert_eq!(saves[0].record.rounds.training.learner().updates, 0);
    sim.reset(&q);
    let saved = &saves[0];
    sim.load_round_checkpoint(
        &q,
        std::fs::File::open(saved.checkpoint()).unwrap(),
        Some((saved.record.seed, saved.record.tick, saved.record.living)),
        &saved.record.rounds.expected_settings().unwrap(),
    )
    .unwrap();
    rounds = saved
        .record
        .rounds
        .clone()
        .restore(&mut sim, &d, &q)
        .unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&original_bodies),
        bytemuck::cast_slice::<AgentGpu, u8>(&sim.agent_snapshot(&d, &q).unwrap())
    );
    // Reach three rounds. Every intermediate extinction is a valid save/load point.
    for world in 0..17 {
        step(&mut sim, &d, &q, 32);
        assert_eq!(sim.metrics(&d, &q).unwrap().living, 0);
        let extinct = rounds.snapshot(&sim, &d, &q).unwrap();
        let checkpoint = experiment.save(&sim, &d, &q, extinct.clone()).unwrap();
        rounds.advance(&mut sim, &d, &q).unwrap();
        let expected = serde_json::to_value(&rounds).unwrap();
        sim.load_round_checkpoint(
            &q,
            std::fs::File::open(checkpoint).unwrap(),
            None,
            &extinct.expected_settings().unwrap(),
        )
        .unwrap();
        rounds = extinct.restore(&mut sim, &d, &q).unwrap();
        rounds.advance(&mut sim, &d, &q).unwrap();
        assert_eq!(
            serde_json::to_value(&rounds).unwrap(),
            expected,
            "world {world}"
        );
        let copies = sim.settings.founder_slots.clone();
        assert_eq!(copies.len(), 4);
        let genes = read::<f32>(&d, &q, &sim.genome_buffer, 4 * GENOME_SIZE);
        for (slot, candidate) in copies.iter().enumerate() {
            assert_eq!(
                &genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE],
                sim.settings.founder_genomes[*candidate as usize]
            );
        }
    }
    assert_eq!(rounds.training.round, 3);
    assert_eq!(rounds.training.learner().updates, 4);
    let crate::selector_comparison::CandidatePool::Retained(pool) =
        &rounds.training.comparison.as_ref().unwrap().pool
    else {
        panic!()
    };
    assert!(pool.entries.iter().all(|e| e.source.round > 0));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn mismatched_round_settings_are_rejected_before_replacing_the_live_world() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    put(&sim, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    let checkpoint = super::temp("round-settings.checkpoint");
    sim.save_checkpoint(&d, &q, &checkpoint).unwrap();
    step(&mut sim, &d, &q, 1);
    let before = sim.agent_snapshot(&d, &q).unwrap();
    let mut wrong = sim.settings.clone();
    wrong.metabolic_cost += 0.01;
    assert!(
        sim.load_round_checkpoint(&q, std::fs::File::open(&checkpoint).unwrap(), None, &wrong)
            .is_err()
    );
    assert_eq!(sim.tick, 1);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&before),
        bytemuck::cast_slice::<AgentGpu, u8>(&sim.agent_snapshot(&d, &q).unwrap())
    );
    std::fs::remove_file(checkpoint).unwrap();
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
