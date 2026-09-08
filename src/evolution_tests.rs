use super::*;
#[test]
fn streamed_birth_mutation_matches_cpu_and_preserves_parent_parameters() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, body([200.0, 200.0]), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let slot = agents
        .iter()
        .position(|a| a.alive != 0 && a.ancestry_depth == 1)
        .unwrap();
    let mut expected = parent;
    let mut expected_traits = agents[0].cognitive_traits();
    crate::brain::mutate_inherited(
        &mut expected,
        &mut expected_traits,
        agents[0].rng ^ slot as u32,
    );
    assert_eq!(agents[slot].active_mask, expected_traits.active_mask);
    near(agents[slot].mutation_scale, expected_traits.mutation_scale);
    let genes = s.read_genomes(&d, &q, slot + 1).unwrap();
    assert_eq!(&genes[..GENOME_SIZE], &parent);
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
    for i in 0..COUNT {
        let mut rng = i as u32 * 7919 + 31;
        let g = crate::brain::random_genome(&mut rng);
        let mut a = body([100.0, 100.0]);
        a.active_mask = crate::brain::random_active_mask(&mut rng);
        a.mutation_scale = if i % 2 == 0 { 0.25 } else { 4.0 };
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
        assert_ne!(actual_child, parent);
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
        near(actual_traits.mutation_scale, traits.mutation_scale);
    }
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
    put(&s, &q, 0, body([200.0, 200.0]), &genome);
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
        assert_eq!(a.target, b.target);
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
