use super::*;
use crate::reproduction_archive::{Archive, Snapshot, priority};
fn snapshot(s: &Simulation, d: &wgpu::Device, q: &wgpu::Queue) -> Snapshot {
    s.reproduction_archive
        .as_ref()
        .unwrap()
        .snapshot(s, d, q)
        .unwrap()
}
#[test]
fn archive_registers_founders_at_zero_and_captures_births_before_cpu_observation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.population = 2;
    put(&s, &q, 0, body([100.0, 100.0]), &fixed(5, [0.0; 2]));
    let mut a = body([1800.0, 1800.0]);
    a.lineage_id = 2;
    put(&s, &q, 1, a, &fixed(0, [0.0; 2]));
    let initial = Snapshot::initial(&s, &d, &q).unwrap();
    s.reproduction_archive = Some(Archive::new(&d, &q, &s, &initial).unwrap());
    let initial = snapshot(&s, &d, &q);
    assert_eq!(initial.eligible_count, 2);
    assert!(
        initial
            .entries
            .iter()
            .all(|e| e.body.observed_tick == Some(0))
    );
    step(&mut s, &d, &q, 1);
    let captured = snapshot(&s, &d, &q);
    assert!(captured.eligible_count > 2);
    assert!(
        captured
            .entries
            .iter()
            .any(|e| e.body.ancestry_depth == 1 && e.body.age == 0.0)
    );
    s.kill_agents_in_region(&d, &q, [WORLD_SIZE * 0.5; 2], WORLD_SIZE);
    let dead = snapshot(&s, &d, &q);
    assert_eq!(
        captured
            .entries
            .iter()
            .map(|e| &e.genome)
            .collect::<Vec<_>>(),
        dead.entries.iter().map(|e| &e.genome).collect::<Vec<_>>()
    );
}
#[test]
fn reservoir_includes_early_deaths_and_uses_one_ticket_per_identity_across_resume() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for slot in 0..300 {
        let mut a = body([
            50.0 + (slot % 20) as f32 * 70.0,
            50.0 + (slot / 20) as f32 * 70.0,
        ]);
        a.lineage_id = slot + 1;
        a.food = 0.0;
        a.energy = if slot % 2 == 0 { 0.001 } else { 65.0 };
        put(&s, &q, slot as usize, a, &fixed(0, [0.0; 2]));
    }
    let initial = Snapshot::initial(&s, &d, &q).unwrap();
    assert_eq!(initial.eligible_count, 300);
    assert_eq!(initial.entries.len(), 256);
    let mut expected: Vec<_> = (1..=300).collect();
    expected.sort_by_key(|&id| priority(id, s.seed));
    expected.truncate(256);
    assert_eq!(
        initial
            .entries
            .iter()
            .map(|e| e.body.lineage_id)
            .collect::<Vec<_>>(),
        expected
    );
    s.reproduction_archive = Some(Archive::new(&d, &q, &s, &initial).unwrap());
    step(&mut s, &d, &q, 4);
    let before = snapshot(&s, &d, &q);
    assert_eq!(before.eligible_count, 300);
    assert!(before.entries.iter().any(|e| e.alive == 0));
    s.reproduction_archive = Some(Archive::new(&d, &q, &s, &before).unwrap());
    step(&mut s, &d, &q, 4);
    assert_eq!(snapshot(&s, &d, &q).eligible_count, 300);
}
#[test]
fn archive_observation_does_not_change_physics_and_bad_snapshots_are_rejected() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([300.0, 300.0]), &fixed(1, [0.1, 0.0]));
    let checkpoint = temp("genome-archive-isolation.checkpoint");
    s.save_checkpoint(&d, &q, &checkpoint).unwrap();
    step(&mut s, &d, &q, 8);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    let metrics = s.metrics(&d, &q).unwrap();
    s.load_checkpoint(&q, &checkpoint).unwrap();
    let initial = Snapshot::initial(&s, &d, &q).unwrap();
    s.reproduction_archive = Some(Archive::new(&d, &q, &s, &initial).unwrap());
    step(&mut s, &d, &q, 8);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap())
    );
    assert_eq!(
        serde_json::to_value(metrics).unwrap(),
        serde_json::to_value(s.metrics(&d, &q).unwrap()).unwrap()
    );
    let mut bad = snapshot(&s, &d, &q);
    bad.policy = "neural-duration-selector-v1".into();
    assert!(bad.validate().is_err());
    let mut bad = snapshot(&s, &d, &q);
    bad.extinction_tick = Some(s.tick + 1);
    assert!(bad.validate().is_err());
    let mut bad = snapshot(&s, &d, &q);
    bad.qualified_lineages.pop();
    assert!(bad.validate().is_err());
    std::fs::remove_file(checkpoint).unwrap();
}

#[test]
fn matched_world_pause_preserves_lifetime_archive_and_natural_extinction() {
    use crate::selector_comparison::run_world;
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let settings = SimSettings {
        population: 4,
        metabolic_cost: 10.0,
        founder_genomes: vec![fixed(0, [0.0; 2]).to_vec()],
        founder_slots: vec![0; 4],
        ..s.settings.clone()
    };
    let root = temp("round-world-pause");
    std::fs::create_dir_all(&root).unwrap();
    let whole = run_world(
        &mut s,
        &d,
        &q,
        settings.clone(),
        123,
        None,
        32,
        &root,
        "whole",
    )
    .unwrap();
    assert!(whole.paused.is_none());
    assert!(crate::candidate_pool::Pool::from_archive(&whole.archive).is_ok());
    let first = run_world(
        &mut s,
        &d,
        &q,
        settings.clone(),
        123,
        None,
        1,
        &root,
        "split",
    )
    .unwrap();
    assert!(first.paused.is_some());
    assert_eq!(first.archive.extinction_tick, None);
    let resumed = run_world(
        &mut s,
        &d,
        &q,
        settings,
        123,
        first.paused,
        31,
        &root,
        "split",
    )
    .unwrap();
    assert!(resumed.paused.is_none());
    assert_eq!(
        serde_json::to_value(&whole.archive).unwrap(),
        serde_json::to_value(&resumed.archive).unwrap()
    );
    std::fs::remove_dir_all(root).unwrap();
}
