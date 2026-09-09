use super::*;

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
    0.002 * size.powf(2.0 / 3.0)
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
        (2, 6.01, 16.0, 0, 0),
        (2, 6.0, 16.0, 1, 0),
        (1, 6.0, 16.0, 0, 0),
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
                (2.0 * (energy - decay(energy)) - s.settings.fusion_loss).min(48.0),
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
    step(&mut s, &d, &q, 200);
    let before = s.agent_snapshot(&d, &q).unwrap()[0];
    assert_eq!(before.alive, 2);
    near(
        before.energy,
        (0..200).fold(32.0, |e, t| e - packet_upkeep(t) * 32.0f32.powf(2.0 / 3.0)),
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
        before.energy - packet_upkeep(200) * 32.0f32.powf(2.0 / 3.0) + 8.0
            - packet_upkeep(200) * 8.0f32.powf(2.0 / 3.0)
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
    step(&mut s, &d, &q, 300);
    let a = s.agent_snapshot(&d, &q).unwrap()[0];
    assert_eq!(a.alive, 2);
    near(a.energy, (0..300).fold(1.0, |e, t| e - packet_upkeep(t)));
    assert_eq!(a.position, [100.0, 100.0]);
    assert_eq!(a.lived_ticks, 0);
    step(&mut s, &d, &q, 300);
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
fn opening_food_allowance_is_smooth_monotone_and_ends_at_one_hundred_thousand() {
    assert_eq!(opening_ground_cover(0), 0.5);
    assert_eq!(opening_ground_cover(50_000), 0.25);
    assert_eq!(opening_ground_cover(100_000), 0.0);
    assert_eq!(opening_ground_cover(u32::MAX), 0.0);
    for tick in (0..100_000).step_by(100) {
        assert!(opening_ground_cover(tick + 100) <= opening_ground_cover(tick));
    }
    assert!((opening_ground_cover(1) - opening_ground_cover(0)).abs() < 0.00001);
    assert!((opening_ground_cover(100_000) - opening_ground_cover(99_999)).abs() < 0.00001);
    assert_eq!(
        build_resources(&[0.0, 0.25, 0.5, 1.0]),
        [275, 275, 275, 550]
    );
    let settings = SimSettings::default();
    assert_eq!(
        params_for(50_000, 750_000, &settings, 1).time_and_costs[0],
        0.25
    );
    assert_eq!(params_for(0, 750_000, &settings, 1).time_and_costs[0], 0.5);
}

#[test]
fn ecology_clock_accelerates_smoothly_without_an_end_of_ramp_jump() {
    near(ecology_speed(0), 0.1);
    near(ecology_speed(50_000), 0.55);
    near(ecology_speed(100_000), 1.0);
    near(ecology_speed(u32::MAX), 1.0);
    assert_eq!(ecology_time(0), 0);
    assert_eq!(ecology_time(10), 1);
    assert_eq!(ecology_time(100_000), 55_000);
    assert_eq!(ecology_time(100_001), 55_001);
    assert_eq!(ecology_time(200_000), 155_000);
    for tick in 0..100_001 {
        assert!(ecology_time(tick + 1) >= ecology_time(tick));
        assert!(ecology_time(tick + 1) - ecology_time(tick) <= 1);
    }
}

#[test]
fn evolving_terrain_and_weather_resume_across_the_opening_ramp_boundary() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.settings.resource_regeneration = 0.01;
    s.reset(&q);
    s.tick = 99_998;
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
        )
    };
    let expected = finish(&mut s);
    s.load_checkpoint(&q, &path).unwrap();
    assert!(finish(&mut s) == expected);
    assert_eq!(s.tick, 100_003);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn opening_cover_preserves_rich_patch_capacity_and_survives_resume() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut amounts = Vec::new();
    for tick in [100_000, 50_000, 0] {
        s.reset(&q);
        q.write_buffer(&s.resource_buffer, 0, bytemuck::bytes_of(&16000u32));
        q.write_buffer(
            &s.ground_buffer,
            0,
            bytemuck::cast_slice(&[0u32, 0, 0, 0, 0, 0, 1.0f32.to_bits(), 1.0f32.to_bits()]),
        );
        let mut params = params_for(tick, 0, &s.settings, s.seed);
        params.environment[0] = 1.0; // Isolate food capacity from soil-speed changes.
        q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "resource", 0, 64, 64);
        q.submit(Some(e.finish()));
        amounts.push(read::<u32>(&d, &q, &s.resource_buffer, 1)[0]);
    }
    assert!(amounts[0] > 0);
    assert_eq!(amounts[1], amounts[0]);
    assert_eq!(amounts[2], amounts[0]);
    s.tick = 50_000;
    s.update_params(&q);
    let path = temp("opening-abundance.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let update_food = |s: &Simulation| {
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "resource", 0, 64, 64);
        q.submit(Some(e.finish()));
        read::<u32>(&d, &q, &s.resource_buffer, 1)[0]
    };
    let expected = update_food(&s);
    s.tick = 0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.tick, 50_000);
    assert_eq!(opening_ground_cover(s.tick), 0.25);
    assert_eq!(update_food(&s), expected);
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
fn opening_cover_feeds_barren_travel_space_and_fades_to_normal() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for initial in [0u32, 1000] {
        let mut amounts = Vec::new();
        for tick in [0, 50_000, 100_000] {
            s.reset(&q);
            q.write_buffer(&s.resource_buffer, 0, bytemuck::bytes_of(&initial));
            q.write_buffer(
                &s.ground_buffer,
                0,
                bytemuck::cast_slice(&[0u32, 0, 0, 0, 0, 0, 1.0f32.to_bits(), 0]),
            );
            q.write_buffer(&s.terrain_buffer, 0, &vec![0; 512 * 512 * 16]);
            let mut params = params_for(tick, 0, &s.settings, s.seed);
            params.mutation[2] = 1.0;
            params.time_and_costs[1] = 1.0;
            params.environment[0] = 1.0;
            q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
            let mut e = d.create_command_encoder(&Default::default());
            for _ in 0..64 {
                s.dispatch(&mut e, "resource", 0, 64, 64);
            }
            q.submit(Some(e.finish()));
            amounts.push(read::<u32>(&d, &q, &s.resource_buffer, 1)[0]);
        }
        assert!(amounts[0] > amounts[1], "{amounts:?}");
        assert!(amounts[1] > 0, "{amounts:?}");
        if initial == 0 {
            assert_eq!(amounts[2], 0);
        } else {
            assert!(amounts[2] > 0 && amounts[2] < amounts[1]);
        }
    }
    let habitat = build_habitat_at(42, 0, 1.0);
    let normal = habitat.iter().filter(|h| **h * 550.0 >= 1.0).count();
    let opening = build_resources(&habitat).iter().filter(|v| **v > 0).count();
    assert_eq!(opening, habitat.len());
    assert!(normal < opening);
    eprintln!(
        "Food coverage: opening {:.1}%, baseline {:.1}%",
        100.0,
        100.0 * normal as f64 / habitat.len() as f64
    );
}

#[test]
fn weather_event_boundaries_do_not_flash_food_or_soil() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let cells = (RESOURCE_GRID * RESOURCE_GRID) as usize;
    for boundary in [640, 1280, 1920, 2560, 3200] {
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
    assert!(remaining[0] > remaining[1] && remaining[1] > remaining[2]);
    assert!(remaining[2] > 300);
}

#[test]
fn packet_assistance_tracks_world_age_on_gpu_and_across_resume() {
    let (d, q) = gpu();
    for (tick, radius, upkeep) in [
        (0, 6.0, 0.002),
        (50_000, 4.0, 0.011),
        (100_000, 2.0, 0.02),
        (200_000, 2.0, 0.02),
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
