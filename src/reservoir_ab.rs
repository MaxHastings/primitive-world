//! Test-only matched continuations with newborn versus mature-parent admission.
use super::*;
use serde_json::json;
use std::io::Write;

#[test]
fn mature_parent_admission_preserves_physical_birth_results() {
    let (d, q) = gpu();
    let mut a = Simulation::new(&d, &q, 91);
    let mut b = Simulation::new(&d, &q, 91);
    for s in [&mut a, &mut b] {
        s.settings.population = 0;
        s.reset(&q);
        for slot in 0..2 {
            let mut agent = body([100.0 + slot as f32, 100.0]);
            agent.energy = 35.0;
            agent.food = 0.0;
            agent.active_mask = 1;
            agent.lineage_id = slot as u32 + 1;
            put(s, &q, slot, agent, &fixed(5, [0.0; 2]));
        }
    }
    let original = b.reservoir_snapshot(&d, &q).unwrap();
    assert_eq!(a.reservoir_snapshot(&d, &q).unwrap(), original);
    b.parent_admission = Some(parent_admission::ParentAdmission::new(&d, &b));
    step(&mut a, &d, &q, 2);
    step(&mut b, &d, &q, 2);
    let am = a.metrics(&d, &q).unwrap();
    let bm = b.metrics(&d, &q).unwrap();
    assert!(am.events[3] > 0);
    assert_eq!(am.events, bm.events);
    assert_ne!(b.reservoir_snapshot(&d, &q).unwrap(), original);
    assert_eq!(
        a.reservoir_snapshot(&d, &q).unwrap().2,
        b.reservoir_snapshot(&d, &q).unwrap().2
    );
    assert!(a.reservoir_snapshot(&d, &q).unwrap() != original);
    let mut aa = a.agent_snapshot(&d, &q).unwrap();
    let mut bb = b.agent_snapshot(&d, &q).unwrap();
    for body in aa.iter_mut().chain(bb.iter_mut()) {
        body.lineage_id = 0;
    }
    assert!(bytemuck::cast_slice::<AgentGpu, u8>(&aa) == bytemuck::cast_slice::<AgentGpu, u8>(&bb));
    for (left, right) in [
        (&a.resource_buffer, &b.resource_buffer),
        (&a.ecology_buffer, &b.ecology_buffer),
    ] {
        assert!(
            observability::read_buffer(&d, &q, left).unwrap()
                == observability::read_buffer(&d, &q, right).unwrap()
        );
    }
}

#[test]
#[ignore = "paired 1M-tick continuations of exactly the same saved hereditary state"]
fn reservoir_admission_ab() {
    let root = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    let provision = std::env::var("PRIMITIVE_C_PROVISIONING").as_deref() == Ok("1");
    let checkpoint = std::env::var("PRIMITIVE_AB_CHECKPOINT").ok();
    let (d, q) = gpu();
    for (label, parent) in [(
        if provision {
            "C-paid-endowment-fresh"
        } else {
            "B-mature-parent"
        },
        true,
    )] {
        let output = root.join(label);
        std::fs::create_dir_all(&output).unwrap();
        let create = |name: &str| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output.join(name))
                .unwrap()
        };
        let mut journal = create("worlds.jsonl");
        let mut s = Simulation::new(&d, &q, 3001);
        if !provision {
            s.load_checkpoint(&q, std::path::Path::new(checkpoint.as_ref().unwrap()))
                .unwrap();
        }
        s.retain_packet_endowment = provision;
        assert_eq!(s.settings.population, 4096);
        assert_eq!(s.settings.maturity_age, 1800.0);
        assert!(!s.assisted);
        // The declared fork has no living descendants. Thus the observer sees
        // every future descendant's entire life, and no prior eligible record
        // requires retrospective admission under B.
        assert!(
            s.agent_snapshot(&d, &q)
                .unwrap()
                .iter()
                .all(|a| a.alive != 1 || a.ancestry_depth == 0)
        );
        s.parent_admission = Some(parent_admission::ParentAdmission::new(&d, &s));
        let mut elapsed = 0u32;
        let mut previous_search = None;
        loop {
            let start_tick = s.tick;
            let start_metrics = s.metrics(&d, &q).unwrap();
            s.funnel_observer = Some(funnel_audit::FunnelObserver::with_capacity(
                &d,
                &s,
                if provision { 2_000_000 } else { 65536 },
            ));
            let initial_search = s.search_snapshot(&d, &q, &mut previous_search).unwrap();
            let candidate = false;
            let metrics = loop {
                let n = (1000000 - elapsed).min(32);
                step(&mut s, &d, &q, n);
                elapsed += n;
                if provision && elapsed.is_multiple_of(8192) {
                    let counts = s.funnel_observer.as_ref().unwrap().counts(&d, &q);
                    eprintln!(
                        "C live elapsed={elapsed} world={} tick={} births={} matured={} adult_descendant_packets={} events={}",
                        s.progress.world, s.tick, counts[4], counts[7], counts[9], counts[0]
                    );
                }
                assert!(!s.refresh_engine_status(&d, &q).unwrap());
                let metrics = s.metrics(&d, &q).unwrap();
                if metrics.living == 0 || elapsed == 1000000 {
                    break metrics;
                }
            };
            let natural_extinction = metrics.living == 0;
            if natural_extinction {
                s.complete_world(&d, &q).unwrap();
            }
            let funnel = s.funnel_observer.as_ref().unwrap().report(&d, &q);
            assert_eq!(
                funnel["births"],
                metrics.events[3] - start_metrics.events[3]
            );
            assert_eq!(
                funnel["packets_produced"],
                metrics.birth_gates[2] - start_metrics.birth_gates[2]
            );
            eprintln!(
                "AB {label} elapsed={elapsed} world={} tick={} births={} matured={} depth={} candidate={candidate}",
                s.progress.world,
                s.tick,
                funnel["births"],
                funnel["juveniles_matured"],
                funnel["maximum_closed_life_cycle_depth"]
            );
            let row = json!({"arm":label,"retain_paid_endowment":provision,"fresh_random_founders":provision,"elapsed_ticks":elapsed,"world":s.progress.world,"seed":s.seed,"start_tick":start_tick,"tick":s.tick,"segment_ticks":s.tick-start_tick,"natural_extinction":natural_extinction,"candidate_before_maturity":candidate,"settings":s.settings,"progress":s.progress,"metrics":metrics,"initial_search":initial_search,"final_search":s.search_snapshot(&d,&q,&mut previous_search).unwrap(),"funnel":funnel});
            create(&format!("world-{}.json", s.progress.world))
                .write_all(&serde_json::to_vec(&row).unwrap())
                .unwrap();
            let mut compact = row;
            compact["funnel"]
                .as_object_mut()
                .unwrap()
                .remove("individuals");
            compact["funnel"]
                .as_object_mut()
                .unwrap()
                .remove("life_events");
            writeln!(journal, "{}", serde_json::to_string(&compact).unwrap()).unwrap();
            journal.flush().unwrap();
            if elapsed == 1000000 || candidate {
                s.save_checkpoint(&d, &q, &output.join("continuation.checkpoint"))
                    .unwrap();
                create("completion.json").write_all(&serde_json::to_vec_pretty(&json!({"elapsed_ticks":elapsed,"candidate_before_maturity":candidate,"complete_horizon":elapsed==1000000,"scope":"One blind mature packet producer per actual successful birth; immutable parent genome and traits. No newborn admission. Same replacement attempts and RNG increment as A. Unlimited natural transitions, exactly 1M additional ticks."})).unwrap()).unwrap();
                break;
            }
            s.advance_world(&d, &q).unwrap();
            assert_eq!(s.parent_admission.is_some(), parent);
            assert_eq!(s.retain_packet_endowment, provision);
        }
    }
}

#[test]
fn mature_parent_admission_copies_whole_producer_record_only_after_real_birth() {
    let (d, q) = gpu();
    for (producer, energy, expected) in [(2, 24.0, 1), (2, 2.0, 0), (1, 24.0, 0)] {
        let mut s = scene(&d, &q);
        let mut a = packets::packet([100.0, 100.0], 1, energy);
        let mut b = packets::packet([101.0, 100.0], producer, energy);
        a.active_mask = 0x5555;
        b.active_mask = 0xaaaa;
        a.plasticity_rate = [0.1; HIDDEN];
        b.plasticity_rate = [-0.1; HIDDEN];
        put(&s, &q, 0, a, &[1.0; GENOME_SIZE]);
        put(&s, &q, 1, b, &[-1.0; GENOME_SIZE]);
        s.parent_admission = Some(parent_admission::ParentAdmission::new(&d, &s));
        let before = s.reservoir_snapshot(&d, &q).unwrap();
        step(&mut s, &d, &q, 1);
        assert_eq!(s.metrics(&d, &q).unwrap().events[3], expected);
        let after = s.reservoir_snapshot(&d, &q).unwrap();
        assert_eq!(after.2, before.2.wrapping_add(expected));
        let changed: Vec<_> = (0..HEREDITARY_RESERVOIR_SIZE as usize)
            .filter(|&i| {
                after.0[i * GENOME_SIZE..(i + 1) * GENOME_SIZE]
                    != before.0[i * GENOME_SIZE..(i + 1) * GENOME_SIZE]
                    || after.1[i] != before.1[i]
            })
            .collect();
        assert_eq!(changed.len(), expected as usize);
        for i in changed {
            let genes = &after.0[i * GENOME_SIZE..(i + 1) * GENOME_SIZE];
            let donor = if genes[0] == 1.0 { a } else { b };
            assert!(genes.iter().all(|&v| v == genes[0]));
            assert!([1.0, -1.0].contains(&genes[0]));
            assert_eq!(after.1[i], donor.cognitive_traits());
            let child_genes = s.read_genomes(&d, &q, 2).unwrap();
            let bodies = s.agent_snapshot(&d, &q).unwrap();
            let ci = bodies.iter().position(|x| x.alive == 1).unwrap();
            assert_ne!(
                genes,
                &child_genes[ci * GENOME_SIZE..(ci + 1) * GENOME_SIZE]
            );
        }
        // No producer bodies exist: packet heredity remains valid posthumously.
    }
}
