use super::{body, fixed, gpu, put, read, scene, step};
use crate::simulation::*;

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
