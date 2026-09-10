use super::*;

#[test]
#[ignore = "protected random-founder diagnostic; requires new PRIMITIVE_RANDOM_TRANSFER_REPORT path"]
fn random_founders_in_food_rich_transfer_arena() {
    let output =
        std::path::PathBuf::from(std::env::var("PRIMITIVE_RANDOM_TRANSFER_REPORT").unwrap());
    assert!(!output.exists());
    let (d, q) = gpu();
    let mut results = Vec::new();
    let count = 256;
    for seed in [7, 91, 2026, 4099] {
        let mut s = scene(&d, &q);
        s.seed = seed;
        s.settings.population = count as u32;
        s.settings.habitat_width = 256.0;
        s.settings.habitat_height = 256.0;
        s.settings.metabolic_cost = 0.0;
        s.settings.movement_energy_cost = 0.0;
        s.reset(&q);
        let mut bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], count);
        for a in &mut bodies {
            a.age = 1800.0;
            a.max_age = 1_000_000.0;
            a.food = 0.0;
        }
        let food = vec![8000u32; 512 * 512];
        let mut selectors = vec![0u32; count];
        let mut receivers = vec![0u32; count];
        for tick in 0..1000 {
            for (i, a) in bodies.iter_mut().enumerate() {
                // Sixteen separated groups of sixteen, all within ordinary
                // contact range in each group. No inventory grants.
                let group = i / 16;
                a.position = [
                    32.0 + (group % 4) as f32 * 56.0 + (i % 4) as f32 * 0.5,
                    32.0 + (group / 4) as f32 * 56.0 + ((i % 16) / 4) as f32 * 0.5,
                ];
                a.velocity = [0.0; 2];
                a.energy = 40.0;
            }
            let mut encoder = d.create_command_encoder(&Default::default());
            encoder.clear_buffer(
                &s.agent_buffers[s.current_buffer],
                (count * std::mem::size_of::<AgentGpu>()) as u64,
                None,
            );
            q.submit(Some(encoder.finish()));
            q.write_buffer(
                &s.agent_buffers[s.current_buffer],
                0,
                bytemuck::cast_slice(&bodies),
            );
            q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
            step(&mut s, &d, &q, 1);
            bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], count);
            for (i, a) in bodies.iter().enumerate() {
                assert_eq!(a.alive, 1, "protected founder must survive");
                selectors[i] += u32::from(a.action == 2);
                receivers[i] += u32::from(a.received > 0.0);
            }
            if tick % 250 == 249 {
                eprintln!("random seed {seed}: {} ticks", tick + 1);
            }
        }
        let m = s.metrics(&d, &q).unwrap();
        let row = serde_json::json!({"seed":seed,"organisms":count,"ticks":1000,
            "transfer_selections":m.action_ticks[2],"successful_transfers":m.events[4],
            "food_transferred_counter":m.events[6] as f64 / 1000.0,
            "organisms_selecting_transfer":selectors.iter().filter(|&&n| n>0).count(),
            "organisms_receiving_food":receivers.iter().filter(|&&n| n>0).count(),
            "action_ticks":m.action_ticks,"deaths":0});
        eprintln!("{row}");
        results.push(row);
    }
    let report = serde_json::json!({"results":results,
        "scope":"Four independent seeds, 256 ordinary random-founder genomes each, unmodified controllers. 1000 ticks each, 256x256 torus, guaranteed contact in groups of 16, full food each tick, zero starting inventory, energy restored to 40 each tick, zero configured body/movement/cognitive costs, mature starting age and extended lifespan. Ordinary gathering and actuator costs remain; new packets removed before next tick. Diagnostic protection is external, not inherited, not evolution. No live experiment writes."});
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
}

#[test]
#[ignore = "isolated saved-genome arena; requires a new PRIMITIVE_TRANSFER_PROBE_REPORT path"]
fn saved_genomes_in_food_rich_transfer_arena() {
    let output =
        std::path::PathBuf::from(std::env::var("PRIMITIVE_TRANSFER_PROBE_REPORT").unwrap());
    assert!(!output.exists(), "preserve earlier diagnostic reports");
    let (d, q) = gpu();
    let mut source = scene(&d, &q);
    let (mut saves, _) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    saves.sort_by_key(|s| std::cmp::Reverse(s.record.saved_at_ms));
    let saved = &saves[0];
    source
        .load_game_checkpoint(
            &q,
            std::fs::File::open(saved.checkpoint()).unwrap(),
            (saved.record.seed, saved.record.tick, saved.record.living),
            saved.record.world,
        )
        .unwrap();
    let agents = source.agent_snapshot(&d, &q).unwrap();
    let decisions = read::<DecisionGpu>(&d, &q, &source.decision_buffer, MAX_AGENTS as usize);
    let live: Vec<_> = (0..agents.len())
        .filter(|&i| agents[i].alive == 1)
        .collect();
    assert!(!live.is_empty());
    let margin = |i: usize| {
        decisions[i].scores[2]
            - [0, 3, 4, 5]
                .into_iter()
                .map(|a| decisions[i].scores[a])
                .fold(f32::NEG_INFINITY, f32::max)
    };
    let mut ranked = live.clone();
    ranked.sort_by(|&a, &b| margin(b).total_cmp(&margin(a)).then(a.cmp(&b)));
    let mut slots: Vec<_> = ranked.into_iter().take(16).collect();
    // Spread additional candidates across the whole population, then fill any
    // overlaps without a modular stride that could cycle through a small subset.
    for i in (0..16)
        .map(|n| live[n * live.len() / 16])
        .chain(live.iter().copied())
    {
        if !slots.contains(&i) {
            slots.push(i);
        }
        if slots.len() >= 32 {
            break;
        }
    }
    let genes = source.read_genome_slots(&d, &q, &slots).unwrap();
    let sample: Vec<_> = slots
        .iter()
        .map(|&i| {
            serde_json::json!({
                "source_slot":i, "lineage":agents[i].lineage_id,
                "saved_transfer_margin":margin(i), "saved_action":agents[i].action
            })
        })
        .collect();
    let mut results = Vec::new();
    let count = slots.len();
    for condition in [
        "free_movement",
        "guaranteed_contact",
        "contact_with_stock",
        "forced_transfer_control",
    ] {
        let mut s = scene(&d, &q);
        s.settings = source.settings.clone();
        s.settings.population = 0;
        s.settings.habitat_width = 256.0;
        s.settings.habitat_height = 256.0;
        s.settings.metabolic_cost = 0.0;
        s.settings.movement_energy_cost = 0.0;
        s.settings.active_unit_upkeep = 0.0;
        s.settings.memory_write_energy = 0.0;
        s.settings.resource_regeneration = 0.0;
        s.settings.evolving_landscape = false;
        s.reset(&q);
        q.write_buffer(&s.ground_buffer, 0, &vec![0; 512 * 512 * 32]);
        let mut bodies: Vec<_> = slots
            .iter()
            .map(|&i| {
                let mut a = agents[i];
                a.energy = 40.0;
                a.food = 0.0;
                a.age = 1800.0;
                a.max_age = 1_000_000.0;
                a.velocity = [0.0; 2];
                a.angular_velocity = 0.0;
                a.hidden = [0.0; HIDDEN];
                a.lived_ticks = 0;
                a.signal_tick = 0;
                a.signal_payload = 0.0;
                a.received = 0.0;
                a.collected = 0.0;
                a.ingested = 0.0;
                a.packets_produced = 0;
                a
            })
            .collect();
        for (i, a) in bodies.iter_mut().enumerate() {
            a.position = [126.0 + (i % 8) as f32 * 0.4, 126.0 + (i / 8) as f32 * 0.4];
            let mut g: [f32; GENOME_SIZE] = genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE]
                .try_into()
                .unwrap();
            if condition == "forced_transfer_control" {
                // Positive control only: override categorical readout, preserving
                // the sampled gathering, motor and amount outputs.
                for action in [0, 2, 3, 4, 5] {
                    g[OUTPUT_BIAS + action] =
                        if (action == 2 && i % 2 == 0) || (action == 0 && i % 2 == 1) {
                            10.0
                        } else {
                            -10.0
                        };
                    g[OUTPUT_BASE + action * HIDDEN..OUTPUT_BASE + (action + 1) * HIDDEN].fill(0.0);
                }
            }
            put(&s, &q, i, *a, &g);
        }
        let food = vec![8000u32; 512 * 512];
        let mut deaths = 0;
        let mut selected_by_body = vec![0u32; count];
        let mut received_by_body = vec![0u32; count];
        for tick in 0..1000 {
            for (i, a) in bodies.iter_mut().enumerate() {
                a.energy = 40.0;
                if condition != "free_movement" {
                    a.position = [126.0 + (i % 8) as f32 * 0.4, 126.0 + (i / 8) as f32 * 0.4];
                    a.velocity = [0.0; 2];
                }
                if condition == "contact_with_stock" || condition == "forced_transfer_control" {
                    a.food = a.food.max(0.5);
                }
            }
            // Keep the original sample only: remove manufactured packets before
            // their next contact phase, so this is not an evolving population.
            let mut encoder = d.create_command_encoder(&Default::default());
            encoder.clear_buffer(
                &s.agent_buffers[s.current_buffer],
                (count * std::mem::size_of::<AgentGpu>()) as u64,
                None,
            );
            q.submit(Some(encoder.finish()));
            q.write_buffer(
                &s.agent_buffers[s.current_buffer],
                0,
                bytemuck::cast_slice(&bodies),
            );
            q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
            step(&mut s, &d, &q, 1);
            bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], count);
            for (i, a) in bodies.iter().enumerate() {
                deaths += u32::from(a.alive != 1);
                selected_by_body[i] += u32::from(a.action == 2);
                received_by_body[i] += u32::from(a.received > 0.0);
            }
            assert_eq!(deaths, 0, "protected sample must survive");
            if tick % 250 == 249 {
                eprintln!("{condition}: {} ticks", tick + 1);
            }
        }
        let m = s.metrics(&d, &q).unwrap();
        let result = serde_json::json!({"condition":condition,"ticks":1000,"sample_size":count,
            "transfer_selections":m.action_ticks[2],"successful_transfers":m.events[4],
            "food_transferred_counter":m.events[6] as f64 / 1000.0,
            "action_ticks":m.action_ticks,"deaths":deaths,
            "transfer_selections_by_body":selected_by_body,"receiving_ticks_by_body":received_by_body});
        eprintln!("{result}");
        if condition == "forced_transfer_control" {
            assert!(m.events[4] > 0);
        }
        results.push(result);
    }
    let report = serde_json::json!({"source_receipt":saved.directory,"source_world":saved.record.world,
        "source_tick":saved.record.tick,"sample":sample,"results":results,
        "scope":"Isolated cloned inherited genomes and traits, fresh recurrent/learned state. 256x256 torus, full food reset each tick, reserves restored to 40 each tick, body/movement/cognitive costs zero, extended lifespan. Original gathering and actuator costs remain. Packets removed before next tick. Contact trials reposition samples each tick; stock trials top up inventory to 0.5. Forced control changes categorical output readout only. No live experiment writes."});
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
}
