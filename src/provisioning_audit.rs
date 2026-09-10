//! Paid packet endowment regression and comparison with the legacy clipping rule.
use super::*;
use serde_json::json;

#[test]
fn packet_endowment_is_continuous_paid_and_does_not_change_low_investment_births() {
    let (d, q) = gpu();
    for size in [2.0_f32, 8.0, 16.0, 24.0, 32.0, 40.0, 48.0] {
        for retain in [false, true] {
            let mut s = scene(&d, &q);
            assert!(
                s.retain_packet_endowment,
                "Production defaults must retain paid energy"
            );
            s.retain_packet_endowment = retain;
            for slot in 0..2 {
                put(
                    &s,
                    &q,
                    slot,
                    packets::packet([100.0 + slot as f32, 100.0], slot as u32 + 1, size),
                    &[0.0; GENOME_SIZE],
                );
            }
            step(&mut s, &d, &q, 1);
            let remaining = 2.0 * (size - 0.02 * size.powf(2.0 / 3.0)) - s.settings.fusion_loss;
            let bodies = s.agent_snapshot(&d, &q).unwrap();
            let child = bodies.iter().find(|a| a.alive == 1);
            if remaining <= 0.0 {
                assert!(child.is_none());
                continue;
            }
            let child = child.unwrap();
            near(
                child.energy,
                if retain {
                    remaining
                } else {
                    remaining.min(48.0)
                },
            );
            assert_eq!(child.food, 0.0);
            assert_eq!(child.age, 0.0);
            assert!(
                child.energy < 90.0,
                "Even maximum investment cannot pay 1800 ticks of basal maintenance without food"
            );
        }
    }
}

#[test]
#[ignore = "predeclared packet-investment sweep in ordinary ecology; controlled sensor foraging and no transfers"]
fn packet_provisioning_sensor_sweep() {
    use std::io::Write;
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("packet-provisioning.json"))
        .unwrap();
    let (d, q) = gpu();
    let ordinary = std::env::var("PRIMITIVE_ORDINARY_PLACEMENTS").as_deref() == Ok("1");
    let mut placements = Vec::new();
    if ordinary {
        for seed in [4101, 4102] {
            let source = Simulation::new(&d, &q, seed);
            for a in source
                .agent_snapshot(&d, &q)
                .unwrap()
                .iter()
                .filter(|a| a.alive == 1)
                .take(4)
            {
                placements.push((seed, a.position));
            }
        }
    } else {
        placements.push((91, [1254.0, 810.0]));
    }
    let mut rows = Vec::new();
    for (seed, origin) in placements {
        for retain in [false, true] {
            for size in if ordinary {
                vec![48.0_f32]
            } else {
                vec![16.0_f32, 24.0, 32.0, 40.0, 48.0]
            } {
                for forage in if ordinary {
                    vec![true]
                } else {
                    vec![false, true]
                } {
                    let mut s = Simulation::new(&d, &q, seed);
                    s.settings.population = 0;
                    s.reset(&q);
                    s.retain_packet_endowment = retain;
                    // Initialization is a physical-investment fixture, not random/evolved founders.
                    for slot in 0..2 {
                        let mut packet = packets::packet(
                            [origin[0] + slot as f32, origin[1]],
                            slot as u32 + 1,
                            size,
                        );
                        packet.active_mask = 1;
                        put(&s, &q, slot, packet, &fixed(0, [0.0; 2]));
                    }
                    step(&mut s, &d, &q, 1);
                    let bodies = s.agent_snapshot(&d, &q).unwrap();
                    let ci = bodies.iter().position(|a| a.alive == 1).unwrap();
                    let initial = bodies[ci].energy;
                    let mut memory = sensor_audit::Memory::default();
                    let mut collected = 0.0_f64;
                    let mut ingested = 0.0_f64;
                    let mut spent = 0.0_f64;
                    let mut trajectory = Vec::new();
                    loop {
                        let before = s.agent_snapshot(&d, &q).unwrap()[ci];
                        if before.alive != 1 || before.age >= 1800.0 {
                            break;
                        }
                        let genes = if forage {
                            let decisions = sensor_audit::inputs(&s, &d, &q, 2);
                            sensor_audit::control(
                                &sensor_audit::View::from_inputs(&decisions[ci].inputs),
                                &mut memory,
                                0,
                            )
                        } else {
                            fixed(0, [0.0; 2])
                        };
                        s.write_genome_slot(&q, ci, &genes);
                        // Controller uses a four-tick commanded-spin memory; collect the ledger each tick.
                        for _ in 0..4 {
                            step(&mut s, &d, &q, 1);
                            let a = s.agent_snapshot(&d, &q).unwrap()[ci];
                            collected += a.collected as f64;
                            ingested += a.ingested as f64;
                            spent += a.spent as f64;
                            if (a.age as u32).is_multiple_of(100) || a.alive != 1 {
                                trajectory
                                    .push(json!({"age":a.age,"energy":a.energy,"food":a.food}));
                            }
                            if a.alive != 1 || a.age >= 1800.0 {
                                break;
                            }
                        }
                    }
                    let a = s.agent_snapshot(&d, &q).unwrap()[ci];
                    let row = json!({"seed":seed,"origin":origin,"ordinary_placement":ordinary,"retain_paid_endowment":retain,"packet_size_each":size,"sensor_foraging":forage,"birth_energy":initial,"final_age":a.age,"alive":a.alive==1,"matured":a.alive==1 && a.age>=1800.0,"final_energy":a.energy,"final_food":a.food,"gathered":collected,"ingested":ingested,"spent":spent,"energy_ledger_residual":initial as f64+ingested*8.0-spent-a.energy as f64,"trajectory":trajectory});
                    eprintln!(
                        "provision retain={retain} size={size} forage={forage} age={} energy={}",
                        a.age, a.energy
                    );
                    rows.push(row);
                }
            }
        }
    }
    file.write_all(&serde_json::to_vec_pretty(&json!({"results":rows,"scope":"Selected seed91 corridor by default; ordinary-placement mode uses first four standard founder positions from each of predeclared seeds4101 and4102, without food selection. Actual fusion of two initialized packets, no reserve edits after fusion, no food painting or climate/cost/gathering changes. One-unit controlled readout; no transfer/donors. Foragers use existing real-region sensor controller, rest controls neither gather nor move. Tests investment consequences, not normal adult funding or evolution. C retains energy paid into packets after normal decay and fixed fusion loss, even above the normal digestion ceiling; B clips birth energy to48."})).unwrap()).unwrap();
}

#[test]
fn paid_newborn_surplus_survives_checkpoint_and_replays_exactly() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for slot in 0..2 {
        put(
            &s,
            &q,
            slot,
            packets::packet([100.0 + slot as f32, 100.0], slot as u32 + 1, 48.0),
            &fixed(0, [0.0; 2]),
        );
    }
    step(&mut s, &d, &q, 1);
    let initial = s.agent_snapshot(&d, &q).unwrap();
    assert!(
        initial
            .iter()
            .any(|a| a.alive == 1 && a.age == 0.0 && a.energy > 85.0)
    );
    let path = temp("paid-newborn-surplus.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 32);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    let resources = observability::read_buffer(&d, &q, &s.resource_buffer).unwrap();
    s.load_checkpoint(&q, &path).unwrap();
    let loaded = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&initial),
        bytemuck::cast_slice::<AgentGpu, u8>(&loaded)
    );
    step(&mut s, &d, &q, 32);
    let actual = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&actual)
    );
    assert_eq!(
        resources,
        observability::read_buffer(&d, &q, &s.resource_buffer).unwrap()
    );
    std::fs::remove_file(path).unwrap();
}

#[test]
fn legacy_opportunity_observer_separates_actual_shortage_from_fraction_veto() {
    let (d, q) = gpu();
    for (energy, shortage, veto, produced) in [(10.0, 1, 0, 0), (35.0, 0, 1, 0), (50.0, 0, 0, 1)] {
        let mut s = scene(&d, &q);
        budget_gate_audit::install_legacy_fraction_gate(&mut s, &d);
        let mut a = body([100.0, 100.0]);
        a.energy = energy;
        a.food = 0.0;
        a.packet_size = 24.0;
        let mut g = fixed(5, [0.0; 2]);
        g[OUTPUT_BIAS + 8] = 0.0;
        put(&s, &q, 0, a, &g);
        s.funnel_observer = Some(funnel_audit::FunnelObserver::new(&d, &s));
        step(&mut s, &d, &q, 1);
        let c = s.funnel_observer.as_ref().unwrap().counts(&d, &q);
        assert_eq!((c[23], c[24], c[25], c[2]), (shortage, veto, 0, produced));
    }
}

#[test]
fn juvenile_contact_opportunities_separate_presence_food_and_intent() {
    let (d, q) = gpu();
    for (food, action, expected) in [
        (0.0, 0, [1, 0, 0]),
        (1.0, 0, [1, 1, 0]),
        (1.0, 2, [1, 1, 1]),
    ] {
        let mut s = scene(&d, &q);
        let mut child = body([100.0, 100.0]);
        child.age = 0.0;
        child.ancestry_depth = 1;
        child.food = 0.0;
        let mut neighbor = body([103.0, 100.0]);
        neighbor.lineage_id = 2;
        neighbor.energy = 100.0;
        neighbor.food = food;
        put(&s, &q, 0, child, &fixed(0, [0.0; 2]));
        put(&s, &q, 1, neighbor, &fixed(action, [0.0; 2]));
        s.funnel_observer = Some(funnel_audit::FunnelObserver::new(&d, &s));
        step(&mut s, &d, &q, 1);
        let c = s.funnel_observer.as_ref().unwrap().contact_counts(&d, &q)[0];
        assert_eq!(c[2..5], expected);
        assert_eq!(c[5], 1);
        let bodies = s.agent_snapshot(&d, &q).unwrap();
        assert_eq!(bodies[0].received > 0.0, action == 2);
    }
}
