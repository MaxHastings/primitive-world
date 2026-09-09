use super::{body, fixed, gpu, put, read, scene, step};
use crate::simulation::*;

#[test]
fn paused_paint_brush_changes_food_inside_the_visible_circle() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    let rect = egui::Rect::from_min_size(egui::pos2(0.0, 56.0), egui::vec2(920.0, 734.0));
    let point = rect.center() + egui::vec2(90.0, -70.0);
    let world = crate::controls::world_position(
        rect,
        [WORLD_SIZE * 0.5; 2],
        2.0,
        point,
        [WORLD_SIZE, WORLD_SIZE],
    );
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

#[test]
fn painted_food_has_a_dense_core_wraps_and_preserves_experiment_state() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    let body_before = read::<AgentGpu>(
        &d,
        &q,
        &sim.agent_buffers[sim.current_buffer],
        MAX_AGENTS as usize,
    );
    let progress_before = serde_json::to_value(&sim.progress).unwrap();
    let seed = sim.seed;
    let ground_before = read::<[u32; 8]>(
        &d,
        &q,
        &sim.ground_buffer,
        (RESOURCE_GRID * RESOURCE_GRID) as usize,
    );
    // Exactly on cell center; paint across both toroidal edges.
    sim.paint_food(&d, &q, [2.0, 2.0], 40.0, 2.0);
    let ground = read::<[u32; 8]>(
        &d,
        &q,
        &sim.ground_buffer,
        (RESOURCE_GRID * RESOURCE_GRID) as usize,
    );
    let dropped = |x: usize, y: usize| ground[y * 512 + x][0] - ground_before[y * 512 + x][0];
    assert_eq!(dropped(0, 0), 2000);
    assert!(dropped(2, 0) > dropped(5, 0));
    assert!(dropped(5, 0) > dropped(9, 0));
    assert_eq!(dropped(10, 0), 0);
    assert_eq!(dropped(1, 0), dropped(511, 0));
    assert_eq!(dropped(0, 1), dropped(0, 511));
    for (before, after) in ground_before.iter().zip(&ground) {
        assert_eq!(before[1..], after[1..]);
    }
    assert_eq!(sim.seed, seed);
    assert_eq!(sim.tick, 0);
    assert_eq!(
        serde_json::to_value(&sim.progress).unwrap(),
        progress_before
    );
    assert_eq!(
        bytemuck::cast_slice::<_, u8>(&body_before),
        bytemuck::cast_slice::<_, u8>(&read::<AgentGpu>(
            &d,
            &q,
            &sim.agent_buffers[sim.current_buffer],
            MAX_AGENTS as usize
        ))
    );
    assert!(sim.assisted);
    sim.paint_food(&d, &q, [2.0, 2.0], 40.0, 8.0);
    let denser = read::<[u32; 8]>(
        &d,
        &q,
        &sim.ground_buffer,
        (RESOURCE_GRID * RESOURCE_GRID) as usize,
    );
    assert_eq!(
        denser[0][0] - ground[0][0],
        8000,
        "Density scales supply independently of radius"
    );
    let ground = denser;
    let path = super::temp("painted-experiment.checkpoint");
    sim.save_checkpoint(&d, &q, &path).unwrap();
    sim.load_checkpoint(&q, &path).unwrap();
    assert_eq!(
        read::<[u32; 8]>(
            &d,
            &q,
            &sim.ground_buffer,
            (RESOURCE_GRID * RESOURCE_GRID) as usize
        ),
        ground
    );
    step(&mut sim, &d, &q, 1);
    assert_eq!(sim.tick, 1);
    assert!(sim.assisted);
    std::fs::remove_file(path).unwrap();
}
