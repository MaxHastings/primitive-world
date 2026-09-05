//! Opt-in controlled comparisons. This module is compiled only into GPU tests.
use super::*;
use serde::{Deserialize, Serialize};
use std::{io::Write, path::Path};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Condition {
    Evolved,
    Random,
    NoMemory,
    NoSignals,
    Simple,
}

fn replace_contract(source: String, before: &str, after: &str) -> String {
    assert_eq!(
        source.matches(before).count(),
        1,
        "Diagnostic shader contract changed"
    );
    source.replacen(before, after, 1)
}

fn install(sim: &mut Simulation, device: &wgpu::Device, condition: Condition) {
    let source = include_str!("../shaders/decide.wgsl").to_owned();
    let source = match condition {
        Condition::Evolved | Condition::Random => source,
        Condition::NoMemory => replace_contract(source, "*a.hidden[k]", "*0.0"),
        Condition::NoSignals => replace_contract(
            source,
            " for(var k=0u;k<INPUT_COUNT;k++){if(!finite(x[k]))",
            " for(var n=0u;n<4u;n++){x[49u+n*8u]=0.0;x[51u+n*8u]=0.0;}\n for(var k=0u;k<INPUT_COUNT;k++){if(!finite(x[k]))",
        ),
        Condition::Simple => replace_contract(
            source,
            " // Fault containment only:",
            r#"
 // Fixed diagnostic policy: random walk, collect, and affordable reproduction.
 let heading=random01(a.lineage_id ^ params.lifecycle.x ^ hash_u32(params.tick/128u))*6.283185307;
 d.movement=vec2<f32>(cos(heading),sin(heading))*0.5;
 d.selected_action=COLLECT;d.amount=1.0;
 if(a.age>=params.sensor_and_padding.y && params.tick>=a.next_birth && a.energy>=75.0){
  d.selected_action=REPRODUCE;d.amount=0.5;
 }
 for(var k=0u;k<6u;k++){d.scores[k]=f32(k==d.selected_action);}
 for(var k=0u;k<HIDDEN_COUNT;k++){d.hidden[k]=0.0;}
 d.payload=0.0;d.force=vec2<f32>(0);d.target_id=INVALID;d.target_generation=0u;d.invalid=0u;
 // Fault containment only:"#,
        ),
    };
    let pass = Compute::new(
        device,
        "capability comparison",
        &source,
        "main",
        "rrwur",
        pair(|s| {
            vec![
                &sim.agent_buffers[s],
                &sim.perception_buffer,
                &sim.decision_buffer,
                &sim.params_buffer,
                &sim.genome_buffer,
            ]
        }),
    );
    sim.passes.insert("decide".into(), pass);
}

fn decide(sim: &Simulation, device: &wgpu::Device, queue: &wgpu::Queue) -> DecisionGpu {
    sim.update_params(queue);
    let mut encoder = device.create_command_encoder(&Default::default());
    sim.dispatch(&mut encoder, "decide", sim.current_buffer, 1, 1);
    queue.submit(Some(encoder.finish()));
    read::<DecisionGpu>(device, queue, &sim.decision_buffer, 1)[0]
}

#[test]
fn diagnostic_memory_removal_matches_zero_state_without_changing_genes() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    let mut genes = fixed(1, [0.0; 2]);
    genes[INPUTS] = 2.0;
    genes[OUTPUT_BASE + 6 * 17] = 1.0;
    put(&sim, &q, 0, a, &genes);
    let zero = decide(&sim, &d, &q);
    a.hidden[0] = 0.7;
    put(&sim, &q, 0, a, &genes);
    let retained = decide(&sim, &d, &q);
    assert!(retained.movement[0] > zero.movement[0] + 0.1);
    install(&mut sim, &d, Condition::NoMemory);
    let removed = decide(&sim, &d, &q);
    assert_eq!(bytemuck::bytes_of(&removed), bytemuck::bytes_of(&zero));
    assert_eq!(read::<f32>(&d, &q, &sim.genome_buffer, GENOME_SIZE), genes);
    assert_eq!(
        read::<AgentGpu>(&d, &q, &sim.agent_buffers[0], 1)[0].hidden,
        a.hidden
    );
}

#[test]
fn diagnostic_signal_mask_matches_absent_signal_and_retains_other_senses() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    sim.tick = 1;
    let a = body([602.0, 902.0]);
    let mut neighbor = body([604.0, 902.0]);
    let mut genes = fixed(1, [0.0; 2]);
    genes[49] = 1.0;
    genes[51] = 1.0;
    genes[OUTPUT_BASE + 6 * 17] = 1.0;
    put(&sim, &q, 0, a, &genes);
    put(&sim, &q, 1, neighbor, &genes);
    let mut perception = PerceptionGpu::zeroed();
    for b in &mut perception.bodies {
        b.slot = MAX_AGENTS;
    }
    perception.resource_here = 0.5;
    perception.bodies[0] = BodyGpu {
        slot: 1,
        offset: [2.0, 0.0],
        ..Default::default()
    };
    q.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&perception));
    let absent = decide(&sim, &d, &q);
    neighbor.signal_tick = 1;
    perception.bodies[0].signal = 0.7;
    put(&sim, &q, 1, neighbor, &genes);
    q.write_buffer(&sim.perception_buffer, 0, bytemuck::bytes_of(&perception));
    let present = decide(&sim, &d, &q);
    assert!(present.movement[0] > absent.movement[0] + 0.1);
    install(&mut sim, &d, Condition::NoSignals);
    let masked = decide(&sim, &d, &q);
    assert_eq!(bytemuck::bytes_of(&masked), bytemuck::bytes_of(&absent));
    assert_eq!(
        read::<AgentGpu>(&d, &q, &sim.agent_buffers[0], 2)[1].signal_tick,
        1
    );
    // Masking reception leaves successful emissions and their energy cost intact.
    let emitter = fixed(4, [0.0; 2]);
    put(&sim, &q, 0, a, &emitter);
    step(&mut sim, &d, &q, 1);
    assert_eq!(sim.metrics(&d, &q).unwrap().signals, 1);
}

#[test]
fn diagnostic_simple_policy_collects_moves_and_can_reproduce() {
    let (d, q) = gpu();
    let mut sim = scene(&d, &q);
    install(&mut sim, &d, Condition::Simple);
    let mut a = body([602.0, 902.0]);
    a.energy = 60.0;
    let genes = fixed(4, [-1.0; 2]);
    put(&sim, &q, 0, a, &genes);
    let collecting = decide(&sim, &d, &q);
    assert_eq!(collecting.selected_action, 1);
    near(collecting.movement[0].hypot(collecting.movement[1]), 0.5);
    assert_eq!(collecting.amount, 1.0);
    a.energy = 80.0;
    put(&sim, &q, 0, a, &genes);
    assert_eq!(decide(&sim, &d, &q).selected_action, 5);
    step(&mut sim, &d, &q, 1);
    assert_eq!(sim.metrics(&d, &q).unwrap().events[3], 1);
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Trial {
    id: String,
    seed: u32,
    rotation: u32,
    replicate: u32,
    condition: Condition,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    settings: SimSettings,
    horizon: u32,
    sample: u32,
    trials: Vec<Trial>,
    provenance: serde_json::Value,
}

fn write_report(path: &Path, report: &serde_json::Value) {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(report).unwrap())
        .unwrap();
    file.write_all(b"\n").unwrap();
}

#[test]
#[ignore = "requires PRIMITIVE_EXPERIMENT_PLAN and a new PRIMITIVE_EXPERIMENT_OUTPUT directory"]
fn run_registered_comparison() {
    let input = std::env::var("PRIMITIVE_EXPERIMENT_PLAN").expect("plan JSON path");
    let output = std::env::var("PRIMITIVE_EXPERIMENT_OUTPUT").expect("new output directory");
    let plan_bytes = std::fs::read(input).unwrap();
    let plan: Plan = serde_json::from_slice(&plan_bytes).unwrap();
    plan.settings.validate().unwrap();
    assert_eq!(plan.settings.founder_genomes.len(), 256);
    assert!(plan.settings.population > 0);
    assert!((1..=32768).contains(&plan.horizon));
    assert!(plan.sample > 0 && plan.sample <= plan.horizon);
    assert!(!plan.trials.is_empty());
    let mut ids = std::collections::HashSet::new();
    for trial in &plan.trials {
        assert!(trial.rotation < 4);
        assert!(
            !trial.id.is_empty()
                && trial
                    .id
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        assert!(ids.insert(&trial.id));
    }
    let output = Path::new(&output);
    std::fs::create_dir(output).expect("output directory must be new");
    std::fs::write(output.join("registered-plan.json"), &plan_bytes).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("GPU");
    let info = format!("{:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
            ..Default::default()
        },
        None,
    ))
    .unwrap();
    let mut sim = Simulation::new(&device, &queue, 1);
    for (index, trial) in plan.trials.iter().enumerate() {
        let start = std::time::Instant::now();
        sim.settings = plan.settings.clone();
        sim.settings.environment_rotation = trial.rotation;
        let expected_bodies = build_agents(trial.seed, &sim.settings);
        if matches!(trial.condition, Condition::Random) {
            sim.settings.founder_genomes = crate::founders::bundled().genomes.clone();
            sim.settings.founder_name = "diagnostic-random-founders".into();
        }
        assert_eq!(
            bytemuck::cast_slice::<AgentGpu, u8>(&build_agents(trial.seed, &sim.settings)),
            bytemuck::cast_slice::<AgentGpu, u8>(&expected_bodies),
            "All conditions must start with identical bodies",
        );
        sim.seed = trial.seed;
        sim.reset(&queue);
        install(&mut sim, &device, trial.condition);
        let mut history = vec![sim.metrics(&device, &queue).unwrap()];
        let mut living = plan.settings.population;
        let mut peak_living = living;
        while sim.tick < plan.horizon && living > 0 {
            let n = (plan.horizon - sim.tick)
                .min(32)
                .min(plan.sample - sim.tick % plan.sample);
            let mut encoder = device.create_command_encoder(&Default::default());
            sim.encode_ticks(&mut encoder, &device, &queue, n);
            sim.copy_alive_count(&mut encoder);
            queue.submit(Some(encoder.finish()));
            living = sim.read_alive_count(&device).unwrap();
            peak_living = peak_living.max(living);
            if sim.tick.is_multiple_of(plan.sample) || sim.tick == plan.horizon || living == 0 {
                let metrics = sim.metrics(&device, &queue).unwrap();
                assert_eq!(metrics.invalid_outputs, 0);
                assert_eq!(
                    u64::from(plan.settings.population) + u64::from(metrics.events[3]),
                    metrics.living
                        + u64::from(metrics.events[1])
                        + u64::from(metrics.events[2])
                        + u64::from(metrics.events[7])
                );
                history.push(metrics);
            }
        }
        let last = history.last().unwrap();
        let agent_ticks: u64 = last.action_ticks.iter().map(|&n| u64::from(n)).sum();
        let report = serde_json::json!({
            "schema":1, "trial":trial, "adapter":info,
            "provenance":plan.provenance,
            "horizon":plan.horizon, "sample":plan.sample,
            "termination_reason":if living==0 {"extinction"} else {"tick_limit"},
            "elapsed_ticks":sim.tick, "agent_ticks":agent_ticks,
            "peak_living_sampled_every_32_ticks":peak_living,
            "history":history, "wall_seconds":start.elapsed().as_secs_f64(),
            "scope":"Fresh matched bodies and environments. Conditions differ only in founder genomes or the documented diagnostic decision intervention. Within-world mutation and physical costs remain active. No cross-world carryover, rescue, or manual interventions. Tick-limit survivors are censored; two repeats do not establish broad generalization."
        });
        write_report(&output.join(format!("{}.json", trial.id)), &report);
        eprintln!(
            "TRIAL {}/{} {}: tick={}, living={}, births={}, food={:.3}, seconds={:.1}",
            index + 1,
            plan.trials.len(),
            trial.id,
            sim.tick,
            living,
            last.events[3],
            last.harvested,
            start.elapsed().as_secs_f64()
        );
    }
}
