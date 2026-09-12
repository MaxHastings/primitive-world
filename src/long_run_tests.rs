//! Boundary fixtures exercise old 32-bit limits without a multi-month soak.
use super::*;

#[test]
fn telemetry_carry_preserves_bodies_learning_ecology_and_reproduction() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut producer = body([500.0, 500.0]);
    producer.energy = 80.0;
    put(&s, &q, 0, producer, &fixed(5, [0.0; 2]));
    let path = temp("wide-counter-baseline.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 32);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    let food = s.vegetation_snapshot(&d, &q).unwrap();
    let pool = s.reservoir_snapshot(&d, &q).unwrap();
    s.load_checkpoint(&q, &path).unwrap();
    for index in [
        1, 2, 3, 4, 5, 6, 7, 9, 12, 13, 15, 16, 17, 19, 20, 21, 22, 24, 25, 26, 27, 28, 29, 31, 32,
        33, 34, 35, 37, 38, 39,
    ] {
        q.write_buffer(
            &s.death_stats_buffer,
            index * 4,
            bytemuck::bytes_of(&u32::MAX),
        );
    }
    // An old-format accounting flag must not suppress births either.
    q.write_buffer(&s.death_stats_buffer, 36 * 4, bytemuck::bytes_of(&1u32));
    step(&mut s, &d, &q, 32);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap())
    );
    assert_eq!(food, s.vegetation_snapshot(&d, &q).unwrap());
    assert_eq!(pool, s.reservoir_snapshot(&d, &q).unwrap());
    assert!(!s.refresh_engine_status(&d, &q).unwrap());
    assert!(s.metrics(&d, &q).unwrap().action_ticks[5] > u64::from(u32::MAX));
    assert_eq!(s.progress.world, 1);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn clock_crosses_u32_boundary_and_checkpoint_resume_is_exact() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([500.0, 500.0]), &fixed(4, [0.0; 2]));
    s.tick = u64::from(u32::MAX) - 1;
    step(&mut s, &d, &q, 4);
    assert_eq!(s.tick, u64::from(u32::MAX) + 3);
    let bodies = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(bodies[0].signal_high, 1);
    assert!(!s.refresh_engine_status(&d, &q).unwrap());
    assert_eq!(s.progress.world, 1);
    let path = temp("wide-clock.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 4);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    s.load_checkpoint(&q, &path).unwrap();
    step(&mut s, &d, &q, 4);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap())
    );
    assert_eq!(s.metrics(&d, &q).unwrap().tick, u64::from(u32::MAX) + 7);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_checkpoint_expands_only_counter_and_identity_storage() {
    use std::io::{Cursor, Read};
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([500.0, 500.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 4);
    let path = temp("legacy-wide-migration.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let original = std::fs::read(&path).unwrap();
    let json_len = u32::from_le_bytes(original[20..24].try_into().unwrap()) as usize;
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&original[24..24 + json_len]).unwrap();
    metadata.as_object_mut().unwrap().remove("tick_high");
    let metadata = serde_json::to_vec(&metadata).unwrap();
    let mut legacy = b"PRIMWORLD058".to_vec();
    legacy.extend_from_slice(&original[12..20]);
    legacy.extend_from_slice(&(metadata.len() as u32).to_le_bytes());
    legacy.extend_from_slice(&metadata);
    let mut cursor = Cursor::new(&original[24 + json_len..]);
    for index in 0..18 {
        let mut len = [0; 8];
        cursor.read_exact(&mut len).unwrap();
        let mut data = vec![0; u64::from_le_bytes(len) as usize];
        cursor.read_exact(&mut data).unwrap();
        if index == 0 {
            data = data
                .chunks_exact(312)
                .flat_map(|body| body[..296].iter().copied())
                .collect();
        }
        if index == 4 {
            data.truncate(160);
        }
        if index == 5 {
            data = data
                .chunks_exact(72)
                .flat_map(|event| event[..56].iter().copied())
                .collect();
        }
        legacy.extend_from_slice(&(data.len() as u64).to_le_bytes());
        legacy.extend_from_slice(&data);
    }
    step(&mut s, &d, &q, 4);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    s.load_checkpoint_reader(&q, Cursor::new(legacy)).unwrap();
    assert_eq!(s.tick, 4);
    step(&mut s, &d, &q, 4);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap())
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn packets_compare_full_parent_identity_across_epochs() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for different in [false, true] {
        s.reset(&q);
        for (slot, high) in [(0, 0), (1, u32::from(different))] {
            let mut a = body([500.0 + slot as f32, 500.0]);
            a.alive = 2;
            a.energy = 16.0;
            a.food = 0.0;
            a.parent_lineage = 123;
            a.parent_high = high;
            put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
        }
        step(&mut s, &d, &q, 1);
        assert_eq!(s.metrics(&d, &q).unwrap().events[3], u64::from(different));
    }
}
