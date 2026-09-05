use crate::simulation::{MAX_AGENTS, MODEL_ID, Simulation};
use std::{collections::HashMap, io::Write, path::Path};
pub const HELP: &str = "Primitive World physiology-v2 (checkpoint 14)\nRun: primitive_world [--seed N] [--founders PATH] [--bootstrap]\nHeadless: --headless --ticks N --sample N --output PATH\nOptions: --population N --regeneration X --no-force --no-signals --static-landscape\n         --metabolic-cost X --movement-cost X --motor-gain X\n         --checkpoint PATH --save-checkpoint PATH --export-founders PATH\n         --journeys PATH [--journey-sample N] (read-only sampled JSONL evidence)\n         --famine-at T --restore-at T --help --version\nFresh runs use a frozen UNPREPARED physiology-v2 bootstrap bank. --bootstrap explicitly uses UNPREPARED mutable seed weights.\nMotor gain calibrates continuous effort, not minimum movement or maximum speed.\nCheckpoint settings take precedence; physical overrides cannot accompany --checkpoint.\nLegacy controllers, neural bridges and old diagnostic flags are not supported.";
pub fn arguments(args: &[String]) -> Result<HashMap<String, String>, String> {
    let flags = [
        "--headless",
        "--bootstrap",
        "--no-force",
        "--no-signals",
        "--static-landscape",
        "--help",
        "--version",
    ];
    let valued = [
        "--environment-rotation",
        "--seed",
        "--founders",
        "--ticks",
        "--sample",
        "--output",
        "--population",
        "--regeneration",
        "--metabolic-cost",
        "--movement-cost",
        "--motor-gain",
        "--checkpoint",
        "--save-checkpoint",
        "--export-founders",
        "--journeys",
        "--journey-sample",
        "--famine-at",
        "--restore-at",
    ];
    let mut out = HashMap::new();
    let mut i = 1;
    while i < args.len() {
        let key = &args[i];
        if out.contains_key(key) {
            return Err(format!("Duplicate option {key}"));
        }
        if flags.contains(&key.as_str()) {
            out.insert(key.clone(), "true".into());
        } else if valued.contains(&key.as_str()) {
            i += 1;
            let v = args
                .get(i)
                .filter(|v| !v.starts_with("--"))
                .ok_or_else(|| format!("Missing {key}"))?;
            out.insert(key.clone(), v.clone());
        } else {
            return Err(format!(
                "Unsupported option {key}. {MODEL_ID} has one controller; use --help."
            ));
        }
        i += 1;
    }
    Ok(out)
}
pub fn configure(sim: &mut Simulation, args: &[String]) -> Result<(), String> {
    let a = arguments(args)?;
    if a.contains_key("--bootstrap") && a.contains_key("--founders") {
        return Err("Choose bootstrap or a founder bank, not both".into());
    }
    if a.contains_key("--checkpoint") {
        for key in [
            "--environment-rotation",
            "--bootstrap",
            "--founders",
            "--seed",
            "--population",
            "--regeneration",
            "--metabolic-cost",
            "--movement-cost",
            "--motor-gain",
            "--no-force",
            "--no-signals",
            "--static-landscape",
        ] {
            if a.contains_key(key) {
                return Err(format!(
                    "Checkpoint restores settings; cannot combine with {key}"
                ));
            }
        }
    }
    if let Some(v) = a.get("--environment-rotation") {
        sim.settings.environment_rotation = v
            .parse()
            .map_err(|_| "Invalid environment rotation (0..3 quarter turns)")?;
    }
    if let Some(v) = a.get("--seed") {
        sim.seed = v.parse().map_err(|_| "Invalid seed")?;
    }
    if let Some(v) = a.get("--population") {
        sim.settings.population = v.parse().map_err(|_| "Invalid population")?;
    }
    if let Some(v) = a.get("--regeneration") {
        sim.settings.resource_regeneration = v.parse().map_err(|_| "Invalid regeneration")?;
    }
    if let Some(v) = a.get("--metabolic-cost") {
        sim.settings.metabolic_cost = v.parse().map_err(|_| "Invalid metabolic cost")?;
    }
    if let Some(v) = a.get("--movement-cost") {
        sim.settings.movement_energy_cost = v.parse().map_err(|_| "Invalid movement cost")?;
    }
    if let Some(v) = a.get("--motor-gain") {
        sim.settings.motor_response_gain = v.parse().map_err(|_| "Invalid motor gain")?;
    }
    sim.settings.force_enabled = !a.contains_key("--no-force");
    sim.settings.communication_enabled = !a.contains_key("--no-signals");
    sim.settings.evolving_landscape = !a.contains_key("--static-landscape");
    if let Some(v) = a.get("--founders") {
        sim.load_founders(Path::new(v))?;
    }
    if a.contains_key("--bootstrap") {
        sim.use_bootstrap_founders();
    }
    sim.settings.validate()
}
pub fn run(args: &[String]) -> Result<(), String> {
    let a = arguments(args)?;
    let number = |key: &str, default: u32| -> Result<u32, String> {
        a.get(key).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("Invalid {key}"))
        })
    };
    let ticks = number("--ticks", 2000)?;
    let sample = number("--sample", 1000)?;
    let journey_sample = number("--journey-sample", 32)?;
    if journey_sample == 0 || journey_sample > 1024 {
        return Err("Journey sample must be in 1..=1024".into());
    }
    if a.contains_key("--journey-sample") && !a.contains_key("--journeys") {
        return Err("--journey-sample requires --journeys PATH".into());
    }
    let mut journey_file = a
        .get("--journeys")
        .map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map(std::io::BufWriter::new)
                .map_err(|e| format!("{path}: {e}"))
        })
        .transpose()?;
    let mut journeys = crate::journey_observer::JourneyObserver::default();
    if sample == 0 || ticks > 1_000_000 {
        return Err("Sample must be positive; ticks must be <= 1000000".into());
    }
    let famine = number("--famine-at", u32::MAX)?;
    let restore = number("--restore-at", u32::MAX)?;
    if restore != u32::MAX && restore <= famine {
        return Err("Restore tick must follow famine".into());
    }
    let output = a
        .get("--output")
        .map(String::as_str)
        .unwrap_or("headless-report.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output)
        .map_err(|e| format!("{output}: {e}"))?;
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).ok_or("No GPU")?;
    let info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("headless recurrent world"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| e.to_string())?;
    let mut sim = Simulation::new(&device, &queue, number("--seed", 1)?);
    configure(&mut sim, args)?;
    sim.reset(&queue);
    if let Some(path) = a.get("--checkpoint") {
        sim.load_checkpoint(&queue, Path::new(path))?;
    }
    let settings = sim.settings.clone();
    let initial_tick = sim.tick;
    let mut history = vec![sim.metrics(&device, &queue)?];
    let mut travel = crate::travel_observer::TravelObserver::default();
    travel.observe(sim.tick, &sim.agent_snapshot(&device, &queue)?)?;
    if let Some(file) = &mut journey_file {
        journeys.observe(
            sim.tick,
            &sim.agent_snapshot(&device, &queue)?,
            &sim.vegetation_snapshot(&device, &queue)?,
        )?;
        let header = serde_json::json!({"type": "header", "model": MODEL_ID, "seed": sim.seed,
            "initial_tick": sim.tick, "observer": journeys.report(journey_sample)});
        writeln!(file, "{header}").map_err(|e| e.to_string())?;
    }
    let start = std::time::Instant::now();
    let target = sim.tick.checked_add(ticks).ok_or("Tick overflow")?;
    while sim.tick < target {
        if sim.tick == famine {
            sim.apply_resource_shock(&device, &queue, [1024.0; 2], 4096.0, -1000.0);
            sim.settings.resource_regeneration = 0.0;
        }
        if sim.tick == restore {
            sim.settings.resource_regeneration = settings.resource_regeneration;
        }
        let next_sample = (sim.tick / sample + 1).saturating_mul(sample);
        let mut n = (target - sim.tick).min(32).min(next_sample - sim.tick);
        if journey_file.is_some() {
            n = n.min(journey_sample - sim.tick % journey_sample);
        }
        for event in [famine, restore] {
            if event > sim.tick {
                n = n.min(event - sim.tick);
            }
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut encoder, &device, &queue, n);
        queue.submit(Some(encoder.finish()));
        if let Some(file) = &mut journey_file
            && (sim.tick.is_multiple_of(journey_sample)
                || sim.tick == target
                || sim.tick.is_multiple_of(sample))
        {
            let events = journeys.observe(
                sim.tick,
                &sim.agent_snapshot(&device, &queue)?,
                &sim.vegetation_snapshot(&device, &queue)?,
            )?;
            for event in events {
                let line = serde_json::json!({"type": "journey", "evidence": event});
                writeln!(file, "{line}").map_err(|e| e.to_string())?;
            }
            for event in journeys.take_ended_attempts() {
                let line = serde_json::json!({"type": "ended_attempt", "evidence": event});
                writeln!(file, "{line}").map_err(|e| e.to_string())?;
            }
        }
        if sim.tick.is_multiple_of(sample) || sim.tick == target {
            let m = sim.metrics(&device, &queue)?;
            travel.observe(sim.tick, &sim.agent_snapshot(&device, &queue)?)?;
            eprintln!(
                "tick {}: {} living, {} births, {} invalid outputs",
                m.tick, m.living, m.events[3], m.invalid_outputs
            );
            history.push(m);
            if m.living == 0 {
                break;
            }
        }
    }
    let evolution = sim.evolution_snapshot(&device, &queue)?;
    if let Some(file) = &mut journey_file {
        journeys.finish(sim.tick);
        for event in journeys.take_ended_attempts() {
            let line = serde_json::json!({"type": "ended_attempt", "evidence": event});
            writeln!(file, "{line}").map_err(|e| e.to_string())?;
        }
        let footer =
            serde_json::json!({"type": "summary", "observer": journeys.report(journey_sample)});
        writeln!(file, "{footer}").map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
    }
    let export = a
        .get("--export-founders")
        .map(|path| sim.export_founders(&device, &queue, Path::new(path)));
    if let Some(path) = a.get("--save-checkpoint") {
        sim.save_checkpoint(&device, &queue, Path::new(path))?;
    }
    let report = serde_json::json!({"schema":2,"build_version":env!("CARGO_PKG_VERSION"),"model":MODEL_ID,"checkpoint_version":14,"capacity":MAX_AGENTS,"seed":sim.seed,
  "initial_tick":initial_tick,"requested_ticks":ticks,"elapsed_ticks":sim.tick-initial_tick,"adapter":format!("{info:?}"),
  "initial_settings":settings,"final_settings":sim.settings,"history":history,"evolution":evolution,
  "travel_observer":travel.report(sample),
  "journey_observer":journey_file.as_ref().map(|_| journeys.report(journey_sample)),
  "famine_at":famine,"restore_at":restore,"wall_seconds":start.elapsed().as_secs_f64(),"founder_export":export,
  "scope":"One recurrent inherited controller. No within-life weight training, reseeding or population objective."});
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    eprintln!("Saved {output}");
    Ok(())
}
