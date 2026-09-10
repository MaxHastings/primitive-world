use super::*;

/// An upper-bound physical feasibility experiment, not an evolved genome or
/// ordinary random initialization. Only action readouts are controlled after
/// initialization; bodies, energy, food, packets and offspring are never edited.
#[test]
fn controlled_finite_reserve_colony_reaches_descendant_reproduction() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings = SimSettings {
        population: 0,
        ..SimSettings::default()
    };
    s.reset(&q);
    let positions = [
        [100.0, 100.0],
        [102.0, 100.0],
        [97.0, 100.0],
        [105.0, 100.0],
        [101.0, 97.0],
        [101.0, 103.0],
    ];
    for (slot, position) in positions.into_iter().enumerate() {
        let a = AgentGpu {
            energy: 100.0,
            food: 8.0,
            packet_size: 16.0,
            lineage_id: slot as u32 + 1,
            active_mask: 1,
            parameter_mutation_rate: 1.0,
            parameter_mutation_step: 1.0,
            topology_mutation_rate: 1.0,
            ..body(position)
        };
        put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
    }
    s.family_observer =
        Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 2200).unwrap());
    for _ in 0..2200 {
        let bodies = s.agent_snapshot(&d, &q).unwrap();
        if bodies.iter().any(|a| a.alive == 1 && a.ancestry_depth >= 2) {
            break;
        }
        for (slot, a) in bodies.iter().enumerate().filter(|(_, a)| a.alive == 1) {
            let breeder = slot < 2 || a.ancestry_depth > 0;
            let adult = a.age >= s.settings.maturity_age;
            let reproduce = breeder && adult && a.packets_produced < 2 && a.energy > 25.0;
            let action = if reproduce {
                5
            } else if adult {
                2
            } else {
                0
            };
            let mut genes = fixed(action, [0.0; 2]);
            // No ancestry-aware transfer targeting: the normal nearest-body
            // rule decides who receives every attempted transfer.
            genes[OUTPUT_BIAS + 1] = if adult && breeder { 1.0 } else { 0.0 };
            s.write_genome_slot(&q, slot, &genes);
        }
        step(&mut s, &d, &q, 1);
    }
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    let f = &report.families[0];
    for (slot, a) in s
        .agent_snapshot(&d, &q)
        .unwrap()
        .iter()
        .enumerate()
        .filter(|(_, a)| a.alive != 0)
    {
        eprintln!(
            "colony slot={slot} state={} depth={} age={} energy={} food={} packets={}",
            a.alive, a.ancestry_depth, a.age, a.energy, a.food, a.packets_produced
        );
    }
    eprintln!(
        "controlled colony: ticks={} births={} matured={} juvenile_transfer_ticks={} food_received={} descendant_parent_births={}",
        s.tick,
        f.births,
        f.matured_descendants,
        f.juvenile_transfers_received,
        f.juvenile_received_milli,
        f.births_to_descendant_parents
    );
    assert!(f.matured_descendants >= 2);
    assert!(f.juvenile_transfers_received > 0);
    assert!(f.births_to_descendant_parents > 0);
}

pub(super) fn packet(position: [f32; 2], producer: u32, energy: f32) -> AgentGpu {
    AgentGpu {
        alive: 2,
        age: 0.0,
        food: 0.0,
        energy,
        packet_size: energy.max(1.0),
        parent_lineage: producer,
        lineage_id: 100 + producer,
        ..body(position)
    }
}

fn decay(size: f32) -> f32 {
    0.02 * size.powf(2.0 / 3.0)
}

#[test]
fn organism_pushes_packet_with_passive_drift_and_intact_snapshot_accounting() {
    let (d, q) = gpu();
    for (actor_slot, packet_slot) in [(0, 1), (1, 0)] {
        let mut s = scene(&d, &q);
        let mut a = body([2046.0, 100.0]);
        a.food = 0.0;
        let p = packet([2047.0, 100.0], 1, 16.0);
        let genes = fixed(4, [1.0, 1.0]);
        put(&s, &q, actor_slot, a, &fixed(3, [0.0; 2]));
        put(&s, &q, packet_slot, p, &genes);
        step(&mut s, &d, &q, 1);
        let after = s.agent_snapshot(&d, &q).unwrap();
        let pushed = after[packet_slot];
        let impulse = 3.0 * 1.0f32.tanh();
        near(pushed.velocity[0], impulse);
        near(after[actor_slot].velocity[0], -impulse);
        assert_eq!(pushed.position, p.position);
        near(pushed.energy, p.energy - decay(p.packet_size));
        near(after[actor_slot].energy + after[actor_slot].spent, a.energy);
        let m = s.metrics(&d, &q).unwrap();
        assert_eq!(m.events[5], 1);
        assert_eq!(m.events[3], 0);
        assert_eq!(m.events[4], 0);
        near(m.force_energy_spent as f32, 0.1 * impulse * impulse);
        near(m.packet_energy as f32, pushed.energy);
        // A passive packet may carry contact-induced velocity in a valid save.
        let checkpoint = temp(&format!("pushed-packet-{packet_slot}.checkpoint"));
        s.save_checkpoint(&d, &q, &checkpoint).unwrap();
        s.load_checkpoint(&q, &checkpoint).unwrap();
        let restored = s.agent_snapshot(&d, &q).unwrap()[packet_slot];
        assert_eq!(restored.velocity, pushed.velocity);
        assert_eq!(restored.position, pushed.position);
        std::fs::remove_file(checkpoint).unwrap();
        s.write_genome_slot(&q, actor_slot, &fixed(0, [0.0; 2]));
        step(&mut s, &d, &q, 1);
        let drifted = s.agent_snapshot(&d, &q).unwrap()[packet_slot];
        near(drifted.position[0], (p.position[0] + impulse) % 2048.0);
        near(drifted.velocity[0], impulse * 0.98);
        near(
            drifted.energy,
            pushed.energy - packet_upkeep(1) * 16.0f32.powf(2.0 / 3.0),
        );
        assert_eq!(drifted.alive, 2);
        assert_eq!(drifted.parent_lineage, p.parent_lineage);
        assert_eq!(drifted.packet_size, p.packet_size);
        assert_eq!(drifted.hidden, [0.0; HIDDEN]);
        assert_eq!(drifted.lived_ticks, 0);
        assert_eq!(drifted.food, 0.0);
        assert_eq!(drifted.signal_tick, 0);
        assert_eq!(
            read::<DecisionGpu>(&d, &q, &s.decision_buffer, 2)[packet_slot].evaluated,
            0
        );
        let snapshots = s.read_genomes(&d, &q, 2).unwrap();
        assert_eq!(
            &snapshots[packet_slot * GENOME_SIZE..(packet_slot + 1) * GENOME_SIZE],
            &genes
        );
    }
}

#[test]
fn pushed_packet_can_drift_into_compatible_fusion_range() {
    let (d, q) = gpu();
    for pushed in [false, true] {
        let mut s = scene(&d, &q);
        let mut a = body([100.0, 100.0]);
        a.food = 0.0;
        put(&s, &q, 0, a, &fixed(if pushed { 3 } else { 0 }, [0.0; 2]));
        put(
            &s,
            &q,
            1,
            packet([102.0, 100.0], 1, 16.0),
            &fixed(0, [0.0; 2]),
        );
        put(
            &s,
            &q,
            2,
            packet([106.0, 100.0], 2, 16.0),
            &fixed(0, [0.0; 2]),
        );
        step(&mut s, &d, &q, 1);
        assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
        s.write_genome_slot(&q, 0, &fixed(0, [0.0; 2]));
        step(&mut s, &d, &q, 1);
        let m = s.metrics(&d, &q).unwrap();
        assert_eq!(m.events[3], u32::from(pushed));
        assert_eq!(m.packets, if pushed { 0 } else { 2 });
        if pushed {
            let after = s.agent_snapshot(&d, &q).unwrap();
            let child = after
                .iter()
                .find(|a| a.alive == 1 && a.ancestry_depth == 1)
                .unwrap();
            near(
                child.energy,
                32.0 - 2.0 * (packet_upkeep(0) + packet_upkeep(1)) * 16.0f32.powf(2.0 / 3.0)
                    - s.settings.fusion_loss,
            );
        }
    }
}

#[test]
fn organism_transfer_skips_packets_without_fusion_or_reserve_transfer() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([100.0, 100.0]), &fixed(2, [0.0; 2]));
    put(
        &s,
        &q,
        1,
        packet([101.0, 100.0], 2, 16.0),
        &fixed(0, [0.0; 2]),
    );
    step(&mut s, &d, &q, 1);
    let after = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(after[1].alive, 2);
    assert_eq!(after[1].food, 0.0);
    assert_eq!(after[1].received, 0.0);
    near(after[1].energy, 16.0 - decay(16.0));
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.events[3], 0);
    assert_eq!(m.events[4], 0);
}

#[test]
fn packet_production_is_local_paid_and_independent_of_partners() {
    let (d, q) = gpu();
    for (size, energy, age, action, expected) in [
        (8.0, 80.0, 1800.0, 5, true),
        (40.0, 80.0, 1800.0, 5, true),
        (16.0, 8.0, 1800.0, 5, false),
        (16.0, 80.0, 0.0, 5, false),
        (16.0, 80.0, 1800.0, 0, false),
    ] {
        let mut s = scene(&d, &q);
        let mut a = body([2047.9, 100.0]);
        a.packet_size = size;
        a.energy = energy;
        a.age = age;
        a.food = 0.0;
        a.hidden = [0.5; HIDDEN];
        let mut g = fixed(action, [0.2, 0.0]);
        g[OUTPUT_BIAS + PLACEMENT_OUTPUT] = 2.0;
        put(&s, &q, 0, a, &g);
        step(&mut s, &d, &q, 1);
        let after = s.agent_snapshot(&d, &q).unwrap();
        let p = after[0];
        let packets: Vec<_> = after.iter().filter(|a| a.alive == 2).collect();
        assert_eq!(packets.len(), usize::from(expected));
        assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
        assert!(p.velocity[0] > 0.0);
        near(p.energy + p.spent, a.energy);
        assert_eq!(p.packets_produced, u32::from(expected));
        if expected {
            let packet = packets[0];
            near(packet.energy, size);
            assert_eq!(packet.parent_lineage, a.lineage_id);
            assert_eq!(packet.hidden, [0.0; HIDDEN]);
            assert_eq!(packet.velocity, [0.0; 2]);
            assert_eq!(packet.packet_size, size);
            near(
                packet.position[0],
                (p.position[0] + 2.0 * 2.0f32.tanh()).rem_euclid(2048.0),
            );
            let genes = s.read_genomes(&d, &q, 2).unwrap();
            assert_eq!(&genes[GENOME_SIZE..], &g);
        }
    }
}

#[test]
fn smaller_packets_allow_more_production_from_the_same_reserves() {
    let (d, q) = gpu();
    let mut counts = Vec::new();
    for size in [2.0, 16.0] {
        let mut s = scene(&d, &q);
        s.settings.metabolic_cost = 0.0;
        let mut a = body([100.0, 100.0]);
        a.energy = 65.0;
        a.food = 0.0;
        a.packet_size = size;
        put(&s, &q, 0, a, &fixed(5, [0.0; 2]));
        step(&mut s, &d, &q, 40);
        let after = s.agent_snapshot(&d, &q).unwrap();
        counts.push(after[0].packets_produced);
        near(
            after[0].energy + size * after[0].packets_produced as f32,
            65.0,
        );
        assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
    }
    assert_eq!(counts, [32, 4]);
}

#[test]
fn packets_fuse_only_locally_and_only_between_different_producers() {
    let (d, q) = gpu();
    for (other_producer, distance, energy, births, failed) in [
        (1, 1.0, 16.0, 0, 0),
        (2, 2.01, 16.0, 0, 0),
        (2, 2.0, 16.0, 1, 0),
        (1, 2.0, 16.0, 0, 0),
        (2, 1.0, 16.0, 1, 0),
        (2, 1.0, 24.0, 1, 0),
        (2, 1.0, 48.0, 1, 0),
        (2, 1.0, 2.0, 0, 1),
    ] {
        let mut s = scene(&d, &q);
        put(
            &s,
            &q,
            0,
            packet([2047.5, 100.0], 1, energy),
            &fixed(0, [0.0; 2]),
        );
        put(
            &s,
            &q,
            1,
            packet(
                [(2047.5 + distance) % 2048.0, 100.0],
                other_producer,
                energy,
            ),
            &fixed(0, [0.0; 2]),
        );
        step(&mut s, &d, &q, 1);
        let m = s.metrics(&d, &q).unwrap();
        assert_eq!(m.events[3], births);
        assert_eq!(m.failed_fusions, failed);
        let after = s.agent_snapshot(&d, &q).unwrap();
        if births == 1 {
            let c = after.iter().find(|a| a.alive == 1).unwrap();
            near(
                c.energy,
                2.0 * (energy - decay(energy)) - s.settings.fusion_loss,
            );
            assert_eq!(c.hidden, [0.0; HIDDEN]);
            assert_eq!(c.ancestry_depth, 1);
            assert!((9000.0..=11000.0).contains(&c.max_age));
            assert_eq!(m.packets, 0);
        } else if failed == 1 {
            assert_eq!(m.living, 0);
        } else {
            assert_eq!(m.packets, 2);
        }
    }
}

#[test]
fn packet_snapshots_outlive_producers_and_do_not_require_synchronized_release() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let genome = fixed(0, [0.0; 2]);
    put(&s, &q, 0, packet([100.0, 100.0], 7, 32.0), &genome);
    step(&mut s, &d, &q, 100);
    let before = s.agent_snapshot(&d, &q).unwrap()[0];
    assert_eq!(before.alive, 2);
    near(
        before.energy,
        (0..100).fold(32.0, |e, t| e - packet_upkeep(t) * 32.0f32.powf(2.0 / 3.0)),
    );
    assert_eq!(before.position, [100.0, 100.0]);
    assert_eq!(
        read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0].evaluated,
        0
    );
    assert!(
        s.complete_world(&d, &q).is_err(),
        "viable packets keep the world alive"
    );
    put(&s, &q, 1, packet([101.0, 100.0], 8, 8.0), &genome);
    step(&mut s, &d, &q, 1);
    let after = s.agent_snapshot(&d, &q).unwrap();
    let child = after.iter().find(|a| a.alive == 1).unwrap();
    near(
        child.energy,
        before.energy - packet_upkeep(100) * 32.0f32.powf(2.0 / 3.0) + 8.0
            - packet_upkeep(100) * 8.0f32.powf(2.0 / 3.0)
            - s.settings.fusion_loss,
    );
    assert!(
        [8.0, 32.0]
            .iter()
            .any(|size| (child.packet_size - size).abs() < 3.3)
    );
}

#[test]
fn unfused_packets_expire_by_resource_depletion_without_cognition() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(
        &s,
        &q,
        0,
        packet([100.0, 100.0], 1, 1.0),
        &fixed(4, [1.0, 1.0]),
    );
    step(&mut s, &d, &q, 30);
    let a = s.agent_snapshot(&d, &q).unwrap()[0];
    assert_eq!(a.alive, 2);
    near(a.energy, (0..30).fold(1.0, |e, t| e - packet_upkeep(t)));
    assert_eq!(a.position, [100.0, 100.0]);
    assert_eq!(a.lived_ticks, 0);
    step(&mut s, &d, &q, 30);
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.living, 0);
    assert_eq!(m.signals, 0);
    assert_eq!(m.action_ticks, [0; 6]);
}

#[test]
fn fusion_consumes_each_packet_once_and_never_needs_an_empty_slot() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut entities: Vec<_> = (0..MAX_AGENTS)
        .map(|i| packet([(i % 128) as f32 * 8.0, (i / 128) as f32 * 8.0], 99, 16.0))
        .collect();
    let mut producer = body([1500.0, 1500.0]);
    producer.food = 0.0;
    entities[0] = producer;
    entities[1] = packet([1600.0, 1600.0], 10, 16.0);
    entities[2] = packet([1601.0, 1600.0], 11, 16.0);
    for b in &s.agent_buffers {
        q.write_buffer(b, 0, bytemuck::cast_slice(&entities));
    }
    s.write_genome_slot(&q, 0, &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.events[3], 1);
    assert_eq!(m.capacity_blocked_packets, 1);
    assert_eq!(m.living, u64::from(MAX_AGENTS - 1));
    assert!(!s.refresh_engine_status(&d, &q).unwrap());
    let after = s.agent_snapshot(&d, &q).unwrap();
    near(after[0].energy, producer.energy - s.settings.metabolic_cost);
    assert_eq!(after[0].packets_produced, 0);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.tick, 2);
    let after = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(after[0].packets_produced, 1);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    assert!(!s.refresh_engine_status(&d, &q).unwrap());
}

#[test]
fn packet_checkpoint_replays_genomes_resources_and_later_fusion() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(
        &s,
        &q,
        0,
        packet([100.0, 100.0], 1, 16.0),
        &fixed(0, [0.0; 2]),
    );
    step(&mut s, &d, &q, 4);
    let path = temp("packet-replay.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let finish = |s: &mut Simulation| {
        put(
            s,
            &q,
            1,
            packet([101.0, 100.0], 2, 24.0),
            &fixed(4, [0.0; 2]),
        );
        step(s, &d, &q, 3);
        (
            s.agent_snapshot(&d, &q).unwrap(),
            s.read_genomes(&d, &q, 2).unwrap(),
        )
    };
    let expected = finish(&mut s);
    s.load_checkpoint(&q, &path).unwrap();
    let actual = finish(&mut s);
    assert!(
        bytemuck::cast_slice::<AgentGpu, u8>(&actual.0)
            == bytemuck::cast_slice::<AgentGpu, u8>(&expected.0)
    );
    assert!(actual.1 == expected.1);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn accounting_horizon_rolls_forward_without_extinction_or_losing_the_pool() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.population = 4;
    s.reset(&q);
    let before = s.reservoir_snapshot(&d, &q).unwrap();
    s.record_engine_saturation();
    s.rollover_world(&d, &q).unwrap();
    assert_eq!(s.progress.world, 2);
    assert!(s.progress.history.is_empty());
    assert!(!s.progress.engine_saturated);
    assert_eq!(s.tick, 0);
    assert!(
        s.agent_snapshot(&d, &q)
            .unwrap()
            .iter()
            .filter(|a| a.alive == 1)
            .all(|a| (9000.0..=11000.0).contains(&a.max_age))
    );
    assert!(s.reservoir_snapshot(&d, &q).unwrap() == before);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.tick, 1);
}

#[test]
fn initial_food_preserves_habitat_edges_without_opening_cover() {
    assert_eq!(build_resources(&[0.0, 0.25, 0.5, 1.0]), [0, 137, 275, 550]);
    let habitat = build_habitat_at(42, 0, 1.0);
    let food = build_resources(&habitat);
    assert!(food.contains(&0));
    assert!(food.iter().any(|v| *v > 0));
}

#[test]
fn evolving_terrain_and_weather_resume_across_million_tick_boundary() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.settings.resource_regeneration = 0.01;
    s.reset(&q);
    s.tick = 999_998;
    step(&mut s, &d, &q, 1);
    let path = temp("ecology-ramp-boundary.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let finish = |s: &mut Simulation| {
        step(s, &d, &q, 4);
        (
            read::<u32>(
                &d,
                &q,
                &s.resource_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            ),
            read::<u32>(
                &d,
                &q,
                &s.ground_buffer,
                (RESOURCE_GRID * RESOURCE_GRID * 8) as usize,
            ),
            read::<f32>(
                &d,
                &q,
                &s.fertility_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            ),
            read::<[f32; 4]>(
                &d,
                &q,
                &s.ecology_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            ),
        )
    };
    let expected = finish(&mut s);
    s.load_checkpoint(&q, &path).unwrap();
    assert!(finish(&mut s) == expected);
    assert_eq!(s.tick, 1_000_003);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn in_place_fusion_keeps_both_packet_genomes_and_module_traits() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = packet([100.0, 100.0], 1, 8.0);
    let mut b = packet([101.0, 100.0], 2, 40.0);
    a.active_mask = 0x5555;
    b.active_mask = 0xaaaa;
    a.plasticity_rate = [0.1; HIDDEN];
    b.plasticity_rate = [-0.1; HIDDEN];
    put(&s, &q, 0, a, &[1.0; GENOME_SIZE]);
    put(&s, &q, 1, b, &[-1.0; GENOME_SIZE]);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let (slot, child) = agents
        .iter()
        .enumerate()
        .find(|(_, a)| a.alive == 1)
        .unwrap();
    let genes = s.read_genomes(&d, &q, 2).unwrap();
    let genes = &genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE];
    assert!([8.0, 40.0].contains(&child.packet_size));
    assert!(genes[..HIDDEN].contains(&1.0) && genes[..HIDDEN].contains(&-1.0));
    for h in 0..HIDDEN {
        let value = genes[NODE_BIAS + h];
        let donor = if value == 1.0 { a } else { b };
        assert!([1.0, -1.0].contains(&value));
        assert_eq!(child.active_mask & (1 << h), donor.active_mask & (1 << h));
        assert_eq!(child.plasticity_rate[h], donor.plasticity_rate[h]);
        assert!(
            genes[INPUT_BASE + h * INPUTS..INPUT_BASE + (h + 1) * INPUTS]
                .iter()
                .all(|v| *v == value)
        );
    }
}

#[test]
fn weather_event_boundaries_do_not_flash_food_or_soil() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let cells = (RESOURCE_GRID * RESOURCE_GRID) as usize;
    for boundary in [997, 1994, 47003, 173003, 1_100_009] {
        let mut frames = Vec::new();
        for environmental_tick in [boundary - 1, boundary, boundary + 1] {
            s.reset(&q);
            q.write_buffer(
                &s.resource_buffer,
                0,
                bytemuck::cast_slice(&vec![1000u32; cells]),
            );
            q.write_buffer(
                &s.fertility_buffer,
                0,
                bytemuck::cast_slice(&vec![0.55f32; cells]),
            );
            let mut params = params_for(100_000, environmental_tick, &s.settings, s.seed);
            params.time_and_costs[1] = 1.0;
            q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
            let mut e = d.create_command_encoder(&Default::default());
            s.dispatch(&mut e, "resource", 0, 64, 64);
            q.submit(Some(e.finish()));
            frames.push((
                read::<u32>(&d, &q, &s.resource_buffer, cells),
                read::<f32>(&d, &q, &s.fertility_buffer, cells),
            ));
        }
        for pair in frames.windows(2) {
            assert!(
                pair[0]
                    .0
                    .iter()
                    .zip(&pair[1].0)
                    .all(|(a, b)| a.abs_diff(*b) <= 1)
            );
            let worst = pair[0]
                .1
                .iter()
                .zip(&pair[1].1)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0f32, f32::max);
            assert!(worst < 0.00001, "soil flash at {boundary}: {worst}");
        }
    }
}

#[test]
fn disappearing_food_fades_at_ecology_speed_and_conserves_loss_accounting() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut remaining = Vec::new();
    for tick in [0, 50_000, 100_000] {
        s.reset(&q);
        q.write_buffer(&s.resource_buffer, 0, bytemuck::bytes_of(&1000u32));
        q.write_buffer(&s.ground_buffer, 0, bytemuck::cast_slice(&[0u32; 8]));
        q.write_buffer(&s.ecology_buffer, 0, bytemuck::cast_slice(&[0.0f32; 4]));
        let mut params = params_for(tick, 0, &s.settings, s.seed);
        params.time_and_costs[0] = 0.0; // Isolate recession of a vanished patch.
        q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "resource", 0, 64, 64);
        q.submit(Some(e.finish()));
        let first = read::<u32>(&d, &q, &s.resource_buffer, 1)[0];
        assert!((990..1000).contains(&first));
        let mut e = d.create_command_encoder(&Default::default());
        for _ in 0..99 {
            s.dispatch(&mut e, "resource", 0, 64, 64);
        }
        q.submit(Some(e.finish()));
        let food = read::<u32>(&d, &q, &s.resource_buffer, 1)[0];
        let loss = read::<u32>(&d, &q, &s.ground_buffer, 8)[4];
        assert_eq!(food + loss, 1000);
        remaining.push(food);
    }
    assert!(remaining.windows(2).all(|pair| pair[0] == pair[1]));
    assert!(remaining[2] > 300);
}

#[test]
fn packet_physics_is_constant_across_world_ages_and_resume() {
    let (d, q) = gpu();
    for (tick, radius, upkeep) in [
        (0, 2.0, 0.02),
        (50_000, 2.0, 0.02),
        (100_000, 2.0, 0.02),
        (3_000_000, 2.0, 0.02),
    ] {
        near(packet_fusion_radius(tick), radius);
        near(packet_upkeep(tick), upkeep);
        for distance in [radius, radius + 0.01] {
            let mut s = scene(&d, &q);
            s.tick = tick;
            put(
                &s,
                &q,
                0,
                packet([100.0, 100.0], 1, 16.0),
                &fixed(0, [0.0; 2]),
            );
            put(
                &s,
                &q,
                1,
                packet([100.0 + distance, 100.0], 2, 16.0),
                &fixed(0, [0.0; 2]),
            );
            let path = temp("packet-ramp.checkpoint");
            s.save_checkpoint(&d, &q, &path).unwrap();
            step(&mut s, &d, &q, 1);
            let expected = s.agent_snapshot(&d, &q).unwrap();
            if distance == radius {
                let child = expected.iter().find(|a| a.alive == 1).unwrap();
                near(
                    child.energy,
                    32.0 - 2.0 * upkeep * 16.0f32.powf(2.0 / 3.0) - s.settings.fusion_loss,
                );
            } else {
                assert_eq!(expected.iter().filter(|a| a.alive == 2).count(), 2);
            }
            s.load_checkpoint(&q, &path).unwrap();
            step(&mut s, &d, &q, 1);
            assert_eq!(
                bytemuck::cast_slice::<AgentGpu, u8>(&expected),
                bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap())
            );
            std::fs::remove_file(path).unwrap();
        }
    }
    for tick in (0..100_000).step_by(100) {
        assert!(packet_upkeep(tick + 100) >= packet_upkeep(tick));
        assert!(packet_fusion_radius(tick + 100) <= packet_fusion_radius(tick));
    }
}

#[test]
fn pending_identity_rollover_keeps_ticking_without_allocating_or_fusing() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut producer = body([500.0, 500.0]);
    producer.age = 1800.0;
    producer.energy = 80.0;
    put(&s, &q, 0, producer, &fixed(5, [0.0; 2]));
    put(
        &s,
        &q,
        1,
        packet([100.0, 100.0], 1, 16.0),
        &fixed(0, [0.0; 2]),
    );
    put(
        &s,
        &q,
        2,
        packet([101.0, 100.0], 2, 16.0),
        &fixed(0, [0.0; 2]),
    );
    let last_id = u32::MAX - 2 * MAX_AGENTS;
    q.write_buffer(&s.death_stats_buffer, 10 * 4, bytemuck::bytes_of(&last_id));
    step(&mut s, &d, &q, 32);
    let counters = read::<u32>(&d, &q, &s.death_stats_buffer, DEATH_STATS_COUNT as usize);
    assert_eq!(s.tick, 32);
    assert_eq!(counters[10], last_id);
    assert_ne!(counters[36], 0);
    assert_eq!(counters[3], 0);
    assert_eq!(counters[19], 0);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(agents[0].packets_produced, 0);
    assert_eq!(agents[1].alive, 2);
    assert_eq!(agents[2].alive, 2);
    assert!(agents[1].energy < 16.0);
}

#[test]
fn water_nutrient_and_weather_drive_coverage_depletion_and_recovery() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let count = (RESOURCE_GRID * RESOURCE_GRID) as usize;
    let mut snapshots = Vec::new();
    let mut states = Vec::new();
    // Dispatch a 64x64 region, preserving the full-size world coordinates.
    for (water, rain, mineral) in [(1.0, 1.5, 3.0), (0.0, 0.0, 3.0), (1.0, 1.5, 0.0)] {
        s.reset(&q);
        q.write_buffer(
            &s.resource_buffer,
            0,
            bytemuck::cast_slice(&vec![0u32; count]),
        );
        q.write_buffer(
            &s.ground_buffer,
            0,
            bytemuck::cast_slice(&vec![0u32; count * 8]),
        );
        q.write_buffer(
            &s.fertility_buffer,
            0,
            bytemuck::cast_slice(&vec![0.55f32; count]),
        );
        q.write_buffer(
            &s.ecology_buffer,
            0,
            bytemuck::cast_slice(&vec![[water, mineral, 0.0f32, 0.0]; count]),
        );
        let mut params = params_for(0, 0, &s.settings, 91);
        params.time_and_costs[0] = rain;
        params.time_and_costs[1] = 0.1;
        params.world_size[3] = 0.5;
        params.mutation[2] = 0.0;
        q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
        let mut e = d.create_command_encoder(&Default::default());
        for _ in 0..512 {
            s.dispatch(&mut e, "resource", 0, 8, 8);
        }
        q.submit(Some(e.finish()));
        snapshots.push(read::<u32>(&d, &q, &s.resource_buffer, count));
        states.push(read::<[f32; 4]>(&d, &q, &s.ecology_buffer, count));
    }
    let total = |v: &Vec<u32>| v.iter().map(|x| u64::from(*x)).sum::<u64>();
    assert!(total(&snapshots[0]) > 1000);
    assert_eq!(total(&snapshots[1]), 0, "dry cells cannot grow broad cover");
    assert!(
        total(&snapshots[0]) > total(&snapshots[2]) * 2,
        "mineral limits wet growth"
    );
    // Restore the same dry physical state, then rain gradually replenishes water.
    q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&snapshots[1]));
    q.write_buffer(&s.ecology_buffer, 0, bytemuck::cast_slice(&states[1]));
    let mut params = params_for(0, 0, &s.settings, 91);
    params.time_and_costs[0] = 5.0;
    params.time_and_costs[1] = 0.1;
    params.world_size[3] = 0.5;
    params.mutation[2] = 0.0;
    q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
    let mut e = d.create_command_encoder(&Default::default());
    for _ in 0..3000 {
        s.dispatch(&mut e, "resource", 0, 8, 8);
    }
    q.submit(Some(e.finish()));
    let recovered = read::<u32>(&d, &q, &s.resource_buffer, count);
    assert!(
        total(&recovered) > 1000,
        "rain restores broad vegetation through stored water"
    );
    let pools = read::<[f32; 4]>(&d, &q, &s.ecology_buffer, count);
    for row in 0..64 {
        for x in 0..64 {
            let p = pools[row * 512 + x];
            assert!(p.iter().all(|v| v.is_finite() && *v >= 0.0));
            assert!(p[0] <= 2.0);
        }
    }
    assert!(
        (pools[0][0] - pools[63 * 512 + 63][0]).abs() > 0.01,
        "substrate/weather create spatial differences"
    );
}

#[test]
fn invalid_ecology_checkpoint_is_rejected_without_mutating_live_water() {
    use std::io::{Seek, SeekFrom, Write};
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let path = temp("invalid-ecology.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let before = read::<[f32; 4]>(&d, &q, &s.ecology_buffer, 1);
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::End(-16)).unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    drop(file);
    let mut s = s;
    assert!(
        s.load_checkpoint(&q, &path)
            .unwrap_err()
            .contains("ecological")
    );
    assert_eq!(before, read::<[f32; 4]>(&d, &q, &s.ecology_buffer, 1));
    std::fs::remove_file(path).unwrap();
}
