use super::*;
#[test]
fn streamed_birth_mutation_matches_cpu_and_preserves_parent_parameters() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, packets::packet([200.0, 200.0], 1, 16.0), &parent);
    put(&s, &q, 1, packets::packet([200.0, 200.0], 2, 16.0), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let slot = agents
        .iter()
        .position(|a| a.alive == 1 && a.ancestry_depth == 1)
        .unwrap();
    let mut expected = parent;
    let pi = agents[slot].birth_parent_slot as usize;
    let mut expected_traits = agents[pi].cognitive_traits();
    crate::brain::mutate_inherited(&mut expected, &mut expected_traits, agents[pi].rng);
    assert_eq!(agents[slot].active_mask, expected_traits.active_mask);
    near(
        agents[slot].parameter_mutation_rate,
        expected_traits.parameter_mutation_rate,
    );
    near(
        agents[slot].parameter_mutation_step,
        expected_traits.parameter_mutation_step,
    );
    near(
        agents[slot].topology_mutation_rate,
        expected_traits.topology_mutation_rate,
    );
    let genes = s.read_genomes(&d, &q, slot + 1).unwrap();
    assert_eq!(&genes[pi * GENOME_SIZE..(pi + 1) * GENOME_SIZE], &parent);
    for (x, y) in genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
        .iter()
        .zip(expected)
    {
        near(*x, y);
    }
    assert_eq!(agents[slot].hidden, [0.0; HIDDEN]);
}

#[test]
fn masked_inheritance_matches_gpu_across_capacities_and_topology_changes() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    const COUNT: usize = 512;
    let mut expected = Vec::new();
    let mut expansions = 0;
    let mut retirements = 0;
    let mut exact = 0;
    let mut mutated = 0;
    for i in 0..COUNT {
        let mut rng = i as u32 * 7919 + 31;
        let g = crate::brain::random_genome(&mut rng);
        let mut a = body([100.0, 100.0]);
        a.active_mask = crate::brain::random_active_mask(&mut rng);
        a.parameter_mutation_rate = if i % 2 == 0 { 0.25 } else { 4.0 };
        a.parameter_mutation_step = if i % 2 == 0 { 4.0 } else { 0.25 };
        a.topology_mutation_rate = if i % 3 == 0 { 0.25 } else { 4.0 };
        a.plasticity_rate = [0.01; HIDDEN];
        a.trace_retention = 0.9;
        a.learned_weight_retention = 0.99;
        put(&s, &q, i, a, &g);
        put(&s, &q, i + COUNT, a, &[0.0; GENOME_SIZE]);
        let mut child = g;
        let mut traits = a.cognitive_traits();
        crate::brain::mutate_inherited(&mut child, &mut traits, i as u32 * 7919);
        expansions += usize::from(traits.active_mask & !a.active_mask != 0);
        retirements += usize::from(a.active_mask & !traits.active_mask != 0);
        expected.push((g, child, traits));
    }
    assert!(expansions > 0 && retirements > 0);
    let mut source =
        include_str!("../shaders/inherit_genomes.wgsl").replace("fn main(", "fn birth_main(");
    source.push_str("\n@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){inherit_child(id.x+512u,id.x,id.x*7919u);}");
    let dummy = buffer(&d, "unused birth fixture indices", MAX_AGENTS as u64 * 4);
    let pass = Compute::new(
        &d,
        "masked mutation parity",
        &source,
        "main",
        "wrrrruwww",
        vec![vec![
            &s.agent_buffers[0],
            &dummy,
            &dummy,
            &dummy,
            &dummy,
            &s.params_buffer,
            &s.genome_buffers[0],
            &s.genome_buffers[1],
            &s.death_stats_buffer,
        ]],
    );
    let mut e = d.create_command_encoder(&Default::default());
    pass.dispatch(&mut e, 0, (COUNT / 64) as u32, 1);
    q.submit(Some(e.finish()));
    let actual = s.read_genomes(&d, &q, COUNT * 2).unwrap();
    let agents = s.agent_snapshot(&d, &q).unwrap();
    for (i, (parent, child, traits)) in expected.iter().enumerate() {
        assert_eq!(&actual[i * GENOME_SIZE..(i + 1) * GENOME_SIZE], parent);
        let actual_child = &actual[(i + COUNT) * GENOME_SIZE..(i + COUNT + 1) * GENOME_SIZE];
        exact += usize::from(actual_child == parent);
        mutated += usize::from(actual_child != parent);
        for (x, y) in actual_child.iter().zip(child) {
            assert!((x - y).abs() < 0.00001, "genome {i}: {x} != {y}");
        }
        let actual_traits = agents[i + COUNT].cognitive_traits();
        assert!(actual_traits.validate());
        assert_eq!(actual_traits.active_mask, traits.active_mask);
        for (x, y) in actual_traits
            .plasticity_rate
            .iter()
            .zip(traits.plasticity_rate)
        {
            near(*x, y);
        }
        near(actual_traits.trace_retention, traits.trace_retention);
        near(
            actual_traits.learned_weight_retention,
            traits.learned_weight_retention,
        );
        near(
            actual_traits.parameter_mutation_rate,
            traits.parameter_mutation_rate,
        );
        near(
            actual_traits.parameter_mutation_step,
            traits.parameter_mutation_step,
        );
        near(
            actual_traits.topology_mutation_rate,
            traits.topology_mutation_rate,
        );
    }
    assert!(exact > 0 && mutated > 0);
}

#[test]
fn local_learning_is_masked_paid_and_retained_on_the_founders_first_tick() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.active_unit_upkeep = 0.002;
    s.settings.memory_write_energy = 0.01;
    let mut a = body([200.0, 200.0]);
    a.food = 0.0;
    a.active_mask = 1;
    a.plasticity_rate = [0.02; HIDDEN];
    a.trace_retention = 0.9;
    a.learned_weight_retention = 0.99;
    a.hidden[0] = 0.3;
    let mut genome = fixed(0, [0.0; 2]);
    genome[NODE_BIAS] = 0.5;
    genome[GATE_BIAS] = 0.75;
    put(&s, &q, 0, a, &genome);
    step(&mut s, &d, &q, 1);
    let after = s.agent_snapshot(&d, &q).unwrap()[0];
    let learned: Vec<_> = s
        .fast_weight_buffers
        .iter()
        .flat_map(|b| read::<f32>(&d, &q, b, FAST_BANK_STRIDE))
        .collect();
    let traces = read::<f32>(&d, &q, &s.trace_buffer, TRACE_COUNT);
    assert!(learned.iter().any(|v| v.abs() > 0.0));
    assert!(traces.iter().any(|v| v.abs() > 0.0));
    for h in 1..HIDDEN {
        assert_eq!(after.hidden[h], 0.0);
        assert_eq!(traces[INPUTS + h], 0.0);
        assert!(
            learned[h * INPUTS..(h + 1) * INPUTS]
                .iter()
                .all(|v| *v == 0.0)
        );
    }
    let change = learned.iter().chain(&traces).map(|x| x.abs()).sum::<f32>()
        + (after.hidden[0] - a.hidden[0]).abs();
    near(
        a.energy - after.energy,
        s.settings.metabolic_cost
            + s.settings.active_unit_upkeep
            + change * s.settings.memory_write_energy,
    );
    let metrics = s.metrics(&d, &q).unwrap();
    near(
        metrics.cognitive_upkeep_energy as f32,
        s.settings.active_unit_upkeep,
    );
    near(
        metrics.cognitive_write_energy as f32,
        change * s.settings.memory_write_energy,
    );
    assert_eq!(s.read_genomes(&d, &q, 1).unwrap(), genome);
    // A fault clears the entire lifetime state, including learned connections.
    genome[NODE_BIAS] = f32::NAN;
    put(&s, &q, 0, after, &genome);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.agent_snapshot(&d, &q).unwrap()[0].hidden, [0.0; HIDDEN]);
    for bank in &s.fast_weight_buffers {
        assert!(
            read::<f32>(&d, &q, bank, FAST_BANK_STRIDE)
                .iter()
                .all(|v| *v == 0.0)
        );
    }
    assert!(
        read::<f32>(&d, &q, &s.trace_buffer, TRACE_COUNT)
            .iter()
            .all(|v| *v == 0.0)
    );
}

#[test]
fn newborns_clear_reused_slot_traces_and_learned_connections() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let genome = fixed(5, [0.0; 2]);
    put(&s, &q, 0, packets::packet([200.0, 200.0], 1, 16.0), &genome);
    put(&s, &q, 1, packets::packet([200.0, 200.0], 2, 16.0), &genome);
    for bank in &s.fast_weight_buffers {
        q.write_buffer(
            bank,
            (FAST_BANK_STRIDE * 4) as u64,
            bytemuck::cast_slice(&vec![0.5f32; FAST_BANK_STRIDE]),
        );
    }
    q.write_buffer(
        &s.trace_buffer,
        (TRACE_COUNT * 4) as u64,
        bytemuck::cast_slice(&vec![0.5f32; TRACE_COUNT]),
    );
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(agents[1].ancestry_depth, 1);
    assert_eq!(agents[1].hidden, [0.0; HIDDEN]);
    for bank in &s.fast_weight_buffers {
        assert!(
            read::<f32>(&d, &q, bank, FAST_BANK_STRIDE * 2)[FAST_BANK_STRIDE..]
                .iter()
                .all(|v| *v == 0.0)
        );
    }
    assert!(
        read::<f32>(&d, &q, &s.trace_buffer, TRACE_COUNT * 2)[TRACE_COUNT..]
            .iter()
            .all(|v| *v == 0.0)
    );
}

#[test]
fn cooperative_decisions_match_serial_reference_with_masks_and_learning() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for i in 0..40 {
        let mut rng = i as u32 * 7919 + 31;
        let genome = crate::brain::random_genome(&mut rng);
        let mut a = body([100.0 + i as f32 * 30.0, 100.0]);
        a.active_mask = crate::brain::random_active_mask(&mut rng);
        a.plasticity_rate = [0.01; HIDDEN];
        a.trace_retention = 0.9;
        a.learned_weight_retention = 0.99;
        a.age = 0.0; // Keep this comparison free of births.
        put(&s, &q, i, a, &genome);
    }
    step(&mut s, &d, &q, 3);
    let mut encoder = d.create_command_encoder(&Default::default());
    s.dispatch(
        &mut encoder,
        "decide",
        s.current_buffer,
        MAX_AGENTS.div_ceil(64),
        1,
    );
    q.submit(Some(encoder.finish()));
    let reference = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 40);
    let mut encoder = d.create_command_encoder(&Default::default());
    s.passes["decide_live"].dispatch_indirect(
        &mut encoder,
        s.current_buffer,
        &s.cognitive_dispatch,
    );
    q.submit(Some(encoder.finish()));
    let actual = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 40);
    for (a, b) in actual.iter().zip(reference) {
        assert_eq!(a.selected_action, b.selected_action);
        assert_eq!(a.placement, b.placement);
        assert_eq!(a.invalid, b.invalid);
        for (x, y) in a
            .hidden
            .iter()
            .chain(&a.candidate)
            .chain(&a.outputs)
            .zip(b.hidden.iter().chain(&b.candidate).chain(&b.outputs))
        {
            assert!((x - y).abs() < 0.00001);
        }
    }
}

#[test]
fn simultaneous_births_replace_whole_reservoir_records_deterministically() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    const COUNT: usize = 512;
    for i in 0..COUNT {
        let mut g = fixed(5, [0.0; 2]);
        g[0] = i as f32 / COUNT as f32;
        let mut a = body([
            100.0 + (i % 32) as f32 * 30.0,
            100.0 + (i / 32) as f32 * 30.0,
        ]);
        a.alive = 2;
        a.energy = 16.0;
        a.food = 0.0;
        a.age = 0.0;
        a.lineage_id = i as u32 + 1;
        a.parent_lineage = a.lineage_id;
        a.hidden = [0.0; HIDDEN];
        put(&s, &q, i, a, &g);
        a.lineage_id += COUNT as u32;
        a.parent_lineage = a.lineage_id;
        put(&s, &q, MAX_AGENTS as usize - COUNT + i, a, &g);
    }
    let before = s.reservoir_snapshot(&d, &q).unwrap();
    step(&mut s, &d, &q, 1);
    let after = s.reservoir_snapshot(&d, &q).unwrap();
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let genes = s.read_genomes(&d, &q, MAX_AGENTS as usize).unwrap();
    let hash = |mut v: u32| {
        v = (v ^ 61) ^ (v >> 16);
        v = v.wrapping_add(v << 3);
        v ^= v >> 4;
        v = v.wrapping_mul(0x27d4eb2d);
        v ^ (v >> 15)
    };
    let mut replacement_claims = std::collections::BTreeMap::new();
    let children: Vec<_> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.alive == 1 && a.ancestry_depth == 1)
        .collect();
    assert_eq!(children.len(), COUNT);
    for (slot, _) in children {
        replacement_claims.insert(
            hash(before.2.wrapping_add(slot as u32)) as usize % HEREDITARY_RESERVOIR_SIZE as usize,
            slot,
        );
    }
    assert!(
        replacement_claims.len() < COUNT,
        "fixture exercises replacement collisions"
    );
    assert_eq!(after.2, before.2.wrapping_add(COUNT as u32));
    for slot in 0..HEREDITARY_RESERVOIR_SIZE as usize {
        let range = slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE;
        if let Some(&child) = replacement_claims.get(&slot) {
            assert_eq!(
                &after.0[range],
                &genes[child * GENOME_SIZE..(child + 1) * GENOME_SIZE]
            );
            assert_eq!(after.1[slot], agents[child].cognitive_traits());
        } else {
            assert_eq!(&after.0[range.clone()], &before.0[range]);
            assert_eq!(after.1[slot], before.1[slot]);
        }
    }
    // Consumed packets cannot fuse again; juvenile learning cannot alter the pool.
    step(&mut s, &d, &q, 1);
    assert!(s.reservoir_snapshot(&d, &q).unwrap() == after);
}

#[test]
fn reservoir_and_world_transitions_resume_without_observer_selection() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.population = 4;
    s.settings.metabolic_cost = 100.0;
    s.reset(&q);
    let original = s.reservoir_snapshot(&d, &q).unwrap();
    step(&mut s, &d, &q, 4);
    s.complete_world(&d, &q).unwrap();
    let path = temp("reservoir-resume.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    let observations = s.progress.clone();
    s.advance_world(&d, &q).unwrap();
    let founders = s.read_genomes(&d, &q, 4).unwrap();
    let bodies = s.agent_snapshot(&d, &q).unwrap();
    let seed = s.seed;
    assert_eq!(s.reservoir_snapshot(&d, &q).unwrap(), original);
    // Prior long durations must be valid even at tick zero of a fresh world.
    s.checkpoint_metadata().unwrap();
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.progress, observations);
    assert_eq!(s.reservoir_snapshot(&d, &q).unwrap(), original);
    for o in s
        .progress
        .history
        .iter_mut()
        .chain(s.progress.completed.iter_mut())
    {
        o.food_ingested += 123.0;
        o.food_collected += 456.0;
        o.energy += 789.0;
    }
    s.advance_world(&d, &q).unwrap();
    assert_eq!(s.seed, seed);
    assert_eq!(s.read_genomes(&d, &q, 4).unwrap(), founders);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&s.agent_snapshot(&d, &q).unwrap()),
        bytemuck::cast_slice::<AgentGpu, u8>(&bodies)
    );
    for (g, body) in founders.chunks_exact(GENOME_SIZE).zip(bodies.iter()) {
        assert!(
            original
                .0
                .chunks_exact(GENOME_SIZE)
                .zip(&original.1)
                .any(|(stored, traits)| stored == g && *traits == body.cognitive_traits())
        );
        assert_eq!(body.hidden, [0.0; HIDDEN]);
    }
    std::fs::remove_file(path).unwrap();
}

#[test]
fn capacity_skips_packet_requests_without_stopping_gameplay() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // Exercise a full body allocator without running thousands of brains.
    q.write_buffer(&s.birth_flags, 0, bytemuck::bytes_of(&1u32));
    let bodies = vec![body([100.0, 100.0]); MAX_AGENTS as usize];
    q.write_buffer(&s.agent_buffers[0], 0, bytemuck::cast_slice(&bodies));
    let before = s.reservoir_snapshot(&d, &q).unwrap();
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "free", 0, MAX_AGENTS.div_ceil(64), 1);
    s.scan(&mut e, "free", MAX_AGENTS);
    s.scan(&mut e, "birth", MAX_AGENTS);
    s.dispatch(&mut e, "birth_compact", 0, 1, 1);
    q.submit(Some(e.finish()));
    assert!(!s.refresh_engine_status(&d, &q).unwrap());
    assert_eq!(s.metrics(&d, &q).unwrap().capacity_blocked_packets, 1);
    assert_eq!(read::<u32>(&d, &q, &s.birth_dispatch, 3)[0], 0);
    assert!(s.complete_world(&d, &q).is_err());
    assert!(s.advance_world(&d, &q).is_err());
    assert!(s.progress.history.is_empty());
    assert_eq!(s.reservoir_snapshot(&d, &q).unwrap(), before);
}

#[test]
fn gathering_composes_with_reproduction_but_requires_a_neural_request() {
    let (d, q) = gpu();
    for effort in [0.0, 0.25, 1.0] {
        let mut s = scene(&d, &q);
        let mut g = fixed(5, [0.0; 2]);
        g[OUTPUT_BIAS + 1] = effort;
        let mut parent = body([602.0, 902.0]);
        parent.food = 0.0;
        put(&s, &q, 0, parent, &g);
        let mut partner = parent;
        partner.position[0] += 4.0;
        put_second_producer(&s, &q, partner, &g);
        q.write_buffer(
            &s.resource_buffer,
            (225 * 512 + 150) * 4,
            bytemuck::bytes_of(&1000u32),
        );
        step(&mut s, &d, &q, 1);
        let agents = s.agent_snapshot(&d, &q).unwrap();
        assert_eq!(agents[0].action, 5);
        assert_eq!(agents[1].alive, 2);
        assert_eq!(agents[1].ancestry_depth, 0);
        near(agents[0].collected, (25.0 * effort) as u32 as f32 / 1000.0);
        assert_eq!(agents[0].collected > 0.0, effort > 0.0);
    }
}

#[test]
fn gathering_effort_is_not_a_redundant_primary_action() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut g = fixed(0, [0.0; 2]);
    g[OUTPUT_BIAS + 1] = 3.0;
    put(&s, &q, 0, body([602.0, 902.0]), &g);
    q.write_buffer(
        &s.resource_buffer,
        (225 * 512 + 150) * 4,
        bytemuck::bytes_of(&1000u32),
    );
    step(&mut s, &d, &q, 1);
    let agent = s.agent_snapshot(&d, &q).unwrap()[0];
    assert_eq!(
        agent.action, 0,
        "gathering leaves the primary action as none"
    );
    assert!(
        agent.collected > 0.0,
        "gathering effort still harvests food"
    );
}

#[test]
fn blind_recombination_inherits_whole_modules_from_both_parents() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let mut a = body([100.0, 100.0]);
    let mut b = a;
    a.active_mask = 0x5555;
    b.active_mask = 0xaaaa;
    a.plasticity_rate = [0.1; HIDDEN];
    b.plasticity_rate = [-0.1; HIDDEN];
    a.trace_retention = 0.2;
    b.trace_retention = 0.8;
    put(&s, &q, 0, a, &[1.0; GENOME_SIZE]);
    put(&s, &q, 1, b, &[-1.0; GENOME_SIZE]);
    let mut source =
        include_str!("../shaders/inherit_genomes.wgsl").replace("fn main(", "fn birth_main(");
    source.push_str("\n@compute @workgroup_size(64) fn main(@builtin(global_invocation_id) id:vec3<u32>){if(id.x<32u){recombine_child(id.x+2u,0u,1u,id.x*7919u);}}");
    let dummy = buffer(&d, "unused indices", MAX_AGENTS as u64 * 4);
    let pass = Compute::new(
        &d,
        "blind module segregation",
        &source,
        "main",
        "wrrrruwwwrr",
        vec![vec![
            &s.agent_buffers[0],
            &dummy,
            &dummy,
            &dummy,
            &dummy,
            &s.params_buffer,
            &s.genome_buffers[0],
            &s.genome_buffers[1],
            &s.death_stats_buffer,
            &dummy,
            &s.agent_buffers[1],
        ]],
    );
    let mut e = d.create_command_encoder(&Default::default());
    pass.dispatch(&mut e, 0, 1, 1);
    q.submit(Some(e.finish()));
    let genes = s.read_genomes(&d, &q, 34).unwrap();
    let agents = s.agent_snapshot(&d, &q).unwrap();
    assert!(genes[..GENOME_SIZE].iter().all(|v| *v == 1.0));
    assert!(
        genes[GENOME_SIZE..2 * GENOME_SIZE]
            .iter()
            .all(|v| *v == -1.0)
    );
    let mut mixed = 0;
    for slot in 2..34 {
        let g = &genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE];
        let c = agents[slot];
        assert_ne!(c.active_mask, 0);
        assert!([0.2, 0.8].contains(&c.trace_retention));
        mixed += usize::from(g[..HIDDEN].contains(&1.0) && g[..HIDDEN].contains(&-1.0));
        for h in 0..HIDDEN {
            let value = g[NODE_BIAS + h];
            assert!([1.0, -1.0].contains(&value));
            let donor = if value == 1.0 { a } else { b };
            assert_eq!(c.active_mask & (1 << h), donor.active_mask & (1 << h));
            assert_eq!(c.plasticity_rate[h], donor.plasticity_rate[h]);
            assert_eq!(g[GATE_BIAS + h], value);
            assert!(
                g[INPUT_BASE + h * INPUTS..INPUT_BASE + (h + 1) * INPUTS]
                    .iter()
                    .all(|v| *v == value)
            );
            for base in [RECURRENT_BASE, GATE_BASE] {
                assert!(
                    g[base + h * HIDDEN..base + (h + 1) * HIDDEN]
                        .iter()
                        .all(|v| *v == value)
                );
            }
            for o in 0..OUTPUTS {
                assert_eq!(g[OUTPUT_BASE + o * HIDDEN + h], value);
            }
        }
    }
    assert!(mixed > 24);
}
