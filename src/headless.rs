use crate::simulation::{MAX_AGENTS, MODEL_ID, Simulation};
use std::{collections::HashMap, io::Write, path::Path};
pub const HELP: &str = "Primitive World
Run: primitive_world [--seed N] [--founders PATH | --random-founders]
Headless: --headless --ticks N --sample N --output PATH
Single-window survivor loop: --watch-loop NEW_DIRECTORY [--view-speed 1x|2x|4x|8x|16x|MAX]
  Extinction saves genes and replaces the world in place, retaining speed and camera.
Options: --habitat-contrast X (0..1) --environment-rotation N (0..3)
         --population N --regeneration X --no-force --no-signals --static-landscape
         --metabolic-cost X --movement-cost X --motor-gain X
         --checkpoint PATH --save-checkpoint PATH --export-founders PATH
Headless observers:
         --families (fresh worlds, 1..200000 ticks; diagnostic only)
         --journeys PATH [--journey-sample N] (read-only sampled JSONL evidence)
         --survivors PATH [--survivor-sample N] (latest nonempty living sample;
           up to 64 current genomes, founders included; period 1..1024, default 128)
         --famine-at T --restore-at T --help --version
Fresh runs use 256 reproducible random founder genomes. --random-founders uses seed-specific random weights for each body. All random founders are untrained.
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
        "--headless",
        "--families",
        "--random-founders",
        "--no-force",
        "--no-signals",
        "--static-landscape",
        "--help",
        "--version",
    ];
    let valued = [
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
        "--watch-loop",
        "--view-speed",
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
    if out
        .get("--view-speed")
        .is_some_and(|v| !["1x", "2x", "4x", "8x", "16x", "MAX"].contains(&v.as_str()))
    {
        return Err("Invalid --view-speed: use 1x, 2x, 4x, 8x, 16x or MAX".into());
    }
    if out.contains_key("--watch-loop") {
        for key in [
            "--headless",
            "--ticks",
            "--families",
            "--output",
            "--survivors",
            "--survivor-sample",
            "--journeys",
            "--journey-sample",
            "--famine-at",
            "--restore-at",
        ] {
            if out.contains_key(key) {
                return Err(format!(
                    "--watch-loop is visible and extinction-only; cannot combine with {key}"
                ));
            }
        }
    }
    Ok(out)
}
pub fn configure(sim: &mut Simulation, args: &[String]) -> Result<(), String> {
    let a = arguments(args)?;
    if a.contains_key("--random-founders") && a.contains_key("--founders") {
        return Err("Choose random or a founder bank, not both".into());
    }
    if a.contains_key("--checkpoint") {
        for key in [
            "--environment-rotation",
            "--random-founders",
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
    if a.contains_key("--random-founders") {
        sim.use_random_founders();
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
    if survivor_file.is_some() {
        crate::survivor_observer::observe(&mut survivors, &sim, &device, &queue)?;
    }
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
    let mut extinct = history[0].living == 0;
    while sim.tick < target && !extinct {
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
        if sim.tick.is_multiple_of(sample) || sim.tick == target || extinct {
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
    if let Some(file) = &mut survivor_file {
        let sample = survivors
            .as_ref()
            .ok_or("No living bodies observed; no survivor bank available")?;
        file.write_all(&serde_json::to_vec(sample).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    }
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
    let export = a
        .get("--export-founders")
        .map(|path| sim.export_founders(&device, &queue, Path::new(path)));
    if let Some(path) = a.get("--save-checkpoint") {
        sim.save_checkpoint(&device, &queue, Path::new(path))?;
    }
    let report = serde_json::json!({"schema":2,"build_version":env!("CARGO_PKG_VERSION"),"model":MODEL_ID,"checkpoint_version":16,"capacity":MAX_AGENTS,"seed":sim.seed,
  "initial_tick":initial_tick,"requested_ticks":ticks,"elapsed_ticks":sim.tick-initial_tick,"adapter":format!("{info:?}"),
  "termination_reason":if extinct {"extinction"} else {"tick_limit"},
  "extinction_detection_max_delay_ticks":31,
  "initial_settings":settings,"final_settings":sim.settings,"history":history,"evolution":evolution,
  "travel_observer":travel.report(sample),
  "family_report":family_report,
  "survivor_observer":survivors.as_ref().map(|s| serde_json::json!({"source_tick":s.bank.source_tick,"source_population":s.source_population,"sampled_bodies":s.bodies.len(),"period":survivor_sample,"selection":s.selection})),
  "journey_observer":journey_file.as_ref().map(|_| journeys.report(journey_sample)),
  "famine_at":famine,"restore_at":restore,"wall_seconds":start.elapsed().as_secs_f64(),"founder_export":export,
  "scope":"One recurrent inherited controller. No within-life weight training, reseeding or population objective."});
    file.write_all(&serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    eprintln!("Saved {output}");
    Ok(())
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
