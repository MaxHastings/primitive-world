use super::*;

#[test]
#[ignore = "read-only validation of the user's current experiment library"]
fn current_experiment_library_loads_without_rewriting_saves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let (saves, skipped) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    eprintln!(
        "Checking {} current receipts; {skipped} historical/incompatible receipts skipped",
        saves.len()
    );
    for saved in saves {
        s.load_game_checkpoint(
            &q,
            std::fs::File::open(saved.checkpoint()).unwrap(),
            (saved.record.seed, saved.record.tick, saved.record.living),
            saved.record.world,
        )
        .unwrap_or_else(|error| panic!("{}: {error}", saved.directory.display()));
        assert_eq!(s.tick, saved.record.tick);
        assert_eq!(s.progress.world, saved.record.world);
        eprintln!(
            "Loaded {} world {} tick {}",
            saved.directory.display(),
            s.progress.world,
            s.tick
        );
    }
}

#[test]
fn terrain_noise_is_bounded_and_continuous_across_negative_grid_lines() {
    for seed in [1, 91, 3137] {
        for boundary in -80..=2 {
            let y = boundary as f32;
            let below = terrain_noise(0.37, y - 0.0001, seed);
            let above = terrain_noise(0.37, y + 0.0001, seed);
            assert!(
                (below - above).abs() < 0.001,
                "noise seam at y={y}: {below} -> {above}"
            );
            for x in [-2.7, -0.3, 0.37] {
                let value = terrain_noise(x, y - 0.3, seed);
                assert!((0.0..=1.0).contains(&value), "noise {value} at {x}, {y}");
            }
        }
    }
}

/// Raw data and the actual world/agent pipelines, without a surface or egui.
#[test]
#[ignore = "writes diagnostic PPM images to PRIMITIVE_RENDER_DIAGNOSTICS"]
fn capture_food_render_diagnostics() {
    let output = std::path::PathBuf::from(std::env::var("PRIMITIVE_RENDER_DIAGNOSTICS").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.settings.resource_regeneration = 1.0;
    s.reset_with_genomes_at(&q, None, 3_114_696);
    step(&mut s, &d, &q, 1);
    let raw = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
    let displayed = read::<u32>(&d, &q, &s.resource_display_buffer, 512 * 512);
    assert_eq!(raw, displayed, "display copy differs from simulation");
    let write = |name: &str, rgb: Vec<u8>| {
        let mut bytes = b"P6\n512 512\n255\n".to_vec();
        bytes.extend(rgb);
        std::fs::write(output.join(name), bytes).unwrap();
    };
    write(
        "raw-food.ppm",
        raw.iter()
            .flat_map(|v| {
                let value = (*v as f32 / 1000.0 * 255.0).clamp(0.0, 255.0) as u8;
                [value, value, value]
            })
            .collect(),
    );
    let texture = d.create_texture(&wgpu::TextureDescriptor {
        label: Some("diagnostic world"),
        size: wgpu::Extent3d {
            width: 512,
            height: 512,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());
    let mut renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8Unorm, &s, 512, 512);
    renderer.camera.lens = 0;
    renderer.update_camera(&q);
    let staging = readback(&d, 512 * 512 * 4);
    let mut previous = None;
    for frame in 0..2 {
        let mut e = d.create_command_encoder(&Default::default());
        {
            let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("diagnostic world pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            renderer.draw(&mut pass, &s);
        }
        e.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &staging,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(512 * 4),
                    rows_per_image: Some(512),
                },
            },
            texture.size(),
        );
        q.submit(Some(e.finish()));
        let (tx, rx) = mpsc::channel();
        staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        d.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let rgba = staging.slice(..).get_mapped_range().to_vec();
        staging.unmap();
        if let Some(old) = previous {
            assert_eq!(old, rgba, "paused render changed");
        }
        write(
            &format!("offscreen-{frame}.ppm"),
            rgba.chunks_exact(4)
                .flat_map(|p| [p[0], p[1], p[2]])
                .collect(),
        );
        previous = Some(rgba);
    }
}

/// Manual timing, never a performance assertion: GPU, driver and contention matter.
#[test]
#[ignore = "manual GPU throughput and per-pass profile"]
fn profile_tick_throughput() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("Adapter: {:?}", adapter.get_info());
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("tick profiler"),
            required_features: wgpu::Features::TIMESTAMP_QUERY,
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap();
    let mut s = Simulation::new(&d, &q, 42);
    for population in [32, 1000, 4096] {
        s.settings.population = population;
        s.reset(&q);
        step(&mut s, &d, &q, 32);
        let start = std::time::Instant::now();
        for _ in 0..16 {
            step(&mut s, &d, &q, 32);
        }
        eprintln!(
            "population={population}, batch=32: {:.0} ticks/s",
            512.0 / start.elapsed().as_secs_f64()
        );
        s.reset(&q);
        step(&mut s, &d, &q, 32);
        let start = std::time::Instant::now();
        for _ in 0..64 {
            step(&mut s, &d, &q, 8);
            crate::survivor_observer::observe(&mut None, &s, &d, &q).unwrap();
        }
        eprintln!(
            "population={population}, batch=8 + survivor snapshot: {:.0} ticks/s",
            512.0 / start.elapsed().as_secs_f64()
        );
        s.reset(&q);
        step(&mut s, &d, &q, 32);
        let telemetry = readback(&d, crate::playback::TELEMETRY_SIZE);
        let start = std::time::Instant::now();
        for _ in 0..64 {
            let mut encoder = d.create_command_encoder(&Default::default());
            s.encode_ticks(&mut encoder, &d, &q, 8);
            s.encode_telemetry(&mut encoder, &telemetry);
            q.submit(Some(encoder.finish()));
            let (tx, rx) = mpsc::channel();
            telemetry
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| {
                    tx.send(r).unwrap();
                });
            d.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let mapped = telemetry.slice(..).get_mapped_range();
            let counters: &[u32] = bytemuck::cast_slice(&mapped);
            let _living = counters[0];
            drop(mapped);
            telemetry.unmap();
        }
        eprintln!(
            "population={population}, batch=8 + telemetry: {:.0} ticks/s (no rendering)",
            512.0 / start.elapsed().as_secs_f64()
        );
        let mut names: Vec<_> = [
            "free",
            "free_blocks",
            "free_sums",
            "free_add",
            "free_compact",
            "resource",
            "clear",
            "count",
            "spatial_blocks",
            "spatial_sums",
            "spatial_add",
            "cursors",
            "scatter",
            "perceive_live",
            "decide_live",
            "consume",
            "body_live",
            "plastic",
            "observe_signals",
            "observe_memory",
            "inherit_genomes",
            "reset_cognitive_birth_state",
            "interact_clear",
            "interact_propose",
            "interact_resolve",
            "birth_blocks",
            "birth_sums",
            "birth_add",
            "birth_compact",
            "birth",
            "release",
            "alive",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        names.sort();
        let queries = d.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("per pass"),
            ty: wgpu::QueryType::Timestamp,
            count: names.len() as u32 * 2,
        });
        for (i, name) in names.iter().enumerate() {
            if let Some(pass) = s.passes.get_mut(name) {
                pass.timing = Some((queries.clone(), i as u32 * 2));
            }
        }
        step(&mut s, &d, &q, 1);
        let resolved = d.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: names.len() as u64 * 16,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut encoder = d.create_command_encoder(&Default::default());
        encoder.resolve_query_set(&queries, 0..names.len() as u32 * 2, &resolved, 0);
        q.submit(Some(encoder.finish()));
        let times = read::<u64>(&d, &q, &resolved, names.len() * 2);
        let mut rows: Vec<_> = names
            .iter()
            .enumerate()
            .filter_map(|(i, name)| {
                let delta = times[2 * i + 1].saturating_sub(times[2 * i]);
                (delta > 0).then_some((delta, name))
            })
            .collect();
        rows.sort_by(|a, b| b.0.cmp(&a.0));
        for (delta, name) in rows {
            eprintln!(
                "  {name}: {:.1} us",
                delta as f64 * f64::from(q.get_timestamp_period()) / 1000.0
            );
        }
        for pass in s.passes.values_mut() {
            pass.timing = None;
        }
        eprintln!(
            "After 545 ticks: {} living, {} births",
            s.metrics(&d, &q).unwrap().living,
            s.metrics(&d, &q).unwrap().events[3]
        );
    }
}

#[path = "experiment_tests.rs"]
mod experiments;

#[path = "evolution_tests.rs"]
mod evolution;

#[path = "sensing_tests.rs"]
mod sensing;

#[test]
fn environment_rotation_preserves_body_traits_and_is_not_a_controller_input() {
    let mut settings = SimSettings::default();
    let original = build_agents(1201, &settings);
    for turns in 0..4 {
        settings.environment_rotation = turns;
        let rotated = build_agents(1201, &settings);
        for (a, b) in original.iter().zip(rotated) {
            let mut expected = *a;
            expected.position = crate::environment::rotate_point(a.position, WORLD_SIZE, turns);
            assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&b));
        }
        let params = params_for(10, 50_010, &settings, 1201);
        assert_eq!(params.lifecycle[2], turns);
        assert_eq!(params.lifecycle[3], 50_010);
        assert_eq!(params.mutation[2], 1.0);
        assert_eq!(
            params.environment,
            [1.0, 1.0, 1.0, settings.memory_write_energy]
        );
        near(params.time_and_costs[3], 0.06);
    }
    for shader in [
        include_str!("../shaders/decide.wgsl"),
        include_str!("../shaders/perceive.wgsl"),
        include_str!("../shaders/update_agents.wgsl"),
        include_str!("../shaders/apply_births.wgsl"),
    ] {
        assert!(!shader.contains("lifecycle.z"));
    }
    settings.environment_rotation = 4;
    assert!(settings.validate().is_err());
}

#[test]
fn environment_rotation_permutates_resources_soil_and_weather_across_renewals() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.settings.resource_regeneration = 0.01;
    for tick in [0, 16384, 24576, 40960, 49152] {
        type EnvironmentSnapshot = (Vec<u32>, Vec<[u32; 8]>, Vec<f32>);
        let mut reference: Option<EnvironmentSnapshot> = None;
        for turns in 0..4 {
            s.settings.environment_rotation = turns;
            s.reset(&q);
            s.tick = tick;
            s.terrain_epoch = u32::MAX;
            step(&mut s, &d, &q, 32);
            let food = read::<u32>(
                &d,
                &q,
                &s.resource_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            let ground = read::<[u32; 8]>(
                &d,
                &q,
                &s.ground_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            let soil = read::<f32>(
                &d,
                &q,
                &s.fertility_buffer,
                (RESOURCE_GRID * RESOURCE_GRID) as usize,
            );
            if let Some((ref_food, ref_ground, ref_soil)) = &reference {
                assert_eq!(
                    food,
                    crate::environment::rotate_grid(
                        ref_food.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
                assert_eq!(
                    ground,
                    crate::environment::rotate_grid(
                        ref_ground.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
                assert_eq!(
                    soil,
                    crate::environment::rotate_grid(
                        ref_soil.clone(),
                        RESOURCE_GRID as usize,
                        turns
                    )
                );
            } else {
                reference = Some((food, ground, soil));
            }
        }
    }
}

#[test]
fn rotation_cli_and_checkpoint_preserve_explicit_environment_settings() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let args = |values: &[&str]| values.iter().map(|v| v.to_string()).collect::<Vec<_>>();
    crate::headless::configure(&mut s, &args(&["world", "--environment-rotation", "2"])).unwrap();
    s.reset(&q);
    let path = temp("rotated.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    s.settings.environment_rotation = 0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.settings.environment_rotation, 2);
    std::fs::remove_file(path).unwrap();
    assert!(
        crate::headless::configure(&mut s, &args(&["world", "--environment-rotation", "4"]))
            .is_err()
    );
    assert!(
        crate::headless::configure(
            &mut s,
            &args(&["world", "--environment-rotation", "1", "--checkpoint", "x"])
        )
        .is_err()
    );
    let value = serde_json::to_value(SimSettings::default()).unwrap();
    assert!(
        value.get("environment_rotation").is_none(),
        "Identity rotation is omitted from serialized settings"
    );
    assert_eq!(
        serde_json::from_value::<SimSettings>(value)
            .unwrap()
            .environment_rotation,
        0
    );
}

/// Explicit diagnostic: outcomes are measured, never required to point a chosen way.
/// Run separately after the frozen campaign; normal tests do not read research banks.
#[test]
#[ignore = "requires PRIMITIVE_DIRECTION_BANK and new PRIMITIVE_DIRECTION_OUTPUT path"]
fn directional_bank_gpu_probe() {
    let bank_path = std::env::var("PRIMITIVE_DIRECTION_BANK").expect("bank path");
    let output_path = std::env::var("PRIMITIVE_DIRECTION_OUTPUT").expect("new report path");
    let bank: crate::founders::FounderBank =
        serde_json::from_slice(&std::fs::read(&bank_path).unwrap()).unwrap();
    bank.validate().unwrap();
    let genomes = &bank.genomes;
    assert!(!genomes.is_empty() && genomes.len() <= 128);
    assert!(
        genomes
            .iter()
            .all(|g| g.len() == GENOME_SIZE && g.iter().all(|v| v.is_finite()))
    );
    let mut output = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(output_path)
        .unwrap();
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let perception = |direction: Option<usize>, food: f32| {
        let mut p = PerceptionGpu::default();
        for b in &mut p.bodies {
            b.slot = MAX_AGENTS;
        }
        for (k, region) in p.regions.iter_mut().enumerate() {
            let sector = direction.map(|d| [6, 0, 2, 4][d]);
            region.food = if sector == Some(k % 8) { food } else { 0.0 };
        }
        p
    };
    let dispatch = |p: PerceptionGpu| {
        q.write_buffer(
            &s.perception_buffer,
            0,
            bytemuck::cast_slice(&vec![p; genomes.len()]),
        );
        let mut encoder = d.create_command_encoder(&Default::default());
        s.dispatch(
            &mut encoder,
            "decide",
            s.current_buffer,
            (genomes.len() as u32).div_ceil(64),
            1,
        );
        q.submit(Some(encoder.finish()));
        let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, genomes.len());
        assert!(
            decisions
                .iter()
                .all(|x| x.invalid == 0 && x.movement.iter().all(|v| v.is_finite()))
        );
        decisions
    };
    let mut cases = Vec::new();
    for (energy, inventory) in [(10.0, 0.0), (50.0, 0.0), (50.0, 2.0), (100.0, 2.0)] {
        for food in [0.02, 0.2] {
            for (name, direction) in [
                ("bare", None),
                ("north", Some(0)),
                ("right", Some(1)),
                ("south", Some(2)),
                ("left", Some(3)),
            ] {
                for (slot, g) in genomes.iter().enumerate() {
                    let mut a = body([1026.0, 1026.0]);
                    a.energy = energy;
                    a.food = inventory;
                    put(&s, &q, slot, a, g.as_slice().try_into().unwrap());
                }
                let decisions = dispatch(perception(direction, food));
                let motors: Vec<_> = decisions
                    .iter()
                    .map(|v| [v.movement[0] * 1.2, v.movement[1] * 1.2])
                    .collect();
                cases.push(
                    serde_json::json!({"energy":energy,"inventory":inventory,"food_on_probes":food,
                    "food_side":name,"motors":motors}),
                );
            }
        }
    }
    let mut sequences = Vec::new();
    for food in [0.02, 0.2] {
        for first in [1usize, 3usize] {
            let mut bodies: Vec<_> = genomes
                .iter()
                .enumerate()
                .map(|(slot, g)| {
                    let mut a = body([1026.0, 1026.0]);
                    a.energy = 50.0;
                    a.food = 2.0;
                    put(&s, &q, slot, a, g.as_slice().try_into().unwrap());
                    a
                })
                .collect();
            let mut steps = Vec::new();
            for tick in 0..128 {
                let direction = if tick < 64 { first } else { 4 - first };
                let decisions = dispatch(perception(Some(direction), food));
                let motors: Vec<_> = decisions
                    .iter()
                    .map(|v| [v.movement[0] * 1.2, v.movement[1] * 1.2])
                    .collect();
                for ((a, decision), motor) in bodies.iter_mut().zip(&decisions).zip(&motors) {
                    a.hidden = decision.hidden;
                    a.velocity = *motor;
                    a.moved = *motor;
                    a.action = decision.selected_action;
                }
                q.write_buffer(
                    &s.agent_buffers[s.current_buffer],
                    0,
                    bytemuck::cast_slice(&bodies),
                );
                steps.push(
                    serde_json::json!({"step":tick+1,"food_direction":direction,"motors":motors}),
                );
            }
            sequences.push(
                serde_json::json!({"first_direction":first,"food_on_probes":food,"steps":steps}),
            );
        }
    }
    let report = serde_json::json!({"bank_path":bank_path,"bank_name":bank.name,"cases":cases,"sequences":sequences,
        "scope":"Actual GPU decision shader with synthetic mirrored perception, not full-world simulation. First decisions have empty state. Sequences hold adult age500, energy50, inventory2 and position fixed, carry hidden state, last action and motor feedback; cue reverses after64 of128 updates. No births, selection, sensing dispatch or ecological fitness measured."});
    std::io::Write::write_all(&mut output, &serde_json::to_vec_pretty(&report).unwrap()).unwrap();
}

#[test]
fn random_founders_are_finite_without_a_mandatory_food_response() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let bank = crate::founders::bundled();
    for energy in [10.0, 30.0, 50.0, 80.0] {
        let mut perceptions = vec![PerceptionGpu::default(); bank.genomes.len()];
        for (i, g) in bank.genomes.iter().enumerate() {
            let mut a = body([1026.0, 1026.0]);
            a.energy = energy;
            a.food = 0.0;
            put(&s, &q, i, a, g.as_slice().try_into().unwrap());
            perceptions[i].resource_here = 0.2;
            for b in &mut perceptions[i].bodies {
                b.slot = MAX_AGENTS;
            }
            for region in &mut perceptions[i].regions {
                region.food = 0.2;
            }
        }
        q.write_buffer(&s.perception_buffer, 0, bytemuck::cast_slice(&perceptions));
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "decide", s.current_buffer, 4, 1);
        q.submit(Some(e.finish()));
        let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, bank.genomes.len());
        let mut counts = [0u32; 6];
        for decision in decisions {
            assert_eq!(decision.invalid, 0);
            counts[decision.selected_action as usize] += 1;
        }
        println!("energy={energy} empty_inventory underfoot_food=0.2 action_counts={counts:?}");
        assert_eq!(counts.iter().sum::<u32>(), bank.genomes.len() as u32);
    }
}

#[test]
fn journey_observation_does_not_modify_physical_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 32);
    let expected = s.agent_snapshot(&d, &q).unwrap();
    s.reset(&q);
    put(&s, &q, 0, a, &g);
    let mut observer = crate::journey_observer::JourneyObserver::default();
    for _ in 0..4 {
        step(&mut s, &d, &q, 8);
        observer
            .observe(
                s.tick,
                &s.agent_snapshot(&d, &q).unwrap(),
                &s.vegetation_snapshot(&d, &q).unwrap(),
            )
            .unwrap();
    }
    let actual = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&expected),
        bytemuck::cast_slice::<AgentGpu, u8>(&actual)
    );
}

/// Sensor invariance, without prescribing what random or evolved brains choose.
/// Paired gradients cancel unconditional drift; only the food field changes.
#[test]
fn fixed_compass_sensors_ignore_reserved_body_padding() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let bank = crate::founders::bundled();
    let mut summary = Vec::new();
    for angle in [
        0.0,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
        -std::f32::consts::FRAC_PI_2,
    ] {
        for (slot, genome) in bank.genomes.iter().enumerate() {
            let mut a = body([1026.0, 1026.0]);
            a.energy = 50.0;
            a.food = 1.0;
            a.body_padding = angle;
            a.lineage_id = slot as u32 + 1;
            put(&s, &q, slot, a, genome.as_slice().try_into().unwrap());
        }
        let mut paired = Vec::new();
        for sign in [1.0f32, -1.0] {
            let resources: Vec<u32> = (0..512 * 512)
                .map(|i| {
                    let x = ((i % 512) as f32 + 0.5) * 4.0;
                    ((0.4 + sign * (x - 1026.0) * 0.01).clamp(0.0, 1.0) * 1000.0).round() as u32
                })
                .collect();
            q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&resources));
            let mut e = d.create_command_encoder(&Default::default());
            s.dispatch(&mut e, "perceive", s.current_buffer, 4, 1);
            s.dispatch(&mut e, "decide", s.current_buffer, 4, 1);
            q.submit(Some(e.finish()));
            let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, bank.genomes.len());
            assert!(decisions.iter().all(|v| v.invalid == 0));
            // Food probes remain world-aligned regardless of reserved padding.
            let east_slot = [0usize, 1, 2, 3]
                .into_iter()
                .max_by(|&a, &b| {
                    decisions[0].inputs[21 + 3 * a].total_cmp(&decisions[0].inputs[21 + 3 * b])
                })
                .unwrap();
            assert!(decisions[0].inputs[21 + 3 * east_slot] > 0.16);
            paired.push(decisions);
        }
        let deltas: Vec<[f32; 2]> = paired[0]
            .iter()
            .zip(&paired[1])
            .map(|(east, west)| {
                [
                    (east.movement[0] - west.movement[0]) * 0.5,
                    (east.movement[1] - west.movement[1]) * 0.5,
                ]
            })
            .collect();
        let mean = |axis: usize| deltas.iter().map(|v| v[axis]).sum::<f32>() / deltas.len() as f32;
        let toward = deltas.iter().filter(|v| v[0] > 0.0).count();
        println!(
            "reserved_padding={angle:.6} paired_food_response=({:.6},{:.6}) toward_east={toward}/{}",
            mean(0),
            mean(1),
            deltas.len()
        );
        summary.push((mean(0), toward));
    }
    // Every tested padding value leaves the food response unchanged.
    for response in &summary[1..] {
        near(response.0, summary[0].0);
    }
    // Sensor geometry must be invariant; random brains need not seek food.
}

#[test]
fn displacement_does_not_impose_a_hidden_reproduction_penalty() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    let amount = 1.0 / (1.0 + (-3.0f32).exp());
    // Displacement does not directly deduct energy from the recipient.
    parent.energy = 50.0 * (0.2 + 0.8 * amount) - 0.5;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, body([604.0, 902.0]), &fixed(3, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    assert_eq!(m.birth_gates[4], 1);
    assert_eq!(m.birth_gates[5], 1);
    assert_eq!(m.events[5], 1);
}
#[test]
fn dead_slot_reuse_resets_experience_and_advances_incarnation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut dead = body([500.0, 500.0]);
    dead.generation = 8;
    dead.hidden = [0.9; HIDDEN];
    put(&s, &q, 0, dead, &fixed(0, [0.0; 2]));
    s.kill_agents_in_region(&d, &q, [500.0, 500.0], 2.0);
    put(&s, &q, 1, body([602.0, 902.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    assert_eq!(bodies[0].generation, 9);
    assert_eq!(bodies[0].hidden, [0.0; HIDDEN]);
    assert_eq!(bodies[0].ancestry_depth, 1);
    assert_eq!(bodies[0].signal_tick, 0);
    near(s.metrics(&d, &q).unwrap().dropped_food as f32, 2.0);
}

#[test]
fn fresh_world_defaults_match_documented_physical_settings() {
    let settings = SimSettings::default();
    assert_eq!(settings.metabolic_cost, 0.06);
    assert_eq!(settings.metabolic_ramp_ticks, 50_000);
    near(crate::simulation::metabolic_cost_at(0, &settings), 0.01);
    near(
        crate::simulation::metabolic_cost_at(25_000, &settings),
        0.035,
    );
    near(
        crate::simulation::metabolic_cost_at(50_000, &settings),
        0.06,
    );
    near(
        crate::simulation::metabolic_cost_at(100_000, &settings),
        0.06,
    );
    let mut fixed = settings.clone();
    fixed.metabolic_ramp_ticks = 0;
    near(crate::simulation::metabolic_cost_at(0, &fixed), 0.06);
    assert!(settings.social_actions_enabled);
    assert!(settings.social_actions_enabled);
    // The shader spreads reproduction over ticks 0–2,500, transfer over
    // 52,500–65,000, signal over 65,000–80,000, and force over
    // 80,000–100,000.
    assert_eq!(settings.movement_energy_cost, 0.01);
    assert_eq!(settings.motor_response_gain, 4.0);
    assert_eq!(settings.resource_regeneration, 0.01);
    assert_eq!(settings.population, 1000);
    assert!(settings.evolving_landscape);
    settings.validate().unwrap();
}

#[test]
fn settings_require_explicit_motor_response_and_reject_bad_gains() {
    let mut value = serde_json::to_value(SimSettings::default()).unwrap();
    value.as_object_mut().unwrap().remove("motor_response_gain");
    assert!(serde_json::from_value::<SimSettings>(value).is_err());
    let mut legacy = serde_json::to_value(SimSettings::default()).unwrap();
    legacy
        .as_object_mut()
        .unwrap()
        .remove("metabolic_ramp_ticks");
    assert_eq!(
        serde_json::from_value::<SimSettings>(legacy)
            .unwrap()
            .metabolic_ramp_ticks,
        0,
        "flat-metabolism checkpoints must retain their original physics"
    );
    let settings = SimSettings::default();
    for gain in [0.0, -1.0, 33.0, f32::NAN, f32::INFINITY] {
        let settings = SimSettings {
            motor_response_gain: gain,
            ..settings.clone()
        };
        assert!(settings.validate().is_err());
    }
}

#[test]
fn motor_response_is_continuous_optional_reversible_and_bounded() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.motor_response_gain = 8.0;
    for effort in [0.0f32, 0.01, -0.01, 4.0] {
        let mut g = fixed(0, [0.0; 2]);
        g[OUTPUT_BIAS + 6] = effort;
        put(&s, &q, 0, body([602.0, 902.0]), &g);
        step(&mut s, &d, &q, 1);
        let a = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
        near(a.velocity[0], (effort * 8.0).tanh() * 1.2);
        near(a.velocity[1], 0.0);
        assert!(a.velocity[0].abs() <= 1.2001);
        near(a.spent, 0.06 + a.velocity[0].abs() * 0.01);
    }
}

#[test]
fn physical_cli_overrides_validate_and_cannot_override_checkpoints() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let args: Vec<String> = [
        "world",
        "--motor-gain",
        "8",
        "--metabolic-cost",
        "0.05",
        "--movement-cost",
        "0.02",
    ]
    .into_iter()
    .map(String::from)
    .collect();
    crate::headless::configure(&mut s, &args).unwrap();
    assert_eq!(s.settings.motor_response_gain, 8.0);
    assert_eq!(s.settings.metabolic_cost, 0.05);
    assert_eq!(s.settings.movement_energy_cost, 0.02);
    for flag in ["--motor-gain", "--metabolic-cost", "--movement-cost"] {
        let args: Vec<String> = [
            "world",
            "--headless",
            "--single-world",
            "--checkpoint",
            "unused.checkpoint",
            flag,
            "1",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert!(
            crate::headless::configure(&mut s, &args)
                .unwrap_err()
                .contains("Checkpoint restores settings")
        );
    }
    let args: Vec<String> = ["world", "--motor-gain", "0"]
        .into_iter()
        .map(String::from)
        .collect();
    assert!(crate::headless::configure(&mut s, &args).is_err());
}

#[test]
fn default_founders_are_seed_specific_random_genomes() {
    let settings = SimSettings::default();
    assert!(settings.founder_genomes.is_empty());
    assert_eq!(settings.founder_name, "primitive-world-random");
}

#[test]
fn concurrent_collection_and_pair_resolution_do_not_double_spend() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let pos = [602.0, 902.0];
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&17u32));
    for i in 0..8 {
        let mut a = body(pos);
        a.food = 0.0;
        a.lineage_id = i as u32 + 1;
        put(&s, &q, i, a, &fixed(1, [0.0; 2]));
    }
    step(&mut s, &d, &q, 1);
    let m = s.metrics(&d, &q).unwrap();
    near(
        (m.carried_food + m.vegetation) as f32 + m.events[0] as f32 / 1000.0,
        0.017,
    );
    assert_eq!(m.living, 8);
    // Every body requests transfer to the same target; accepted pairs must be disjoint.
    for i in 0..8 {
        let mut a = body(pos);
        a.lineage_id = i as u32 + 1;
        a.food = if i == 7 { 0.0 } else { 1.0 };
        put(&s, &q, i, a, &fixed(0, [0.0; 2]));
    }
    let mut decisions = vec![DecisionGpu::default(); 8];
    for item in &mut decisions[..7] {
        item.selected_action = 2;
        item.target = 7;
        item.target_generation = 1;
        item.amount = 1.0;
    }
    q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(&decisions));
    let mut e = d.create_command_encoder(&Default::default());
    for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
        s.dispatch(&mut e, pass, s.current_buffer, MAX_AGENTS / 64, 1);
    }
    q.submit(Some(e.finish()));
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 8);
    near(bodies.iter().map(|a| a.food).sum(), 7.0);
    near(bodies[7].food, 1.0);
}
#[test]
fn stale_targets_out_of_range_and_disabled_actions_cannot_claim() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.force_enabled = false;
    s.update_params(&q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    let run = |s: &Simulation, decisions: &[DecisionGpu]| {
        q.write_buffer(&s.decision_buffer, 0, bytemuck::cast_slice(decisions));
        let mut e = d.create_command_encoder(&Default::default());
        for pass in ["interact_clear", "interact_propose", "interact_resolve"] {
            s.dispatch(&mut e, pass, 0, MAX_AGENTS / 64, 1);
        }
        q.submit(Some(e.finish()));
        read::<AgentGpu>(&d, &q, &s.agent_buffers[0], 2)
    };
    let mut intent = DecisionGpu {
        selected_action: 2,
        target: 1,
        target_generation: 2,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent])[1].food, 0.0);
    intent.target_generation = 1;
    b.position = [620.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    near(run(&s, &[intent])[1].food, 0.0);
    b.position = [604.0, 902.0];
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    // Disabled force from receiver must not defeat a valid transfer in arbitration.
    let force = DecisionGpu {
        selected_action: 3,
        target: 0,
        target_generation: 1,
        amount: 1.0,
        ..Default::default()
    };
    near(run(&s, &[intent, force])[1].food, 1.0);
}

#[test]
fn disabled_social_actions_mask_social_logits_without_prescribing_behavior() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.social_actions_enabled = false;
    s.update_params(&q);
    // Transfer has the strongest logit, but disabled social actions leave the
    // controller's ordinary non-social options. Tied finite logits choose NONE
    // by its ordinary lower action index; no food response is injected.
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(2, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    assert_eq!(s.agent_snapshot(&d, &q).unwrap()[0].action, 0);
}

#[test]
fn reproduction_is_available_without_a_bootstrap_curriculum() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.reset(&q);
    let genome = fixed(5, [0.0; 2]);
    for slot in 0..256 {
        let mut a = body([602.0, 902.0]);
        a.lineage_id = slot as u32 + 1;
        put(&s, &q, slot, a, &genome);
    }

    s.tick = 0;
    step(&mut s, &d, &q, 1);
    let halfway = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 256)
        .into_iter()
        .filter(|decision| decision.selected_action == 5)
        .count();
    assert_eq!(halfway, 256);

    s.tick = 2_500;
    step(&mut s, &d, &q, 1);
    let complete = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 256)
        .into_iter()
        .filter(|decision| decision.selected_action == 5)
        .count();
    assert_eq!(complete, 256);
}
#[test]
fn amount_and_failed_actions_remain_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(2, [0.2, 0.0]); // no target exists
    g[OUTPUT_BIAS + 8] = -2.0;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let intent = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(b.action, 2);
    near(b.food + b.ingested, a.food);
    assert!(b.position[0] > a.position[0]);
    near(intent.amount, 1.0 / (1.0 + 2.0f32.exp()));
}
#[test]
fn founder_export_requires_descendants_and_preserves_existing_files() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    let path = temp("founders.json");
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert!(!path.exists());
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    s.export_founders(&d, &q, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(s.export_founders(&d, &q, &path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    s.load_founders(&path).unwrap();
    assert_eq!(s.settings.founder_genomes.len(), 1);
    assert_eq!(s.settings.founder_genomes[0].len(), GENOME_SIZE);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn survivor_sample_keeps_current_child_genes_after_extinction() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, body([602.0, 902.0]), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let child_slot = agents
        .iter()
        .position(|a| a.alive != 0 && a.ancestry_depth > 0)
        .unwrap();
    let actual = s.read_genomes(&d, &q, MAX_AGENTS as usize).unwrap();
    let child = actual[child_slot * GENOME_SIZE..(child_slot + 1) * GENOME_SIZE].to_vec();
    crate::brain::validate(&child).unwrap();
    let mut latest = None;
    crate::survivor_observer::observe(&mut latest, &s, &d, &q).unwrap();
    let saved = latest.as_ref().unwrap();
    let index = saved
        .bodies
        .iter()
        .position(|a| a.slot == child_slot)
        .unwrap();
    assert_eq!(saved.bank.genomes[index], child);
    assert!(saved.bodies.iter().any(|a| a.ancestry_depth == 0));
    let before = serde_json::to_vec(&latest).unwrap();
    let mut dead = agents;
    for a in &mut dead {
        a.alive = 0;
    }
    q.write_buffer(
        &s.agent_buffers[s.current_buffer],
        0,
        bytemuck::cast_slice(&dead),
    );
    s.tick += 128;
    crate::survivor_observer::observe(&mut latest, &s, &d, &q).unwrap();
    assert_eq!(serde_json::to_vec(&latest).unwrap(), before);
}

#[test]
fn birth_can_copy_a_parent_genome_exactly() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, body([602.0, 902.0]), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let child_slot = agents
        .iter()
        .position(|a| a.alive != 0 && a.ancestry_depth == 1)
        .expect("fixture must produce a child");
    let genomes = s.read_genomes(&d, &q, MAX_AGENTS as usize).unwrap();
    assert_eq!(
        &genomes[child_slot * GENOME_SIZE..(child_slot + 1) * GENOME_SIZE],
        parent.as_slice(),
        "an unchanged inheritance is a valid birth"
    );
}

#[test]
fn checkpoint_rejects_corrupt_trace_and_memory_without_mutating_live_world() {
    use std::io::{Seek, SeekFrom, Write};
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    put(&s, &q, 0, body([602.0, 902.0]), &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let path = temp("corrupt.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    assert!(s.save_checkpoint(&d, &q, &path).is_err());
    let lengths = [
        s.agent_buffers[0].size(),
        s.resource_buffer.size(),
        s.fertility_buffer.size(),
        s.ground_buffer.size(),
        s.death_stats_buffer.size(),
        s.event_buffer.size(),
    ];
    let pos = 24
        + s.checkpoint_metadata().unwrap().len() as u64
        + lengths.iter().map(|v| v + 8).sum::<u64>()
        + 8;
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(pos)).unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    drop(file);
    let before = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert!(
        s.load_checkpoint(&q, &path)
            .unwrap_err()
            .contains("perception")
    );
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&before),
        bytemuck::cast_slice::<AgentGpu, u8>(&after)
    );
    // Nonfinite recurrent memory must be rejected before changing live buffers.
    let memory_pos = 24
        + s.checkpoint_metadata().unwrap().len() as u64
        + 8
        + std::mem::offset_of!(AgentGpu, hidden) as u64;
    let mut file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.seek(SeekFrom::Start(pos)).unwrap();
    file.write_all(&0f32.to_le_bytes()).unwrap();
    file.seek(SeekFrom::Start(memory_pos)).unwrap();
    file.write_all(&f32::NAN.to_le_bytes()).unwrap();
    drop(file);
    assert!(
        s.load_checkpoint(&q, &path)
            .unwrap_err()
            .contains("body checkpoint")
    );
    assert_eq!(s.tick, 1);
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&before),
        bytemuck::cast_slice::<AgentGpu, u8>(&read::<AgentGpu>(
            &d,
            &q,
            &s.agent_buffers[s.current_buffer],
            1
        ))
    );
    std::fs::remove_file(path).unwrap();
}
fn gpu() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).expect("GPU");
    pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("recurrent tests"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap()
}
fn read<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    b: &wgpu::Buffer,
    count: usize,
) -> Vec<T> {
    let staging = readback(device, (count * std::mem::size_of::<T>()) as u64);
    let mut e = device.create_command_encoder(&Default::default());
    e.copy_buffer_to_buffer(b, 0, &staging, 0, staging.size());
    queue.submit(Some(e.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();
    let out = bytemuck::cast_slice(&staging.slice(..).get_mapped_range()).to_vec();
    staging.unmap();
    out
}
fn step(s: &mut Simulation, d: &wgpu::Device, q: &wgpu::Queue, n: u32) {
    let mut e = d.create_command_encoder(&Default::default());
    s.encode_ticks(&mut e, d, q, n);
    q.submit(Some(e.finish()));
    d.poll(wgpu::Maintain::Wait);
}
fn scene(d: &wgpu::Device, q: &wgpu::Queue) -> Simulation {
    let mut s = Simulation::new(d, q, 91);
    s.settings.population = 0;
    s.settings.social_actions_enabled = true;
    s.settings.metabolic_cost = 0.06;
    s.settings.metabolic_ramp_ticks = 0;
    // These fixtures isolate physical actions; cognitive costs have dedicated tests.
    s.settings.active_unit_upkeep = 0.0;
    s.settings.memory_write_energy = 0.0;
    s.settings.resource_regeneration = 0.0;
    s.settings.evolving_landscape = false;
    s.reset(q);
    q.write_buffer(&s.resource_buffer, 0, &vec![0; 512 * 512 * 4]);
    q.write_buffer(&s.ground_buffer, 0, &vec![0; 512 * 512 * 32]);
    s
}
fn body(pos: [f32; 2]) -> AgentGpu {
    AgentGpu {
        position: pos,
        energy: 80.0,
        food: 2.0,
        age: 500.0,
        max_speed: 1.2,
        sensor_radius: 24.0,
        max_age: 11000.0,
        alive: 1,
        generation: 1,
        target: MAX_AGENTS,
        lineage_id: 1,
        ..Default::default()
    }
}
fn fixed(action: usize, motion: [f32; 2]) -> [f32; GENOME_SIZE] {
    let mut g = crate::brain::blank();
    g[OUTPUT_BIAS + action] = 2.0;
    g[OUTPUT_BIAS + 6] = motion[0];
    g[OUTPUT_BIAS + 7] = motion[1];
    g[OUTPUT_BIAS + 8] = 3.0;
    if action == 3 {
        g[OUTPUT_BIAS + FORCE_OUTPUT] = 1.0;
    }
    g
}
fn put(s: &Simulation, q: &wgpu::Queue, slot: usize, a: AgentGpu, g: &[f32; GENOME_SIZE]) {
    for b in &s.agent_buffers {
        q.write_buffer(
            b,
            (slot * std::mem::size_of::<AgentGpu>()) as u64,
            bytemuck::bytes_of(&a),
        );
    }
    s.write_genome_slot(q, slot, g);
}
fn near(a: f32, b: f32) {
    assert!((a - b).abs() < 0.002, "{a} != {b}");
}
fn temp(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("recurrent-{}-{name}", std::process::id()))
}

#[test]
fn layout_and_cli_contract() {
    assert_eq!(GENOME_SIZE, 2612);
    assert_eq!(GENOME_BANK_COUNT, 2);
    assert_eq!(std::mem::size_of::<AgentGpu>(), 296);
    assert_eq!(std::mem::size_of::<PerceptionGpu>(), 400);
    assert_eq!(std::mem::size_of::<DecisionGpu>(), 800);
    assert_eq!(std::mem::size_of::<SelectionOutput>(), 1520);
    assert_eq!(std::mem::size_of::<SimParams>(), 144);
    assert!(MAX_AGENTS as usize * GENOME_BANK_STRIDE * 4 <= 256 * 1024 * 1024);
    for flag in [
        "--unknown-option",
        "--population-typo",
        "--unsupported-observer",
    ] {
        assert!(crate::headless::arguments(&["world".into(), flag.into()]).is_err());
    }
    assert!(
        crate::headless::arguments(&["world".into(), "--comparisons".into(), "1".into(),]).is_err()
    );
    assert!(
        crate::headless::arguments(&[
            "world".into(),
            "--headless".into(),
            "--single-world".into(),
            "--comparisons".into(),
            "1".into(),
        ])
        .is_err()
    );
    assert!(
        crate::headless::arguments(&[
            "world".into(),
            "--headless".into(),
            "--comparisons".into(),
            "2".into(),
        ])
        .is_err()
    );
    let settings = SimSettings {
        sensor_radius: f32::NAN,
        ..Default::default()
    };
    assert!(settings.validate().is_err());
    assert!(crate::founders::validate_genomes(&[vec![0.0; 128]]).is_err());
}
#[test]
fn recurrent_cpu_gpu_parity_and_observer_isolation() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.update_params(&q);
    let mut a = body([602.0, 902.0]);
    a.hidden = [0.1; HIDDEN];
    let mut g = crate::brain::random_genome(&mut 7351);
    g[GATE_BIAS..GATE_BIAS + HIDDEN].fill(0.5);
    g[GATE_BIAS + HIDDEN - 1] = 0.8;
    crate::brain::add_edge(&mut g, INPUTS - 1, HIDDEN - 1, 0.7);
    crate::brain::add_edge(&mut g, INPUTS + HIDDEN - 1, HIDDEN - 1, 0.4);
    crate::brain::add_edge(&mut g, INPUTS + HIDDEN - 1, 2 * HIDDEN - 1, 0.2);
    crate::brain::add_edge(&mut g, INPUTS + HIDDEN - 1, 2 * HIDDEN, 0.6);
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let expected = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    let (hidden, outputs) = crate::brain::evaluate(&g, &expected.inputs, &a.hidden);
    for (x, y) in hidden.iter().zip(expected.hidden) {
        near(*x, y);
    }
    for (x, y) in outputs.iter().zip(expected.scores) {
        near(*x, y);
    }
    a.lineage_id = 123456;
    a.ancestry_depth = 100;
    a.lifetime_births = 1000;
    a.distance_travelled = 30000.0;
    put(&s, &q, 0, a, &g);
    let mut e = d.create_command_encoder(&Default::default());
    s.dispatch(&mut e, "decide", 0, 1, 1);
    q.submit(Some(e.finish()));
    let actual = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 1)[0];
    assert_eq!(bytemuck::bytes_of(&actual), bytemuck::bytes_of(&expected));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_ne!(after.hidden, a.hidden);
    assert_eq!(&s.read_genomes(&d, &q, 1).unwrap(), &g);
}
#[test]
fn perception_is_local_and_compass_aligned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.body_padding = std::f32::consts::FRAC_PI_2;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let mut food = vec![0u32; 512 * 512];
    food[225 * 512 + 151] = 700;
    q.write_buffer(&s.resource_buffer, 0, bytemuck::cast_slice(&food));
    let mut far = body([1000.0, 1000.0]);
    far.lineage_id = 2;
    put(&s, &q, 1, far, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let p = read::<PerceptionGpu>(&d, &q, &s.perception_buffer, 1)[0];
    assert!(p.regions[0].food > 0.0);
    assert!(p.regions.iter().skip(1).all(|p| p.food == 0.0));
    assert!(p.bodies.iter().all(|b| b.slot == MAX_AGENTS));
    let renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8UnormSrgb, &s, 800, 600);
    drop(renderer);
}
#[test]
fn physical_collection_ingestion_and_movement_conserve_reserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.food = 0.0;
    a.energy = 50.0;
    put(&s, &q, 0, a, &fixed(1, [0.5, 0.0]));
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&1000u32));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    let food = read::<u32>(&d, &q, &s.resource_buffer, 512 * 512);
    near(
        food[idx as usize] as f32 / 1000.0 + after.food + after.ingested,
        1.0,
    );
    near(after.food + after.ingested, after.collected);
    near(after.energy + after.spent, 50.0 + 8.0 * after.ingested);
    assert!(after.velocity[0] > 0.0);
    assert!(after.collected > 0.0);
    let mut a = after;
    a.energy = 50.0;
    a.food = 1.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    near(after.energy + after.food * 8.0 + after.spent, 58.0);
    assert!(after.ingested > 0.0);
}

#[test]
fn agents_wrap_across_world_edges_without_a_teleport_movement_cost() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([WORLD_SIZE - 0.1, 902.0]);
    a.energy = 50.0;
    put(&s, &q, 0, a, &fixed(1, [0.5, 0.0]));

    step(&mut s, &d, &q, 1);

    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert!(
        after.position[0] < 1.2,
        "agent should reappear at the left edge"
    );
    assert!(after.velocity[0] > 0.0);
    assert!(
        after.spent < 0.1,
        "wrapping must not charge a world-width move"
    );
}
#[test]
fn digestion_is_inventory_limited_rate_limited_and_energy_capped() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // Independent of action amount; no food or energy is granted when empty.
    for (energy, inventory, expected) in [
        (50.0, 2.0, 0.1),
        (50.0, 0.03, 0.03),
        (50.0, 0.0, 0.0),
        (99.8, 2.0, 0.025),
        (100.0, 2.0, 0.0),
        (0.01, 0.1, 0.1),
    ] {
        let mut a = body([602.0, 902.0]);
        a.energy = energy;
        a.food = inventory;
        let mut g = fixed(0, [0.0; 2]);
        g[OUTPUT_BIAS + 8] = -4.0;
        put(&s, &q, 0, a, &g);
        step(&mut s, &d, &q, 1);
        let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
        near(b.ingested, expected);
        near(b.food + b.ingested, inventory);
        near(b.energy + b.spent, energy + 8.0 * expected);
        assert!(b.energy <= 100.0 && b.food >= 0.0);
        assert_eq!(b.action, 0);
        assert_eq!(b.alive, 1);
    }
}

#[test]
fn automatic_digestion_does_not_gather_unrequested_ground_food() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 10.0;
    a.food = 0.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    let idx = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, idx * 4, bytemuck::bytes_of(&1000u32));
    step(&mut s, &d, &q, 1);
    let b = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    near(b.collected, 0.0);
    near(b.ingested, 0.0);
    near(b.food, 0.0);
    near(b.energy, 9.94);
    assert_eq!(
        read::<u32>(&d, &q, &s.resource_buffer, 512 * 512)[idx as usize],
        1000
    );
}

#[test]
fn vegetation_does_not_survive_a_barren_habitat_cell() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.evolving_landscape = true;
    s.update_params(&q);
    let cell = 225 * 512 + 150;
    q.write_buffer(&s.resource_buffer, cell * 4, bytemuck::bytes_of(&1000u32));
    q.write_buffer(
        &s.ground_buffer,
        cell * 32,
        bytemuck::bytes_of(&[0, 0, 0, 0, 0, 0, 1.0f32.to_bits(), 1.0f32.to_bits()]),
    );
    q.write_buffer(&s.terrain_buffer, 0, &vec![0; 512 * 512 * 16]);

    let mut encoder = d.create_command_encoder(&Default::default());
    s.dispatch(&mut encoder, "resource", 0, 64, 64);
    q.submit(Some(encoder.finish()));

    assert_eq!(
        read::<u32>(&d, &q, &s.resource_buffer, 512 * 512)[cell as usize],
        0
    );
}

#[test]
fn reproduction_is_requested_can_coexist_with_motion_and_conserves() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    a.hidden = [0.5; HIDDEN];
    let g = fixed(5, [0.5, 0.0]);
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let p = agents[0];
    let c = agents[1];
    assert_eq!(c.alive, 1);
    assert_eq!(c.ancestry_depth, 1);
    assert_eq!(c.parent_lineage, p.lineage_id);
    assert_eq!(c.hidden, [0.0; HIDDEN]);
    assert!(p.velocity[0] > 0.0);
    near(p.food + c.food + p.ingested, a.food);
    near(
        p.energy
            + c.energy
            + s.settings.metabolic_cost
            + p.velocity[0].abs() * s.settings.movement_energy_cost
            + 10.0,
        90.0 + 8.0 * p.ingested,
    );
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 1);
    let genes = s.read_genomes(&d, &q, 2).unwrap();
    assert_eq!(&genes[..GENOME_SIZE], &g);
    assert!(genes.iter().all(|x| x.is_finite() && x.abs() <= 4.0));
}
#[test]
fn abundant_reserves_do_not_trigger_automatic_birth() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 100.0;
    a.food = 8.0;
    put(&s, &q, 0, a, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 32);
    assert_eq!(s.metrics(&d, &q).unwrap().events[3], 0);
}
#[test]
fn transfer_and_signal_are_local_and_payload_is_controller_owned() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut b = body([604.0, 902.0]);
    b.food = 0.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(2, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(
        bodies[0].food + bodies[1].food + bodies[0].ingested + bodies[1].ingested,
        2.0,
    );
    assert!(bodies[1].received > 0.0);
    let mut g = fixed(4, [0.0; 2]);
    g[OUTPUT_BIAS + 9] = -0.7;
    let mut a = bodies[0];
    a.signal_tick = 0;
    s.tick = 10;
    put(&s, &q, 0, a, &g);
    put(&s, &q, 1, bodies[1], &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let bodies = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    near(bodies[0].signal_payload, (-0.7f32).tanh());
    assert_eq!(bodies[0].signal_tick, 11);
    assert_eq!(bodies[1].signal_tick, 0);
    step(&mut s, &d, &q, 1);
    let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 2);
    let observed = decisions[1].inputs[NEIGHBOR_BASE..]
        .chunks_exact(NEIGHBOR_INPUTS)
        .find(|v| v[5] == 1.0)
        .unwrap();
    assert_eq!(observed[6], 1.0);
    near(observed[4], (-0.7f32).tanh());
}
#[test]
fn force_is_paid_displacement_without_recipient_damage_or_food_loss() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.energy = 90.0;
    let mut b = body([604.0, 902.0]);
    b.energy = 10.0;
    b.lineage_id = 2;
    put(&s, &q, 0, a, &fixed(3, [0.0; 2]));
    put(&s, &q, 1, b, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let agents = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    let m = s.metrics(&d, &q).unwrap();
    near(agents[0].food + agents[0].ingested, a.food);
    near(
        (m.carried_food + m.dropped_food) as f32 + agents[0].ingested + agents[1].ingested,
        4.0,
    );
    assert_eq!(m.events[5], 1);
    near(m.dropped_food as f32, 0.0);
    near(agents[1].food + agents[1].ingested, b.food);
    near(
        agents[1].energy + s.settings.metabolic_cost,
        b.energy + 8.0 * agents[1].ingested,
    );
    assert!(agents[1].position[0] > b.position[0]);
    near(
        (m.energy + m.force_energy_spent) as f32,
        100.0 + 8.0 * (agents[0].ingested + agents[1].ingested) - 2.0 * s.settings.metabolic_cost,
    );
}

#[test]
fn force_direction_effort_and_available_energy_bound_actual_displacement() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for (effort, energy) in [(0.0f32, 10.0), (0.1, 10.0), (-1.0, 10.0), (1.0, 0.1)] {
        let mut actor = body([602.0, 902.0]);
        actor.energy = energy;
        actor.food = 0.0;
        let mut target = body([604.0, 902.0]);
        target.food = 0.0;
        target.lineage_id = 2;
        let mut genes = fixed(3, [0.0; 2]);
        genes[OUTPUT_BIAS + FORCE_OUTPUT] = effort;
        put(&s, &q, 0, actor, &genes);
        put(&s, &q, 1, target, &fixed(0, [0.0; 2]));
        step(&mut s, &d, &q, 1);
        let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
        let displacement = after[1].position[0] - target.position[0];
        let budget = (energy - s.settings.metabolic_cost).max(0.0);
        near(
            displacement,
            effort.signum() * (3.0 * effort.tanh().abs()).min(budget / 0.2),
        );
        near(after[0].energy + after[0].spent, energy);
        near(after[1].energy + after[1].spent, target.energy);
        near(
            after[0].spent,
            s.settings.metabolic_cost + displacement.abs() * 0.2,
        );
        assert!(after.iter().all(|a| a.energy >= 0.0 && a.food == 0.0));
    }
}

#[test]
fn zero_signal_is_present_local_and_does_not_claim_a_physical_pair() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let sender = body([602.0, 902.0]);
    let mut receiver = body([604.0, 902.0]);
    receiver.lineage_id = 2;
    let mut distant = body([1000.0, 1000.0]);
    distant.lineage_id = 3;
    put(&s, &q, 0, sender, &fixed(4, [0.0; 2]));
    put(&s, &q, 1, receiver, &fixed(3, [0.0; 2]));
    put(&s, &q, 2, distant, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 3);
    assert_eq!(after[0].signal_tick, 1);
    assert_eq!(after[0].signal_payload, 0.0);
    assert!(
        after[0].position[0] > sender.position[0],
        "Emission must not shield a body from contact"
    );
    step(&mut s, &d, &q, 1);
    let decisions = read::<DecisionGpu>(&d, &q, &s.decision_buffer, 3);
    let observed = decisions[1].inputs[NEIGHBOR_BASE..]
        .chunks_exact(NEIGHBOR_INPUTS)
        .find(|v| v[5] == 1.0)
        .unwrap();
    assert_eq!(observed[6], 1.0, "Zero-valued signal presence");
    assert_eq!(observed[4], 0.0);
    assert!(
        decisions[2].inputs[NEIGHBOR_BASE..]
            .iter()
            .all(|v| *v == 0.0),
        "No remote signal leakage"
    );
}

#[test]
fn reproduction_requires_paid_energy_not_an_arbitrary_food_stockpile() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 2);
    assert_eq!(after[1].alive, 1);
    assert_eq!(after[0].food + after[1].food, 0.0);
    near(
        after[0].energy + after[1].energy + s.settings.metabolic_cost + 10.0,
        parent.energy,
    );
}

#[test]
fn contrast_preserves_mean_and_invalid_environment_settings_are_rejected() {
    let full = build_habitat_at(42, 3, 1.0);
    let uniform = build_habitat_at(42, 3, 0.0);
    let mean = |values: &[f32]| values.iter().map(|v| *v as f64).sum::<f64>() / values.len() as f64;
    assert!(uniform.iter().all(|v| *v == uniform[0]));
    assert!((mean(&full) - mean(&uniform)).abs() < 0.00001);
    for contrast in [-0.1, 1.1, f32::NAN] {
        let settings = SimSettings {
            habitat_contrast: contrast,
            ..Default::default()
        };
        assert!(settings.validate().is_err());
    }
    assert_eq!(MODEL_ID, "primitive-v29-composable-reservoir");
    assert_eq!(crate::founders::bundled().model, MODEL_ID);
    assert_eq!(crate::founders::bundled().version, FOUNDER_BANK_VERSION);
}

#[test]
fn ecological_dynamics_have_no_age_or_progress_curriculum() {
    for age in [0, 50_000, 150_000, 375_000, 625_000, 750_000, u32::MAX] {
        assert_eq!(ecological_pressures(age), [1.0, 1.0, 1.0, 0.0]);
    }
}

#[test]
fn fragmentation_preserves_habitat_mean() {
    let before = build_habitat_at(42, 30, 1.0);
    let mut after = before.clone();
    let mean = |values: &[f32]| values.iter().map(|v| *v as f64).sum::<f64>() / values.len() as f64;
    apply_fragmentation(&mut after, 1.0, 30, 42);
    assert!((mean(&before) - mean(&after)).abs() < 0.00001);
}
#[test]
fn nonfinite_controller_output_is_contained() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let mut g = fixed(1, [1.0, 1.0]);
    g[NODE_BIAS] = f32::NAN;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let after = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(after.action, 0);
    assert_eq!(after.position, a.position);
    assert!(after.hidden.iter().all(|x| x.is_finite()));
    assert_eq!(s.metrics(&d, &q).unwrap().invalid_outputs, 1);
}
#[test]
fn live_selection_follows_identity_without_changing_simulation_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let a = body([602.0, 902.0]);
    let g = fixed(0, [0.1, 0.2]);
    put(&s, &q, 0, a, &g);
    let original = s.select_agent(&d, &q, a.position, 2.0).unwrap();
    step(&mut s, &d, &q, 16);
    let buffers = [
        &s.agent_buffers[0],
        &s.agent_buffers[1],
        &s.genome_buffers[0],
        &s.genome_buffers[1],
        &s.resource_buffer,
        &s.ground_buffer,
    ];
    let before: Vec<_> = buffers
        .iter()
        .map(|buffer| observability::read_buffer(&d, &q, buffer).unwrap())
        .collect();
    let metrics_before = serde_json::to_value(s.metrics(&d, &q).unwrap()).unwrap();
    let current = s
        .refresh_selected_agent(&d, &q, &original)
        .unwrap()
        .unwrap();
    assert_eq!(current.selected, original.selected);
    assert_eq!(current.agent.lineage_id, a.lineage_id);
    assert!(current.agent.position[0] > a.position[0] + 2.0);
    assert_eq!(current.agent.age, a.age + 16.0);
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(
        bytemuck::bytes_of(&current.agent),
        bytemuck::bytes_of(&actual)
    );
    for (buffer, expected) in buffers.iter().zip(before) {
        assert_eq!(
            observability::read_buffer(&d, &q, buffer).unwrap(),
            expected
        );
    }
    assert_eq!(
        serde_json::to_value(s.metrics(&d, &q).unwrap()).unwrap(),
        metrics_before
    );
    assert_eq!(s.tick, 16);

    // A dead body can yield a terminal snapshot, but no replacement is followed.
    let mut dead = current.agent;
    dead.alive = 0;
    put(&s, &q, 0, dead, &g);
    assert_eq!(
        s.refresh_selected_agent(&d, &q, &original)
            .unwrap()
            .unwrap()
            .agent
            .alive,
        0
    );
    for component in 0..3 {
        let mut replacement = current.agent;
        match component {
            0 => replacement.generation += 1,
            1 => replacement.lineage_id += 1,
            _ => replacement.birth_tick += 1,
        }
        put(&s, &q, 0, replacement, &g);
        assert!(
            s.refresh_selected_agent(&d, &q, &original)
                .unwrap()
                .is_none()
        );
    }
    let invalid = SelectionOutput::default();
    assert!(
        s.refresh_selected_agent(&d, &q, &invalid)
            .unwrap()
            .is_none()
    );
    let invalid = SelectionOutput {
        selected: MAX_AGENTS + 1,
        ..original
    };
    assert!(
        s.refresh_selected_agent(&d, &q, &invalid)
            .unwrap()
            .is_none()
    );
}

#[test]
fn dead_selection_after_a_batch_does_not_claim_a_fresh_decision_trace() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut a = body([602.0, 902.0]);
    a.max_age = a.age + 1.0;
    put(&s, &q, 0, a, &fixed(0, [0.1, 0.2]));
    let original = s.select_agent(&d, &q, a.position, 2.0).unwrap();
    let mut view = crate::inspection::Inspection::default();
    view.select(Some(original), 0);
    // Death occurs on the first tick; remaining ticks clear the trace buffers.
    step(&mut s, &d, &q, 4);
    let result = s.refresh_selected_agent(&d, &q, &original);
    view.refresh(result, s.tick);
    let terminal = view.snapshot.unwrap();
    assert_eq!(terminal.agent.alive, 0);
    assert_eq!(terminal.agent.age, a.max_age);
    assert_eq!(terminal.decision.scores, [0.0; 6]);
    assert_eq!(terminal.decision.inputs, [0.0; INPUTS]);
    assert!(!view.following);
    assert!(!view.has_decision_trace());
    assert_eq!(view.highlight().0, u32::MAX);
    assert!(view.notice.contains("terminal snapshot"));
}

#[test]
fn inspector_render_pipelines_accept_incarnation_aware_camera() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let mut renderer =
        crate::renderer::Renderer::new(&d, wgpu::TextureFormat::Rgba8Unorm, &s, 1280, 820);
    assert_eq!(std::mem::size_of::<crate::renderer::CameraUniform>(), 40);
    renderer.camera.selected_id = 4;
    renderer.camera.selected_generation = 7;
    renderer.update_camera(&q);
    d.poll(wgpu::Maintain::Wait);
}

#[test]
fn batching_checkpoint_and_selection_preserve_state() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    // Loading restores saved physical settings.
    s.settings.metabolic_cost = 0.06;
    // These fixtures isolate physical actions; cognitive costs have dedicated tests.
    s.settings.active_unit_upkeep = 0.0;
    s.settings.memory_write_energy = 0.0;
    s.settings.movement_energy_cost = 0.01;
    let mut a = body([602.0, 902.0]);
    a.plasticity_rate = [0.02; HIDDEN];
    a.trace_retention = 0.9;
    a.learned_weight_retention = 0.99;
    let mut g = fixed(0, [0.1, 0.2]);
    g[NODE_BIAS] = 0.5;
    g[GATE_BIAS] = 0.75;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 12);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.reset(&q);
    // Recreate the same empty-resource fixture, not a newly seeded habitat.
    q.write_buffer(&s.resource_buffer, 0, &vec![0; 512 * 512 * 4]);
    q.write_buffer(&s.ground_buffer, 0, &vec![0; 512 * 512 * 32]);
    put(&s, &q, 0, a, &g);
    for _ in 0..12 {
        step(&mut s, &d, &q, 1);
    }
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    assert!(
        read::<f32>(&d, &q, &s.fast_weight_buffers[0], FAST_BANK_STRIDE)
            .iter()
            .any(|v| *v != 0.0)
    );
    let selection = s.select_agent(&d, &q, actual.position, 2.0).unwrap();
    assert_eq!(selection.selected, 1);
    let path = temp("state.checkpoint");
    s.save_checkpoint(&d, &q, &path).unwrap();
    step(&mut s, &d, &q, 8);
    let expected = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    s.settings.metabolic_cost = 0.005;
    s.settings.movement_energy_cost = 0.002;
    let saved_motor_gain = s.settings.motor_response_gain;
    s.settings.motor_response_gain = 16.0;
    s.load_checkpoint(&q, &path).unwrap();
    assert_eq!(s.settings.metabolic_cost, 0.06);
    assert_eq!(s.settings.movement_energy_cost, 0.01);
    assert_eq!(s.settings.motor_response_gain, saved_motor_gain);
    step(&mut s, &d, &q, 8);
    let actual = read::<AgentGpu>(&d, &q, &s.agent_buffers[s.current_buffer], 1)[0];
    assert_eq!(bytemuck::bytes_of(&expected), bytemuck::bytes_of(&actual));
    std::fs::remove_file(path).unwrap();
    let path = temp("unsupported.checkpoint");
    std::fs::write(&path, b"PRIMWORLD000").unwrap();
    let tick = s.tick;
    assert!(s.load_checkpoint(&q, &path).is_err());
    assert_eq!(s.tick, tick);
    assert!(path.exists());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn headless_extinction_stops_without_waiting_for_the_report_or_tick_limit() {
    for (label, population, metabolism, limit, expected_tick, reason) in [
        ("empty", "0", "0.06", "200000", 0, "extinction"),
        ("dies", "1", "100", "200000", 192, "extinction"),
        ("alive", "1", "0.06", "7", 7, "tick_limit"),
    ] {
        let report = temp(&format!("early-stop-{label}.json"));
        let journeys = temp(&format!("early-stop-{label}.jsonl"));
        let args: Vec<String> = [
            "world",
            "--headless",
            "--single-world",
            "--population",
            population,
            "--metabolic-cost",
            metabolism,
            "--ticks",
            limit,
            "--sample",
            "100000",
            "--journeys",
            journeys.to_str().unwrap(),
            "--journey-sample",
            "1000",
            "--output",
            report.to_str().unwrap(),
        ]
        .into_iter()
        .map(String::from)
        .collect();
        crate::headless::run(&args).unwrap();
        let output: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
        assert_eq!(output["elapsed_ticks"], expected_tick);
        assert_eq!(output["termination_reason"], reason);
        let history = output["history"].as_array().unwrap();
        assert_eq!(history.last().unwrap()["tick"], expected_tick);
        if reason == "extinction" {
            assert_eq!(history.last().unwrap()["living"], 0);
        }
        assert_eq!(history.len(), if expected_tick == 0 { 1 } else { 2 });
        let lines = std::fs::read_to_string(&journeys).unwrap();
        let footer: serde_json::Value =
            serde_json::from_str(lines.lines().last().unwrap()).unwrap();
        assert_eq!(footer["type"], "summary");
        assert_eq!(footer["observer"], output["journey_observer"]);
        std::fs::remove_file(report).unwrap();
        std::fs::remove_file(journeys).unwrap();
    }
}

#[test]
fn family_observer_counts_every_tick_and_preserves_dead_family_outcomes() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    let mut dying = body([1000.0, 1000.0]);
    dying.food = 0.0;
    dying.energy = 0.1;
    dying.founder_family = 1;
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, dying, &fixed(0, [0.0; 2]));
    s.family_observer = Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 8).unwrap());
    let mut expected = [[0u32; 7]; 2];
    for tick in 1..=8 {
        step(&mut s, &d, &q, 1);
        for a in s
            .agent_snapshot(&d, &q)
            .unwrap()
            .iter()
            .filter(|a| a.alive != 0)
        {
            let row = &mut expected[a.founder_family as usize];
            row[5] = row[5].max(a.ancestry_depth);
            row[6] = tick;
            if a.ancestry_depth == 0 {
                row[0] += 1;
            } else {
                row[1] += 1;
                row[2] += u32::from(tick > 4);
                row[3] += u32::from(a.age >= s.settings.maturity_age);
                row[4] += u32::from(a.birth_tick + 1 == tick);
            }
        }
    }
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    for (actual, expected) in report.families.iter().zip(expected) {
        assert_eq!(
            [
                actual.founder_body_ticks,
                actual.descendant_body_ticks,
                actual.late_descendant_body_ticks,
                actual.mature_descendant_body_ticks,
                actual.births,
                actual.maximum_depth,
                actual.last_alive_tick
            ],
            expected
        );
    }
    assert_eq!(report.families[1].last_alive_tick, 1);
    assert_eq!(report.families[0].births, 1);
    let observed = s.agent_snapshot(&d, &q).unwrap();
    s.reset(&q);
    assert!(s.family_observer.is_none());
    put(&s, &q, 0, parent, &fixed(5, [0.0; 2]));
    put(&s, &q, 1, dying, &fixed(0, [0.0; 2]));
    step(&mut s, &d, &q, 8);
    let unobserved = s.agent_snapshot(&d, &q).unwrap();
    assert_eq!(
        bytemuck::cast_slice::<AgentGpu, u8>(&observed),
        bytemuck::cast_slice::<AgentGpu, u8>(&unobserved)
    );
}

#[test]
fn family_diagnostics_record_underfunded_births_and_terminal_juvenile_deaths_once() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut parent = body([602.0, 902.0]);
    parent.food = 0.0;
    let mut genes = fixed(5, [0.0; 2]);
    genes[OUTPUT_BIAS + 8] = 0.0; // 20 energy, below stationary 24.
    put(&s, &q, 0, parent, &genes);
    s.family_observer =
        Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 2048).unwrap());
    for _ in 0..64 {
        step(&mut s, &d, &q, 32);
    }
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    let f = &report.families[0];
    assert!(f.births > 0);
    assert_eq!(f.births_below_stationary_maturity_energy, f.births);
    assert_eq!(f.birth_energy_milli, u64::from(f.births) * 20000);
    assert_eq!(f.juvenile_starvation_deaths, f.births);
    assert_eq!(f.matured_descendants, 0);
    assert_eq!(f.births_to_descendant_parents, 0);
    assert_eq!(f.collected_milli, 0);
    assert_eq!(f.juvenile_ingested_milli, 0);
    assert_eq!(f.juvenile_food_present_ticks, 0);
    assert!(f.juvenile_processed_ticks > 0);
    assert_eq!(s.metrics(&d, &q).unwrap().living, 0);
}

#[test]
fn family_diagnostics_count_juvenile_feeding_maturity_and_terminal_flow() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut juvenile = body([602.0, 902.0]);
    juvenile.age = 399.0;
    juvenile.ancestry_depth = 1;
    juvenile.energy = 0.01;
    juvenile.food = 0.0;
    // This injected fixture was born before the measured window, not a new birth.
    juvenile.birth_tick = u32::MAX;
    put(&s, &q, 0, juvenile, &fixed(1, [0.0; 2]));
    let cell = (902 / 4 * 512 + 602 / 4) as u64;
    q.write_buffer(&s.resource_buffer, cell * 4, bytemuck::bytes_of(&1000u32));
    s.family_observer = Some(crate::family_observer::FamilyObserver::new(&d, &q, &s, 3).unwrap());
    step(&mut s, &d, &q, 3);
    let report = s.family_observer.as_ref().unwrap().report(&d, &q).unwrap();
    let f = &report.families[0];
    assert_eq!(f.matured_descendants, 1);
    assert_eq!(f.juvenile_processed_ticks, 1);
    assert_eq!(f.juvenile_collect_action_ticks, 1);
    assert_eq!(f.juvenile_food_present_ticks, 1);
    assert_eq!(f.juvenile_food_present_collect_ticks, 1);
    assert!(f.juvenile_collected_milli > 0);
    assert!(f.juvenile_ingested_milli > 0);
    assert!(f.energy_at_maturity_milli > 0);
    let metrics = s.metrics(&d, &q).unwrap();
    assert_eq!(f.ingested_milli, u64::from(metrics.events[0]));
    assert_eq!(
        f.collected_milli,
        (metrics.harvested * 1000.0).round() as u64
    );
}

#[test]
fn birth_variation_preserves_parent_cost_and_resets_child_memory() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.metabolic_cost = 0.1;
    let g = fixed(5, [0.0; 2]);
    let mut a = body([602.0, 902.0]);
    a.food = 0.0;
    a.hidden = [0.8; HIDDEN];
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let (slot, child) = agents
        .iter()
        .enumerate()
        .find(|(_, b)| b.alive != 0 && b.ancestry_depth == 1)
        .unwrap();
    assert!(
        agents[0].energy < a.energy - child.energy,
        "parent pays body, active-capacity, and local-memory write upkeep as well as birth cost"
    );
    assert_eq!(child.hidden, [0.0; HIDDEN]);
    let genes = s.read_genomes(&d, &q, slot + 1).unwrap();
    assert_eq!(
        &genes[..GENOME_SIZE],
        &g,
        "birth must not alter the parent genome"
    );
    crate::brain::validate(&genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]).unwrap();
    // Rejected birth spends only ordinary upkeep and never publishes a child genome.
    let mut s = scene(&d, &q);
    a.energy = 20.0;
    put(&s, &q, 0, a, &g);
    step(&mut s, &d, &q, 1);
    assert_eq!(s.metrics(&d, &q).unwrap().living, 1);
    assert!(s.agent_snapshot(&d, &q).unwrap()[0].energy < a.energy);
    assert!(
        s.read_genomes(&d, &q, 2).unwrap()[GENOME_SIZE..]
            .iter()
            .all(|v| *v == 0.0)
    );
}
