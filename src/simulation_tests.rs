use super::*;

#[test]
fn environment_rotation_preserves_body_traits_and_is_not_a_controller_input() {
    let mut settings = SimSettings::default();
    let original = build_agents(1201, &settings);
    for turns in 0..4 {
        settings.environment_rotation = turns;
        let rotated = build_agents(1201, &settings);
        for (a, b) in original.iter().zip(rotated) {
            let mut expected = *a;
            expected.position = crate::environment::rotate_point(a.position, WORLD_SIZE, turns);
            assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&b));
        }
        assert_eq!(params_for(10, &settings, 1201).lifecycle[2], turns);
    }
    for shader in [
        include_str!("../shaders/decide.wgsl"),
        include_str!("../shaders/perceive.wgsl"),
        include_str!("../shaders/update_agents.wgsl"),
        include_str!("../shaders/apply_births.wgsl"),
    ] {
        assert!(!shader.contains("lifecycle.z"));
    }
    settings.environment_rotation = 4;
    assert!(settings.validate().is_err());
}

#[test]
fn environment_rotation_permutates_resources_soil_and_weather_across_renewals() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.settings.resource_regeneration = 0.01;
    for tick in [0, 16384, 24576, 40960, 49152] {
        type EnvironmentSnapshot = (Vec<u32>, Vec<[u32; 8]>, Vec<f32>);
        let mut reference: Option<EnvironmentSnapshot> = None;
        for turns in 0..4 {
            s.settings.environment_rotation = turns;
            s.reset(&q);
            s.tick = tick;
            s.terrain_epoch = u32::MAX;
            step(&mut s, &d, &q, 32);
            let food = read::<u32>(
                &d,
                &q,
                &s.resource_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            let ground = read::<[u32; 8]>(
                &d,
                &q,
                &s.ground_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            let soil = read::<f32>(
                &d,
                &q,
                &s.fertility_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            if let Some((ref_food, ref_ground, ref_soil)) = &reference {
                assert_eq!(
                    food,
                    crate::environment::rotate_grid(
                        ref_food.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
                assert_eq!(
                    ground,
                    crate::environment::rotate_grid(
                        ref_ground.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
                assert_eq!(
                    soil,
                    crate::environment::rotate_grid(
                        ref_soil.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
            } else {
                reference = Some((food, ground, soil));
            }
        }
    }
}

#[test]
fn rotation_cli_and_checkpoint_preserve_explicit_environment_settings() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let args = |values: &[&str]| values.iter().map(|v| v.to_string()).collect::<Vec<_>>();
    crate::headless::configure(&mut s, &args(&["world", "--environment-rotation", "2"])).unwrap();
    s.reset(&q);
    let path = temp("rotated.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    s.settings.environment_rotation = 0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.settings.environment_rotation, 2);
    std::fs::remove_file(path).unwrap();
    assert!(
        crate::headless::configure(&mut s, &args(&["world", "--environment-rotation", "4"]))
            .is_err()
    );
    assert!(
        crate::headless::configure(
            &mut s,
            &args(&["world", "--environment-rotation", "1", "--checkpoint", "x"])
        )
        .is_err()
    );
    let value = serde_json::to_value(SimSettings::default()).unwrap();
    assert!(
        value.get("environment_rotation").is_none(),
        "Identity rotation is omitted from serialized settings"
    );
    assert_eq!(
        serde_json::from_value::<SimSettings>(value)
            .unwrap()
            .environment_rotation,
        0
    );
}

/// Explicit diagnostic: outcomes are measured, never required to point a chosen way.
/// Run separately after the frozen campaign; normal tests do not read research banks.
#[test]
#[ignore = "requires PRIMITIVE_DIRECTION_BANK and new PRIMITIVE_DIRECTION_OUTPUT path"]
fn directional_bank_gpu_probe() {
    let bank_path = std::env::var("PRIMITIVE_DIRECTION_BANK").expect("bank path");
    let output_path = std::env::var("PRIMITIVE_DIRECTION_OUTPUT").expect("new report path");
    let bank: crate::founders::FounderBank =
        serde_json::from_slice(&std::fs::read(&bank_path).unwrap()).unwrap();
    bank.validate().unwrap();
    let genomes = &bank.genomes;
    assert!(!genomes.is_empty() && genomes.len() <= 128);
    assert!(
        genomes
            .iter()
            .all(|g| g.len() == GENOME_SIZE && g.iter().all(|v| v.is_finite()))
    );
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output_path)
        .unwrap();
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let perception = |direction: Option<usize>, food: f32| {
        let mut p = PerceptionGpu::default();
        for b in &mut p.bodies {
            b.slot = MAX_AGENTS;
        }
        for (k, sample) in p.samples.iter_mut().enumerate() {
            let dir = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][k % 4];
            let range = if k < 4 { 4.0 } else { 24.0 };
            sample.offset = [dir[0] * range, dir[1] * range];
            sample.food = if direction == Some(k % 4) { food } else { 0.0 };
        }
        p
    };
    let dispatch = |p: PerceptionGpu| {
        q.write_buffer(
            &s.perception_buffer,
            0,
            bytemuck::cast_slice(&vec![p; genomes.len()]),
        );
        let mut encoder = d.create_command_encoder(&Default::default());
        s.dispatch(
            &mut encoder,
            "decide",
            s.current_buffer,
            (genomes.len() as u32).div_ceil(64),
            1,
        );
        q.submit(Some(encoder.finish()));
        let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, genomes.len());
        assert!(
            decisions
                .iter()
                .all(|x| x.invalid == 0 && x.movement.iter().all(|v| v.is_finite()))
        );
        decisions
    };
    let mut cases = Vec::new();
    for (energy, inventory) in [(10.0, 0.0), (50.0, 0.0), (50.0, 2.0), (100.0, 2.0)] {
        for food in [0.02, 0.2] {
            for (name, direction) in [
                ("bare", None),
                ("north", Some(0)),
                ("right", Some(1)),
                ("south", Some(2)),
                ("left", Some(3)),
            ] {
                for (slot, g) in genomes.iter().enumerate() {
                    let mut a = body([1026.0, 1026.0]);
                    a.energy = energy;
                    a.food = inventory;
                    put(&s, &q, slot, a, g.as_slice().try_into().unwrap());
                }
                let decisions = dispatch(perception(direction, food));
                let motors: Vec<_> = decisions
                    .iter()
                    .map(|v| [v.movement[0] * 1.2, v.movement[1] * 1.2])
                    .collect();
                cases.push(
                    serde_json::json!({"energy":energy,"inventory":inventory,"food_on_probes":food,
                    "food_side":name,"motors":motors}),
                );
            }
        }
    }
    let mut sequences = Vec::new();
    for food in [0.02, 0.2] {
        for first in [1usize, 3usize] {
            let mut bodies: Vec<_> = genomes
                .iter()
                .enumerate()
                .map(|(slot, g)| {
                    let mut a = body([1026.0, 1026.0]);
                    a.energy = 50.0;
                    a.food = 2.0;
                    put(&s, &q, slot, a, g.as_slice().try_into().unwrap());
                    a
                })
                .collect();
            let mut steps = Vec::new();
            for tick in 0..128 {
                let direction = if tick < 64 { first } else { 4 - first };
                let decisions = dispatch(perception(Some(direction), food));
                let motors: Vec<_> = decisions
                    .iter()
                    .map(|v| [v.movement[0] * 1.2, v.movement[1] * 1.2])
                    .collect();
                for ((a, decision), motor) in bodies.iter_mut().zip(&decisions).zip(&motors) {
                    a.hidden = decision.hidden;
                    a.velocity = *motor;
                    a.moved = *motor;
                    a.action = decision.selected_action;
                }
                q.write_buffer(
                    &s.agent_buffers[s.current_buffer],
                    0,
                    bytemuck::cast_slice(&bodies),
                );
                steps.push(
                    serde_json::json!({"step":tick+1,"food_direction":direction,"motors":motors}),
                );
            }
            sequences.push(
                serde_json::json!({"first_direction":first,"food_on_probes":food,"steps":steps}),
            );
        }
    }
    let report = serde_json::json!({"bank_path":bank_path,"bank_name":bank.name,"cases":cases,"sequences":sequences,
        "scope":"Actual GPU decision shader with synthetic mirrored perception, not full-world simulation. First decisions have empty state. Sequences hold adult age500, energy50, inventory2 and position fixed, carry hidden state, last action and motor feedback; cue reverses after64 of128 updates. No births, selection, sensing dispatch or ecological fitness measured."});
    std::io::Write::write_all(&mut output, &serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

#[test]
fn random_founders_are_finite_without_a_mandatory_food_response() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let bank = crate::founders::bundled();
    for energy in [10.0, 30.0, 50.0, 80.0] {
        let mut perceptions = vec![PerceptionGpu::default(); bank.genomes.len()];
        for (i, g) in bank.genomes.iter().enumerate() {
            let mut a = body([1026.0, 1026.0]);
            a.energy = energy;
            a.food = 0.0;
            put(&s, &q, i, a, g.as_slice().try_into().unwrap());
            perceptions[i].resource_here = 0.2;
            for b in &mut perceptions[i].bodies {
                b.slot = MAX_AGENTS;
            }
            for (k, sample) in perceptions[i].samples.iter_mut().enumerate() {
                sample.food = 0.2;
                let direction = [[0.0, -1.0], [1.0, 0.0], [0.0, 1.0], [-1.0, 0.0]][k % 4];
                let range = if k < 4 { 4.0 } else { 24.0 };
                sample.offset = [direction[0] * range, direction[1] * range];
            }
        }
        q.write_buffer(&s.perception_buffer, 0, bytemuck::cast_slice(&perceptions));
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "decide", s.current_buffer, 4, 1);
        q.submit(Some(e.finish()));
        let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, bank.genomes.len());
        let mut counts = [0u32; 6];
        for decision in decisions {
            assert_eq!(decision.invalid, 0);
            counts[decision.selected_action as usize] += 1;
        }
        println!("energy={energy} empty_inventory underfoot_food=0.2 action_counts={counts:?}");
        assert_eq!(counts.iter().sum::<u32>(), bank.genomes.len() as u32);
    }
}

#[test]
fn journey_observation_does_not_modify_physical_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 32);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    s.reset(&q);
    put(&s, &q, 0, a, &g);
    let mut observer = crate::journey_observer::JourneyObserver::default();
    for _ in 0..4 {
        step(&mut s, &d, &q, 8);
        observer
            .observe(
                s.tick,
                &s.agent_snapshot(&d, &q).unwrap(),
                &s.vegetation_snapshot(&d, &q).unwrap(),
            )
            .unwrap();
    }
    let actual = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&actual)
    );
}

/// Sensor invariance, without prescribing what random or evolved brains choose.
/// Paired gradients cancel unconditional drift; only the food field changes.
#[test]
fn fixed_compass_sensors_ignore_reserved_body_padding() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let bank = crate::founders::bundled();
    let mut summary = Vec::new();
    for angle in [
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -std::f32::consts::FRAC_PI_2,
    ] {
        for (slot, genome) in bank.genomes.iter().enumerate() {
            let mut a = body([1026.0, 1026.0]);
            a.energy = 50.0;
            a.food = 1.0;
            a.body_padding = angle;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, genome.as_slice().try_into().unwrap());
        }
        let mut paired = Vec::new();
        for sign in [1.0f32, -1.0] {
            let resources: Vec<u32> = (0..512 * 512)
                .map(|i| {
                    let x = ((i % 512) as f32 + 0.5) * 4.0;
                    ((0.4 + sign * (x - 1026.0) * 0.01).clamp(0.0, 1.0) * 1000.0).round() as u32
                })
                .collect();
            q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&resources));
            let mut e = d.create_command_encoder(&Default::default());
            s.dispatch(&mut e, "perceive", s.current_buffer, 4, 1);
            s.dispatch(&mut e, "decide", s.current_buffer, 4, 1);
            q.submit(Some(e.finish()));
            let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, bank.genomes.len());
            assert!(decisions.iter().all(|v| v.invalid == 0));
            // Food probes remain world-aligned regardless of reserved padding.
            let east_slot = [0usize, 1, 2, 3]
                .into_iter()
                .max_by(|&a, &b| {
                    decisions[0].inputs[21 + 3 * a].total_cmp(&decisions[0].inputs[21 + 3 * b])
                })
                .unwrap();
            assert!(decisions[0].inputs[21 + 3 * east_slot] > 0.16);
            paired.push(decisions);
        }
        let deltas: Vec<[f32; 2]> = paired[0]
            .iter()
            .zip(&paired[1])
            .map(|(east, west)| {
                [
                    (east.movement[0] - west.movement[0]) * 0.5,
                    (east.movement[1] - west.movement[1]) * 0.5,
                ]
            })
            .collect();
        let mean = |axis: usize| deltas.iter().map(|v| v[axis]).sum::<f32>() / deltas.len() as f32;
        let toward = deltas.iter().filter(|v| v[0] > 0.0).count();
        println!(
            "reserved_padding={angle:.6} paired_food_response=({:.6},{:.6}) toward_east={toward}/{}",
            mean(0),
            mean(1),
            deltas.len()
        );
        summary.push((mean(0), toward));
    }
    // Every tested padding value leaves the food response unchanged.
    for response in &summary[1..] {
        near(response.0, summary[0].0);
    }
    // Sensor geometry must be invariant; random brains need not seek food.
}

#[test]
fn displacement_does_not_impose_a_hidden_reproduction_penalty() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    let amount = 1.0 / (1.0 + (-3.0f32).exp());
    // Displacement does not directly deduct energy from the recipient.
    parent.energy = 50.0 * (0.2 + 0.8 * amount) - 0.5;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, body([604.0, 902.0]), &fixed(3, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.birth_gates[4], 1);
    assert_eq!(m.birth_gates[5], 1);
    assert_eq!(m.events[5], 1);
}
#[test]
fn dead_slot_reuse_resets_experience_and_advances_incarnation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut dead = body([500.0, 500.0]);
    dead.generation = 8;
    dead.hidden = [0.9; 16];
    put(&s, &q, 0, dead, &fixed(0, [0.0; 2]));
    s.kill_agents_in_region(&d, &q, [500.0, 500.0], 2.0);
    put(&s, &q, 1, body([602.0, 902.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    assert_eq!(bodies[0].generation, 9);
    assert_eq!(bodies[0].hidden, [0.0; 16]);
    assert_eq!(bodies[0].ancestry_depth, 1);
    assert_eq!(bodies[0].signal_tick, 0);
    near(s.metrics(&d, &q).unwrap().dropped_food as f32, 2.0);
}
#[test]
fn fresh_world_defaults_match_documented_physical_settings() {
    let settings = SimSettings::default();
    assert_eq!(settings.metabolic_cost, 0.06);
    assert_eq!(settings.movement_energy_cost, 0.01);
    assert_eq!(settings.motor_response_gain, 4.0);
    assert_eq!(settings.resource_regeneration, 0.01);
    assert_eq!(settings.population, 1000);
    assert!(settings.evolving_landscape);
    settings.validate().unwrap();
}

#[test]
fn settings_require_explicit_motor_response_and_reject_bad_gains() {
    let mut value = serde_json::to_value(SimSettings::default()).unwrap();
    value.as_object_mut().unwrap().remove("motor_response_gain");
    assert!(serde_json::from_value::<SimSettings>(value).is_err());
    let settings = SimSettings::default();
    for gain in [0.0, -1.0, 33.0, f32::NAN, f32::INFINITY] {
        let settings = SimSettings {
            motor_response_gain: gain,
            ..settings.clone()
        };
        assert!(settings.validate().is_err());
    }
}

#[test]
fn motor_response_is_continuous_optional_reversible_and_bounded() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.motor_response_gain = 8.0;
    for effort in [0.0f32, 0.01, -0.01, 4.0] {
        let mut g = fixed(0, [0.0; 2]);
        g[OUTPUT_BASE + 6 * 17 + 16] = effort;
        put(&s, &q, 0, body([602.0, 902.0]), &g);
        step(&mut s, &d, &q, 1);
        let a = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
        near(a.velocity[0], (effort * 8.0).tanh() * 1.2);
        near(a.velocity[1], 0.0);
        assert!(a.velocity[0].abs() <= 1.2001);
        near(a.spent, 0.06 + a.velocity[0].abs() * 0.01);
    }
}

#[test]
fn physical_cli_overrides_validate_and_cannot_override_checkpoints() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let args: Vec<String> = [
        "world",
        "--motor-gain",
        "8",
        "--metabolic-cost",
        "0.05",
        "--movement-cost",
        "0.02",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    crate::headless::configure(&mut s, &args).unwrap();
    assert_eq!(s.settings.motor_response_gain, 8.0);
    assert_eq!(s.settings.metabolic_cost, 0.05);
    assert_eq!(s.settings.movement_energy_cost, 0.02);
    for flag in ["--motor-gain", "--metabolic-cost", "--movement-cost"] {
        let args: Vec<String> = ["world", "--checkpoint", "unused.checkpoint", flag, "1"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!(
            crate::headless::configure(&mut s, &args)
                .unwrap_err()
                .contains("Checkpoint restores settings")
        );
    }
    let args: Vec<String> = ["world", "--motor-gain", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    assert!(crate::headless::configure(&mut s, &args).is_err());
}

#[test]
fn default_founders_use_the_declared_random_bank() {
    let settings = SimSettings::default();
    let bank = crate::founders::bundled();
    assert_eq!(bank.genomes.len(), 256);
    assert_eq!(settings.founder_genomes, bank.genomes);
    assert_eq!(settings.founder_name, bank.name);
    assert_eq!(bank.name, "primitive-world-random-256");
}

#[test]
fn concurrent_collection_and_pair_resolution_do_not_double_spend() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let pos = [602.0, 902.0];
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&17u32));
    for i in 0..8 {
        let mut a = body(pos);
        a.food = 0.0;
        a.lineage_id = i as u32 + 1;
        put(&s, &q, i, a, &fixed(1, [0.0; 2]));
    }
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    near(
        (m.carried_food + m.vegetation) as f32 + m.events[0] as f32 / 1000.0,
        0.017,
    );
    assert_eq!(m.living, 8);
    // Every body requests transfer to the same target; accepted pairs must be disjoint.
    for i in 0..8 {
        let mut a = body(pos);
        a.lineage_id = i as u32 + 1;
        a.food = if i == 7 { 0.0 } else { 1.0 };
        put(&s, &q, i, a, &fixed(0, [0.0; 2]));
    }
    let mut decisions = vec![DecisionGpu::default(); 8];
    for item in &mut decisions[..7] {
        item.selected_action = 2;
        item.target = 7;
        item.target_generation = 1;
        item.amount = 1.0;
    }
    q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(&decisions));
    let mut e = d.create_command_encoder(&Default::default());
    for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
        s.dispatch(&mut e, pass, s.current_buffer, MAX_AGENTS / 64, 1);
    }
    q.submit(Some(e.finish()));
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 8);
    near(bodies.iter().map(|a| a.food).sum(), 7.0);
    near(bodies[7].food, 1.0);
}
#[test]
fn stale_targets_out_of_range_and_disabled_actions_cannot_claim() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.force_enabled = false;
    s.update_params(&q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    let run = |s: &Simulation, decisions: &[DecisionGpu]| {
        q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(decisions));
        let mut e = d.create_command_encoder(&Default::default());
        for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
            s.dispatch(&mut e, pass, 0, MAX_AGENTS / 64, 1);
        }
        q.submit(Some(e.finish()));
        read::<AgentGpu>(&d, &q, &s.agent_buffers[0], 2)
    };
    let mut intent = DecisionGpu {
        selected_action: 2,
        target: 1,
        target_generation: 2,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent])[1].food, 0.0);
    intent.target_generation = 1;
    b.position = [620.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    near(run(&s, &[intent])[1].food, 0.0);
    b.position = [604.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    // Disabled force from receiver must not defeat a valid transfer in arbitration.
    let force = DecisionGpu {
        selected_action: 3,
        target: 0,
        target_generation: 1,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent, force])[1].food, 1.0);
}
#[test]
fn amount_and_failed_actions_remain_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(2, [0.2, 0.0]); // no target exists
    g[OUTPUT_BASE + 8 * 17 + 16] = -2.0;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let intent = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(b.action, 2);
    near(b.food + b.ingested, a.food);
    assert!(b.position[0] > a.position[0]);
    near(intent.amount, 1.0 / (1.0 + 2.0f32.exp()));
}
#[test]
fn founder_export_requires_descendants_and_preserves_existing_files() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    let path = temp("founders.json");
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert!(!path.exists());
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    s.export_founders(&d, &q, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    s.load_founders(&path).unwrap();
    assert_eq!(s.settings.founder_genomes.len(), 1);
    assert_eq!(s.settings.founder_genomes[0].len(), GENOME_SIZE);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn survivor_sample_keeps_current_child_genes_after_extinction() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, body([602.0, 902.0]), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let child_slot = agents
        .iter()
        .position(|a| a.alive != 0 && a.ancestry_depth > 0)
        .unwrap();
    let actual = read::<f32>(&d, &q, &s.genome_buffer, MAX_AGENTS as usize * GENOME_SIZE);
    let child = actual[child_slot * GENOME_SIZE..(child_slot + 1) * GENOME_SIZE].to_vec();
    assert_ne!(
        child.as_slice(),
        parent.as_slice(),
        "fixture must have real birth mutations"
    );
    let mut latest = None;
    crate::survivor_observer::observe(&mut latest, &s, &d, &q).unwrap();
    let saved = latest.as_ref().unwrap();
    let index = saved
        .bodies
        .iter()
        .position(|a| a.slot == child_slot)
        .unwrap();
    assert_eq!(saved.bank.genomes[index], child);
    assert!(saved.bodies.iter().any(|a| a.ancestry_depth == 0));
    let before = serde_json::to_vec(&latest).unwrap();
    let mut dead = agents;
    for a in &mut dead {
        a.alive = 0;
    }
    q.write_buffer(
        &s.agent_buffers[s.current_buffer],
        0,
        bytemuck::cast_slice(&dead),
    );
    s.tick += 128;
    crate::survivor_observer::observe(&mut latest, &s, &d, &q).unwrap();
    assert_eq!(serde_json::to_vec(&latest).unwrap(), before);
}
#[test]
fn visible_trial_has_no_tick_cutoff_and_exports_only_on_extinction_or_user_close() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let directory = temp("visible-extinction");
    let mut trial = crate::visible_trial::VisibleTrial::new(&directory, &s, &d, &q).unwrap();
    assert!(crate::visible_trial::VisibleTrial::new(&directory, &s, &d, &q).is_err());
    for tick in [8192, 200000, 1000001] {
        s.tick = tick;
        trial.observe(&s, &d, &q).unwrap();
        assert!(trial.finish(&s, &d, &q, false).is_err());
        assert!(!trial.finished);
        assert!(!directory.join("report.json").exists());
    }
    let dead = AgentGpu::default();
    q.write_buffer(
        &s.agent_buffers[s.current_buffer],
        0,
        bytemuck::bytes_of(&dead),
    );
    step(&mut s, &d, &q, 1);
    trial.finish(&s, &d, &q, false).unwrap();
    assert!(trial.finished);
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["termination_reason"], "extinction");
    assert!(report["tick_limit"].is_null());
    s.load_founders(&directory.join("survivors.bank.json"))
        .unwrap();
    assert_eq!(s.settings.founder_genomes.len(), 1);
    for name in ["ready.json", "report.json", "survivors.bank.json"] {
        std::fs::remove_file(directory.join(name)).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();

    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let directory = temp("visible-user-close");
    let mut trial = crate::visible_trial::VisibleTrial::new(&directory, &s, &d, &q).unwrap();
    trial.finish(&s, &d, &q, true).unwrap();
    let report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("report.json")).unwrap()).unwrap();
    assert_eq!(report["termination_reason"], "user_closed");
    assert_eq!(report["end"]["living"], 1);
    s.load_checkpoint(&q, &directory.join("paused.checkpoint"))
        .unwrap();
    for name in [
        "ready.json",
        "report.json",
        "survivors.bank.json",
        "paused.checkpoint",
    ] {
        std::fs::remove_file(directory.join(name)).unwrap();
    }
    std::fs::remove_dir(directory).unwrap();
}

#[test]
fn visible_trial_cli_rejects_headless_and_tick_limits() {
    let args = |items: &[&str]| items.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert!(crate::headless::arguments(&args(&["world", "--watch-loop", "new"])).is_ok());
    assert!(crate::headless::arguments(&args(&["world", "--watch-output", "new"])).is_err());
    for option in ["--headless", "--ticks", "--families", "--output"] {
        let mut values = args(&["world", "--watch-loop", "new", option]);
        if option == "--ticks" || option == "--output" {
            values.push("123".into());
        }
        assert!(crate::headless::arguments(&values).is_err());
    }
}

#[test]
fn native_visible_loop_reuses_simulation_and_carries_genomes_across_two_extinctions() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.population = 4;
    s.settings.metabolic_cost = 0.083;
    s.settings.motor_response_gain = 7.0;
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1); // Includes an actual newborn and its current mutations.
    let root = temp("native-visible-loop");
    let mut trial = crate::visible_trial::VisibleTrial::new_loop(&root, &s, &d, &q).unwrap();
    let first_save = trial.autosave(&s, &d, &q).unwrap();
    let second_save = trial.autosave(&s, &d, &q).unwrap();
    assert_ne!(first_save, second_save);
    assert_eq!(first_save.extension().unwrap(), "checkpoint");
    assert_eq!(
        std::fs::read(&first_save).unwrap(),
        std::fs::read(&second_save).unwrap()
    );
    s.load_checkpoint(&q, &first_save).unwrap();
    std::fs::remove_file(first_save).unwrap();
    std::fs::remove_file(second_save).unwrap();
    std::fs::remove_dir(root.join("checkpoints")).unwrap();
    assert!(trial.advance(&mut s, &d, &q).is_err());
    for expected_world in 2..=3 {
        trial.observe(&s, &d, &q).unwrap();
        let old_dir = trial.directory.clone();
        let old_seed = s.seed;
        let mut dead = s.agent_snapshot(&d, &q).unwrap();
        for body in &mut dead {
            body.alive = 0;
        }
        q.write_buffer(
            &s.agent_buffers[s.current_buffer],
            0,
            bytemuck::cast_slice(&dead),
        );
        step(&mut s, &d, &q, 1);
        trial.advance(&mut s, &d, &q).unwrap();
        assert_eq!(trial.world_number, expected_world);
        assert!(trial.is_loop());
        assert!(!trial.finished); // The same AppState must not exit its event loop.
        assert_eq!(s.tick, 0);
        assert_ne!(s.seed, old_seed);
        assert_eq!(s.settings.metabolic_cost, 0.083);
        assert_eq!(s.settings.motor_response_gain, 7.0);
        assert_eq!(s.metrics(&d, &q).unwrap().living, 4);
        let saved: crate::founders::FounderBank =
            serde_json::from_slice(&std::fs::read(old_dir.join("survivors.bank.json")).unwrap())
                .unwrap();
        assert_eq!(
            s.settings.founder_genomes[..saved.genomes.len()],
            saved.genomes
        );
        let transfer: serde_json::Value =
            serde_json::from_slice(&std::fs::read(old_dir.join("transfer.json")).unwrap()).unwrap();
        assert_eq!(transfer["provenance"].as_array().unwrap().len(), 256);
        for (child, p) in s
            .settings
            .founder_genomes
            .iter()
            .zip(transfer["provenance"].as_array().unwrap())
        {
            let parent = &saved.genomes[p["parent"].as_u64().unwrap() as usize];
            assert!(
                child
                    .iter()
                    .zip(parent)
                    .all(|(a, b)| (a - b).abs() <= 0.030001)
            );
        }
        for name in [
            "ready.json",
            "report.json",
            "survivors.bank.json",
            "next.bank.json",
            "transfer.json",
        ] {
            std::fs::remove_file(old_dir.join(name)).unwrap();
        }
        std::fs::remove_dir(old_dir).unwrap();
    }
    std::fs::remove_file(trial.directory.join("ready.json")).unwrap();
    std::fs::remove_dir(&trial.directory).unwrap();
    std::fs::remove_file(root.join("registration.json")).unwrap();
    std::fs::remove_dir(root).unwrap();
}
#[test]
fn checkpoint_rejects_corrupt_trace_without_mutating_live_world() {
    use std::io::{Seek, SeekFrom, Write};
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let path = temp("corrupt.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    assert!(s.save_checkpoint(&d, &q, &path).is_err());
    let lengths = [
        s.agent_buffers[0].size(),
        s.resource_buffer.size(),
        s.fertility_buffer.size(),
        s.ground_buffer.size(),
        s.death_stats_buffer.size(),
        s.event_buffer.size(),
    ];
    let pos = 24
        + serde_json::to_vec(&s.settings).unwrap().len() as u64
        + lengths.iter().map(|v| v + 8).sum::<u64>()
        + 8;
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(pos)).unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    drop(file);
    let before = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert!(
        s.load_checkpoint(&q, &path)
            .unwrap_err()
            .contains("perception")
    );
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&before),
        bytemuck::cast_slice::<AgentGpu, u8>(&after)
    );
    std::fs::remove_file(path).unwrap();
}
fn gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("GPU");
    pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("recurrent tests"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap()
}
fn read<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    b: &wgpu::Buffer,
    count: usize,
) -> Vec<T> {
    let staging = readback(device, (count * std::mem::size_of::<T>()) as u64);
    let mut e = device.create_command_encoder(&Default::default());
    e.copy_buffer_to_buffer(b, 0, &staging, 0, staging.size());
    queue.submit(Some(e.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let out = bytemuck::cast_slice(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    out
}
fn step(s: &mut Simulation, d: &wgpu::Device, q: &wgpu::Queue, n: u32) {
    let mut e = d.create_command_encoder(&Default::default());
    s.encode_ticks(&mut e, d, q, n);
    q.submit(Some(e.finish()));
    d.poll(wgpu::Maintain::Wait);
}
fn scene(d: &wgpu::Device, q: &wgpu::Queue) -> Simulation {
    let mut s = Simulation::new(d, q, 91);
    s.settings.population = 0;
    s.settings.resource_regeneration = 0.0;
    s.settings.evolving_landscape = false;
    s.reset(q);
    q.write_buffer(&s.resource_buffer, 0, &vec![0; 512 * 512 * 4]);
    q.write_buffer(&s.ground_buffer, 0, &vec![0; 512 * 512 * 32]);
    s
}
fn body(pos: [f32; 2]) -> AgentGpu {
    AgentGpu {
        position: pos,
        energy: 80.0,
        food: 2.0,
        age: 500.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        max_age: 11000.0,
        alive: 1,
        generation: 1,
        target: MAX_AGENTS,
        lineage_id: 1,
        ..Default::default()
    }
}
fn fixed(action: usize, motion: [f32; 2]) -> [f32; GENOME_SIZE] {
    let mut g = [0.0; GENOME_SIZE];
    g[OUTPUT_BASE + action * 17 + 16] = 2.0;
    g[OUTPUT_BASE + 6 * 17 + 16] = motion[0];
    g[OUTPUT_BASE + 7 * 17 + 16] = motion[1];
    g[OUTPUT_BASE + 8 * 17 + 16] = 3.0;
    if action == 3 {
        g[OUTPUT_BASE + 14 * 17 + 16] = 1.0;
    }
    g
}
fn put(s: &Simulation, q: &wgpu::Queue, slot: usize, a: AgentGpu, g: &[f32; GENOME_SIZE]) {
    for b in &s.agent_buffers {
        q.write_buffer(
            b,
            (slot * std::mem::size_of::<AgentGpu>()) as u64,
            bytemuck::bytes_of(&a),
        );
    }
    q.write_buffer(
        &s.genome_buffer,
        (slot * GENOME_SIZE * 4) as u64,
        bytemuck::cast_slice(g),
    );
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.002, "{a} != {b}");
}
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("recurrent-{}-{name}", std::process::id()))
}

#[test]
fn layout_and_cli_contract() {
    assert_eq!(GENOME_SIZE, 1760);
    assert_eq!(std::mem::size_of::<AgentGpu>(), 208);
    assert_eq!(std::mem::size_of::<PerceptionGpu>(), 272);
    assert_eq!(std::mem::size_of::<DecisionGpu>(), 440);
    assert_eq!(std::mem::size_of::<SelectionOutput>(), 928);
    assert_eq!(std::mem::size_of::<SimParams>(), 96);
    assert!(MAX_AGENTS as usize * GENOME_SIZE * 4 <= 128 * 1024 * 1024);
    for flag in [
        "--unknown-option",
        "--population-typo",
        "--unsupported-observer",
    ] {
        assert!(crate::headless::arguments(&["world".into(), flag.into()]).is_err());
    }
    let settings = SimSettings {
        sensor_radius: f32::NAN,
        ..Default::default()
    };
    assert!(settings.validate().is_err());
    assert!(crate::founders::validate_genomes(&[vec![0.0; 128]]).is_err());
}
#[test]
fn recurrent_cpu_gpu_parity_and_observer_isolation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.update_params(&q);
    let mut a = body([602.0, 902.0]);
    a.hidden = [0.1; 16];
    let mut g = [0.0; GENOME_SIZE];
    for (i, x) in g.iter_mut().enumerate() {
        *x = ((i % 17) as f32 - 8.0) * 0.015;
    }
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let expected = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    let mut hidden = [0.0; 16];
    for (h, value) in hidden.iter_mut().enumerate() {
        let row = h * RECURRENT_ROW;
        let mut v = g[row + RECURRENT_ROW - 1];
        for k in 0..crate::model::INPUTS {
            v += g[row + k] * expected.inputs[k];
        }
        for k in 0..16 {
            v += g[row + crate::model::INPUTS + k] * a.hidden[k];
        }
        *value = v.tanh();
        near(*value, expected.hidden[h]);
    }
    for o in 0..6 {
        let row = OUTPUT_BASE + o * 17;
        let mut v = g[row + 16];
        for h in 0..16 {
            v += g[row + h] * hidden[h];
        }
        near(v, expected.scores[o]);
    }
    a.lineage_id = 123456;
    a.ancestry_depth = 100;
    a.lifetime_births = 1000;
    a.distance_travelled = 30000.0;
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let actual = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(bytemuck::bytes_of(&actual), bytemuck::bytes_of(&expected));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_ne!(after.hidden, a.hidden);
    assert_eq!(read::<f32>(&d, &q, &s.genome_buffer, GENOME_SIZE), g);
}
#[test]
fn perception_is_local_and_compass_aligned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.body_padding = std::f32::consts::FRAC_PI_2;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let mut food = vec![0u32; 512 * 512];
    food[225 * 512 + 151] = 700;
    q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
    let mut far = body([1000.0, 1000.0]);
    far.lineage_id = 2;
    put(&s, &q, 1, far, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let p = read::<PerceptionGpu>(&d, &q, &s.perception_buffer, 1)[0];
    near(p.samples[1].offset[0], 4.0);
    near(p.samples[1].offset[1], 0.0);
    near(p.samples[1].food, 0.7);
    assert!(
        p.samples
            .iter()
            .all(|p| p.offset[0].hypot(p.offset[1]) <= 24.001)
    );
    assert!(p.bodies.iter().all(|b| b.slot == MAX_AGENTS));
    let renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8UnormSrgb, &s, 800, 600);
    drop(renderer);
}
#[test]
fn physical_collection_ingestion_and_movement_conserve_reserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.food = 0.0;
    a.energy = 50.0;
    put(&s, &q, 0, a, &fixed(1, [0.5, 0.0]));
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&1000u32));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
    near(
        food[idx as usize] as f32 / 1000.0 + after.food + after.ingested,
        1.0,
    );
    near(after.food + after.ingested, after.collected);
    near(after.energy + after.spent, 50.0 + 8.0 * after.ingested);
    assert!(after.velocity[0] > 0.0);
    assert!(after.collected > 0.0);
    let mut a = after;
    a.energy = 50.0;
    a.food = 1.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    near(after.energy + after.food * 8.0 + after.spent, 58.0);
    assert!(after.ingested > 0.0);
}
#[test]
fn digestion_is_inventory_limited_rate_limited_and_energy_capped() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // Independent of action amount; no food or energy is granted when empty.
    for (energy, inventory, expected) in [
        (50.0, 2.0, 0.1),
        (50.0, 0.03, 0.03),
        (50.0, 0.0, 0.0),
        (99.8, 2.0, 0.025),
        (100.0, 2.0, 0.0),
        (0.01, 0.1, 0.1),
    ] {
        let mut a = body([602.0, 902.0]);
        a.energy = energy;
        a.food = inventory;
        let mut g = fixed(0, [0.0; 2]);
        g[OUTPUT_BASE + 8 * 17 + 16] = -4.0;
        put(&s, &q, 0, a, &g);
        step(&mut s, &d, &q, 1);
        let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
        near(b.ingested, expected);
        near(b.food + b.ingested, inventory);
        near(b.energy + b.spent, energy + 8.0 * expected);
        assert!(b.energy <= 100.0 && b.food >= 0.0);
        assert_eq!(b.action, 0);
        assert_eq!(b.alive, 1);
    }
}

#[test]
fn automatic_digestion_does_not_gather_unrequested_ground_food() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 10.0;
    a.food = 0.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&1000u32));
    step(&mut s, &d, &q, 1);
    let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    near(b.collected, 0.0);
    near(b.ingested, 0.0);
    near(b.food, 0.0);
    near(b.energy, 9.94);
    assert_eq!(
        read::<u32>(&d, &q, &s.resource_buffer, 512 * 512)[idx as usize],
        1000
    );
}
#[test]
fn reproduction_is_requested_can_coexist_with_motion_and_conserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    a.hidden = [0.5; 16];
    let g = fixed(5, [0.5, 0.0]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let p = agents[0];
    let c = agents[1];
    assert_eq!(c.alive, 1);
    assert_eq!(c.ancestry_depth, 1);
    assert_eq!(c.parent_lineage, p.lineage_id);
    assert_eq!(c.hidden, [0.0; 16]);
    assert!(p.velocity[0] > 0.0);
    near(p.food + c.food + p.ingested, a.food);
    near(
        p.energy
            + c.energy
            + s.settings.metabolic_cost
            + p.velocity[0].abs() * s.settings.movement_energy_cost
            + 10.0,
        90.0 + 8.0 * p.ingested,
    );
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    let genes = read::<f32>(&d, &q, &s.genome_buffer, 2 * GENOME_SIZE);
    assert_eq!(&genes[..GENOME_SIZE], &g);
    assert!(genes.iter().all(|x| x.is_finite() && x.abs() <= 4.0));
}
#[test]
fn abundant_reserves_do_not_trigger_automatic_birth() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 100.0;
    a.food = 8.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 32);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
}
#[test]
fn transfer_and_signal_are_local_and_payload_is_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(2, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(
        bodies[0].food + bodies[1].food + bodies[0].ingested + bodies[1].ingested,
        2.0,
    );
    assert!(bodies[1].received > 0.0);
    let mut g = fixed(4, [0.0; 2]);
    g[OUTPUT_BASE + 9 * 17 + 16] = -0.7;
    let mut a = bodies[0];
    a.signal_tick = 0;
    s.tick = 10;
    put(&s, &q, 0, a, &g);
    put(&s, &q, 1, bodies[1], &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(bodies[0].signal_payload, (-0.7f32).tanh());
    assert_eq!(bodies[0].signal_tick, 11);
    assert_eq!(bodies[1].signal_tick, 0);
    step(&mut s, &d, &q, 1);
    let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 2);
    assert_eq!(decisions[1].inputs[51], 1.0);
    near(decisions[1].inputs[49], (-0.7f32).tanh());
}
#[test]
fn force_is_paid_displacement_without_recipient_damage_or_food_loss() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    let mut b = body([604.0, 902.0]);
    b.energy = 10.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(3, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let m = s.metrics(&d, &q).unwrap();
    near(agents[0].food + agents[0].ingested, a.food);
    near(
        (m.carried_food + m.dropped_food) as f32 + agents[0].ingested + agents[1].ingested,
        4.0,
    );
    assert_eq!(m.events[5], 1);
    near(m.dropped_food as f32, 0.0);
    near(agents[1].food + agents[1].ingested, b.food);
    near(
        agents[1].energy + s.settings.metabolic_cost,
        b.energy + 8.0 * agents[1].ingested,
    );
    assert!(agents[1].position[0] > b.position[0]);
    near(
        (m.energy + m.force_energy_spent) as f32,
        100.0 + 8.0 * (agents[0].ingested + agents[1].ingested) - 2.0 * s.settings.metabolic_cost,
    );
}

#[test]
fn force_direction_effort_and_available_energy_bound_actual_displacement() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for (effort, energy) in [(0.0f32, 10.0), (0.1, 10.0), (-1.0, 10.0), (1.0, 0.1)] {
        let mut actor = body([602.0, 902.0]);
        actor.energy = energy;
        actor.food = 0.0;
        let mut target = body([604.0, 902.0]);
        target.food = 0.0;
        target.lineage_id = 2;
        let mut genes = fixed(3, [0.0; 2]);
        genes[OUTPUT_BASE + 14 * 17 + 16] = effort;
        put(&s, &q, 0, actor, &genes);
        put(&s, &q, 1, target, &fixed(0, [0.0; 2]));
        step(&mut s, &d, &q, 1);
        let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
        let displacement = after[1].position[0] - target.position[0];
        let budget = (energy - s.settings.metabolic_cost).max(0.0);
        near(
            displacement,
            effort.signum() * (3.0 * effort.tanh().abs()).min(budget / 0.2),
        );
        near(after[0].energy + after[0].spent, energy);
        near(after[1].energy + after[1].spent, target.energy);
        near(
            after[0].spent,
            s.settings.metabolic_cost + displacement.abs() * 0.2,
        );
        assert!(after.iter().all(|a| a.energy >= 0.0 && a.food == 0.0));
    }
}

#[test]
fn zero_signal_is_present_local_and_does_not_claim_a_physical_pair() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let sender = body([602.0, 902.0]);
    let mut receiver = body([604.0, 902.0]);
    receiver.lineage_id = 2;
    let mut distant = body([1000.0, 1000.0]);
    distant.lineage_id = 3;
    put(&s, &q, 0, sender, &fixed(4, [0.0; 2]));
    put(&s, &q, 1, receiver, &fixed(3, [0.0; 2]));
    put(&s, &q, 2, distant, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 3);
    assert_eq!(after[0].signal_tick, 1);
    assert_eq!(after[0].signal_payload, 0.0);
    assert!(
        after[0].position[0] > sender.position[0],
        "Emission must not shield a body from contact"
    );
    step(&mut s, &d, &q, 1);
    let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 3);
    assert_eq!(decisions[1].inputs[50], 1.0, "Visible body presence");
    assert_eq!(decisions[1].inputs[51], 1.0, "Zero-valued signal presence");
    assert_eq!(decisions[1].inputs[49], 0.0);
    assert!(
        decisions[2].inputs[44..].iter().all(|v| *v == 0.0),
        "No remote signal leakage"
    );
}

#[test]
fn reproduction_requires_paid_energy_not_an_arbitrary_food_stockpile() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    assert_eq!(after[1].alive, 1);
    assert_eq!(after[0].food + after[1].food, 0.0);
    near(
        after[0].energy + after[1].energy + s.settings.metabolic_cost + 10.0,
        parent.energy,
    );
}

#[test]
fn contrast_preserves_mean_and_invalid_environment_settings_are_rejected() {
    let full = build_habitat_at(42, 3, 1.0);
    let uniform = build_habitat_at(42, 3, 0.0);
    let mean = |values: &[f32]| values.iter().map(|v| *v as f64).sum::<f64>() / values.len() as f64;
    assert!(uniform.iter().all(|v| *v == uniform[0]));
    assert!((mean(&full) - mean(&uniform)).abs() < 0.00001);
    for contrast in [-0.1, 1.1, f32::NAN] {
        let settings = SimSettings {
            habitat_contrast: contrast,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }
    assert_eq!(MODEL_ID, "primitive-world");
    assert_eq!(crate::founders::bundled().model, MODEL_ID);
    assert_eq!(crate::founders::bundled().version, 4);
}
#[test]
fn nonfinite_controller_output_is_contained() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(1, [1.0, 1.0]);
    g[0] = f32::NAN;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(after.action, 0);
    assert_eq!(after.position, a.position);
    assert!(after.hidden.iter().all(|x| x.is_finite()));
    assert_eq!(s.metrics(&d, &q).unwrap().invalid_outputs, 1);
}
#[test]
fn live_selection_follows_identity_without_changing_simulation_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    let original = s.select_agent(&d, &q, a.position, 2.0).unwrap();
    step(&mut s, &d, &q, 16);
    let buffers = [
        &s.agent_buffers[0],
        &s.agent_buffers[1],
        &s.genome_buffer,
        &s.resource_buffer,
        &s.ground_buffer,
    ];
    let before: Vec<_> = buffers
        .iter()
        .map(|buffer| observability::read_buffer(&d, &q, buffer).unwrap())
        .collect();
    let metrics_before = serde_json::to_value(s.metrics(&d, &q).unwrap()).unwrap();
    let current = s
        .refresh_selected_agent(&d, &q, &original)
        .unwrap()
        .unwrap();
    assert_eq!(current.selected, original.selected);
    assert_eq!(current.agent.lineage_id, a.lineage_id);
    assert!(current.agent.position[0] > a.position[0] + 2.0);
    assert_eq!(current.agent.age, a.age + 16.0);
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(
        bytemuck::bytes_of(&current.agent),
        bytemuck::bytes_of(&actual)
    );
    for (buffer, expected) in buffers.iter().zip(before) {
        assert_eq!(
            observability::read_buffer(&d, &q, buffer).unwrap(),
            expected
        );
    }
    assert_eq!(
        serde_json::to_value(s.metrics(&d, &q).unwrap()).unwrap(),
        metrics_before
    );
    assert_eq!(s.tick, 16);

    // A dead body can yield a terminal snapshot, but no replacement is followed.
    let mut dead = current.agent;
    dead.alive = 0;
    put(&s, &q, 0, dead, &g);
    assert_eq!(
        s.refresh_selected_agent(&d, &q, &original)
            .unwrap()
            .unwrap()
            .agent
            .alive,
        0
    );
    for component in 0..3 {
        let mut replacement = current.agent;
        match component {
            0 => replacement.generation += 1,
            1 => replacement.lineage_id += 1,
            _ => replacement.birth_tick += 1,
        }
        put(&s, &q, 0, replacement, &g);
        assert!(
            s.refresh_selected_agent(&d, &q, &original)
                .unwrap()
                .is_none()
        );
    }
    let invalid = SelectionOutput::default();
    assert!(
        s.refresh_selected_agent(&d, &q, &invalid)
            .unwrap()
            .is_none()
    );
    let invalid = SelectionOutput {
        selected: MAX_AGENTS + 1,
        ..original
    };
    assert!(
        s.refresh_selected_agent(&d, &q, &invalid)
            .unwrap()
            .is_none()
    );
}

#[test]
fn dead_selection_after_a_batch_does_not_claim_a_fresh_decision_trace() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.max_age = a.age + 1.0;
    put(&s, &q, 0, a, &fixed(0, [0.1, 0.2]));
    let original = s.select_agent(&d, &q, a.position, 2.0).unwrap();
    let mut view = crate::inspection::Inspection::default();
    view.select(Some(original), 0);
    // Death occurs on the first tick; remaining ticks clear the trace buffers.
    step(&mut s, &d, &q, 4);
    let result = s.refresh_selected_agent(&d, &q, &original);
    view.refresh(result, s.tick);
    let terminal = view.snapshot.unwrap();
    assert_eq!(terminal.agent.alive, 0);
    assert_eq!(terminal.agent.age, a.max_age);
    assert_eq!(terminal.decision.scores, [0.0; 6]);
    assert_eq!(terminal.decision.inputs, [0.0; 76]);
    assert!(!view.following);
    assert!(!view.has_decision_trace());
    assert_eq!(view.highlight().0, u32::MAX);
    assert!(view.notice.contains("terminal snapshot"));
}

#[test]
fn inspector_render_pipelines_accept_incarnation_aware_camera() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let mut renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8Unorm, &s, 1280, 820);
    assert_eq!(std::mem::size_of::<crate::renderer::CameraUniform>(), 32);
    renderer.camera.selected_id = 4;
    renderer.camera.selected_generation = 7;
    renderer.update_camera(&q);
    d.poll(wgpu::Maintain::Wait);
}

#[test]
fn batching_checkpoint_and_selection_preserve_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // Loading restores saved physical settings.
    s.settings.metabolic_cost = 0.06;
    s.settings.movement_energy_cost = 0.01;
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 12);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.reset(&q);
    put(&s, &q, 0, a, &g);
    for _ in 0..12 {
        step(&mut s, &d, &q, 1);
    }
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    let selection = s.select_agent(&d, &q, actual.position, 2.0).unwrap();
    assert_eq!(selection.selected, 1);
    let path = temp("state.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 8);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.settings.metabolic_cost = 0.005;
    s.settings.movement_energy_cost = 0.002;
    let saved_motor_gain = s.settings.motor_response_gain;
    s.settings.motor_response_gain = 16.0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.settings.metabolic_cost, 0.06);
    assert_eq!(s.settings.movement_energy_cost, 0.01);
    assert_eq!(s.settings.motor_response_gain, saved_motor_gain);
    step(&mut s, &d, &q, 8);
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    std::fs::remove_file(path).unwrap();
    let path = temp("unsupported.checkpoint");
    std::fs::write(&path, b"PRIMWORLD000").unwrap();
    let tick = s.tick;
    assert!(s.load_checkpoint(&q, &path).is_err());
    assert_eq!(s.tick, tick);
    assert!(path.exists());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn headless_extinction_stops_without_waiting_for_the_report_or_tick_limit() {
    for (label, population, metabolism, limit, expected_tick, reason) in [
        ("empty", "0", "0.06", "200000", 0, "extinction"),
        ("dies", "1", "100", "200000", 32, "extinction"),
        ("alive", "1", "0.06", "7", 7, "tick_limit"),
    ] {
        let report = temp(&format!("early-stop-{label}.json"));
        let journeys = temp(&format!("early-stop-{label}.jsonl"));
        let args: Vec<String> = [
            "world",
            "--headless",
            "--population",
            population,
            "--metabolic-cost",
            metabolism,
            "--ticks",
            limit,
            "--sample",
            "100000",
            "--journeys",
            journeys.to_str().unwrap(),
            "--journey-sample",
            "1000",
            "--output",
            report.to_str().unwrap(),
        ]
        .into_iter()
        .map(String::from)
        .collect();
        crate::headless::run(&args).unwrap();
        let output: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(output["elapsed_ticks"], expected_tick);
        assert_eq!(output["termination_reason"], reason);
        let history = output["history"].as_array().unwrap();
        assert_eq!(history.last().unwrap()["tick"], expected_tick);
        if reason == "extinction" {
            assert_eq!(history.last().unwrap()["living"], 0);
        }
        assert_eq!(history.len(), if expected_tick == 0 { 1 } else { 2 });
        let lines = std::fs::read_to_string(&journeys).unwrap();
        let footer: serde_json::Value =
            serde_json::from_str(lines.lines().last().unwrap()).unwrap();
        assert_eq!(footer["type"], "summary");
        assert_eq!(footer["observer"], output["journey_observer"]);
        std::fs::remove_file(report).unwrap();
        std::fs::remove_file(journeys).unwrap();
    }
}

#[test]
fn family_observer_counts_every_tick_and_preserves_dead_family_outcomes() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    let mut dying = body([1000.0, 1000.0]);
    dying.food = 0.0;
    dying.energy = 0.1;
    dying.founder_family = 1;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, dying, &fixed(0, [0.0; 2]));
    s.family_observer = Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 8).unwrap());
    let mut expected = [[0u32; 7]; 2];
    for tick in 1..=8 {
        step(&mut s, &d, &q, 1);
        for a in s
            .agent_snapshot(&d, &q)
            .unwrap()
            .iter()
            .filter(|a| a.alive != 0)
        {
            let row = &mut expected[a.founder_family as usize];
            row[5] = row[5].max(a.ancestry_depth);
            row[6] = tick;
            if a.ancestry_depth == 0 {
                row[0] += 1;
            } else {
                row[1] += 1;
                row[2] += u32::from(tick > 4);
                row[3] += u32::from(a.age >= s.settings.maturity_age);
                row[4] += u32::from(a.birth_tick + 1 == tick);
            }
        }
    }
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    for (actual, expected) in report.families.iter().zip(expected) {
        assert_eq!(
            [
                actual.founder_body_ticks,
                actual.descendant_body_ticks,
                actual.late_descendant_body_ticks,
                actual.mature_descendant_body_ticks,
                actual.births,
                actual.maximum_depth,
                actual.last_alive_tick
            ],
            expected
        );
    }
    assert_eq!(report.families[1].last_alive_tick, 1);
    assert_eq!(report.families[0].births, 1);
    let observed = s.agent_snapshot(&d, &q).unwrap();
    s.reset(&q);
    assert!(s.family_observer.is_none());
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, dying, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 8);
    let unobserved = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&observed),
        bytemuck::cast_slice::<AgentGpu, u8>(&unobserved)
    );
}

#[test]
fn family_diagnostics_record_underfunded_births_and_terminal_juvenile_deaths_once() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    let mut genes = fixed(5, [0.0; 2]);
    genes[OUTPUT_BASE + 8 * 17 + 16] = 0.0; // 20 energy, below stationary 24.
    put(&s, &q, 0, parent, &genes);
    s.family_observer =
        Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 2048).unwrap());
    for _ in 0..64 {
        step(&mut s, &d, &q, 32);
    }
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    let f = &report.families[0];
    assert!(f.births > 0);
    assert_eq!(f.births_below_stationary_maturity_energy, f.births);
    assert_eq!(f.birth_energy_milli, u64::from(f.births) * 20000);
    assert_eq!(f.juvenile_starvation_deaths, f.births);
    assert_eq!(f.matured_descendants, 0);
    assert_eq!(f.births_to_descendant_parents, 0);
    assert_eq!(f.collected_milli, 0);
    assert_eq!(f.juvenile_ingested_milli, 0);
    assert_eq!(f.juvenile_food_present_ticks, 0);
    assert!(f.juvenile_processed_ticks > 0);
    assert_eq!(s.metrics(&d, &q).unwrap().living, 0);
}

#[test]
fn family_diagnostics_count_juvenile_feeding_maturity_and_terminal_flow() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut juvenile = body([602.0, 902.0]);
    juvenile.age = 399.0;
    juvenile.ancestry_depth = 1;
    juvenile.energy = 0.01;
    juvenile.food = 0.0;
    // This injected fixture was born before the measured window, not a new birth.
    juvenile.birth_tick = u32::MAX;
    put(&s, &q, 0, juvenile, &fixed(1, [0.0; 2]));
    let cell = (902 / 4 * 512 + 602 / 4) as u64;
    q.write_buffer(&s.resource_buffer, cell * 4, bytemuck::bytes_of(&1000u32));
    s.family_observer = Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 3).unwrap());
    step(&mut s, &d, &q, 3);
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    let f = &report.families[0];
    assert_eq!(f.matured_descendants, 1);
    assert_eq!(f.juvenile_processed_ticks, 1);
    assert_eq!(f.juvenile_collect_action_ticks, 1);
    assert_eq!(f.juvenile_food_present_ticks, 1);
    assert_eq!(f.juvenile_food_present_collect_ticks, 1);
    assert!(f.juvenile_collected_milli > 0);
    assert!(f.juvenile_ingested_milli > 0);
    assert!(f.energy_at_maturity_milli > 0);
    let metrics = s.metrics(&d, &q).unwrap();
    assert_eq!(f.ingested_milli, u64::from(metrics.events[0]));
    assert_eq!(
        f.collected_milli,
        (metrics.harvested * 1000.0).round() as u64
    );
}
