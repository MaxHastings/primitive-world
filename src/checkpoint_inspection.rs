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

/// Counterfactual next world from a copied checkpoint. Never writes a game save.
#[test]
#[ignore = "isolated hereditary-pool restart; explicit copied checkpoint required"]
fn inspect_pool_restart_diversity() {
    use std::collections::{HashMap, HashSet};
    use std::io::Write;

    let out = std::path::PathBuf::from(std::env::var("PRIMITIVE_POOL_TRIAL_DIR").unwrap());
    let source = std::env::var("PRIMITIVE_POOL_TRIAL_SOURCE_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| out.clone());
    let receipt: Value =
        serde_json::from_slice(&std::fs::read(source.join("source.json")).unwrap()).unwrap();
    let (device, queue) = gpu();
    let mut simulation = Simulation::new(&device, &queue, 1);
    simulation
        .load_game_checkpoint(
            &queue,
            std::fs::File::open(source.join("source.checkpoint")).unwrap(),
            (
                receipt["seed"].as_u64().unwrap() as u32,
                receipt["tick"].as_u64().unwrap(),
                receipt["living"].as_u64().unwrap() as u32,
            ),
            receipt["world"].as_u64().unwrap(),
        )
        .unwrap();
    if let Ok(seed) = std::env::var("PRIMITIVE_POOL_TRIAL_SEED") {
        simulation.progress.rng = seed.parse().unwrap();
    }
    simulation.rollover_world(&device, &queue).unwrap();

    let milestones = [
        0, 128, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536, 131072, 262144,
    ];
    for target in milestones {
        while simulation.tick < target {
            let count = (target - simulation.tick).min(1024) as u32;
            step(&mut simulation, &device, &queue, count);
        }
        let agents = simulation.agent_snapshot(&device, &queue).unwrap();
        let live: Vec<_> = agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.alive == 1)
            .collect();
        let mut families = HashMap::<u32, usize>::new();
        let mut masks = HashMap::<u32, usize>::new();
        let mut depth_max = 0;
        let mut descendant_count = 0;
        for (_, agent) in &live {
            *families.entry(agent.founder_family).or_default() += 1;
            *masks.entry(agent.active_mask).or_default() += 1;
            depth_max = depth_max.max(agent.closed_depth);
            descendant_count += usize::from(agent.ancestry_depth > 0);
        }
        let mut top_families: Vec<_> = families.into_iter().collect();
        top_families.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let mut top_masks: Vec<_> = masks.into_iter().collect();
        top_masks.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
        let family_concentration = if live.is_empty() {
            0.0
        } else {
            top_families
                .iter()
                .map(|(_, count)| (*count as f64 / live.len() as f64).powi(2))
                .sum::<f64>()
        };

        // Sample evenly across live slots to bound GPU readback in the initial 8192-body cohort.
        let sample_slots: Vec<usize> = (0..live.len().min(1024))
            .map(|i| live[i * live.len() / live.len().min(1024)].0)
            .collect();
        let genomes = simulation
            .read_genome_slots(&device, &queue, &sample_slots)
            .unwrap();
        let mut expressed = HashSet::<Vec<u32>>::new();
        let mut common_mask_vectors = Vec::<Vec<f64>>::new();
        for (row, &slot) in sample_slots.iter().enumerate() {
            let mask = agents[slot].active_mask;
            let genome = &genomes[row * GENOME_SIZE..(row + 1) * GENOME_SIZE];
            let indices = crate::brain::expressed_indices(mask);
            let mut fingerprint = Vec::with_capacity(indices.len() + 1);
            fingerprint.push(mask);
            fingerprint.extend(indices.iter().map(|&i| genome[i].to_bits()));
            expressed.insert(fingerprint);
            if mask == 0x19 {
                common_mask_vectors.push(indices.iter().map(|&i| genome[i] as f64).collect());
            }
        }
        let common_mask_rms_pair_distance = if common_mask_vectors.len() < 2 {
            None
        } else {
            let rows = common_mask_vectors.len() as f64;
            let width = common_mask_vectors[0].len();
            let mean_variance = (0..width)
                .map(|col| {
                    let mean = common_mask_vectors.iter().map(|g| g[col]).sum::<f64>() / rows;
                    common_mask_vectors
                        .iter()
                        .map(|g| (g[col] - mean).powi(2))
                        .sum::<f64>()
                        / rows
                })
                .sum::<f64>()
                / width as f64;
            Some((2.0 * mean_variance).sqrt())
        };

        let metrics = simulation.metrics(&device, &queue).unwrap();
        let pool = if [0, 8192, 65536, 131072, 262144].contains(&target) {
            let (genomes, traits, _) = simulation.reservoir_snapshot(&device, &queue).unwrap();
            let rows: Vec<_> = genomes.chunks_exact(GENOME_SIZE).collect();
            let exact_genomes = rows
                .iter()
                .map(|row| bytemuck::cast_slice::<f32, u8>(row))
                .collect::<HashSet<_>>()
                .len();
            let mut expressed_pool = HashSet::<Vec<u32>>::new();
            let mut pool_capacities = HashMap::<u32, usize>::new();
            for (row, t) in rows.iter().zip(&traits) {
                *pool_capacities
                    .entry(t.active_mask.count_ones())
                    .or_default() += 1;
                let indices = crate::brain::expressed_indices(t.active_mask);
                let mut fingerprint = Vec::with_capacity(indices.len() + 1);
                fingerprint.push(t.active_mask);
                fingerprint.extend(indices.iter().map(|&i| row[i].to_bits()));
                expressed_pool.insert(fingerprint);
            }
            Some(
                json!({"exact_genomes":exact_genomes,"exact_expressed_brains":expressed_pool.len(),"capacity_histogram":pool_capacities}),
            )
        } else {
            None
        };
        let mut capacity_histogram = HashMap::<u32, usize>::new();
        for (_, agent) in &live {
            *capacity_histogram
                .entry(agent.active_mask.count_ones())
                .or_default() += 1;
        }
        let result = json!({
            "scope": "Counterfactual next world reset from a copied world-268 pool. Current live wallpaper and saves untouched. Family is a one-sided observer tag, not complete genetic ancestry.",
            "source": {"world":receipt["world"], "tick":receipt["tick"], "total_ticks":receipt["total_ticks"]},
            "trial_world":simulation.progress.world,
            "trial_tick":simulation.tick,
            "trial_seed":simulation.seed,
            "living_organisms":live.len(),
            "packets":metrics.packets,
            "births":metrics.birth_gates[5],
            "closed_births":metrics.closed_births,
            "living_descendants":descendant_count,
            "maximum_living_closed_depth":depth_max,
            "family_tag_count":top_families.len(),
            "family_tag_effective_number":if family_concentration > 0.0 {Some(1.0/family_concentration)} else {None},
            "top_family_tags":top_families.iter().take(8).collect::<Vec<_>>(),
            "top_family_tag_share":top_families.first().map(|(_,n)|*n as f64/live.len() as f64),
            "capacity_histogram":capacity_histogram,
            "top_active_masks":top_masks.iter().take(8).collect::<Vec<_>>(),
            "genome_sample_size":sample_slots.len(),
            "sample_distinct_expressed_genomes":expressed.len(),
            "sample_common_mask_0x19_size":common_mask_vectors.len(),
            "common_mask_0x19_expressed_parameter_rms_pair_distance":common_mask_rms_pair_distance
            ,"hereditary_pool":pool
        });
        let path = out.join(format!("pool-trial-tick-{target:06}.json"));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&result).unwrap())
            .unwrap();
        eprintln!(
            "pool trial tick={target} living={} births={} families={} effective={:.1} top={:.1}% three-unit={} expressed={}/{} rms={:?}",
            live.len(),
            metrics.birth_gates[5],
            top_families.len(),
            if family_concentration > 0.0 {
                1.0 / family_concentration
            } else {
                0.0
            },
            top_families
                .first()
                .map(|(_, n)| *n as f64 * 100.0 / live.len() as f64)
                .unwrap_or(0.0),
            live.iter()
                .filter(|(_, a)| a.active_mask.count_ones() == 3)
                .count(),
            expressed.len(),
            sample_slots.len(),
            common_mask_rms_pair_distance
        );
        if live.is_empty() {
            break;
        }
    }
}

/// Reconstruct the full two-producer birth DAG from an isolated pool restart.
#[test]
#[ignore = "isolated dual-parent ancestry trial; explicit copied checkpoint required"]
fn inspect_pool_two_parent_ancestry() {
    use std::collections::HashMap;
    use std::io::Write;

    let source = std::path::PathBuf::from(std::env::var("PRIMITIVE_ANCESTRY_SOURCE_DIR").unwrap());
    let out = std::path::PathBuf::from(std::env::var("PRIMITIVE_ANCESTRY_OUTPUT_DIR").unwrap());
    let receipt: Value =
        serde_json::from_slice(&std::fs::read(source.join("source.json")).unwrap()).unwrap();
    let (device, queue) = gpu();
    let mut simulation = Simulation::new(&device, &queue, 1);
    simulation
        .load_game_checkpoint(
            &queue,
            std::fs::File::open(source.join("source.checkpoint")).unwrap(),
            (
                receipt["seed"].as_u64().unwrap() as u32,
                receipt["tick"].as_u64().unwrap(),
                receipt["living"].as_u64().unwrap() as u32,
            ),
            receipt["world"].as_u64().unwrap(),
        )
        .unwrap();
    simulation.progress.rng = std::env::var("PRIMITIVE_ANCESTRY_SEED")
        .unwrap_or_else(|_| "2718281828".to_string())
        .parse()
        .unwrap();
    simulation.rollover_world(&device, &queue).unwrap();
    let initial = simulation.agent_snapshot(&device, &queue).unwrap();
    let founder_by_lineage: HashMap<u32, u32> = initial
        .iter()
        .filter(|a| a.alive == 1)
        .map(|a| (a.lineage_id, a.founder_family))
        .collect();
    assert_eq!(
        founder_by_lineage.len(),
        simulation.settings.population as usize
    );
    let words = simulation.settings.population.div_ceil(64) as usize;
    simulation.funnel_observer = Some(funnel_audit::FunnelObserver::with_capacity(
        &device,
        &simulation,
        2_000_000,
    ));

    for target in [8192, 65536, 131072, 262144] {
        while simulation.tick < target {
            let count = (target - simulation.tick).min(1024) as u32;
            step(&mut simulation, &device, &queue, count);
        }
        let metrics = simulation.metrics(&device, &queue).unwrap();
        let agents = simulation.agent_snapshot(&device, &queue).unwrap();
        let live: Vec<_> = agents.iter().filter(|a| a.alive == 1).collect();
        let (edges, event_count) = simulation
            .funnel_observer
            .as_ref()
            .unwrap()
            .birth_edges(&device, &queue);
        assert_eq!(edges.len() as u64, metrics.birth_gates[5]);

        let mut ancestry =
            HashMap::<u32, Vec<u64>>::with_capacity(founder_by_lineage.len() + edges.len());
        let mut tagged = HashMap::<u32, u32>::with_capacity(ancestry.capacity());
        for (&lineage, &founder) in &founder_by_lineage {
            let mut mask = vec![0u64; words];
            mask[founder as usize / 64] |= 1u64 << (founder % 64);
            ancestry.insert(lineage, mask);
            tagged.insert(lineage, founder);
        }
        let mut cross_tag_births = 0usize;
        for edge in &edges {
            let mut mask = ancestry
                .get(&edge.parents[0])
                .unwrap_or_else(|| {
                    panic!(
                        "missing first parent {} at tick {}",
                        edge.parents[0], edge.tick
                    )
                })
                .clone();
            let other = ancestry.get(&edge.parents[1]).unwrap_or_else(|| {
                panic!(
                    "missing second parent {} at tick {}",
                    edge.parents[1], edge.tick
                )
            });
            for (a, b) in mask.iter_mut().zip(other) {
                *a |= b;
            }
            assert!(
                ancestry.insert(edge.child, mask).is_none(),
                "lineage id reused"
            );
            let first_tag = tagged[&edge.parents[0]];
            let second_tag = tagged[&edge.parents[1]];
            cross_tag_births += usize::from(first_tag != second_tag);
            assert!(tagged.insert(edge.child, first_tag).is_none());
        }

        let mut union = vec![0u64; words];
        let mut intersection = vec![!0u64; words];
        let mut counts = Vec::with_capacity(live.len());
        let mut tag_counts = HashMap::<u32, usize>::new();
        let mut represented_by = vec![0u32; simulation.settings.population as usize];
        for agent in &live {
            let mask = &ancestry[&agent.lineage_id];
            assert_eq!(tagged[&agent.lineage_id], agent.founder_family);
            counts.push(mask.iter().map(|w| w.count_ones()).sum::<u32>());
            *tag_counts.entry(agent.founder_family).or_default() += 1;
            for (i, &word) in mask.iter().enumerate() {
                union[i] |= word;
                intersection[i] &= word;
                let mut bits = word;
                while bits != 0 {
                    let bit = bits.trailing_zeros() as usize;
                    represented_by[i * 64 + bit] += 1;
                    bits &= bits - 1;
                }
            }
        }
        counts.sort_unstable();
        let mut top_tags: Vec<_> = tag_counts.into_iter().collect();
        top_tags.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let mut top_ancestors: Vec<_> = represented_by
            .iter()
            .enumerate()
            .filter(|(_, n)| **n > 0)
            .map(|(i, &n)| (i, n))
            .collect();
        top_ancestors.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
        let genealogical_founders = union.iter().map(|w| w.count_ones()).sum::<u32>();
        let common_founders = if live.is_empty() {
            0
        } else {
            intersection.iter().map(|w| w.count_ones()).sum::<u32>()
        };
        let result = json!({
            "scope":"Copied-pool counterfactual world. Full two-packet-producer genealogy of viable births. An ancestral founder may no longer contribute any surviving genome parameter after recombination; this is reproductive ancestry, not exact gene provenance.",
            "source_world":receipt["world"],
            "source_tick":receipt["tick"],
            "trial_seed":simulation.seed,
            "tick":simulation.tick,
            "living":live.len(),
            "births":edges.len(),
            "observer_events":event_count,
            "cross_one_sided_tag_births":cross_tag_births,
            "one_sided_tags_alive":top_tags.len(),
            "top_one_sided_tags":top_tags.iter().take(8).collect::<Vec<_>>(),
            "distinct_founders_in_living_genealogies":genealogical_founders,
            "founders_ancestral_to_every_living_organism":common_founders,
            "founders_ancestral_to_at_least_half_living_organisms":represented_by.iter().filter(|&&n| n as usize * 2 >= live.len() && n > 0).count(),
            "ancestors_per_living_organism_min":counts.first(),
            "ancestors_per_living_organism_median":counts.get(counts.len()/2),
            "ancestors_per_living_organism_max":counts.last(),
            "top_founders_by_living_descendant_count":top_ancestors.iter().take(8).collect::<Vec<_>>()
        });
        let path = out.join(format!("two-parent-tick-{target:06}.json"));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&result).unwrap())
            .unwrap();
        eprintln!(
            "two-parent tick={target} living={} births={} events={} tags={} genealogical_founders={} common={} median={:?} cross_tag_births={}",
            live.len(),
            edges.len(),
            event_count,
            top_tags.len(),
            genealogical_founders,
            common_founders,
            counts.get(counts.len() / 2),
            cross_tag_births
        );
        if live.is_empty() {
            break;
        }
    }
}
