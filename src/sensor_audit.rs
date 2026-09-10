//! Test-only sensor-limited compositional reachability controls.
use super::*;
use serde_json::json;

#[test]
#[ignore = "unassisted consecutive worlds with the production hereditary reservoir"]
fn unassisted_reservoir_sequence() {
    use std::io::Write;
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
    let resume = std::env::var("PRIMITIVE_EVOLUTION_CHECKPOINT").ok();
    let prior_journal = std::env::var("PRIMITIVE_EVOLUTION_PRIOR_JOURNAL").ok();
    create("protocol.json").write_all(&serde_json::to_vec_pretty(&json!({
        "initial_seed":3001,"cumulative_tick_budget":1000000,
        "resume":resume,"prior_journal":prior_journal,"scope":"One unassisted hereditary chain. Ordinary default random founders initially; subsequent founders sampled by production advance_world from the blind hereditary reservoir. No controller, genotype selection, physical overrides, forced extinction, world-count target, or per-world time limit. Read-only funnel observer. Stop and save at 1000000 cumulative simulation ticks, retaining any living world as censored without restart. A successful observation does not shorten the run."
    })).unwrap()).unwrap();
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 3001);
    let mut elapsed = 0u32;
    if let Some(path) = resume {
        s.load_checkpoint(&q, std::path::Path::new(&path)).unwrap();
        assert!(
            s.progress.completed.is_some(),
            "continuation requires a naturally completed world so the observer captures entire lifetimes"
        );
        let prior: Vec<serde_json::Value> = std::fs::read_to_string(prior_journal.unwrap())
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        assert!(!prior.is_empty());
        for (index, row) in prior.iter().enumerate() {
            assert_eq!(row["world"], index as u64 + 1);
            assert_eq!(row["censored"], false);
            elapsed = elapsed
                .checked_add(row["tick"].as_u64().unwrap().try_into().unwrap())
                .unwrap();
        }
        let last = prior.last().unwrap();
        assert_eq!(last["world"], s.progress.world);
        assert_eq!(last["seed"], s.seed);
        assert_eq!(last["tick"], s.tick);
        assert_eq!(last["progress"], serde_json::to_value(&s.progress).unwrap());
        s.advance_world(&d, &q).unwrap();
    } else {
        assert!(prior_journal.is_none());
    }
    assert!(elapsed < 1000000, "requested horizon already completed");
    assert_eq!(s.settings.population, 4096);
    assert!(!s.assisted);
    let mut search_previous = None;
    loop {
        s.funnel_observer = Some(funnel_audit::FunnelObserver::new(&d, &s));
        let has_previous = search_previous.is_some();
        let initial_search = s.search_snapshot(&d, &q, &mut search_previous).unwrap();
        if has_previous {
            assert_eq!(
                initial_search["pool_slots_with_changed_record_since_sample"],
                0
            );
        }
        let metrics = loop {
            let n = (1000000 - elapsed).min(32);
            step(&mut s, &d, &q, n);
            elapsed += n;
            assert!(
                !s.refresh_engine_status(&d, &q).unwrap(),
                "accounting horizon requires separate evidence"
            );
            let m = s.metrics(&d, &q).unwrap();
            if m.living == 0 || elapsed == 1000000 {
                break m;
            }
        };
        let censored = metrics.living > 0;
        if !censored {
            s.complete_world(&d, &q).unwrap();
        }
        let funnel = s.funnel_observer.as_ref().unwrap().report(&d, &q);
        assert_eq!(funnel["births"], metrics.events[3]);
        assert_eq!(funnel["packets_produced"], metrics.birth_gates[2]);
        eprintln!(
            "evolution cumulative={elapsed} world={} seed={} tick={} births={} matured={} depth={} censored={censored}",
            s.progress.world,
            s.seed,
            s.tick,
            metrics.events[3],
            funnel["juveniles_matured"],
            funnel["maximum_closed_life_cycle_depth"]
        );
        let row = json!({"world":s.progress.world,"seed":s.seed,"tick":s.tick,"cumulative_ticks":elapsed,"censored":censored,"settings":s.settings,"progress":s.progress,"metrics":metrics,"initial_search":initial_search,"final_search":s.search_snapshot(&d,&q,&mut search_previous).unwrap(),"funnel":funnel});
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
        if elapsed == 1000000 {
            s.save_checkpoint(&d, &q, &output.join("continuation.checkpoint"))
                .unwrap();
            break;
        }
        s.advance_world(&d, &q).unwrap();
    }
}

// The policy cannot access the Simulation, an Agent, positions, or the food map.
pub(super) struct View {
    food_regions: [f32; 16],
    energy: f32,
    food: f32,
    age: f32,
}
impl View {
    pub(super) fn from_inputs(x: &[f32; INPUTS]) -> Self {
        Self {
            food_regions: std::array::from_fn(|k| x[11 + k * 6]),
            energy: x[0] * 100.0,
            food: x[1] * 8.0,
            age: x[3] * 10000.0,
        }
    }
}
#[derive(Default)]
pub(super) struct Memory {
    estimated_spin: f32,
    cooldown: u32,
    previous_occupancy: f32,
}
pub(super) fn control(v: &View, m: &mut Memory, variant: u32) -> [f32; GENOME_SIZE] {
    let values: [f32; 8] =
        std::array::from_fn(|k| 0.65 * v.food_regions[k] + 0.35 * v.food_regions[k + 8]);
    let error = if variant == 0 {
        let mut vector = [0.02_f32, 0.0_f32]; // Forward persistence when food is isotropic.
        for (k, value) in values.iter().enumerate() {
            let angle = k as f32 * std::f32::consts::FRAC_PI_4;
            vector[0] += value * angle.cos();
            vector[1] += value * angle.sin();
        }
        vector[1].atan2(vector[0])
    } else {
        let mut best = (f32::NEG_INFINITY, 0.0_f32);
        for (k, value) in values.iter().enumerate() {
            let angle = (k as f32 * std::f32::consts::FRAC_PI_4 + std::f32::consts::PI)
                .rem_euclid(std::f32::consts::TAU)
                - std::f32::consts::PI;
            let score = value + 0.025 * angle.cos();
            if score > best.0 {
                best = (score, angle);
            }
        }
        best.1
    };
    let turn = (0.35 * error - 4.0 * m.estimated_spin).clamp(-0.7, 0.7);
    let speed = 0.8 * error.cos().max(0.15);
    let produce = v.age >= 1800.0 && v.energy > 75.0 && v.food > 0.1 && m.cooldown == 0;
    let mut genes = fixed(if produce { 5 } else { 0 }, [0.0; 2]);
    genes[OUTPUT_BIAS + 1] = 1.0;
    genes[OUTPUT_BIAS + 6] = turn.atanh();
    genes[OUTPUT_BIAS + 7] = (speed / 1.2).atanh() / 4.0;
    // Update every four ticks; this is commanded-motion memory, never hidden telemetry.
    for _ in 0..4 {
        m.estimated_spin = 0.85 * m.estimated_spin + 0.0375 * turn;
    }
    m.cooldown = if produce {
        200
    } else {
        m.cooldown.saturating_sub(4)
    };
    genes
}

pub(super) fn inputs(
    s: &Simulation,
    d: &wgpu::Device,
    q: &wgpu::Queue,
    count: usize,
) -> Vec<DecisionGpu> {
    s.update_params(q);
    let mut e = d.create_command_encoder(&Default::default());
    let groups = MAX_AGENTS.div_ceil(64);
    s.dispatch(&mut e, "clear", 0, 32, 32);
    s.dispatch(&mut e, "count", s.current_buffer, groups, 1);
    s.scan(&mut e, "spatial", SPATIAL_CELL_COUNT);
    s.dispatch(&mut e, "cursors", 0, 1024, 1);
    s.dispatch(&mut e, "scatter", s.current_buffer, groups, 1);
    s.dispatch(&mut e, "perceive", s.current_buffer, groups, 1);
    s.dispatch(&mut e, "decide", s.current_buffer, groups, 1);
    q.submit(Some(e.finish()));
    read::<DecisionGpu>(d, q, &s.decision_buffer, count)
}

fn trial(
    d: &wgpu::Device,
    q: &wgpu::Queue,
    seed: u32,
    count: usize,
    selected: bool,
    variant: u32,
) -> serde_json::Value {
    let mut s = Simulation::new(d, q, seed);
    s.settings.population = count as u32;
    s.reset(q);
    // Standard constructor's positions, headings, ages, and reserves are retained
    // for held-out placements. Only the explicit selected control changes location.
    let mut founders = build_agent_records(seed, &s.settings, &[], count as u32);
    for (slot, a) in founders.iter_mut().enumerate() {
        if selected {
            a.position = [1254.0 + slot as f32, 810.0];
            a.heading = 0.0;
        }
        put(&s, q, slot, *a, &fixed(0, [0.0; 2]));
    }
    let mut memory: Vec<Memory> = (0..count).map(|_| Memory::default()).collect();
    let mut peak_food = vec![0.0_f32; count];
    let mut peak_energy = vec![35.0_f32; count];
    let mut surplus_observations = vec![0u32; count];
    let mut food_seen_observations = vec![0u32; count];
    let mut records = Vec::new();
    for _ in 0..1000 {
        let agents = s.agent_snapshot(d, q).unwrap();
        if !agents[..count]
            .iter()
            .any(|a| a.alive == 1 && a.ancestry_depth == 0)
        {
            break;
        }
        let decisions = inputs(&s, d, q, count);
        for slot in 0..count {
            if agents[slot].alive != 1 || agents[slot].ancestry_depth != 0 {
                continue;
            }
            let view = View::from_inputs(&decisions[slot].inputs);
            food_seen_observations[slot] += u32::from(view.food_regions.iter().any(|&f| f > 0.05));
            let genes = control(&view, &mut memory[slot], variant);
            s.write_genome_slot(q, slot, &genes);
        }
        // Descendants are neutral in this adult-only test; no accidental inherited
        // external readout gets counted as a sensor-valid offspring policy.
        for (slot, a) in agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.alive == 1 && a.ancestry_depth > 0)
        {
            let _ = a;
            s.write_genome_slot(q, slot, &fixed(0, [0.0; 2]));
        }
        step(&mut s, d, q, 4);
        let after = s.agent_snapshot(d, q).unwrap();
        for slot in 0..count {
            if agents[slot].alive != 1 || agents[slot].lineage_id != after[slot].lineage_id {
                continue;
            }
            peak_energy[slot] = peak_energy[slot].max(after[slot].energy);
            peak_food[slot] = peak_food[slot].max(after[slot].food);
            surplus_observations[slot] += u32::from(after[slot].food > 0.1);
        }
        if s.tick.is_multiple_of(100) {
            records.push(json!({"tick":s.tick,"energy":after[..count].iter().map(|a|a.energy).collect::<Vec<_>>(),"food":after[..count].iter().map(|a|a.food).collect::<Vec<_>>()}));
        }
    }
    let after = s.agent_snapshot(d, q).unwrap();
    let packets: Vec<_> = (0..count).map(|i| after[i].packets_produced).collect();
    let buffered = peak_food.iter().filter(|&&f| f > 0.1).count();
    let funded = packets.iter().filter(|&&n| n > 0).count();
    let alive = after[..count]
        .iter()
        .filter(|a| a.alive == 1 && a.ancestry_depth == 0)
        .count();
    let metrics = s.metrics(d, q).unwrap();
    eprintln!(
        "sensor seed={seed} selected={selected} policy={variant} tick={} buffered={buffered}/{count} funded={funded} alive={alive} births={}",
        s.tick, metrics.events[3]
    );
    json!({"seed":seed,"selected":selected,"policy":variant,"count":count,"tick":s.tick,"buffered":buffered,"funded":funded,"alive":alive,"births":metrics.events[3],"peak_food":peak_food,"peak_energy":peak_energy,"packets":packets,"surplus_observations":surplus_observations,"food_seen_observations":food_seen_observations,"metrics":metrics,"records":records})
}

#[test]
#[ignore = "predeclared sensor-only adult controls and held-out placement cohort"]
fn sensor_only_adult_foraging() {
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let path = output.join("sensor-adults.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    let (d, q) = gpu();
    let selected: Vec<_> = (0..2)
        .map(|variant| trial(&d, &q, 91, 2, true, variant))
        .collect();
    // Freeze by lexicographic buffer, packet-funding, survivors, births; ties use 0.
    let rank = |r: &serde_json::Value| {
        ["buffered", "funded", "alive", "births"].map(|k| r[k].as_u64().unwrap())
    };
    let winner = u32::from(rank(&selected[1]) > rank(&selected[0]));
    let unselected: Vec<_> = (101..=108)
        .map(|seed| trial(&d, &q, seed, 16, false, winner))
        .collect();
    use std::io::Write;
    file.write_all(&serde_json::to_vec_pretty(&json!({"selected":selected,"winner":winner,"unselected":unselected,"scope":"Biology unchanged. Controller gets only 16 actual GPU food-region input values plus own energy, inventory and age. Command-integrated turn memory; no actual angle/angular velocity/position/map access. Four-tick sample-and-hold controls. One-unit bodies with ordinary 35-energy zero-food reserves and size16 packets. Two policies compared at previously declared seed91 corridor location; lexicographic winner frozen before seeds101..108, 16 standard constructor placements each, horizon4000. Sparse controlled adults, not ordinary 4096-founder density. Exact social targeting and contact chain are not part of this adult stage. Four-tick observer samples are not per-tick event counts; peaks are sampled lower bounds. Genome readouts are externally controlled, not a heritable controller implementation."})).unwrap()).unwrap();
}

// Social extension uses only actual anonymous region channels and private state.
// Different behavior by own age is explicit test policy, not a production rule.
fn care_control(
    x: &[f32; INPUTS],
    m: &mut Memory,
    transfer: bool,
    reproduction: bool,
    donor_min_energy: f32,
    body_directed_packets: bool,
    react_to_occupancy: bool,
) -> [f32; GENOME_SIZE] {
    let v = View::from_inputs(x);
    let mut food = [0.02_f32, 0.0_f32];
    let mut bodies = [0.0_f32; 2];
    let mut pressure = 0.0_f32;
    let mut count = 0.0_f32;
    for k in 0..16 {
        let angle = (k % 8) as f32 * std::f32::consts::FRAC_PI_4;
        let weight = if k < 8 { 0.65 } else { 0.35 };
        food[0] += weight * v.food_regions[k] * angle.cos();
        food[1] += weight * v.food_regions[k] * angle.sin();
        let n = x[11 + k * 6 + 1] * 16.0;
        let p = x[11 + k * 6 + 5];
        bodies[0] += n * p * angle.cos();
        bodies[1] += n * p * angle.sin();
        if n > 0.0 {
            pressure = pressure.max(p);
        }
        count += n;
    }
    let young = v.age < 1800.0;
    let follow = count > 0.0 && (young || pressure < 0.82);
    let direction = if follow { bodies } else { food };
    let error = direction[1].atan2(direction[0]);
    let turn = (0.35 * error - 4.0 * m.estimated_spin).clamp(-0.7, 0.7);
    let speed = if young {
        if pressure > 0.94 { 0.1 } else { 0.5 }
    } else if count > 0.0 && pressure < 0.82 {
        0.2
    } else {
        0.35
    };
    let speed = speed * error.cos().max(0.0);
    let occupancy_rise = count > m.previous_occupancy + 0.5;
    m.previous_occupancy = count;
    let produce = reproduction
        && !young
        && m.cooldown == 0
        && ((v.energy > 85.0 && v.food > 0.3)
            || (react_to_occupancy && occupancy_rise && v.energy > 40.0));
    let give =
        transfer && !young && v.energy > donor_min_energy && v.food > 0.01 && pressure > 0.75;
    let mut genes = fixed(
        if produce {
            5
        } else if give {
            2
        } else {
            0
        },
        [0.0; 2],
    );
    genes[OUTPUT_BIAS + 1] = 1.0;
    genes[OUTPUT_BIAS + 6] = turn.atanh();
    let development = 0.6 + 0.4 * (v.age / 1800.0).clamp(0.0, 1.0);
    genes[OUTPUT_BIAS + 7] = (speed / (1.2 * development)).clamp(0.0, 0.95).atanh() / 4.0;
    if body_directed_packets && produce && count > 0.0 {
        let norm = bodies[0].hypot(bodies[1]).max(0.0001);
        genes[OUTPUT_BIAS + PLACEMENT_OUTPUT] = 2.0 * bodies[0] / norm;
        genes[OUTPUT_BIAS + PLACEMENT_OUTPUT + 1] = 2.0 * bodies[1] / norm;
    }
    m.estimated_spin = 0.85 * m.estimated_spin + 0.0375 * turn;
    m.cooldown = if produce {
        200
    } else {
        m.cooldown.saturating_sub(1)
    };
    genes
}

fn genuine_newborn(d: &wgpu::Device, q: &wgpu::Queue) -> (AgentGpu, Vec<f32>) {
    let mut s = Simulation::new(d, q, 91);
    s.settings.population = 0;
    s.reset(q);
    for i in 0..2 {
        let mut a = body([100.0 + i as f32, 100.0]);
        a.energy = 35.0;
        a.food = 0.0;
        a.active_mask = 1;
        a.lineage_id = i as u32 + 1;
        put(&s, q, i, a, &fixed(5, [0.0; 2]));
    }
    for _ in 0..5 {
        step(&mut s, d, q, 1);
        let agents = s.agent_snapshot(d, q).unwrap();
        if let Some((slot, a)) = agents
            .iter()
            .enumerate()
            .find(|(_, a)| a.alive == 1 && a.ancestry_depth > 0)
        {
            assert_eq!(a.age, 0.0);
            assert_eq!(a.food, 0.0);
            return (*a, s.read_genome_slots(d, q, &[slot]).unwrap());
        }
    }
    panic!("normal packet fixture did not produce a newborn");
}

#[derive(Default, serde::Serialize)]
struct ChildTrace {
    parent_lineage: u32,
    birth_tick: u32,
    birth_energy: f32,
    last_age: f32,
    mature_tick: Option<u32>,
    dead: bool,
    received: f64,
    gathered: f64,
    ingested: f64,
    spent: f64,
    transfer_ticks: u32,
    nearby_adult_ticks: u32,
    available_food_ticks: u32,
    selected_transfer_ticks: u32,
    recipient_target_ticks: u32,
    other_target_offers: u32,
    full_nearest_offers: u32,
    eligible_no_delivery_offers: u32,
    packet_count: u32,
    descendant_births: u32,
    events: Vec<serde_json::Value>,
}
fn distance(a: &AgentGpu, b: &AgentGpu) -> f32 {
    let delta: [f32; 2] = std::array::from_fn(|k| {
        let v = b.position[k] - a.position[k];
        v - 2048.0 * (v / 2048.0 + 0.5).floor()
    });
    delta[0].hypot(delta[1])
}
fn pre_stock(before: &AgentGpu, after: &AgentGpu) -> f32 {
    (before.food - after.ingested + after.collected).max(0.0)
}

#[test]
#[ignore = "natural-surplus care controls and sensor-valid closed-loop attempt; per-tick observation"]
fn sensor_care_and_closed_loop() {
    // Isolated dose-response experiment only: stop donor transfer after this
    // many actual juvenile delivery ticks. Never used by the hereditary chain.
    let delivery_cap: Option<u32> = std::env::var("PRIMITIVE_AUDIT_DELIVERY_CAP")
        .ok()
        .map(|v| v.parse().unwrap());
    let react_to_occupancy =
        std::env::var("PRIMITIVE_AUDIT_OCCUPANCY_RESPONSE").is_ok_and(|v| v == "1");
    let body_directed_packets =
        std::env::var("PRIMITIVE_AUDIT_BODY_PACKETS").is_ok_and(|v| v == "1");
    let closed_only = std::env::var("PRIMITIVE_AUDIT_CLOSED_ONLY").is_ok_and(|v| v == "1");
    let donor_min_energy: f32 = std::env::var("PRIMITIVE_AUDIT_DONOR_ENERGY")
        .unwrap_or_else(|_| "50".into())
        .parse()
        .unwrap();
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("sensor-care.json"))
        .unwrap();
    let (d, q) = gpu();
    let (newborn, genes) = genuine_newborn(&d, &q);
    let mut results = Vec::new();
    // Paired care/no-care experiments, then the complete same-policy composition.
    for (adults, transfer, closed) in [
        (1usize, false, false),
        (1, true, false),
        (2, false, false),
        (2, true, false),
        (2, true, true),
    ] {
        if delivery_cap.is_some() && (adults != 2 || !transfer || closed) {
            continue;
        }
        if closed_only && !closed {
            continue;
        }
        let mut s = Simulation::new(&d, &q, 91);
        s.settings.population = 0;
        s.reset(&q);
        for slot in 0..adults {
            let mut a = body([1254.0 + slot as f32, 810.0]);
            a.energy = 35.0;
            a.food = 0.0;
            a.active_mask = 1;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
        }
        let mut traces = std::collections::BTreeMap::<u32, ChildTrace>::new();
        if !closed {
            let mut a = newborn;
            a.position = [1256.0, 810.0];
            a.lineage_id = 100;
            a.birth_tick = 0;
            put(&s, &q, adults, a, &genes.clone().try_into().unwrap());
            traces.insert(
                100,
                ChildTrace {
                    birth_energy: a.energy,
                    ..Default::default()
                },
            );
        }
        let mut memories = std::collections::BTreeMap::<u32, Memory>::new();
        if closed {
            s.funnel_observer = Some(funnel_audit::FunnelObserver::new(&d, &s));
        }
        let mut peaks = vec![[35.0_f32, 0.0]; adults];
        let mut seen = vec![0u32; adults];
        let mut gathered = vec![0.0_f64; adults];
        let mut adult_timeline = Vec::new();
        let mut packet_timeline = Vec::new();
        let mut compatible_packet_ticks = 0u32;
        let mut closest_compatible_packets: Option<f32> = None;
        let horizon = if closed { 7000 } else { 2200 };
        for _ in 0..horizon {
            let before = s.agent_snapshot(&d, &q).unwrap();
            if !before.iter().any(|a| a.alive == 1) {
                break;
            }
            if !closed && traces.values().all(|c| c.dead || c.mature_tick.is_some()) {
                break;
            }
            let extent = before.iter().rposition(|a| a.alive != 0).unwrap() + 1;
            let decisions = inputs(&s, &d, &q, extent);
            for (slot, a) in before.iter().enumerate().filter(|(_, a)| a.alive == 1) {
                let x = &decisions[slot].inputs;
                let genes = care_control(
                    x,
                    memories.entry(a.lineage_id).or_default(),
                    transfer
                        && delivery_cap.is_none_or(|cap| {
                            traces.values().map(|t| t.transfer_ticks).sum::<u32>() < cap
                        }),
                    closed,
                    donor_min_energy,
                    body_directed_packets,
                    react_to_occupancy,
                );
                s.write_genome_slot(&q, slot, &genes);
                if slot < adults && a.lineage_id == slot as u32 + 1 {
                    seen[slot] += u32::from((0..16).any(|k| x[11 + k * 6] > 0.05));
                }
            }
            step(&mut s, &d, &q, 1);
            let after = s.agent_snapshot(&d, &q).unwrap();
            for (slot, b) in before
                .iter()
                .enumerate()
                .filter(|(_, a)| a.alive == 1 && a.age >= 1800.0)
            {
                let a = &after[slot];
                if a.lineage_id != b.lineage_id {
                    continue;
                }
                let x = &decisions[slot].inputs;
                adult_timeline.push(json!({"tick":s.tick,"lineage":a.lineage_id,"food_seen_max":(0..16).map(|k|x[11+k*6]).fold(0.0_f32,f32::max),"food_underfoot":x[2],"energy_before":b.energy,"energy_after":a.energy,"food_after":a.food,"available_pre_interaction":pre_stock(b,a),"gathered":a.collected,"received":a.received,"moved":a.moved[0].hypot(a.moved[1]),"action":a.action,"packets_produced":a.packets_produced}));
            }
            let packets: Vec<_> = after.iter().filter(|a| a.alive == 2).collect();
            let mut compatible = false;
            for (i, a) in packets.iter().enumerate() {
                for b in packets.iter().skip(i + 1) {
                    if a.parent_lineage != b.parent_lineage {
                        compatible = true;
                        let separation = distance(a, b);
                        closest_compatible_packets = Some(
                            closest_compatible_packets
                                .map_or(separation, |old| old.min(separation)),
                        );
                    }
                }
            }
            compatible_packet_ticks += u32::from(compatible);
            if after.iter().enumerate().any(|(i, a)| {
                a.alive == 2 && (before[i].alive != 2 || a.lineage_id != before[i].lineage_id)
            }) {
                packet_timeline.push(json!({"tick":s.tick,"packets":packets.iter().map(|a|json!({"lineage":a.lineage_id,"producer":a.parent_lineage,"age":a.age,"energy":a.energy,"position":a.position})).collect::<Vec<_>>(),"adults":after.iter().filter(|a|a.alive==1 && a.age>=1800.0).map(|a|json!({"lineage":a.lineage_id,"energy":a.energy,"food":a.food,"position":a.position})).collect::<Vec<_>>()}));
            }
            for slot in 0..adults {
                if before[slot].alive == 1 && before[slot].lineage_id == slot as u32 + 1 {
                    peaks[slot][0] = peaks[slot][0].max(after[slot].energy);
                    peaks[slot][1] = peaks[slot][1].max(after[slot].food);
                    gathered[slot] += f64::from(after[slot].collected);
                }
            }
            for (slot, a) in after
                .iter()
                .enumerate()
                .filter(|(_, a)| a.alive == 1 && a.ancestry_depth > 0)
            {
                if !traces.contains_key(&a.lineage_id) {
                    if let Some(parent) = traces.get_mut(&a.parent_lineage)
                        && parent.mature_tick.is_some()
                    {
                        parent.descendant_births += 1;
                    }
                    traces.insert(
                        a.lineage_id,
                        ChildTrace {
                            parent_lineage: a.parent_lineage,
                            birth_tick: s.tick - 1,
                            birth_energy: a.energy,
                            ..Default::default()
                        },
                    );
                }
                let _ = slot;
            }
            for (slot, b) in before
                .iter()
                .enumerate()
                .filter(|(_, a)| a.alive == 1 && a.ancestry_depth > 0)
            {
                let a = &after[slot];
                if a.lineage_id != b.lineage_id {
                    continue;
                }
                let trace = traces.get_mut(&b.lineage_id).unwrap();
                trace.last_age = a.age;
                trace.dead = a.alive == 0;
                trace.packet_count = a.packets_produced;
                if a.alive == 1 && a.age >= 1800.0 && trace.mature_tick.is_none() {
                    trace.mature_tick = Some(s.tick);
                }
                if b.age >= 1800.0 {
                    continue;
                }
                trace.received += f64::from(a.received);
                trace.gathered += f64::from(a.collected);
                trace.ingested += f64::from(a.ingested);
                trace.spent += f64::from(a.spent);
                trace.transfer_ticks += u32::from(a.received > 0.0);
                let mut nearby = false;
                let mut available = false;
                let mut selecting = false;
                let mut targeted = false;
                for (donor, c) in after
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.alive == 1 && c.age >= 1800.0)
                {
                    if donor == slot
                        || before[donor].alive != 1
                        || before[donor].lineage_id != c.lineage_id
                        || distance(c, a) > 6.0
                    {
                        continue;
                    }
                    nearby = true;
                    let stock = pre_stock(&before[donor], c);
                    if stock <= 0.0 {
                        continue;
                    }
                    available = true;
                    if c.action != 2 {
                        continue;
                    }
                    selecting = true;
                    let nearest = after
                        .iter()
                        .enumerate()
                        .filter(|(j, r)| {
                            *j != donor
                                && r.alive == 1
                                && before[*j].alive == 1
                                && before[*j].lineage_id == r.lineage_id
                                && distance(c, r) <= 6.0
                        })
                        .min_by(|(_, x), (_, y)| distance(c, x).total_cmp(&distance(c, y)));
                    if let Some((j, r)) = nearest {
                        let capacity = 1.0 + 7.0 * (r.age / 1800.0).clamp(0.0, 1.0);
                        if pre_stock(&before[j], r) >= capacity {
                            trace.full_nearest_offers += 1;
                        }
                        if j == slot {
                            targeted = true;
                            if a.received <= 0.0 {
                                trace.eligible_no_delivery_offers += 1;
                            }
                        } else {
                            trace.other_target_offers += 1;
                        }
                    }
                }
                trace.nearby_adult_ticks += u32::from(nearby);
                trace.available_food_ticks += u32::from(available);
                trace.selected_transfer_ticks += u32::from(selecting);
                trace.recipient_target_ticks += u32::from(targeted);
                if a.received > 0.0 || a.alive == 0 || (a.age as u32).is_multiple_of(100) {
                    trace.events.push(json!({"tick":s.tick,"age":a.age,"energy":a.energy,"food":a.food,"received":a.received,"gathered":a.collected,"nearby_adult":nearby,"available_food":available,"selected_transfer":selecting,"nearest_target":targeted}));
                }
            }
        }
        let metrics = s.metrics(&d, &q).unwrap();
        let matured = traces.values().filter(|t| t.mature_tick.is_some()).count();
        let funnel = s.funnel_observer.as_ref().map(|f| f.report(&d, &q));
        let depth = traces.values().map(|t| t.descendant_births).sum::<u32>();
        if closed && react_to_occupancy && body_directed_packets && donor_min_energy == 25.0 {
            assert!(
                funnel.as_ref().unwrap()["maximum_closed_life_cycle_depth"]
                    .as_u64()
                    .unwrap()
                    >= 1
            );
            assert!(
                matured > 0 && depth > 0,
                "declared sensor-valid closed-loop certificate failed"
            );
            assert!(traces.iter().any(|(&lineage, c)| {
                c.descendant_births > 0
                    && c.packet_count > 0
                    && adult_timeline
                        .iter()
                        .any(|e| e["lineage"] == lineage && e["gathered"].as_f64().unwrap() > 0.0)
            }));
        }
        eprintln!(
            "care adults={adults} transfer={transfer} closed={closed} tick={} births={} matured={matured} descendant_births={depth} peaks={peaks:?}",
            s.tick, metrics.events[3]
        );
        results.push(json!({"adults":adults,"transfer":transfer,"closed":closed,"tick":s.tick,"peaks":peaks,"adult_food_seen_ticks":seen,"adult_gathered":gathered,"metrics":metrics,"matured":matured,"descendant_births":depth,"children":traces,"adult_timeline":adult_timeline,"packet_timeline":packet_timeline,"compatible_packet_ticks":compatible_packet_ticks,"closest_compatible_packets":closest_compatible_packets,"funnel":funnel}));
    }
    file.write_all(&serde_json::to_vec_pretty(&json!({"results":results,"delivery_cap":delivery_cap,"dose_scope":"If delivery_cap is set, only the two-adult care fixture runs. An external diagnostic gate disables transfer after that many actual juvenile delivery ticks; all production settings remain unchanged. The gate is not an agent capability, evolved policy, or intervention in the hereditary chain.","donor_min_energy":donor_min_energy,"body_directed_packets":body_directed_packets,"react_to_occupancy":react_to_occupancy,"scope":"Production unchanged. Selected seed91 location; adults start35 energy/zero food. Care fixtures place at initialization an actual age0 newborn created by ordinary size16 packet fusion; no reserve/food edits after initialization. Closed trial starts only two adults and all births are in-world fusion. Controller reads actual raw food/occupancy/pressure region inputs and own energy/food/age; command-memory estimates spin. All organisms use same age-conditional policy, updated every tick, with no identity/position/heading/recipient info. Adult .35 speed, young body-following, adult cohesion when distant, generic transfer above the recorded donor_min_energy and .01food and local pressure>.75; production above85energy/.3food every200ticks. No transfer targeting changes. Observers reconstruct pre-interaction inventory from exact digestion/collection and use post-motion geometry (no force); nearest ties use iteration order rather than GPU priority and are a limitation. Opportunity counts are unique child-ticks; blocking/diversion counts are donor-child offers. Birth ancestry attribution follows one recorded producer branch; positive descendant reproduction is conservative. Sensor-only hand controller, not heritable implementation or random-founder cohort."})).unwrap()).unwrap();
}

#[test]
fn sensor_policy_ignores_undeclared_channels() {
    let mut x = [0.0; INPUTS];
    x[0] = 0.9;
    x[1] = 0.1;
    x[3] = 0.2;
    for k in 0..16 {
        x[11 + k * 6] = 0.05 + 0.01 * k as f32;
        x[11 + k * 6 + 1] = 0.0625;
        x[11 + k * 6 + 5] = 0.8;
    }
    let mut other = x;
    for (k, v) in other.iter_mut().enumerate() {
        let allowed = [0, 1, 3].contains(&k) || (k >= 11 && [0, 1, 5].contains(&((k - 11) % 6)));
        if !allowed {
            *v = 7.5;
        }
    }
    assert_eq!(
        control(&View::from_inputs(&x), &mut Memory::default(), 0),
        control(&View::from_inputs(&other), &mut Memory::default(), 0)
    );
    assert_eq!(
        care_control(&x, &mut Memory::default(), true, true, 25.0, true, true),
        care_control(&other, &mut Memory::default(), true, true, 25.0, true, true)
    );
}

#[test]
#[ignore = "predeclared 100 independent ordinary random-founder worlds; no controlled policies"]
fn random_founder_cohort_after_sensor_certificate() {
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    use std::io::Write;
    let mut summary_file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("random-cohort.json"))
        .unwrap();
    let (d, q) = gpu();
    let mut rows = Vec::new();
    for seed in 2001..=2100 {
        let mut s = Simulation::new(&d, &q, seed);
        assert_eq!(s.settings.population, 4096);
        s.family_observer =
            Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 20000).unwrap());
        s.funnel_observer = Some(funnel_audit::FunnelObserver::new(&d, &s));
        let mut at_10000 = None;
        while s.tick < 20000 {
            let boundary = if s.tick < 10000 { 10000 } else { 20000 };
            let n = (boundary - s.tick).min(64);
            step(&mut s, &d, &q, n);
            let metrics = s.metrics(&d, &q).unwrap();
            if s.tick == 10000 {
                at_10000 = Some(
                    json!({"living":metrics.living,"packets":metrics.packets,"births":metrics.events[3]}),
                );
            }
            if metrics.living == 0 && metrics.packets == 0 {
                break;
            }
        }
        let metrics = s.metrics(&d, &q).unwrap();
        let families = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
        let funnel = s.funnel_observer.as_ref().unwrap().report(&d, &q);
        let matured: u32 = families
            .families
            .iter()
            .map(|f| f.matured_descendants)
            .sum();
        let descendant_births: u32 = families
            .families
            .iter()
            .map(|f| f.births_to_descendant_parents)
            .sum();
        let transfer_ticks: u32 = families
            .families
            .iter()
            .map(|f| f.juvenile_transfers_received)
            .sum();
        let received_milli: u64 = families
            .families
            .iter()
            .map(|f| f.juvenile_received_milli)
            .sum();
        eprintln!(
            "random seed={seed} tick={} births={} matured={matured} descendant_births={descendant_births} juvenile_transfers={transfer_ticks}",
            s.tick, metrics.events[3]
        );
        assert_eq!(funnel["births"], metrics.events[3]);
        assert_eq!(funnel["packets_produced"], metrics.birth_gates[2]);
        assert_eq!(funnel["juveniles_matured"], matured);
        assert_eq!(funnel["juvenile_transfer_ticks"], transfer_ticks);
        assert_eq!(funnel["juvenile_received_milli"], received_milli);
        let censored = metrics.living > 0 || metrics.packets > 0;
        let row = json!({"seed":seed,"tick":s.tick,"censored":censored,"at_10000":at_10000,"births":metrics.events[3],"matured":matured,"descendant_births":descendant_births,"juvenile_transfer_ticks":transfer_ticks,"juvenile_received_milli":received_milli,"metrics":metrics,"settings":s.settings,"families":families,"funnel":funnel});
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(format!("random-{seed}.json")))
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&row).unwrap())
            .unwrap();
        let mut compact = funnel.clone();
        compact.as_object_mut().unwrap().remove("individuals");
        compact.as_object_mut().unwrap().remove("life_events");
        rows.push(json!({"seed":seed,"tick":s.tick,"censored":censored,"at_10000":at_10000,"funnel":compact}));
    }
    summary_file.write_all(&serde_json::to_vec_pretty(&json!({"worlds":rows,"scope":"Predeclared seeds2001..2100 inclusive, default4096 genuinely random founders per fresh independent world, no policy/genome/body/ecology edits. All default physics including fractional gathering and flourishing_start retained. Per-tick GPU family observer; state sampled every64 ticks only to stop after all organisms and packets are gone, or at20000; survivors at the horizon are censored, not failures. No cross-world reservoir carryover between independent cohort replicates. Full funnel uses a per-tick, dual-parent GPU event observer, separately cross-checked against family counters. Protocol amended after the initial10000-tick run began; prior partial reports retained, all100 seeds replayed under the same20000-tick rule. Random extinctions do not invalidate the separate controlled sensor-valid reachability certificate. No biology tuning or reranking of seeds."})).unwrap()).unwrap();
}
