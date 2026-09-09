use crate::simulation::{
    CHECKPOINT_VERSION, EVENT_RING_SIZE, MAX_AGENTS, MAX_WORLD_TICKS, MODEL_ID, Simulation,
};
use std::{collections::HashMap, io::Write, path::Path};
pub const HELP: &str = "Primitive World
Run: primitive_world [--seed N] [--founders PATH]
Headless: --headless --ticks N --sample N --output PATH
Windowed viewer: --viewer opens New Game / Load Game. On Windows, no arguments resumes wallpaper.
Wallpaper viewer: --wallpaper uses the desktop host as a native-resolution habitat.
Windows integration: --install-startup registers wallpaper + auto-resume at login; --uninstall-startup removes it; --stop-wallpaper asks the wallpaper to save and close.
Wallpaper startup: --resume opens the latest saved experiment, or creates one if none exists.
Save cleanup: --prune-saves retains the newest six snapshots per experiment and caps the library at 16 GiB.
Headless rolling worlds: --headless --ticks N [--checkpoint PATH] [--save-checkpoint NEW_PATH]
Use --headless --single-world for diagnostics that stop at extinction.
  --load-game RECEIPT.json opens a saved experiment in the viewer.
Playback: --view-fps 10|30|60|120|144|240 (default 30; wallpaper defaults to monitor refresh) --compute-budget 10..100 (default 100)\n  1x targets 60 ticks/second; MAX is uncapped. Budget controls work/idle time, not hardware power.\nOptions: --viewer (regular window; double-click on Windows resumes wallpaper) --wallpaper --habitat-contrast X (0..1) --environment-rotation N (0..3)
         --population N --regeneration X --no-force --no-signals --static-landscape
         --metabolic-cost X (stationary upkeep) --movement-cost X --motor-gain X
         --checkpoint PATH --save-checkpoint PATH --export-founders PATH
Headless observers:
         --families (fresh worlds, 1..200000 ticks; diagnostic only)
         --journeys PATH [--journey-sample N] (read-only sampled JSONL evidence)
         --communication-trace PATH (read-only signal emissions and receiver responses)
         --survivors PATH [--survivor-sample N] (latest nonempty living sample;
           up to 64 current genomes, founders included; period 1..1024, default 128)
         --famine-at T --restore-at T [--famine-radius X --famine-delta X] --help --version
New Game and fresh command-line runs use seed-specific random weights for every founder. --founders imports a specified bank.
Motor gain calibrates continuous effort, not minimum movement or maximum speed.
Checkpoint settings take precedence; physical overrides cannot accompany --checkpoint.";
fn new_report(path: &str) -> Result<std::fs::File, String> {
    if let Some(parent) = Path::new(path)
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|e| format!("{path}: {e}"))?;
    }
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{path}: {e}"))
}

pub fn arguments(args: &[String]) -> Result<HashMap<String, String>, String> {
    let flags = [
        "--single-world",
        "--headless",
        "--families",
        "--no-force",
        "--no-signals",
        "--static-landscape",
        "--wallpaper",
        "--viewer",
        "--resume",
        "--help",
        "--version",
        "--install-startup",
        "--uninstall-startup",
    ];
    let valued = [
        "--load-game",
        "--habitat-contrast",
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
        "--survivors",
        "--survivor-sample",
        "--view-speed",
        "--view-fps",
        "--compute-budget",
        "--journeys",
        "--journey-sample",
        "--communication-trace",
        "--famine-at",
        "--restore-at",
        "--famine-radius",
        "--famine-delta",
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
    if out.contains_key("--viewer")
        && (out.contains_key("--wallpaper") || out.contains_key("--headless"))
    {
        return Err("Use --viewer by itself as the display mode".into());
    }
    if out
        .get("--view-speed")
        .is_some_and(|v| !crate::playback::SPEED_LABELS.contains(&v.as_str()))
    {
        return Err("Invalid --view-speed: use 1x, 2x, 4x, 8x, 16x, 32x, 64x, 128x or MAX".into());
    }
    if out
        .get("--view-fps")
        .is_some_and(|v| !["10", "30", "60", "120", "144", "240"].contains(&v.as_str()))
    {
        return Err("Invalid --view-fps: use 10, 30, 60, 120, 144 or 240".into());
    }
    if out
        .get("--compute-budget")
        .is_some_and(|v| v.parse::<u32>().map_or(true, |n| !(10..=100).contains(&n)))
    {
        return Err("Invalid --compute-budget: use an integer from 10 to 100".into());
    }
    if out.contains_key("--headless")
        && ["--view-speed", "--view-fps", "--compute-budget"]
            .iter()
            .any(|k| out.contains_key(*k))
    {
        return Err("Playback controls apply to the viewer; headless runs are uncapped".into());
    }
    if out.contains_key("--headless") && out.contains_key("--wallpaper") {
        return Err("--wallpaper applies to the viewer; headless runs are not rendered".into());
    }
    if out.contains_key("--resume") && !out.contains_key("--wallpaper") {
        return Err("--resume is for the wallpaper viewer".into());
    }
    if out.contains_key("--resume") && out.contains_key("--load-game") {
        return Err("--resume and --load-game are mutually exclusive".into());
    }
    if out.contains_key("--communication-trace") && !out.contains_key("--single-world") {
        return Err("--communication-trace requires --single-world".into());
    }
    if !out.contains_key("--headless") {
        for key in [
            "--ticks",
            "--sample",
            "--output",
            "--save-checkpoint",
            "--export-founders",
            "--families",
            "--journeys",
            "--journey-sample",
            "--communication-trace",
            "--survivors",
            "--survivor-sample",
            "--famine-at",
            "--restore-at",
            "--famine-radius",
            "--famine-delta",
        ] {
            if out.contains_key(key) {
                return Err(format!("{key} requires --headless"));
            }
        }
        if out.contains_key("--checkpoint") {
            return Err("Load the world save with --load-game RECEIPT.json".into());
        }
        if out.contains_key("--load-game") {
            for key in out.keys() {
                if ![
                    "--load-game",
                    "--wallpaper",
                    "--view-speed",
                    "--view-fps",
                    "--compute-budget",
                ]
                .contains(&key.as_str())
                {
                    return Err(format!(
                        "Load Game preserves the experiment; cannot override {key}"
                    ));
                }
            }
        }
    } else if out.contains_key("--load-game") {
        return Err("--load-game opens a saved world in the viewer".into());
    }
    if out.contains_key("--single-world") && !out.contains_key("--headless") {
        return Err("--single-world requires --headless".into());
    }
    if out.contains_key("--headless") && !out.contains_key("--single-world") {
        for key in [
            "--families",
            "--journeys",
            "--survivors",
            "--famine-at",
            "--restore-at",
            "--famine-radius",
            "--famine-delta",
            "--export-founders",
        ] {
            if out.contains_key(key) {
                return Err(format!("{key} requires --single-world"));
            }
        }
    }
    Ok(out)
}
pub fn configure(sim: &mut Simulation, args: &[String]) -> Result<(), String> {
    let a = arguments(args)?;
    if a.contains_key("--checkpoint") {
        for key in [
            "--environment-rotation",
            "--habitat-contrast",
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
    if let Some(v) = a.get("--habitat-contrast") {
        sim.settings.habitat_contrast = v.parse().map_err(|_| "Invalid habitat contrast")?;
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
    sim.settings.validate()
}
pub fn run(args: &[String]) -> Result<(), String> {
    let a = arguments(args)?;
    if !a.contains_key("--single-world") {
        return run_evolution(args, &a);
    }
    let number = |key: &str, default: u32| -> Result<u32, String> {
        a.get(key).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("Invalid {key}"))
        })
    };
    let ticks = number("--ticks", 2000)?;
    if a.contains_key("--families")
        && (a.contains_key("--checkpoint") || ticks == 0 || ticks > 200000)
    {
        return Err("--families requires a fresh world and 1..=200000 ticks".into());
    }
    let sample = number("--sample", 1000)?;
    let survivor_sample = number("--survivor-sample", 128)?;
    if survivor_sample == 0 || survivor_sample > 1024 {
        return Err("Survivor sample must be in 1..=1024".into());
    }
    if a.contains_key("--survivor-sample") && !a.contains_key("--survivors") {
        return Err("--survivor-sample requires --survivors PATH".into());
    }
    let mut survivor_file = a
        .get("--survivors")
        .map(|path| new_report(path))
        .transpose()?;
    let mut survivors = None;
    let journey_sample = number("--journey-sample", 32)?;
    if journey_sample == 0 || journey_sample > 1024 {
        return Err("Journey sample must be in 1..=1024".into());
    }
    if a.contains_key("--journey-sample") && !a.contains_key("--journeys") {
        return Err("--journey-sample requires --journeys PATH".into());
    }
    let mut journey_file = a
        .get("--journeys")
        .map(|path| new_report(path).map(std::io::BufWriter::new))
        .transpose()?;
    let mut journeys = crate::journey_observer::JourneyObserver::default();
    let mut communication_file = a
        .get("--communication-trace")
        .map(|path| new_report(path).map(std::io::BufWriter::new))
        .transpose()?;
    if sample == 0 || ticks > 1_000_000 {
        return Err("Sample must be positive; ticks must be <= 1000000".into());
    }
    let famine = number("--famine-at", u32::MAX)?;
    let restore = number("--restore-at", u32::MAX)?;
    let decimal = |key: &str, default: f32| -> Result<f32, String> {
        a.get(key).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("Invalid {key}"))
        })
    };
    let famine_radius = decimal("--famine-radius", 4096.0)?;
    let famine_delta = decimal("--famine-delta", -1000.0)?;
    if famine_radius <= 0.0 || !famine_radius.is_finite() {
        return Err("Famine radius must be finite and positive".into());
    }
    if famine_delta >= 0.0 || !famine_delta.is_finite() {
        return Err("Famine delta must be finite and negative".into());
    }
    if restore != u32::MAX && restore <= famine {
        return Err("Restore tick must follow famine".into());
    }
    let output = a
        .get("--output")
        .map(String::as_str)
        .unwrap_or("headless-report.json");
    let mut file = new_report(output)?;
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
    if a.contains_key("--families") {
        sim.family_observer = Some(crate::family_observer::FamilyObserver::new(
            &device, &queue, &sim, ticks,
        )?);
    }
    let initial_tick = sim.tick;
    let mut communication_next_sequence = if communication_file.is_some() {
        sim.event_sequence(&device, &queue)?
    } else {
        0
    };
    let mut communication_event_count = 0u64;
    let mut communication_dropped_events = 0u64;
    if let Some(file) = &mut communication_file {
        file.write_all(b"{\"events\":[")
            .map_err(|e| e.to_string())?;
    }
    if survivor_file.is_some() {
        crate::survivor_observer::observe(&mut survivors, &sim, &device, &queue)?;
    }
    let mut history = vec![sim.metrics(&device, &queue)?];
    let mut travel = crate::travel_observer::TravelObserver::default();
    travel.observe(sim.tick, &sim.agent_snapshot(&device, &queue)?)?;
    if let Some(file) = &mut journey_file {
        journeys.observe_in_habitat(
            sim.tick,
            &sim.agent_snapshot(&device, &queue)?,
            &sim.vegetation_snapshot(&device, &queue)?,
            [sim.settings.habitat_width, sim.settings.habitat_height],
        )?;
        let header = serde_json::json!({"type": "header", "model": MODEL_ID, "seed": sim.seed,
            "initial_tick": sim.tick, "observer": journeys.report(journey_sample)});
        writeln!(file, "{header}").map_err(|e| e.to_string())?;
    }
    let start = std::time::Instant::now();
    let target = sim.tick.checked_add(ticks).ok_or("Tick overflow")?;
    let mut extinct = history[0].living == 0;
    while sim.tick < target && !extinct && !sim.progress.engine_saturated {
        if sim.tick == famine {
            sim.apply_resource_shock(&device, &queue, [1024.0; 2], famine_radius, famine_delta);
            sim.settings.resource_regeneration = 0.0;
        }
        if sim.tick == restore {
            sim.settings.resource_regeneration = settings.resource_regeneration;
        }
        let next_sample = (sim.tick / sample + 1).saturating_mul(sample);
        let mut n = (target - sim.tick)
            .min(32)
            .min(next_sample - sim.tick)
            .min(MAX_WORLD_TICKS.saturating_sub(sim.tick));
        if n == 0 {
            break;
        }
        if journey_file.is_some() {
            n = n.min(journey_sample - sim.tick % journey_sample);
        }
        if survivor_file.is_some() {
            n = n.min(survivor_sample - sim.tick % survivor_sample);
        }
        for event in [famine, restore] {
            if event > sim.tick {
                n = n.min(event - sim.tick);
            }
        }
        let mut encoder = device.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut encoder, &device, &queue, n);
        // A four-byte readback bounds wasted work after extinction to this batch,
        // independently of the much less frequent full reporting interval.
        sim.copy_alive_count(&mut encoder);
        queue.submit(Some(encoder.finish()));
        if let Some(file) = &mut communication_file {
            let total = sim.event_sequence(&device, &queue)?;
            let events = sim.recent_events(&device, &queue)?;
            communication_dropped_events += u64::from(
                total
                    .wrapping_sub(communication_next_sequence)
                    .saturating_sub(EVENT_RING_SIZE),
            );
            for event in events.into_iter().filter(|event| {
                event.sequence >= communication_next_sequence
                    && event.sequence < total
                    && (event.action == crate::model::EMIT
                        || event.action == crate::model::SIGNAL_OBSERVED
                        || event.action == crate::model::MEMORY_SAMPLE)
            }) {
                if communication_event_count > 0 {
                    file.write_all(b",").map_err(|e| e.to_string())?;
                }
                serde_json::to_writer(&mut *file, &event).map_err(|e| e.to_string())?;
                communication_event_count += 1;
            }
            communication_next_sequence = total;
        }
        sim.refresh_engine_status(&device, &queue)?;
        extinct = sim
            .read_alive_count(&device)
            .ok_or("Could not read living population; refusing to guess extinction")?
            == 0;
        if survivor_file.is_some()
            && (sim.tick.is_multiple_of(survivor_sample) || sim.tick == target || extinct)
        {
            crate::survivor_observer::observe(&mut survivors, &sim, &device, &queue)?;
        }
        if let Some(file) = &mut journey_file
            && (sim.tick.is_multiple_of(journey_sample)
                || sim.tick == target
                || extinct
                || sim.tick.is_multiple_of(sample))
        {
            let events = journeys.observe_in_habitat(
                sim.tick,
                &sim.agent_snapshot(&device, &queue)?,
                &sim.vegetation_snapshot(&device, &queue)?,
                [sim.settings.habitat_width, sim.settings.habitat_height],
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
        if sim.tick.is_multiple_of(sample) || sim.tick == target || extinct {
            let m = sim.metrics(&device, &queue)?;
            travel.observe(sim.tick, &sim.agent_snapshot(&device, &queue)?)?;
            eprintln!(
                "tick {}: {} living, {} births, {} invalid outputs",
                m.tick, m.living, m.events[3], m.invalid_outputs
            );
            if history.len() == 4096 {
                history.remove(0);
            }
            history.push(m);
            if m.living == 0 {
                break;
            }
        }
    }
    let evolution = sim.evolution_snapshot(&device, &queue)?;
    // Diagnostics may intervene; preserve that exclusion instead of inventing a score.
    let population_completion = if extinct && sim.settings.population > 0 {
        Some(sim.complete_world(&device, &queue))
    } else {
        None
    };
    if let Some(file) = &mut survivor_file {
        let sample = survivors
            .as_ref()
            .ok_or("No living bodies observed; no survivor bank available")?;
        file.write_all(&serde_json::to_vec(sample).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
    sim.refresh_engine_status(&device, &queue)?;
    let family_report = sim
        .family_observer
        .as_ref()
        .map(|o| o.report(&device, &queue))
        .transpose()?;
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
    if let Some(file) = &mut communication_file {
        let trace = serde_json::json!({
            "schema": 1,
            "model": MODEL_ID,
            "initial_tick": initial_tick,
            "final_tick": sim.tick,
            "event_kinds": {"emit": crate::model::EMIT, "signal_observed": crate::model::SIGNAL_OBSERVED, "memory_sample": crate::model::MEMORY_SAMPLE},
            "event_count": communication_event_count,
            "overwritten_ring_events": communication_dropped_events,
            "limits": [
                "Signals are aggregate signed local activity, averaged over bodies in each body-relative sample for one tick.",
                "A signal_observed event records the receiver action for the first nonzero aggregate sample, at most once per receiver per tick; cancellation and zero payload are not observable presence.",
                "For signal_observed events, other_lineage stores the receiver action code because the event ring is shared with physical interactions.",
                "For memory_sample events, amount is the selected action score contribution from the carried-forward recurrent state, other stores the action selected with that contribution removed, and position stores old/new hidden-state norms; samples are roughly 1 in 512 decisions.",
                "This is behavioral correlation, not proof that the signal caused the response or carries a shared semantic code."
            ]
        });
        let metadata = serde_json::to_vec(&trace).map_err(|e| e.to_string())?;
        file.write_all(b"],").map_err(|e| e.to_string())?;
        file.write_all(&metadata[1..]).map_err(|e| e.to_string())?;
        file.flush().map_err(|e| e.to_string())?;
    }
    let export = a
        .get("--export-founders")
        .map(|path| sim.export_founders(&device, &queue, Path::new(path)));
    if let Some(path) = a.get("--save-checkpoint") {
        sim.save_checkpoint(&device, &queue, Path::new(path))?;
    }
    let report = serde_json::json!({"schema":3,"build_version":env!("CARGO_PKG_VERSION"),"model":MODEL_ID,"checkpoint_version":CHECKPOINT_VERSION,"capacity":MAX_AGENTS,"seed":sim.seed,
  "initial_tick":initial_tick,"requested_ticks":ticks,"elapsed_ticks":sim.tick-initial_tick,"adapter":format!("{info:?}"),
  "termination_reason":if sim.progress.engine_saturated {"engine_capacity"} else if extinct {"extinction"} else if sim.tick >= MAX_WORLD_TICKS {"tick_capacity"} else {"tick_limit"},
  "extinction_detection_max_delay_ticks":31,
  "initial_settings":settings,"final_settings":sim.settings,"history_limit":4096,"history":history,"evolution":evolution,
  "travel_observer":travel.report(sample),
  "family_report":family_report,
  "survivor_observer":survivors.as_ref().map(|s| serde_json::json!({"source_tick":s.bank.source_tick,"source_population":s.source_population,"sampled_bodies":s.bodies.len(),"period":survivor_sample,"selection":s.selection})),
  "journey_observer":journey_file.as_ref().map(|_| journeys.report(journey_sample)),
  "famine_at":famine,"restore_at":restore,"famine_radius":famine_radius,"famine_delta":famine_delta,"wall_seconds":start.elapsed().as_secs_f64(),"founder_export":export,
  "population_completion":population_completion,
  "scope":"Explicit single-world diagnostic. Observations do not affect population selection."});
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    eprintln!("Saved {output}");
    Ok(())
}

/// Bounded execution of the same extinction transitions used by the viewer.
fn run_evolution(args: &[String], a: &HashMap<String, String>) -> Result<(), String> {
    let number = |key: &str, default: u32| -> Result<u32, String> {
        a.get(key).map_or(Ok(default), |v| {
            v.parse().map_err(|_| format!("Invalid {key}"))
        })
    };
    let ticks = number("--ticks", 10000)?;
    let sample = number("--sample", 1024)?;
    if ticks > 1_000_000 || sample == 0 {
        return Err("Ticks must be <=1000000 and sample positive".into());
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).ok_or("No GPU")?;
    let info = adapter.get_info();
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("evolution"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| e.to_string())?;
    let mut sim = Simulation::new(&d, &q, number("--seed", 1)?);
    configure(&mut sim, args)?;
    sim.reset(&q);
    if let Some(path) = a.get("--checkpoint") {
        sim.load_checkpoint(&q, Path::new(path))?;
    }
    if sim.settings.population == 0 {
        return Err("Evolution requires at least one founder".into());
    }
    let mut report = new_report(
        a.get("--output")
            .map(String::as_str)
            .unwrap_or("evolution-report.json"),
    )?;
    let mut search_previous = None;
    let initial_search = sim.search_snapshot(&d, &q, &mut search_previous)?;
    let initial = sim.progress.clone();
    let settings = sim.settings.clone();
    let mut restart_seconds = 0.0;
    let mut simulation_seconds = 0.0;
    let mut elapsed = 0;
    let mut history = Vec::new();
    let start = std::time::Instant::now();
    let mut living = sim.metrics(&d, &q)?.living;
    while elapsed < ticks && !sim.progress.engine_saturated {
        if living == 0 {
            let at = std::time::Instant::now();
            sim.advance_world(&d, &q)?;
            restart_seconds += at.elapsed().as_secs_f64();
        }
        let batch_at = std::time::Instant::now();
        let n = (ticks - elapsed)
            .min(32)
            .min(sample - elapsed % sample)
            .min(MAX_WORLD_TICKS.saturating_sub(sim.tick));
        if n == 0 {
            sim.record_engine_saturation();
            break;
        }
        let mut encoder = d.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut encoder, &d, &q, n);
        sim.copy_alive_count(&mut encoder);
        q.submit(Some(encoder.finish()));
        living = u64::from(
            sim.read_alive_count(&d)
                .ok_or("Population readback failed")?,
        );
        elapsed += n;
        sim.refresh_engine_status(&d, &q)?;
        if living == 0 && !sim.progress.engine_saturated {
            sim.complete_world(&d, &q)?;
        }
        simulation_seconds += batch_at.elapsed().as_secs_f64();
        if living == 0 || elapsed == ticks || elapsed % sample == 0 {
            if history.len() == 4096 {
                history.remove(0);
            }
            history.push(serde_json::json!({"elapsed_ticks":elapsed,"progress":sim.progress,"metrics":sim.metrics(&d,&q)?,"evolution":sim.evolution_snapshot(&d,&q)?,"search":sim.search_snapshot(&d,&q,&mut search_previous)?}));
        }
    }
    if let Some(path) = a.get("--save-checkpoint") {
        sim.save_checkpoint(&d, &q, Path::new(path))?;
    }
    let termination_reason = if sim.progress.engine_saturated {
        "engine_capacity"
    } else if sim.tick >= MAX_WORLD_TICKS {
        "tick_capacity"
    } else {
        "tick_budget"
    };
    let value = serde_json::json!({"schema":6,"model":MODEL_ID,"build_version":env!("CARGO_PKG_VERSION"),"adapter":format!("{info:?}"),"requested_ticks":ticks,"elapsed_ticks":elapsed,
        "wall_seconds":start.elapsed().as_secs_f64(),"restart_seconds":restart_seconds,"simulation_and_sync_seconds":simulation_seconds,"settings":settings,"initial_progress":initial,"final_progress":sim.progress,
        "initial_search":initial_search,"history_limit":4096,"history":history,"termination_reason":termination_reason,"extinction_detection_max_delay_ticks":31,
        "scope":"Fresh founders are uniform samples of one rolling hereditary reservoir. Completed-world observations do not affect heredity."});
    report
        .write_all(&serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod file_tests {
    use super::*;

    #[test]
    fn new_reports_create_parents_but_never_overwrite() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root =
            std::env::temp_dir().join(format!("primitive-report-{}-{stamp}", std::process::id()));
        let path = root.join("nested/report.json");
        let name = path.to_str().unwrap();
        let mut file = new_report(name).unwrap();
        file.write_all(b"preserve").unwrap();
        drop(file);
        assert!(new_report(name).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"preserve");
        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(root.join("nested")).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
