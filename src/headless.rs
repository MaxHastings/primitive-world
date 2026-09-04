use crate::simulation::{MAX_AGENTS, Simulation, WORLD_SIZE};

pub fn run(args: &[String]) -> Result<(), String> {
    let value = |name: &str, default: u32| -> Result<u32, String> {
        match args.iter().position(|a| a == name) {
            Some(i) => args
                .get(i + 1)
                .ok_or_else(|| format!("Missing value for {name}"))?
                .parse()
                .map_err(|_| format!("Invalid {name}")),
            None => Ok(default),
        }
    };
    let ticks = value("--ticks", 2000)?;
    let population = value("--population", 1000)?;
    if population > MAX_AGENTS {
        return Err(format!("Population exceeds {MAX_AGENTS}"));
    }
    let seed = value("--seed", 1)?;
    let sample = value("--sample", 200)?.max(1);
    let shuffle = value("--shuffle-at", u32::MAX)?;
    let famine = value("--famine-at", u32::MAX)?;
    let restore = value("--restore-at", u32::MAX)?;
    let output = args
        .iter()
        .position(|a| a == "--output")
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
        .unwrap_or("headless-report.json");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .ok_or("No GPU adapter")?;
    let adapter_info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("headless world"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| e.to_string())?;
    let mut sim = Simulation::new(&device, &queue, seed);
    sim.settings.population = population;
    if let Some(i) = args.iter().position(|a| a == "--regeneration") {
        let regeneration: f32 = args
            .get(i + 1)
            .ok_or("Missing --regeneration")?
            .parse()
            .map_err(|_| "Invalid --regeneration")?;
        if !regeneration.is_finite() || regeneration < 0.0 {
            return Err("Regeneration must be finite and nonnegative".into());
        }
        sim.settings.resource_regeneration = regeneration;
    }
    if args.iter().any(|a| a == "--no-social") {
        sim.settings.social_access = 0.0;
        sim.settings.social_concern = 0.0;
        sim.settings.reciprocity = 0.0;
        sim.settings.communication_enabled = false;
    }
    if args.iter().any(|a| a == "--no-force") {
        sim.settings.force_enabled = false;
    }
    if args.iter().any(|a| a == "--static-landscape") {
        sim.settings.evolving_landscape = false;
    }
    sim.reset(&queue);
    let initial_settings = sim.settings.clone();
    let mut history = vec![sim.metrics(&device, &queue)?];
    let start = std::time::Instant::now();
    while sim.tick < ticks {
        if sim.tick == shuffle {
            sim.shuffle_relationships(&device, &queue)?;
        }
        if sim.tick == famine {
            sim.settings.resource_regeneration = 0.0;
            sim.apply_resource_shock(&device, &queue, [WORLD_SIZE / 2.0; 2], WORLD_SIZE, -1.0);
        }
        if sim.tick == restore {
            sim.settings.resource_regeneration = initial_settings.resource_regeneration;
            // Restore growing conditions; let the actual fertile regions refill.
            // A global food refill would temporarily erase the spatial ecology.
        }
        let next_sample = (sim.tick / sample + 1).saturating_mul(sample);
        let boundary = [ticks, next_sample, famine, restore, shuffle]
            .into_iter()
            .filter(|t| *t > sim.tick)
            .min()
            .unwrap();
        let count = (boundary - sim.tick).min(32);
        let mut encoder = device.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut encoder, &device, &queue, count);
        queue.submit(Some(encoder.finish()));
        device.poll(wgpu::Maintain::Wait);
        if sim.tick % sample == 0 || [ticks, famine, restore, shuffle].contains(&sim.tick) {
            let metrics = sim.metrics(&device, &queue)?;
            eprintln!(
                "tick {}: {} living, {} births, {} gifts, {} force",
                metrics.tick,
                metrics.living,
                metrics.events[3],
                metrics.events[4],
                metrics.events[5]
            );
            history.push(metrics);
        }
    }
    let seconds = start.elapsed().as_secs_f64();
    let report = serde_json::json!({"seed":seed,"initial_population":population,"ticks":ticks,"elapsed_seconds":seconds,"ticks_per_second":ticks as f64/seconds,"adapter":adapter_info.name,"backend":format!("{:?}",adapter_info.backend),"settings":initial_settings,"final_settings":sim.settings,"treatments":{"famine_at":(famine<ticks).then_some(famine),"restore_at":(restore<ticks).then_some(restore),"shuffle_at":(shuffle<ticks).then_some(shuffle)},"history":history});
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    eprintln!(
        "Saved {output}; {:.1} ticks/s (includes readbacks)",
        ticks as f64 / seconds
    );
    Ok(())
}
