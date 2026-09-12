//! Local, read-only checkpoint diagnosis; never changes production biology.
use super::*;
use serde_json::{Value, json};
fn summary(a: &[AgentGpu]) -> Value {
    let mut founders = 0;
    let mut juveniles = Vec::new();
    let mut adults = Vec::new();
    let mut packets = 0;
    let mut packet_sizes = Vec::new();
    let mut actions = [0u32; 6];
    for b in a {
        if b.alive == 2 {
            packets += 1;
            packet_sizes.push(b.packet_size);
        }
        if b.alive != 1 {
            continue;
        }
        if b.action < 6 {
            actions[b.action as usize] += 1;
        }
        if b.ancestry_depth == 0 {
            founders += 1;
            continue;
        }
        let row = json!({"lineage":b.lineage_id,"age":b.age,"energy":b.energy,"food":b.food,"received":b.received,"collected":b.collected,"ingested":b.ingested,"spent":b.spent,"packet_size":b.packet_size,"packets_produced":b.packets_produced,"ancestry_depth":b.ancestry_depth,"position":b.position,"distance":b.distance_travelled});
        if b.age >= 1800.0 {
            adults.push(row);
        } else {
            juveniles.push(row);
        }
    }
    juveniles.sort_by(|a, b| {
        b["age"]
            .as_f64()
            .unwrap()
            .total_cmp(&a["age"].as_f64().unwrap())
    });
    json!({"founders":founders,"packets":packets,"packet_sizes":packet_sizes,"juveniles":juveniles,"adult_descendants":adults,"actions":actions})
}

/// Current controller outputs are an observation, not a policy intervention.
fn juvenile_decisions(a: &[AgentGpu], d: &[DecisionGpu]) -> Value {
    let live: Vec<_> = a
        .iter()
        .zip(d)
        .filter(|(a, d)| a.alive == 1 && a.ancestry_depth > 0 && a.age < 1800.0 && d.evaluated != 0)
        .collect();
    let n = live.len().max(1) as f64;
    let mut selected = [0u32; 6];
    let mut gather = Vec::with_capacity(live.len());
    let mut movement = Vec::with_capacity(live.len());
    let mut force = Vec::with_capacity(live.len());
    for (a, d) in live {
        selected[d.selected_action.min(5) as usize] += 1;
        gather.push(d.outputs[1].clamp(0.0, 1.0));
        movement.push((d.movement[0].powi(2) + d.movement[1].powi(2)).sqrt());
        force.push((d.force[0].powi(2) + d.force[1].powi(2)).sqrt());
        assert!(a.energy.is_finite());
    }
    let mean = |v: &[f32]| v.iter().map(|x| f64::from(*x)).sum::<f64>() / n;
    let nonzero = |v: &[f32]| v.iter().filter(|x| **x > 0.001).count();
    json!({
        "juveniles_observed": gather.len(),
        "selected_action": selected,
        "mean_gather_effort": mean(&gather),
        "juveniles_requesting_gather": nonzero(&gather),
        "mean_movement_command": mean(&movement),
        "mean_force_command": mean(&force),
        "juveniles_requesting_force": nonzero(&force),
        "scope": "One saved-tick controller output. Gathering is continuous output 1, independent of the selected primary action. Commands are not delivered food or realized force."
    })
}
#[test]
#[ignore = "local saved-world observation; explicit copied checkpoint and output directory"]
fn inspect_saved_world_and_pool() {
    let out = std::path::PathBuf::from(std::env::var("PRIMITIVE_INSPECTION_DIR").unwrap());
    let write = |name: &str, value: &Value| {
        use std::io::Write;
        let mut f = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(out.join(name))
            .unwrap();
        f.write_all(&serde_json::to_vec(value).unwrap()).unwrap();
    };
    let receipt: Value =
        serde_json::from_slice(&std::fs::read(out.join("source.json")).unwrap()).unwrap();
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 1);
    let load = |s: &mut Simulation| {
        s.load_game_checkpoint(
            &q,
            std::fs::File::open(out.join("source.checkpoint")).unwrap(),
            (
                receipt["seed"].as_u64().unwrap() as u32,
                receipt["tick"].as_u64().unwrap(),
                receipt["living"].as_u64().unwrap() as u32,
            ),
            receipt["world"].as_u64().unwrap(),
        )
        .unwrap();
    };
    load(&mut s);
    let mut search = None;
    let bodies = s.agent_snapshot(&d, &q).unwrap();
    let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, MAX_AGENTS as usize);
    write(
        "snapshot.json",
        &json!({"maximum_world_ancestry_depth":read::<u32>(&d,&q,&s.death_stats_buffer,DEATH_STATS_COUNT as usize)[23],"pool_depths":s.reservoir_snapshot(&d,&q).unwrap().1.iter().map(|t|t.padding[0]).collect::<Vec<_>>(),"receipt":receipt,"settings":s.settings,"progress":s.progress,"metrics":s.metrics(&d,&q).unwrap(),"search":s.search_snapshot(&d,&q,&mut search).unwrap(),"population":summary(&bodies),"juvenile_decisions":juvenile_decisions(&bodies,&decisions)}),
    );
    if std::env::var_os("PRIMITIVE_SNAPSHOT_ONLY").is_some() {
        return;
    }
    let mut series = Vec::new();
    for n in 0..96 {
        step(&mut s, &d, &q, 128);
        let m = s.metrics(&d, &q).unwrap();
        series.push(json!({"tick":s.tick,"metrics":m,"population":summary(&s.agent_snapshot(&d,&q).unwrap())}));
        if n % 16 == 0 {
            eprintln!("saved continuation tick={} living={}", s.tick, m.living);
        }
        if m.living == 0 {
            break;
        }
    }
    write(
        "saved-continuation.json",
        &json!({"scope":"Unmodified saved world continued up to 12288 ticks or extinction. Population snapshots every128ticks can miss short lives. Existing offspring lack full birth histories.","series":series}),
    );
    // Fresh bodies sampled by the existing blind world-reset rule from the ORIGINAL saved pool.
    load(&mut s);
    s.rollover_world(&d, &q).unwrap();
    s.funnel_observer = Some(funnel_audit::FunnelObserver::with_capacity(
        &d, &s, 2_000_000,
    ));
    write(
        "sampled-world-start.json",
        &json!({"settings":s.settings,"seed":s.seed,"progress":s.progress,"metrics":s.metrics(&d,&q).unwrap(),"scope":"Normal full-size founder sample from original saved pool, new normal world seed. No selected genomes, controller edits, food gifts or physiological changes."}),
    );
    let mut timeline = Vec::new();
    for n in 0..750 {
        step(&mut s, &d, &q, 32);
        let m = s.metrics(&d, &q).unwrap();
        if n % 32 == 0 || m.living == 0 || n == 749 {
            let c = s.funnel_observer.as_ref().unwrap().counts(&d, &q);
            timeline.push(json!({"tick":s.tick,"living":m.living,"births":c[4],"matured":c[7],"adult_descendant_packets":c[9]}));
            eprintln!(
                "sampled world tick={} births={} matured={} living={}",
                s.tick, c[4], c[7], m.living
            );
        }
        if m.living == 0 {
            break;
        }
    }
    let funnel = s.funnel_observer.as_ref().unwrap().report(&d, &q);
    write(
        "sampled-world-result.json",
        &json!({"model":MODEL_ID,"tick":s.tick,"metrics":s.metrics(&d,&q).unwrap(),"funnel":funnel,"timeline":timeline,"population":summary(&s.agent_snapshot(&d,&q).unwrap())}),
    );
}
