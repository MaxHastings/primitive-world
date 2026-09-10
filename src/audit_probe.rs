//! Isolated diagnostic experiments. Never linked into the production executable.
use super::*;
use serde_json::json;

#[test]
#[ignore = "matched resting bodies measure actual production maintenance across development"]
fn juvenile_maintenance_accounting() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 91);
    s.settings.population = 0;
    s.reset(&q);
    let mut cases = Vec::new();
    for mask in [1u32, 0xffff] {
        for age in [0.0, 450.0, 900.0, 1350.0, 1800.0] {
            let slot = cases.len();
            let mut a = body([100.0 + slot as f32 * 64.0, 100.0]);
            a.energy = 20.0;
            a.food = 0.0;
            a.age = age;
            a.active_mask = mask;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
            cases.push((age, mask.count_ones()));
        }
    }
    // Inspect the production body-update pass before lifetime-learning charges.
    // Reset decision/request buffers are zero: rest, no gathering or motor effort.
    let mut encoder = d.create_command_encoder(&Default::default());
    s.dispatch(&mut encoder, "body", 0, MAX_AGENTS.div_ceil(64), 1);
    q.submit(Some(encoder.finish()));
    s.current_buffer = 1;
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let rows: Vec<_> = cases.iter().enumerate().map(|(slot, &(age, units))| {
        let a = agents[slot];
        let expected = s.settings.metabolic_cost + s.settings.active_unit_upkeep * units as f32;
        assert_eq!(a.collected, 0.0);
        assert_eq!(a.ingested, 0.0);
        assert!((20.0 - a.energy - expected).abs() < 0.00001);
        assert!((a.spent - expected).abs() < 0.00001);
        json!({"initial_age":age,"active_units":units,"energy_debit":20.0-a.energy,"spent":a.spent,"basal_body_cost":s.settings.metabolic_cost,"unit_upkeep":s.settings.active_unit_upkeep * units as f32})
    }).collect();
    save_report(
        "maintenance.json",
        json!({"rows":rows,"settings":s.settings,"scope":"One actual GPU body-update pass for matched resting organisms at five developmental ages, with one or sixteen active units. Ordinary production costs; zero decision/request buffers imply no movement or gathering, with no carried inventory. Initialization only; no physiology overrides. Inspected before separate lifetime-learning charges: this isolates basal and expressed-unit upkeep, not the complete cost of moving, learning, foraging or care."}),
    );
}

#[test]
fn fractional_gathering_preserves_curve_and_exact_food_debits() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut resources = vec![0u32; 512 * 512 * 8];
    // Independent cells avoid conflating request rounding with scarcity sharing.
    let cases = [
        (0.0, 1.0),
        (900.0, 1.0),
        (1800.0, 0.01),
        (1800.0, 1.0),
        (0.0, 0.0),
    ];
    for (group, &(age, effort)) in cases.iter().enumerate() {
        for n in 0..1024 {
            let slot = group * 1024 + n;
            let cell = slot;
            let mut a = body([
                (cell % 512) as f32 * 4.0 + 2.0,
                (cell / 512) as f32 * 4.0 + 2.0,
            ]);
            a.age = age;
            a.food = 0.0;
            a.rng = slot as u32 + 1;
            let mut genes = fixed(0, [0.0; 2]);
            genes[OUTPUT_BIAS + 1] = effort;
            put(&s, &q, slot, a, &genes);
            resources[cell * 8] = 1000; // Dropped food has no vegetation decay.
        }
    }
    q.write_buffer(&s.ground_buffer, 0, bytemuck::cast_slice(&resources));
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let remaining = read::<u32>(&d, &q, &s.ground_buffer, 512 * 512 * 8);
    for (group, &(age, effort)) in cases.iter().enumerate() {
        let expected = 25.0 * effort * (0.01 + 0.99 * (age / 1800.0_f32).min(1.0).powi(6));
        let mut sum = 0.0;
        for slot in group * 1024..(group + 1) * 1024 {
            let actual = (agents[slot].collected * 1000.0).round();
            assert!(actual >= expected.floor() && actual <= expected.ceil());
            assert_eq!(1000 - remaining[slot * 8], actual as u32);
            sum += actual;
        }
        assert!(
            (sum / 1024.0 - expected).abs() < 0.05,
            "{age}: {sum} vs {expected}"
        );
    }
    let mut saved = serde_json::to_value(SimSettings::default()).unwrap();
    assert!(
        serde_json::from_value::<SimSettings>(saved.clone())
            .unwrap()
            .fractional_gathering
    );
    saved
        .as_object_mut()
        .unwrap()
        .remove("fractional_gathering");
    assert!(
        !serde_json::from_value::<SimSettings>(saved)
            .unwrap()
            .fractional_gathering
    );
}

fn save_report(name: &str, value: serde_json::Value) {
    use std::io::Write;
    let directory = std::path::PathBuf::from(std::env::var("PRIMITIVE_AUDIT_OUTPUT").unwrap());
    std::fs::create_dir_all(&directory).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(name))
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(&value).unwrap())
        .unwrap();
}

#[test]
#[ignore = "ordinary-reserve local food-seeking controls; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn local_foraging_economy() {
    let (d, q) = gpu();
    let mut results = Vec::new();
    for (spinup, count) in [(0u32, 1usize), (0, 2), (10_000, 1), (10_000, 2)] {
        let mut s = Simulation::new(&d, &q, 91);
        s.settings.population = 0;
        s.reset(&q);
        for start in (0..spinup).step_by(1000) {
            step(&mut s, &d, &q, (spinup - start).min(1000));
        }
        let initial = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
        let start = initial
            .iter()
            .enumerate()
            .max_by_key(|(_, v)| *v)
            .unwrap()
            .0;
        let origin = [
            (start % 512) as f32 * 4.0 + 2.0,
            (start / 512) as f32 * 4.0 + 2.0,
        ];
        for slot in 0..count {
            let mut a = body([origin[0] + slot as f32, origin[1]]);
            a.energy = 35.0;
            a.food = 0.0;
            a.active_mask = 1;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
        }
        let mut target = origin;
        let mut peak_energy = vec![35.0_f32; count];
        let mut peak_food = vec![0.0_f32; count];
        let mut gathered = vec![0.0_f64; count];
        let mut spent = vec![0.0_f64; count];
        let mut surplus_ticks = vec![0u32; count];
        let mut records = Vec::new();
        for _ in 0..4000 {
            let agents = s.agent_snapshot(&d, &q).unwrap();
            if !agents[..count].iter().any(|a| a.alive == 1) {
                break;
            }
            let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
            let leader = agents[..count].iter().find(|a| a.alive == 1).unwrap();
            let tx = (target[0] / 4.0).floor() as i32;
            let ty = (target[1] / 4.0).floor() as i32;
            if food[ty.rem_euclid(512) as usize * 512 + tx.rem_euclid(512) as usize]
                < 25 * count as u32
                || s.tick == 0
            {
                let cx = (leader.position[0] / 4.0) as i32;
                let cy = (leader.position[1] / 4.0) as i32;
                let mut best = -1.0_f32;
                for dy in -6i32..=6 {
                    for dx in -6i32..=6 {
                        let x = (cx + dx).rem_euclid(512) as usize;
                        let y = (cy + dy).rem_euclid(512) as usize;
                        let distance = ((dx * dx + dy * dy) as f32).sqrt() * 4.0;
                        if distance > 24.0 {
                            continue;
                        }
                        let score = food[y * 512 + x] as f32 / (4.0 + distance);
                        if score > best {
                            best = score;
                            target = [x as f32 * 4.0 + 2.0, y as f32 * 4.0 + 2.0];
                        }
                    }
                }
            }
            for (slot, a) in agents.iter().enumerate().filter(|(_, a)| a.alive == 1) {
                // Founders can spend only after earning back more than a packet.
                // Descendants receive no care or controller assistance in this economy test.
                if slot >= count {
                    s.write_genome_slot(&q, slot, &fixed(0, [0.0; 2]));
                    continue;
                }
                let reproduce = count == 2 && a.energy > 60.0 && a.packets_produced < 2;
                let mut genes = fixed(if reproduce { 5 } else { 0 }, [0.0; 2]);
                let cell = (a.position[1] / 4.0) as usize * 512 + (a.position[0] / 4.0) as usize;
                genes[OUTPUT_BIAS + 1] = if food[cell] > 0 { 1.0 } else { 0.0 };
                let dx = target[0] - a.position[0];
                let dx = dx - 2048.0 * (dx / 2048.0 + 0.5).floor();
                let dy = target[1] - a.position[1];
                let dy = dy - 2048.0 * (dy / 2048.0 + 0.5).floor();
                let distance = dx.hypot(dy);
                if distance > 0.6 {
                    let error = (dy.atan2(dx) - a.heading + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    genes[OUTPUT_BIAS + 6] = (0.6 * error - 4.0 * a.angular_velocity)
                        .clamp(-0.8, 0.8)
                        .atanh();
                    let speed = (distance * 0.3).min(0.8) * error.cos().max(0.0);
                    genes[OUTPUT_BIAS + 7] = (speed / 1.2).atanh() / 4.0;
                }
                s.write_genome_slot(&q, slot, &genes);
            }
            step(&mut s, &d, &q, 1);
            let after = s.agent_snapshot(&d, &q).unwrap();
            for slot in 0..count {
                if agents[slot].alive != 1 {
                    continue;
                }
                let a = &after[slot];
                peak_energy[slot] = peak_energy[slot].max(a.energy);
                peak_food[slot] = peak_food[slot].max(a.food);
                gathered[slot] += f64::from(a.collected);
                spent[slot] += f64::from(a.spent);
                surplus_ticks[slot] += u32::from(a.food > 0.1);
            }
            if s.tick.is_multiple_of(100) {
                records.push(json!({"tick":s.tick,"agents":after[..count].iter().map(|a|json!({"energy":a.energy,"food":a.food,"packets":a.packets_produced,"alive":a.alive})).collect::<Vec<_>>()}));
            }
        }
        let metrics = s.metrics(&d, &q).unwrap();
        eprintln!(
            "local forage spinup={spinup} count={count} tick={} peak_energy={peak_energy:?} peak_food={peak_food:?} births={}",
            s.tick, metrics.events[3]
        );
        results.push(json!({"spinup":spinup,"founders":count,"tick":s.tick,"peak_energy":peak_energy,"peak_food":peak_food,"gathered":gathered,"spent":spent,"surplus_ticks_over_0_1":surplus_ticks,"metrics":metrics,"records":records}));
    }
    save_report(
        "local-foraging.json",
        json!({"results":results,"scope":"Predeclared seed 91; 4000 ticks; one and two ordinary-reserve adults (35 energy, zero food), selected minimum-cost one-unit brains and size-16 packets. Richest natural initial cell. Unmodified finite initial ecology and ordinary ongoing rainfall/regrowth, no painted food or reserve edits. Controller uses exact food cells within radius 24, chooses food/(4+distance) when target is depleted, and steers with real motor costs. Pair shares target. This observer-assisted upper-bound controller exceeds raw regional sensing resolution and is not an evolved or heritable strategy. No juvenile care; measures adult economy and births only. Climate initialization held fixed for causal comparison."}),
    );
}

#[test]
#[ignore = "selected natural straight corridor; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn corridor_foraging_economy() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 91);
    s.settings.population = 0;
    s.reset(&q);
    let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
    // Rank 128-cell eastbound corridors by the weaker lane's available food,
    // capped at five full harvests per cell at the declared .8 speed.
    let mut best = (0u32, 0usize, 0usize);
    for y in 0..512 {
        for x in 0..512 {
            let sum = |row: usize| {
                (0..128)
                    .map(|dx| food[row * 512 + (x + dx) % 512].min(125))
                    .sum::<u32>()
            };
            let score = sum(y).min(sum((y + 1) % 512));
            if score > best.0 {
                best = (score, x, y);
            }
        }
    }
    for slot in 0..2 {
        let mut a = body([
            best.1 as f32 * 4.0 + 2.0,
            ((best.2 + slot) % 512) as f32 * 4.0 + 2.0,
        ]);
        a.energy = 35.0;
        a.food = 0.0;
        a.active_mask = 1;
        a.lineage_id = slot as u32 + 1;
        put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
    }
    let mut peaks = [[35.0_f32, 0.0]; 2];
    let mut gathered = [0.0_f64; 2];
    let mut spent = [0.0_f64; 2];
    let mut records = Vec::new();
    for _ in 0..4000 {
        let agents = s.agent_snapshot(&d, &q).unwrap();
        if !agents[..2].iter().any(|a| a.alive == 1) {
            break;
        }
        let reproduce = s.tick >= 800
            && agents[..2]
                .iter()
                .all(|a| a.alive == 1 && a.energy > 60.0 && a.packets_produced < 2);
        for (slot, a) in agents.iter().enumerate().filter(|(_, a)| a.alive == 1) {
            let mut genes = fixed(0, [0.0; 2]);
            if slot < 2 {
                genes = fixed(if reproduce { 5 } else { 0 }, [0.0; 2]);
                genes[OUTPUT_BIAS + 1] = 1.0;
                genes[OUTPUT_BIAS + 7] = (0.8_f32 / 1.2).atanh() / 4.0;
                // Independent lanes, inward packet placement via ordinary actuator.
                genes[OUTPUT_BIAS + PLACEMENT_OUTPUT + 1] = if slot == 0 { 1.0 } else { -1.0 };
            }
            let _ = a;
            s.write_genome_slot(&q, slot, &genes);
        }
        step(&mut s, &d, &q, 1);
        let after = s.agent_snapshot(&d, &q).unwrap();
        for slot in 0..2 {
            if agents[slot].alive == 1 {
                peaks[slot][0] = peaks[slot][0].max(after[slot].energy);
                peaks[slot][1] = peaks[slot][1].max(after[slot].food);
                gathered[slot] += f64::from(after[slot].collected);
                spent[slot] += f64::from(after[slot].spent);
            }
        }
        if s.tick.is_multiple_of(100) {
            records.push(json!({"tick":s.tick,"agents":after[..2].iter().map(|a|json!({"energy":a.energy,"food":a.food,"packets":a.packets_produced,"alive":a.alive})).collect::<Vec<_>>()}));
        }
    }
    let metrics = s.metrics(&d, &q).unwrap();
    eprintln!(
        "corridor tick={} peaks={peaks:?} gathered={gathered:?} births={}",
        s.tick, metrics.events[3]
    );
    save_report(
        "corridor-foraging.json",
        json!({"seed":91,"origin_cell":[best.1,best.2],"initial_weaker_lane_milli":best.0,"ticks":s.tick,"peaks_energy_food":peaks,"gathered":gathered,"spent":spent,"metrics":metrics,"records":records,"scope":"Two ordinary-reserve one-unit size-16 founders, unmodified finite natural initial stocks and default climate/regrowth. Globally selected best 128-cell eastbound two-lane corridor, ranked by weaker lane capped harvestable initial food at .8 speed. Constant paid forward movement and full gathering; after tick800 both can manufacture up to two packets if each has >60 energy. Inward physical packet placement. No transfer or juvenile assistance. This is an oracle-selected route feasibility control, not a discovered policy or a test of sustainable descendants. Horizon 4000 ticks."}),
    );
}

#[derive(Default, serde::Serialize)]
struct Life {
    lineage: u32,
    birth_tick: u32,
    birth_energy: f32,
    last_age: f32,
    mature: bool,
    packets_produced: u32,
    dead: bool,
    received: f64,
    gathered: f64,
    ingested: f64,
    transfer_ticks: u32,
    first_transfer_age: Option<f32>,
    nearby_adult_ticks: u32,
    surplus_adult_ticks: u32,
    selecting_surplus_adult_ticks: u32,
    points: Vec<serde_json::Value>,
}

#[test]
#[ignore = "per-tick replay of three declared diagnostic seeds; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn trace_random_juvenile_opportunities() {
    let (d, q) = gpu();
    let mut worlds = Vec::new();
    for seed in [1, 3, 9] {
        let mut s = Simulation::new(&d, &q, seed);
        let mut before = s.agent_snapshot(&d, &q).unwrap();
        let mut lives = std::collections::BTreeMap::<u32, Life>::new();
        for _ in 0..10_000 {
            step(&mut s, &d, &q, 1);
            let after = s.agent_snapshot(&d, &q).unwrap();
            let adults: Vec<_> = after
                .iter()
                .enumerate()
                .filter(|(_, a)| a.alive == 1 && a.age >= s.settings.maturity_age)
                .collect();
            for (slot, a) in after
                .iter()
                .enumerate()
                .filter(|(_, a)| a.ancestry_depth > 0)
            {
                if a.alive == 1 && a.birth_tick == s.tick - 1 {
                    lives.entry(a.lineage_id).or_insert_with(|| Life {
                        lineage: a.lineage_id,
                        birth_tick: a.birth_tick,
                        birth_energy: a.energy,
                        ..Life::default()
                    });
                }
                let old = &before[slot];
                if old.alive != 1
                    || old.lineage_id != a.lineage_id
                    || old.generation != a.generation
                {
                    continue;
                }
                let Some(life) = lives.get_mut(&a.lineage_id) else {
                    continue;
                };
                life.last_age = a.age;
                life.dead = a.alive == 0;
                life.mature |= a.alive == 1 && a.age >= s.settings.maturity_age;
                life.packets_produced = a.packets_produced;
                if old.age < s.settings.maturity_age {
                    life.received += f64::from(a.received);
                    life.gathered += f64::from(a.collected);
                    life.ingested += f64::from(a.ingested);
                    if a.received > 0.0 {
                        life.transfer_ticks += 1;
                        life.first_transfer_age.get_or_insert(old.age);
                    }
                    let mut nearby = false;
                    let mut surplus = false;
                    let mut selecting = false;
                    for &(donor, b) in &adults {
                        if donor == slot {
                            continue;
                        }
                        let mut distance2 = 0.0;
                        for (axis, size) in [s.settings.habitat_width, s.settings.habitat_height]
                            .into_iter()
                            .enumerate()
                        {
                            let delta = b.position[axis] - a.position[axis];
                            let delta = delta - size * (delta / size + 0.5).floor();
                            distance2 += delta * delta;
                        }
                        if distance2 > 36.0 {
                            continue;
                        }
                        nearby = true;
                        let previous = &before[donor];
                        // Inventory immediately after digestion/gathering, before
                        // interactions, reconstructed only for the same incarnation.
                        let stock = if previous.lineage_id == b.lineage_id
                            && previous.generation == b.generation
                        {
                            (previous.food - b.ingested + b.collected).max(0.0)
                        } else {
                            0.0
                        };
                        if stock > 0.0 {
                            surplus = true;
                            selecting |= b.action == 2;
                        }
                    }
                    life.nearby_adult_ticks += u32::from(nearby);
                    life.surplus_adult_ticks += u32::from(surplus);
                    life.selecting_surplus_adult_ticks += u32::from(selecting);
                }
                if (a.age as u32).is_multiple_of(20) || a.received > 0.0 || a.alive == 0 {
                    life.points.push(json!({"tick":s.tick,"age":a.age,"energy":a.energy,
                        "food":a.food,"received":a.received,"collected":a.collected,"alive":a.alive}));
                }
            }
            let any = after.iter().any(|a| a.alive != 0);
            before = after;
            if !any {
                break;
            }
        }
        let metrics = s.metrics(&d, &q).unwrap();
        eprintln!(
            "audit seed {seed}: tick={} births={} traced={}",
            s.tick,
            metrics.events[3],
            lives.len()
        );
        worlds.push(
            json!({"seed":seed,"metrics":metrics,"offspring":lives.values().collect::<Vec<_>>()}),
        );
    }
    save_report(
        "juvenile-traces.json",
        json!({"worlds":worlds,"scope":"Seeds 1,3,9 selected from the prior ten-world cohort to inspect zero-transfer and occasional-transfer cases. Full per-tick state readback, unchanged default simulation, no interventions. Incarnation checked using lineage and generation. Contact opportunities use post-movement positions and reconstructed pre-interaction inventory; they are approximate eligibility opportunities, not arbitration outcomes or targeted offers. Juvenile transfer totals use actual received food. Not a new unbiased success-rate estimate."}),
    );
}

#[test]
fn gathering_quantization_and_nearest_full_recipient_are_explicit_constraints() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.fractional_gathering = false; // Historical comparison.
    let boundary = (0..1800)
        .find(|&age| {
            let x = age as f32 / 1800.0;
            (25.0 * (0.01 + 0.99 * x.powi(6))) as u32 > 0
        })
        .unwrap();
    for (slot, age) in [boundary - 1, boundary].into_iter().enumerate() {
        let mut a = body([602.0 + slot as f32 * 100.0, 902.0]);
        a.age = age as f32;
        a.food = 0.0;
        put(&s, &q, slot, a, &fixed(1, [0.0; 2]));
        let cell = 225 * 512 + 150 + slot * 25;
        q.write_buffer(
            &s.resource_buffer,
            cell as u64 * 4,
            bytemuck::bytes_of(&8000u32),
        );
    }
    step(&mut s, &d, &q, 1);
    let bodies = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(bodies[0].collected, 0.0);
    assert_eq!(bodies[1].collected, 0.001);
    eprintln!("First nonzero maximum juvenile harvest at age {boundary}");
    s.reset(&q);
    let mut donor = body([100.0, 100.0]);
    donor.food = 1.0;
    let mut full = body([101.0, 100.0]);
    full.food = 8.0;
    full.energy = 100.0;
    full.lineage_id = 2;
    let mut child = body([103.0, 100.0]);
    child.food = 0.0;
    child.age = 0.0;
    child.lineage_id = 3;
    put(&s, &q, 0, donor, &fixed(2, [0.0; 2]));
    put(&s, &q, 1, full, &fixed(0, [0.0; 2]));
    put(&s, &q, 2, child, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(bodies[2].received, 0.0);
    // The closest full body blocks a more distant valid receiver for this tick.
    assert_eq!(s.metrics(&d, &q).unwrap().events[4], 0);
}

#[test]
#[ignore = "controlled ordinary-reserve foraging experiments; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn controlled_foraging_life_cycle() {
    let (d, q) = gpu();
    let mut results = Vec::new();
    for moving in [false, true] {
        let mut s = Simulation::new(&d, &q, 91);
        s.settings.population = 0;
        s.reset(&q);
        let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
        // Choose a rich natural starting location explicitly for this upper-bound
        // feasibility trial. No food, terrain or climate fields are changed.
        let start = food.iter().enumerate().max_by_key(|(_, v)| *v).unwrap().0;
        let origin = [
            (start % 512) as f32 * 4.0 + 2.0,
            (start / 512) as f32 * 4.0 + 2.0,
        ];
        let offsets = [
            [-1.0, 0.0],
            [1.0, 0.0],
            [-4.0, 0.0],
            [4.0, 0.0],
            [0.0, -3.0],
            [0.0, 3.0],
        ];
        for (slot, offset) in offsets.iter().enumerate() {
            let a = AgentGpu {
                energy: 35.0,
                food: 0.0,
                packet_size: 16.0,
                lineage_id: slot as u32 + 1,
                active_mask: 1,
                parameter_mutation_rate: 1.0,
                parameter_mutation_step: 1.0,
                topology_mutation_rate: 1.0,
                ..body([origin[0] + offset[0], origin[1] + offset[1]])
            };
            put(&s, &q, slot, a, &fixed(0, [0.0; 2]));
        }
        s.family_observer =
            Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 6000).unwrap());
        let mut harvested = 0.0f64;
        let mut travelled = 0.0f64;
        for _ in 0..6000 {
            let bodies = s.agent_snapshot(&d, &q).unwrap();
            if !bodies.iter().any(|a| a.alive != 0) {
                break;
            }
            for (slot, a) in bodies.iter().enumerate().filter(|(_, a)| a.alive == 1) {
                let adult = a.age >= 1800.0;
                let breeder = slot < 2 || a.ancestry_depth > 0;
                let reproduce = adult && breeder && a.packets_produced < 2 && a.energy > 60.0;
                let mut genes = fixed(
                    if reproduce {
                        5
                    } else if adult && a.food > 0.1 {
                        2
                    } else {
                        0
                    },
                    [0.0; 2],
                );
                genes[OUTPUT_BIAS + 1] = 1.0;
                if moving {
                    let offset = if a.ancestry_depth == 0 {
                        offsets[slot]
                    } else {
                        [0.0, 0.0]
                    };
                    let dx = origin[0] + s.tick as f32 * 0.32 + offset[0] - a.position[0];
                    let dx = dx - 2048.0 * (dx / 2048.0 + 0.5).floor();
                    let dy = origin[1] + offset[1] - a.position[1];
                    let dy = dy - 2048.0 * (dy / 2048.0 + 0.5).floor();
                    let error = (dy.atan2(dx) - a.heading + std::f32::consts::PI)
                        .rem_euclid(std::f32::consts::TAU)
                        - std::f32::consts::PI;
                    let turn = (0.6 * error - 4.0 * a.angular_velocity).clamp(-0.8, 0.8);
                    genes[OUTPUT_BIAS + 6] = turn.atanh();
                    let development = 0.6 + 0.4 * (a.age / 1800.0).clamp(0.0, 1.0);
                    let speed = (0.32 + 0.2 * dx.hypot(dy)).min(0.8) * error.cos().max(0.0);
                    genes[OUTPUT_BIAS + 7] =
                        (speed / (1.2 * development)).clamp(0.0, 0.95).atanh() / 4.0;
                }
                s.write_genome_slot(&q, slot, &genes);
            }
            step(&mut s, &d, &q, 1);
            let after = s.agent_snapshot(&d, &q).unwrap();
            for (slot, a) in after.iter().enumerate() {
                if bodies[slot].alive == 1 && bodies[slot].lineage_id == a.lineage_id {
                    harvested += f64::from(a.collected);
                    travelled += f64::from(a.moved[0].hypot(a.moved[1]));
                }
            }
        }
        let families = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
        let row = json!({"moving":moving,"ticks":s.tick,"origin":origin,"initial_food_at_origin":food[start],
            "harvested":harvested,"distance":travelled,"metrics":s.metrics(&d,&q).unwrap(),"families":families});
        eprintln!(
            "controlled moving={moving}: tick={} births={} matured={} descendant births={}",
            s.tick,
            families.families[0].births,
            families.families[0].matured_descendants,
            families.families[0].births_to_descendant_parents
        );
        results.push(row);
    }
    save_report(
        "controlled-foraging.json",
        json!({"results":results,"scope":"Seed 91, unchanged default ecology/physics. Six mature bodies start with ordinary founder reserves (35 energy, zero food), but deliberately selected one-unit brains, packet size 16 and clustered rich starting positions. External scripted action readouts, full gathering, reproduction above 60 energy at most two packets, generic transfer above .1 carried food. Moving variant follows a common eastward formation at .32 units/tick using privileged position information and physical steering. No body/food/energy edits after initialization. Upper-bound controller experiment, not inherited evolution. 6000-tick predeclared ceiling; no guarantee of success."}),
    );
}

#[test]
#[ignore = "GPU ecology response snapshots across climate horizons; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn ecology_supply_response_snapshots() {
    let (d, q) = gpu();
    let mut rows = Vec::new();
    let mut s = Simulation::new(&d, &q, 91);
    s.settings.population = 0;
    for tick in [0, 500_000, 2_000_000, 5_000_000, 10_000_000, 24_000_000] {
        s.reset(&q);
        let cells = (RESOURCE_GRID * RESOURCE_GRID) as usize;
        let initial = read::<u32>(&d, &q, &s.resource_buffer, cells);
        let mut sorted = initial.clone();
        sorted.sort_unstable();
        let initial_total = initial.iter().map(|&n| f64::from(n) / 1000.0).sum::<f64>();
        // Hold geography fixed and start a depleted producer field with identical
        // water/mineral/detritus stocks. This isolates response at a forcing phase,
        // not the historical ecology that would have developed by that world age.
        s.settings.evolving_landscape = false;
        q.write_buffer(
            &s.resource_buffer,
            0,
            bytemuck::cast_slice(&vec![0u32; cells]),
        );
        let params = params_for(tick, tick, &s.settings, s.seed);
        q.write_buffer(&s.params_buffer, 0, bytemuck::bytes_of(&params));
        for _ in 0..8 {
            let mut encoder = d.create_command_encoder(&Default::default());
            for _ in 0..256 {
                s.dispatch(&mut encoder, "resource", 0, 64, 64);
            }
            q.submit(Some(encoder.finish()));
            d.poll(wgpu::Maintain::Wait);
        }
        let food = read::<u32>(&d, &q, &s.resource_buffer, cells);
        let pools = read::<[f32; 4]>(&d, &q, &s.ecology_buffer, cells);
        let mean_water = pools.iter().map(|p| f64::from(p[0])).sum::<f64>() / cells as f64;
        let food_total = food.iter().map(|&n| f64::from(n) / 1000.0).sum::<f64>();
        let dry = pools.iter().filter(|p| p[0] < 0.12).count();
        let mut produced = food.clone();
        produced.sort_unstable();
        rows.push(json!({"forcing_tick":tick,"rainfall":params.time_and_costs[0],"temperature":params.world_size[3],
            "initial_food_total":initial_total,"initial_food_min_milli":sorted[0],"initial_food_median_milli":sorted[cells/2],"initial_food_max_milli":sorted[cells-1],
            "food_after_2048":food_total,"net_food_per_tick":food_total/2048.0,"mean_water":mean_water,"dry_cell_fraction":dry as f64/cells as f64,
            "food_p10_milli":produced[cells/10],"food_p50_milli":produced[cells/2],"food_p90_milli":produced[cells*9/10]}));
    }
    save_report(
        "ecology-response.json",
        json!({"results":rows,"scope":"Actual GPU ecology kernel, seed91 fixed initial geography and identical initial stores at each trial, food depleted once before 2048 resource-only steps. Global and local weather frozen at each stated forcing tick. No agents or harvesting. Measures short response to selected phases, not a continuous 24-million-tick ecosystem, carrying capacity, connectivity history or sustained harvest yield."}),
    );
}

#[test]
#[ignore = "continuous 24-million-tick GPU ecology at 64 spatial sites; new PRIMITIVE_AUDIT_OUTPUT directory"]
fn continuous_ecology_history() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 91);
    s.settings.population = 0;
    s.reset(&q);
    let source=include_str!("../shaders/resource_update.wgsl")
        .replace("@group(0) @binding(1) var<uniform> params: SimParams;",
            "@group(0) @binding(1) var<storage,read> param_steps: array<SimParams>; var<private> params: SimParams;")
        .replace("@compute @workgroup_size(8,8,1)\nfn main(@builtin(global_invocation_id) id:vec3<u32>) {",
            "fn advance_cell(id:vec3<u32>) {");
    assert!(source.contains("fn advance_cell"));
    let source = format!(
        "{source}\n@compute @workgroup_size(8,8,1) fn main(@builtin(global_invocation_id) sample:vec3<u32>) {{ let id=vec3<u32>(sample.xy*64u+vec2<u32>(4u),0u); for(var t=0u;t<arrayLength(&param_steps);t++) {{ params=param_steps[t]; advance_cell(id); }} }}"
    );
    let steps = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("audit ecology time series"),
        size: 1000 * std::mem::size_of::<SimParams>() as u64,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let compute = Compute::new(
        &d,
        "audit independent cell history",
        &source,
        "main",
        "wrwwrw",
        vec![vec![
            &s.resource_buffer,
            &steps,
            &s.fertility_buffer,
            &s.ground_buffer,
            &s.terrain_buffer,
            &s.ecology_buffer,
        ]],
    );
    let sites: Vec<_> = (0..8)
        .flat_map(|y| (0..8).map(move |x| (y * 64 + 4) * 512 + x * 64 + 4))
        .collect();
    let mut records = Vec::new();
    for batch in 0..24_000u32 {
        let tick = batch * 1000;
        if tick.is_multiple_of(1_000_000) {
            q.write_buffer(
                &s.terrain_buffer,
                0,
                bytemuck::cast_slice(&build_terrain_pair(
                    s.seed,
                    tick / 1_000_000,
                    s.settings.habitat_contrast,
                )),
            );
        }
        let parameters: Vec<_> = (tick..tick + 1000)
            .map(|t| params_for(t, t, &s.settings, s.seed))
            .collect();
        q.write_buffer(&steps, 0, bytemuck::cast_slice(&parameters));
        let mut encoder = d.create_command_encoder(&Default::default());
        compute.dispatch(&mut encoder, 0, 1, 1);
        q.submit(Some(encoder.finish()));
        if batch == 0 || (batch + 1).is_multiple_of(100) {
            let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
            let pools = read::<[f32; 4]>(&d, &q, &s.ecology_buffer, 512 * 512);
            if batch == 0 {
                let mut reference = Simulation::new(&d, &q, 91);
                reference.settings.population = 0;
                reference.reset(&q);
                step(&mut reference, &d, &q, 1000);
                let expected_food = read::<u32>(&d, &q, &reference.resource_buffer, 512 * 512);
                let expected_pools = read::<[f32; 4]>(&d, &q, &reference.ecology_buffer, 512 * 512);
                for &site in &sites {
                    assert!(food[site].abs_diff(expected_food[site]) <= 1);
                    for k in 0..3 {
                        assert!((pools[site][k] - expected_pools[site][k]).abs() < 0.0001);
                    }
                }
            }
            for &site in &sites {
                assert!(pools[site][..3].iter().all(|x| x.is_finite() && *x >= 0.0));
            }
            records.push(json!({"tick":tick+1000,"food_milli":sites.iter().map(|&i|food[i]).collect::<Vec<_>>(),"pools":sites.iter().map(|&i|pools[i]).collect::<Vec<_>>()}));
        }
        if (batch + 1).is_multiple_of(1000) {
            eprintln!("continuous ecology: {} ticks", tick + 1000);
        }
    }
    save_report(
        "continuous-ecology.json",
        json!({"seed":91,"sites":sites,"records":records,"scope":"Continuous 24-million-tick, 64-site spatial sample using the actual resource kernel equations. The audit wrapper batches 1000 ordinary ticks inside each independent cell, with every tick's real climate parameters and normal million-tick terrain updates. First 1000 ticks cross-checked against the full simulation. No agents, harvesting or lateral transport; valid because current ecology cells have no cross-cell flux. Not a full-world population run, sustained harvest test or connectivity measurement."}),
    );
}
