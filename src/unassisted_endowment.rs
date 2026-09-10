//! Fresh production-rule evolution with read-only life-cycle observations.
use super::*;
use serde_json::json;
use std::io::Write;

#[test]
#[ignore = "fresh unassisted production-rule evolution, 100000 ticks or actual descendant reproduction"]
fn paid_endowment_unassisted() {
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let create = |name: &str| {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))
            .unwrap()
    };
    let mut journal = create("worlds.jsonl");
    let (d, q) = gpu();
    let seed = std::env::var("PRIMITIVE_AUDIT_SEED")
        .map(|v| v.parse::<u32>().unwrap())
        .unwrap_or(3001);
    let mut s = Simulation::new(&d, &q, seed);
    let resume = std::env::var("PRIMITIVE_AUDIT_CHECKPOINT").ok();
    let population = std::env::var("PRIMITIVE_AUDIT_POPULATION").ok();
    let regeneration = std::env::var("PRIMITIVE_AUDIT_REGENERATION").ok();
    if population.is_some() || regeneration.is_some() {
        assert!(
            resume.is_none(),
            "Opportunity settings are fresh-cohort only"
        );
        if let Some(value) = population {
            s.settings.population = value.parse().unwrap();
        }
        if let Some(value) = regeneration {
            s.settings.resource_regeneration = value.parse().unwrap();
        }
        s.reset(&q);
    }

    if let Some(path) = &resume {
        s.load_checkpoint(&q, std::path::Path::new(path)).unwrap();
        // Observer lifetime evidence is not serialized. Resume only when no
        // descendant is alive, so every newly observed life has a birth record.
        assert!(
            s.agent_snapshot(&d, &q)
                .unwrap()
                .iter()
                .all(|a| a.alive != 1 || a.ancestry_depth == 0)
        );
        assert_eq!(
            s.metrics(&d, &q).unwrap().packets,
            0,
            "Resume at a packet-free boundary"
        );
        if s.metrics(&d, &q).unwrap().living == 0 {
            s.complete_world(&d, &q).unwrap();
            s.advance_world(&d, &q).unwrap();
        }
    }
    let legacy_fraction_gate =
        std::env::var("PRIMITIVE_LEGACY_FRACTION_GATE").as_deref() == Ok("1");
    if legacy_fraction_gate {
        assert!(
            resume.is_none(),
            "Counterfactual gate starts from fresh founders"
        );
        budget_gate_audit::install_legacy_fraction_gate(&mut s, &d);
    }
    assert!(s.retain_packet_endowment && !s.assisted);
    assert!(s.parent_admission.is_none() && !s.defer_reservoir_admission);
    let horizon = std::env::var("PRIMITIVE_AUDIT_TICKS")
        .map(|v| v.parse::<u32>().unwrap())
        .unwrap_or(100_000);
    assert!(horizon > 0);
    create("protocol.json").write_all(&serde_json::to_vec_pretty(&json!({
        "model":MODEL_ID,"experimental_legacy_fraction_gate":legacy_fraction_gate,"seed":s.seed,"resume_checkpoint":resume,"settings":s.settings,"horizon":horizon,
        "scope":"Fresh random founders unless a resume checkpoint is named; production newborn reservoir admission and paid endowment. No controller or genome edits, no assistance. Natural restarts. Stop and preserve evidence on actual descendant reproduction; this is an accessibility search, not a fixed-horizon rate comparison. Observer-only funnel records birth energy, packet manufacture energy, dual parentage, juvenile received food and maturity. Checkpoints every 8192 cumulative ticks and at world boundaries; observers are not serialized."
    })).unwrap()).unwrap();
    let mut elapsed = 0u32;
    let mut previous_search = None;
    loop {
        let start_tick = s.tick;
        let initial_search = s.search_snapshot(&d, &q, &mut previous_search).unwrap();
        s.funnel_observer = Some(funnel_audit::FunnelObserver::with_capacity(
            &d, &s, 2_000_000,
        ));
        let mut passed = false;
        let metrics = loop {
            let n = (horizon - elapsed).min(32);
            step(&mut s, &d, &q, n);
            elapsed += n;
            assert!(!s.refresh_engine_status(&d, &q).unwrap());
            let m = s.metrics(&d, &q).unwrap();
            if elapsed.is_multiple_of(8192) || elapsed == horizon || m.living == 0 {
                let counts = s.funnel_observer.as_ref().unwrap().counts(&d, &q);
                eprintln!(
                    "unassisted elapsed={elapsed} world={} tick={} births={} matured={} descendant_packets={}",
                    s.progress.world, s.tick, counts[4], counts[7], counts[9]
                );
                if counts[7] > 0 {
                    let report = s.funnel_observer.as_ref().unwrap().report(&d, &q);
                    passed = report["births_involving_descendant_parents"]
                        .as_u64()
                        .unwrap()
                        > 0;
                }
                s.save_checkpoint(&d, &q, &output.join(format!("state-{elapsed}.checkpoint")))
                    .unwrap();
            }
            if m.living == 0 || elapsed == horizon || passed {
                break m;
            }
        };
        let natural_extinction = metrics.living == 0;
        if natural_extinction {
            s.complete_world(&d, &q).unwrap();
        }
        let funnel = s.funnel_observer.as_ref().unwrap().report(&d, &q);
        let row = json!({"model":MODEL_ID,"experimental_legacy_fraction_gate":legacy_fraction_gate,"elapsed_ticks":elapsed,"world":s.progress.world,"seed":s.seed,"start_tick":start_tick,"tick":s.tick,"segment_ticks":s.tick-start_tick,"natural_extinction":natural_extinction,"settings":s.settings,"progress":s.progress,"metrics":metrics,"initial_search":initial_search,"final_search":s.search_snapshot(&d,&q,&mut previous_search).unwrap(),"funnel":funnel});
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
        if elapsed == horizon || passed {
            s.save_checkpoint(&d, &q, &output.join("continuation.checkpoint"))
                .unwrap();
            create("completion.json").write_all(&serde_json::to_vec_pretty(&json!({"experimental_legacy_fraction_gate":legacy_fraction_gate,"elapsed_ticks":elapsed,"unassisted_descendant_reproduction":passed,"complete_horizon":elapsed==horizon})).unwrap()).unwrap();
            break;
        }
        s.advance_world(&d, &q).unwrap();
        assert!(s.retain_packet_endowment && s.parent_admission.is_none() && !s.assisted);
    }
}
