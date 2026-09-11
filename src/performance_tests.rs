use super::*;

#[test]
fn linked_grid_contains_every_live_slot_once_in_wrapped_and_crowded_cells() {
    let (d, q) = gpu();
    let s = scene(&d, &q);
    let mut agents = vec![AgentGpu::zeroed(); MAX_AGENTS as usize];
    let world = [s.settings.habitat_width, s.settings.habitat_height];
    let mut expected = vec![Vec::new(); SPATIAL_CELL_COUNT as usize];
    for (slot, a) in agents.iter_mut().enumerate() {
        if slot % 3 == 0 {
            continue;
        }
        let position = if slot < 2048 {
            [1.0, 1.0]
        } else {
            [
                ((slot * 1973) % 8192) as f32 - 4096.0,
                ((slot * 9277) % 8192) as f32 - 4096.0,
            ]
        };
        *a = body(position);
        a.alive = if slot % 2 == 0 { 2 } else { 1 };
        let x = (position[0].rem_euclid(world[0]) / world[0] * 256.0).floor() as usize;
        let y = (position[1].rem_euclid(world[1]) / world[1] * 256.0).floor() as usize;
        expected[y.min(255) * 256 + x.min(255)].push(slot as u32);
    }
    q.write_buffer(
        &s.agent_buffers[s.current_buffer],
        0,
        bytemuck::cast_slice(&agents),
    );
    for populated in [true, false] {
        if !populated {
            q.write_buffer(
                &s.agent_buffers[s.current_buffer],
                0,
                bytemuck::cast_slice(&vec![AgentGpu::zeroed(); MAX_AGENTS as usize]),
            );
        }
        let mut e = d.create_command_encoder(&Default::default());
        s.dispatch(&mut e, "linked_clear", s.current_buffer, 1024, 1);
        s.dispatch(
            &mut e,
            "linked_link",
            s.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        q.submit(Some(e.finish()));
        let heads = read::<u32>(&d, &q, &s.audit_cell_offsets, SPATIAL_CELL_COUNT as usize);
        let links = read::<u32>(&d, &q, &s.audit_indices, MAX_AGENTS as usize);
        let counts = read::<u32>(&d, &q, &s.occupancy_buffer, SPATIAL_CELL_COUNT as usize);
        let mut seen = vec![false; MAX_AGENTS as usize];
        for (cell, head) in heads.into_iter().enumerate() {
            let mut at = head;
            let mut actual = Vec::new();
            while at != MAX_AGENTS {
                assert!(
                    at < MAX_AGENTS && !seen[at as usize],
                    "invalid link or cycle in cell {cell}"
                );
                seen[at as usize] = true;
                actual.push(at);
                at = links[at as usize];
            }
            actual.sort_unstable();
            assert_eq!(actual.len(), counts[cell] as usize);
            assert_eq!(
                actual.as_slice(),
                if populated {
                    expected[cell].as_slice()
                } else {
                    &[]
                }
            );
        }
    }
}

#[test]
fn linked_sensing_matches_contiguous_neighborhoods_at_wrapped_edges() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    let mut agents = vec![AgentGpu::zeroed(); MAX_AGENTS as usize];
    for (slot, a) in agents.iter_mut().take(130).enumerate() {
        *a = body([
            if slot % 2 == 0 {
                1.0
            } else {
                s.settings.habitat_width - 1.0
            },
            1.0 + (slot % 7) as f32 * 0.25,
        ]);
        a.velocity = [slot as f32 * 0.003, -(slot as f32) * 0.002];
        a.heading = slot as f32 * 0.17;
        a.signal_payload = (slot as f32 - 65.0) * 0.01;
        a.signal_tick = 1;
        if slot % 5 == 0 {
            a.alive = 2;
        }
    }
    s.tick = 1;
    s.update_params(&q);
    q.write_buffer(
        &s.agent_buffers[s.current_buffer],
        0,
        bytemuck::cast_slice(&agents),
    );
    let mut expected = Vec::new();
    for linked in [false, true] {
        install_linked_spatial(&mut s, &d, linked);
        let mut e = d.create_command_encoder(&Default::default());
        if linked {
            s.dispatch(&mut e, "linked_clear", s.current_buffer, 1024, 1);
            s.dispatch(
                &mut e,
                "linked_link",
                s.current_buffer,
                MAX_AGENTS.div_ceil(64),
                1,
            );
        } else {
            s.dispatch(&mut e, "clear", 0, 32, 32);
            s.dispatch(
                &mut e,
                "count",
                s.current_buffer,
                MAX_AGENTS.div_ceil(64),
                1,
            );
            s.scan(&mut e, "spatial", SPATIAL_CELL_COUNT);
            s.dispatch(&mut e, "cursors", 0, 1024, 1);
            s.dispatch(
                &mut e,
                "scatter",
                s.current_buffer,
                MAX_AGENTS.div_ceil(64),
                1,
            );
        }
        s.dispatch(
            &mut e,
            "perceive",
            s.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        q.submit(Some(e.finish()));
        let actual = read::<PerceptionGpu>(&d, &q, &s.perception_buffer, 130);
        if !linked {
            expected = actual;
            continue;
        }
        for (a, b) in actual.iter().zip(&expected) {
            assert_eq!(a.nearby_count, b.nearby_count);
            assert_eq!(a.resource_here, b.resource_here);
            for (a, b) in a.regions.iter().zip(&b.regions) {
                assert_eq!(a.food, b.food);
                assert_eq!(a.bodies, b.bodies);
            }
        }
        // Both grids insert neighbors in GPU scheduling order. Reduction
        // rounding may differ, but no neighbor or sensory channel may be lost.
        for (&a, &b) in bytemuck::cast_slice::<_, f32>(&actual)
            .iter()
            .zip(bytemuck::cast_slice::<_, f32>(&expected))
        {
            assert!(
                (a - b).abs() <= 1e-5 * (1.0 + a.abs().max(b.abs())),
                "{a} != {b}"
            );
        }
    }
}

fn install_linked_spatial(s: &mut Simulation, d: &wgpu::Device, linked: bool) {
    s.linked_spatial = linked;
    for (name, source, entry) in [
        (
            "perceive",
            include_str!("../shaders/perceive.wgsl").to_owned(),
            "main",
        ),
        (
            "perceive_live",
            live_source(include_str!("../shaders/perceive.wgsl"), 8),
            "main",
        ),
        (
            "consume",
            include_str!("../shaders/consume.wgsl").to_owned(),
            "main",
        ),
        (
            "interact_propose",
            include_str!("../shaders/interactions.wgsl").to_owned(),
            "propose",
        ),
    ] {
        let source = source.replace(
            "// SPATIAL_ITERATION",
            &include_str!("../shaders/spatial_iteration.wgsl").replace(
                "LINKED_SPATIAL:bool=true",
                &format!("LINKED_SPATIAL:bool={linked}"),
            ),
        );
        let compute = s.passes.get_mut(name).unwrap();
        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&compute.pipeline.get_bind_group_layout(0)],
            push_constant_ranges: &[],
        });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
        });
        compute.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        });
    }
}

fn install_perception_inner_loops(s: &mut Simulation, d: &wgpu::Device, hoisted_rotation: bool) {
    let source = live_source(include_str!("../shaders/perceive.wgsl"), 8).replace(
        "HOISTED_ROTATION:bool=true",
        &format!("HOISTED_ROTATION:bool={hoisted_rotation}"),
    );
    s.passes.insert(
        "perceive_live".into(),
        Compute::new(
            d,
            "perception inner-loop probe",
            &source,
            "main",
            "rrwrrrwur",
            pair(|bank| {
                vec![
                    &s.agent_buffers[bank],
                    &s.resource_buffer,
                    &s.ground_buffer,
                    &s.occupancy_buffer,
                    &s.audit_cell_offsets,
                    &s.audit_indices,
                    &s.perception_buffer,
                    &s.params_buffer,
                    &s.active_indices,
                ]
            }),
        ),
    );
}

fn install_hoisted_weather(s: &mut Simulation, d: &wgpu::Device, hoisted: bool) {
    let source = include_str!("../shaders/resource_update.wgsl").replace(
        "HOISTED_WEATHER:bool=true",
        &format!("HOISTED_WEATHER:bool={hoisted}"),
    );
    let compute = s.passes.get_mut("resource").unwrap();
    let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("weather interpolation probe"),
        bind_group_layouts: &[&compute.pipeline.get_bind_group_layout(0)],
        push_constant_ranges: &[],
    });
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("weather interpolation probe"),
        source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
    });
    compute.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("weather interpolation probe"),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
}

#[test]
fn hoisted_weather_preserves_tick_state() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    s.settings.population = 32;
    let mut expected = Vec::new();
    for (seed, tick) in [(42, 0), (91, 996), (3137, 47_002), (42, 999_998)] {
        for hoisted in [false, true] {
            install_hoisted_weather(&mut s, &d, hoisted);
            s.seed = seed;
            s.reset(&q);
            s.tick = tick;
            s.environment_start_age = 0;
            s.update_params(&q);
            step(&mut s, &d, &q, 2);
            for (index, buffer) in [
                &s.agent_buffers[s.current_buffer],
                &s.resource_buffer,
                &s.fertility_buffer,
                &s.ground_buffer,
                &s.ecology_buffer,
                &s.death_stats_buffer,
            ]
            .into_iter()
            .enumerate()
            {
                let bytes = observability::read_buffer(&d, &q, buffer).unwrap();
                if hoisted {
                    assert!(
                        bytes == expected[index],
                        "weather interpolation changed seed={seed} tick={tick} buffer={index}"
                    );
                } else if expected.len() <= index {
                    expected.push(bytes);
                } else {
                    expected[index] = bytes;
                }
            }
        }
    }
}

fn install_retained_inner_loops(s: &mut Simulation, d: &wgpu::Device, optimized: bool) {
    install_neural_inner_loops(s, d, true, optimized, optimized);
    install_perception_inner_loops(s, d, optimized);
    install_hoisted_weather(s, d, optimized);
}

#[test]
fn retained_inner_loops_preserve_tick_state() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    s.settings.population = 32;
    let mut expected = Vec::new();
    for optimized in [false, true] {
        install_retained_inner_loops(&mut s, &d, optimized);
        s.reset(&q);
        step(&mut s, &d, &q, 32);
        for (index, buffer) in [
            &s.agent_buffers[s.current_buffer],
            &s.perception_buffer,
            &s.decision_buffer,
            &s.fast_weight_buffers[0],
            &s.fast_weight_buffers[1],
            &s.trace_buffer,
            &s.resource_buffer,
            &s.fertility_buffer,
            &s.ground_buffer,
            &s.ecology_buffer,
            &s.death_stats_buffer,
        ]
        .into_iter()
        .enumerate()
        {
            let bytes = observability::read_buffer(&d, &q, buffer).unwrap();
            if optimized {
                assert!(
                    bytes == expected[index],
                    "retained inner loop changed buffer {index}"
                );
            } else {
                expected.push(bytes);
            }
        }
    }
}

#[test]
#[ignore = "manual paired retained inner-loop throughput probe"]
fn profile_retained_inner_loops() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let (mut saves, _) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    saves.sort_by_key(|saved| std::cmp::Reverse(saved.record.saved_at_ms));
    let saved = &saves[0];
    eprintln!(
        "Saved world={} tick={} living={}",
        saved.record.world, saved.record.tick, saved.record.living
    );
    for population in [1000, 4096, 0] {
        for repeat in 0..5 {
            let modes = if repeat % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            };
            for optimized in modes {
                install_retained_inner_loops(&mut s, &d, optimized);
                if population == 0 {
                    s.load_game_checkpoint(
                        &q,
                        std::fs::File::open(saved.checkpoint()).unwrap(),
                        (saved.record.seed, saved.record.tick, saved.record.living),
                        saved.record.world,
                    )
                    .unwrap();
                } else {
                    s.settings.population = population;
                    s.reset(&q);
                }
                step(&mut s, &d, &q, 32);
                let start = std::time::Instant::now();
                for _ in 0..16 {
                    step(&mut s, &d, &q, 32);
                }
                eprintln!(
                    "population={population} repeat={repeat} optimized={optimized}: {:.1} ticks/s",
                    512.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

#[test]
#[ignore = "manual paired linked spatial benchmark"]
fn profile_linked_spatial() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let (mut saves, _) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    saves.sort_by_key(|s| std::cmp::Reverse(s.record.saved_at_ms));
    let saved = &saves[0];
    for population in [32, 1000, 4096, 8192, 0] {
        for repeat in 0..3 {
            for linked in if repeat % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                install_linked_spatial(&mut s, &d, linked);
                if population == 0 {
                    s.load_game_checkpoint(
                        &q,
                        std::fs::File::open(saved.checkpoint()).unwrap(),
                        (saved.record.seed, saved.record.tick, saved.record.living),
                        saved.record.world,
                    )
                    .unwrap();
                } else {
                    s.settings.population = population;
                    s.reset(&q);
                }
                step(&mut s, &d, &q, 32);
                let start = std::time::Instant::now();
                for _ in 0..32 {
                    step(&mut s, &d, &q, 32);
                }
                eprintln!(
                    "spatial population={population} repeat={repeat} linked={linked}: {:.1} ticks/s",
                    1024.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

fn install_tick_boundaries(s: &mut Simulation, d: &wgpu::Device, optimized: bool) {
    s.reference_tick_boundaries = !optimized;
    let compute = s.passes.get_mut("free").unwrap();
    let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("tick boundary reference"),
        bind_group_layouts: &[&compute.pipeline.get_bind_group_layout(0)],
        push_constant_ranges: &[],
    });
    let source = include_str!("../shaders/free_flags.wgsl").replace(
        "CLEAR_CLAIMS:bool=true",
        &format!("CLEAR_CLAIMS:bool={optimized}"),
    );
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("claim clearing"),
        source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
    });
    compute.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("claim clearing"),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
}

#[test]
#[ignore = "manual paired tick boundary benchmark including latest saved world"]
fn profile_tick_boundaries() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let (mut saves, _) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    saves.sort_by_key(|s| std::cmp::Reverse(s.record.saved_at_ms));
    let saved = &saves[0];
    eprintln!(
        "Saved world={} tick={} living={}",
        saved.record.world, saved.record.tick, saved.record.living
    );
    for population in [32, 1000, 4096, 8192, 0] {
        for repeat in 0..3 {
            for optimized in if repeat % 2 == 0 {
                [false, true]
            } else {
                [true, false]
            } {
                install_tick_boundaries(&mut s, &d, optimized);
                if population == 0 {
                    s.load_game_checkpoint(
                        &q,
                        std::fs::File::open(saved.checkpoint()).unwrap(),
                        (saved.record.seed, saved.record.tick, saved.record.living),
                        saved.record.world,
                    )
                    .unwrap();
                } else {
                    s.settings.population = population;
                    s.reset(&q);
                }
                step(&mut s, &d, &q, 32);
                let start = std::time::Instant::now();
                for _ in 0..32 {
                    step(&mut s, &d, &q, 32);
                }
                eprintln!(
                    "boundaries population={population} repeat={repeat} optimized={optimized}: {:.1} ticks/s",
                    1024.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

#[test]
fn batched_metrics_match_synchronous_metrics() {
    use crate::playback::{METRICS_OFFSET, READBACK_SIZE, TELEMETRY_SIZE};
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let output = readback(&d, READBACK_SIZE);
    for ticks in [1, 8, 32] {
        let mut e = d.create_command_encoder(&Default::default());
        s.encode_ticks(&mut e, &d, &q, ticks);
        s.encode_telemetry(&mut e, &output);
        s.encode_metrics(&mut e, &output, METRICS_OFFSET);
        q.submit(Some(e.finish()));
        let (tx, rx) = mpsc::channel();
        output.slice(..).map_async(wgpu::MapMode::Read, move |r| {
            tx.send(r).unwrap();
        });
        d.poll(wgpu::Maintain::Wait);
        rx.recv().unwrap().unwrap();
        let bytes = output.slice(..).get_mapped_range();
        let actual = s
            .decode_metrics(
                &bytes[METRICS_OFFSET as usize..],
                &bytes[4..TELEMETRY_SIZE as usize],
            )
            .unwrap();
        let expected = s.metrics(&d, &q).unwrap();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
        assert_eq!(actual.tick, s.tick);
        assert!(actual.living > 0);
        assert!(actual.vegetation > 0.0);
        drop(bytes);
        output.unmap();
    }
}

/// Models the viewer's one-millisecond completion polling without rendering.
/// Compares the reference neural layout + 8 ms with coalesced inputs + 32 ms.
/// Alternates paired runs to reduce drift from concurrent GPU workloads.
#[test]
#[ignore = "manual adaptive playback batching throughput probe"]
fn profile_playback_optimizations() {
    use std::time::{Duration, Instant};
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let output = readback(&d, crate::playback::TELEMETRY_SIZE);
    for population in [32, 1000, 4096] {
        s.settings.population = population;
        for repeat in 0..3 {
            let targets = if repeat % 2 == 0 { [8, 32] } else { [32, 8] };
            for target_ms in targets {
                install_neural_input_layout(&mut s, &d, target_ms == 32);
                s.reset(&q);
                step(&mut s, &d, &q, 32);
                let mut seconds_per_tick = 0.002;
                let start = Instant::now();
                let mut elapsed_ticks = 0;
                let mut batches = 0;
                while elapsed_ticks < 512 {
                    let ticks = ((f64::from(target_ms) / 1000.0 / seconds_per_tick)
                        .floor()
                        .clamp(1.0, 32.0) as u32)
                        .min(512 - elapsed_ticks);
                    let batch_start = Instant::now();
                    let mut e = d.create_command_encoder(&Default::default());
                    s.encode_ticks(&mut e, &d, &q, ticks);
                    s.encode_telemetry(&mut e, &output);
                    q.submit(Some(e.finish()));
                    let (tx, rx) = mpsc::channel();
                    output.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                        tx.send(r).unwrap();
                    });
                    loop {
                        d.poll(wgpu::Maintain::Poll);
                        match rx.try_recv() {
                            Ok(result) => {
                                result.unwrap();
                                break;
                            }
                            Err(mpsc::TryRecvError::Empty) => {
                                std::thread::sleep(Duration::from_millis(1))
                            }
                            Err(error) => panic!("{error}"),
                        }
                    }
                    output.unmap();
                    seconds_per_tick = seconds_per_tick * 0.8
                        + batch_start.elapsed().as_secs_f64() / f64::from(ticks) * 0.2;
                    elapsed_ticks += ticks;
                    batches += 1;
                }
                eprintln!(
                    "population={population} repeat={repeat} target_ms={target_ms}: {:.1} ticks/s, {batches} batches",
                    512.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

fn install_neural_input_layout(s: &mut Simulation, d: &wgpu::Device, coalesced: bool) {
    install_neural_inner_loops(s, d, coalesced, true, true);
}

fn install_neural_inner_loops(
    s: &mut Simulation,
    d: &wgpu::Device,
    coalesced: bool,
    specialized: bool,
    row_wise: bool,
) {
    let source = |text: &str| {
        text.replace(
            "COALESCED_INPUTS:bool=true",
            &format!("COALESCED_INPUTS:bool={coalesced}"),
        )
        .replace(
            "SPECIALIZED_BANKS:bool=true",
            &format!("SPECIALIZED_BANKS:bool={specialized}"),
        )
        .replace(
            "ROW_WISE_INPUTS:bool=true",
            &format!("ROW_WISE_INPUTS:bool={row_wise}"),
        )
    };
    s.passes.insert(
        "decide_live".into(),
        Compute::new(
            d,
            "decide layout probe",
            &source(include_str!("../shaders/decide_parallel.wgsl")),
            "main",
            "rrwurrrrr",
            pair(|bank| {
                vec![
                    &s.agent_buffers[bank],
                    &s.perception_buffer,
                    &s.decision_buffer,
                    &s.params_buffer,
                    &s.genome_buffers[0],
                    &s.genome_buffers[1],
                    &s.fast_weight_buffers[0],
                    &s.fast_weight_buffers[1],
                    &s.active_indices,
                ]
            }),
        ),
    );
    s.passes.insert(
        "plastic".into(),
        Compute::new(
            d,
            "plasticity layout probe",
            &source(include_str!("../shaders/plasticity.wgsl")),
            "main",
            "rwrwwwuwr",
            pair(|bank| {
                vec![
                    &s.agent_buffers[bank],
                    &s.agent_buffers[1 - bank],
                    &s.decision_buffer,
                    &s.fast_weight_buffers[0],
                    &s.fast_weight_buffers[1],
                    &s.trace_buffer,
                    &s.params_buffer,
                    &s.death_stats_buffer,
                    &s.active_indices,
                ]
            }),
        ),
    );
}

#[test]
fn coalesced_neural_inputs_preserve_tick_state() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    s.settings.population = 32;
    let mut expected = Vec::new();
    for coalesced in [false, true] {
        install_neural_input_layout(&mut s, &d, coalesced);
        s.reset(&q);
        step(&mut s, &d, &q, 32);
        let buffers = [
            &s.agent_buffers[s.current_buffer],
            &s.decision_buffer,
            &s.fast_weight_buffers[0],
            &s.fast_weight_buffers[1],
            &s.trace_buffer,
            &s.resource_buffer,
            &s.death_stats_buffer,
        ];
        for (index, buffer) in buffers.into_iter().enumerate() {
            let bytes = observability::read_buffer(&d, &q, buffer).unwrap();
            if coalesced {
                assert!(
                    bytes == expected[index],
                    "neural layout changed buffer {index}"
                );
            } else {
                expected.push(bytes);
            }
        }
    }
}

#[test]
#[ignore = "manual neural input layout and expressed-capacity throughput probe"]
fn profile_neural_input_layout_and_capacity() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    for population in [1000, 4096] {
        s.settings.population = population;
        for capacity in [0, 8, 16] {
            for repeat in 0..3 {
                let layouts = if repeat % 2 == 0 {
                    [false, true]
                } else {
                    [true, false]
                };
                for coalesced in layouts {
                    install_neural_input_layout(&mut s, &d, coalesced);
                    s.reset(&q);
                    if capacity != 0 {
                        let mut agents = read::<AgentGpu>(
                            &d,
                            &q,
                            &s.agent_buffers[s.current_buffer],
                            MAX_AGENTS as usize,
                        );
                        for a in agents.iter_mut().filter(|a| a.alive != 0) {
                            a.active_mask = (1 << capacity) - 1;
                            // Keep these controllers juvenile for this 288-tick probe.
                            a.age = 0.0;
                        }
                        q.write_buffer(
                            &s.agent_buffers[s.current_buffer],
                            0,
                            bytemuck::cast_slice(&agents),
                        );
                    }
                    step(&mut s, &d, &q, 32);
                    let start = std::time::Instant::now();
                    for _ in 0..8 {
                        step(&mut s, &d, &q, 32);
                    }
                    eprintln!(
                        "population={population} capacity={capacity} coalesced={coalesced} repeat={repeat}: {:.1} ticks/s",
                        256.0 / start.elapsed().as_secs_f64()
                    );
                }
            }
        }
    }
}

// Keep a reference kernel to compare exact numerical state and paired throughput.
fn install_tick_optimizations(
    s: &mut Simulation,
    d: &wgpu::Device,
    grouped: bool,
    accounting: bool,
) {
    s.separate_compute_passes = !grouped;
    let name = "plastic";
    let source = include_str!("../shaders/plasticity.wgsl").replace(
        "DIRECT_ACCOUNTING:bool=true",
        &format!("DIRECT_ACCOUNTING:bool={accounting}"),
    );
    let compute = s.passes.get_mut(name).unwrap();
    let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("optimization reference"),
        bind_group_layouts: &[&compute.pipeline.get_bind_group_layout(0)],
        push_constant_ranges: &[],
    });
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(name),
        source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
    });
    compute.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(name),
        layout: Some(&layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });
}

#[test]
fn tick_optimizations_preserve_state_across_climate_epochs() {
    compare_tick_state(false);
}

#[test]
fn linked_spatial_preserves_isolated_state_across_climate_epochs() {
    compare_tick_state(true);
}

fn compare_tick_state(compare_spatial: bool) {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    s.settings.population = 32;
    for (seed, rotation, age) in [(42, 0, 0), (91, 1, 995), (3137, 3, 47001), (42, 2, 999998)] {
        let mut expected = Vec::new();
        for optimized in [false, true] {
            install_tick_optimizations(&mut s, &d, optimized, optimized);
            install_tick_boundaries(&mut s, &d, optimized);
            install_linked_spatial(&mut s, &d, !compare_spatial || optimized);
            s.seed = seed;
            s.settings.environment_rotation = rotation;
            s.reset(&q);
            if compare_spatial {
                // Neighbor reductions have unordered GPU insertion in both
                // layouts. Separate bodies for a byte-exact execution check;
                // crowded-cell membership and sensing are checked separately.
                let mut bodies = s.agent_snapshot(&d, &q).unwrap();
                for (slot, body) in bodies.iter_mut().enumerate().filter(|(_, a)| a.alive != 0) {
                    body.position = [
                        128.0 + (slot % 8) as f32 * 256.0,
                        128.0 + (slot / 8) as f32 * 256.0,
                    ];
                    body.age = 0.0;
                }
                q.write_buffer(
                    &s.agent_buffers[s.current_buffer],
                    0,
                    bytemuck::cast_slice(&bodies),
                );
            }
            s.environment_start_age = age;
            step(&mut s, &d, &q, 8);
            // A second batch verifies parameter updates across command submissions.
            step(&mut s, &d, &q, 8);
            for (index, buffer) in [
                &s.agent_buffers[s.current_buffer],
                &s.decision_buffer,
                &s.fast_weight_buffers[0],
                &s.fast_weight_buffers[1],
                &s.trace_buffer,
                &s.resource_buffer,
                &s.fertility_buffer,
                &s.ground_buffer,
                &s.ecology_buffer,
                &s.death_stats_buffer,
            ]
            .into_iter()
            .enumerate()
            {
                let bytes = observability::read_buffer(&d, &q, buffer).unwrap();
                if optimized {
                    assert!(
                        bytes == expected[index],
                        "seed={seed} rotation={rotation} age={age} buffer={index}"
                    );
                } else {
                    expected.push(bytes);
                }
            }
        }
    }
}

#[test]
#[ignore = "manual paired compute-pass and plasticity-accounting throughput probe"]
fn profile_tick_optimizations() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("Adapter: {:?}", adapter.get_info());
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("tick optimization profiler"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap();
    let mut s = Simulation::new(&d, &q, 42);
    for population in [32, 1000, 4096] {
        s.settings.population = population;
        for repeat in 0..3 {
            let mut modes = vec![(false, false), (true, false), (true, true)];
            if repeat % 2 == 1 {
                modes.reverse();
            }
            for (grouped, accounting) in modes {
                install_tick_optimizations(&mut s, &d, grouped, accounting);
                s.reset(&q);
                step(&mut s, &d, &q, 32);
                let start = std::time::Instant::now();
                for _ in 0..16 {
                    step(&mut s, &d, &q, 32);
                }
                eprintln!(
                    "population={population} repeat={repeat} grouped={grouped} accounting={accounting}: {:.1} ticks/s",
                    512.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

// Retain the previous scheduling and shader paths for paired measurements.
fn install_streaming_optimizations(
    s: &mut Simulation,
    d: &wgpu::Device,
    parallel_inheritance: bool,
    cooperative: bool,
) {
    s.reference_inheritance = !parallel_inheritance;
    for (name, source) in [
        (
            "inherit_genomes",
            include_str!("../shaders/inherit_genomes.wgsl").to_string(),
        ),
        (
            "decide_live",
            include_str!("../shaders/decide_parallel.wgsl").replace(
                "COOPERATIVE_OUTPUTS:bool=true",
                &format!("COOPERATIVE_OUTPUTS:bool={cooperative}"),
            ),
        ),
    ] {
        let compute = s.passes.get_mut(name).unwrap();
        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("streaming reference"),
            bind_group_layouts: &[&compute.pipeline.get_bind_group_layout(0)],
            push_constant_ranges: &[],
        });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(shader_source(&source).into()),
        });
        compute.pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(&layout),
            module: &shader,
            entry_point: Some(if name == "inherit_genomes" && parallel_inheritance {
                "parallel"
            } else {
                "main"
            }),
            compilation_options: Default::default(),
            cache: None,
        });
    }
}

#[test]
fn streaming_optimizations_preserve_sparse_state() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    for (seed, population) in [(42, 0), (91, 32), (3137, 1000)] {
        let mut expected = Vec::new();
        for optimized in [false, true] {
            install_streaming_optimizations(&mut s, &d, optimized, optimized);
            s.seed = seed;
            s.settings.population = population;
            s.reset(&q);
            // Deliberately leave different dead history in the other bank.
            // Preservation cannot rely on the two banks initially matching.
            let mut agents = read::<AgentGpu>(
                &d,
                &q,
                &s.agent_buffers[s.current_buffer],
                MAX_AGENTS as usize,
            );
            for (i, a) in agents.iter_mut().enumerate() {
                if i % 3 == 0 {
                    a.alive = 0;
                    a.generation = 73;
                    a.food = 0.5;
                }
                if a.alive != 0 {
                    // Separate bodies and keep them juvenile: avoid atomic birth-ID
                    // allocation and neighbor summation order in exact comparisons.
                    a.position = [32.0 + (i % 32) as f32 * 64.0, 32.0 + (i / 32) as f32 * 64.0];
                    a.age = 0.0;
                    a.max_age = 3.0 + (i % 17) as f32;
                }
            }
            q.write_buffer(
                &s.agent_buffers[s.current_buffer],
                0,
                bytemuck::cast_slice(&agents),
            );
            for ticks in [1, 3, 8, 17] {
                step(&mut s, &d, &q, ticks);
            }
            for (index, buffer) in [
                &s.agent_buffers[s.current_buffer],
                &s.decision_buffer,
                &s.fast_weight_buffers[0],
                &s.fast_weight_buffers[1],
                &s.trace_buffer,
                &s.resource_buffer,
                &s.fertility_buffer,
                &s.ground_buffer,
                &s.ecology_buffer,
                &s.death_stats_buffer,
                &s.reservoir_genome_buffers[0],
                &s.reservoir_genome_buffers[1],
                &s.reservoir_traits_buffer,
                &s.reservoir_rng_buffer,
            ]
            .into_iter()
            .enumerate()
            {
                let bytes = observability::read_buffer(&d, &q, buffer).unwrap();
                if optimized {
                    if bytes != expected[index] {
                        let first = bytes
                            .iter()
                            .zip(&expected[index])
                            .position(|(a, b)| a != b)
                            .unwrap();
                        eprintln!(
                            "first difference at byte {first}, body stride {}",
                            std::mem::size_of::<AgentGpu>()
                        );
                        panic!("seed={seed} buffer={index}");
                    }
                } else {
                    expected.push(bytes);
                }
            }
        }
    }
}

#[test]
#[ignore = "manual paired inheritance and neural-output throughput probe"]
fn profile_streaming_optimizations() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    for population in [32, 1000, 4096, 8192] {
        s.settings.population = population;
        for repeat in 0..3 {
            let mut modes = vec![(false, false), (true, false), (true, true)];
            if repeat % 2 == 1 {
                modes.reverse();
            }
            for (parallel_inheritance, cooperative) in modes {
                install_streaming_optimizations(&mut s, &d, parallel_inheritance, cooperative);
                s.reset(&q);
                step(&mut s, &d, &q, 32);
                let start = std::time::Instant::now();
                for _ in 0..16 {
                    step(&mut s, &d, &q, 32);
                }
                eprintln!(
                    "population={population} repeat={repeat} parallel_inheritance={parallel_inheritance} cooperative={cooperative}: {:.1} ticks/s",
                    512.0 / start.elapsed().as_secs_f64()
                );
            }
        }
    }
}

#[test]
fn cooperative_packet_inheritance_copies_every_parameter() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    for slot in 0..130 {
        let mut genes = fixed(5, [0.0; 2]);
        genes[INPUT_BASE] = slot as f32 / 100.0;
        let a = AgentGpu {
            lineage_id: slot as u32 + 1,
            active_mask: 1,
            ..body([
                32.0 + (slot % 16) as f32 * 100.0,
                32.0 + (slot / 16) as f32 * 100.0,
            ])
        };
        put(&s, &q, slot, a, &genes);
    }
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let packets: Vec<_> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.alive == 2)
        .collect();
    assert_eq!(packets.len(), 130);
    let slots: Vec<_> = packets.iter().map(|(slot, _)| *slot).collect();
    let genes = s.read_genome_slots(&d, &q, &slots).unwrap();
    for ((_, packet), actual) in packets.iter().zip(genes.chunks_exact(GENOME_SIZE)) {
        let mut expected = fixed(5, [0.0; 2]);
        expected[INPUT_BASE] = packet.birth_parent_slot as f32 / 100.0;
        assert_eq!(actual, expected.as_slice());
    }
}

#[test]
#[ignore = "read-only paired benchmark of latest saved experiment"]
fn profile_saved_experiment_streaming() {
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let (mut saves, _) = crate::experiments::list(&crate::experiments::save_root()).unwrap();
    saves.sort_by_key(|s| std::cmp::Reverse(s.record.saved_at_ms));
    let saved = &saves[0];
    eprintln!(
        "Saved world={} tick={} living={}",
        saved.record.world, saved.record.tick, saved.record.living
    );
    for repeat in 0..3 {
        let modes = if repeat % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for optimized in modes {
            install_streaming_optimizations(&mut s, &d, optimized, optimized);
            s.load_game_checkpoint(
                &q,
                std::fs::File::open(saved.checkpoint()).unwrap(),
                (saved.record.seed, saved.record.tick, saved.record.living),
                saved.record.world,
            )
            .unwrap();
            step(&mut s, &d, &q, 32);
            let start = std::time::Instant::now();
            for _ in 0..16 {
                step(&mut s, &d, &q, 32);
            }
            eprintln!(
                "saved repeat={repeat} optimized={optimized}: {:.1} ticks/s",
                512.0 / start.elapsed().as_secs_f64()
            );
            // Model completion polling at 32x without surface rendering.
            // Both paths start this phase after the same number of ticks.
            let output = readback(&d, crate::playback::TELEMETRY_SIZE);
            let target: f64 = if optimized { 0.032 } else { 0.008 };
            let mut seconds_per_tick = start.elapsed().as_secs_f64() / 512.0;
            let start = std::time::Instant::now();
            let mut elapsed_ticks = 0;
            let mut batches = 0;
            while elapsed_ticks < 512 {
                let ticks = ((target / seconds_per_tick).floor().clamp(1.0, 32.0) as u32)
                    .min(512 - elapsed_ticks);
                let batch_start = std::time::Instant::now();
                let mut e = d.create_command_encoder(&Default::default());
                s.encode_ticks(&mut e, &d, &q, ticks);
                s.encode_telemetry(&mut e, &output);
                q.submit(Some(e.finish()));
                let (tx, rx) = mpsc::channel();
                output.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    tx.send(r).unwrap();
                });
                loop {
                    d.poll(wgpu::Maintain::Poll);
                    match rx.try_recv() {
                        Ok(result) => {
                            result.unwrap();
                            break;
                        }
                        Err(mpsc::TryRecvError::Empty) => {
                            std::thread::sleep(std::time::Duration::from_millis(1))
                        }
                        Err(error) => panic!("{error}"),
                    }
                }
                output.unmap();
                seconds_per_tick = seconds_per_tick * 0.8
                    + batch_start.elapsed().as_secs_f64() / f64::from(ticks) * 0.2;
                elapsed_ticks += ticks;
                batches += 1;
            }
            eprintln!(
                "saved polling repeat={repeat} optimized={optimized}: {:.1} ticks/s ({batches} batches)",
                512.0 / start.elapsed().as_secs_f64()
            );
        }
    }
}
