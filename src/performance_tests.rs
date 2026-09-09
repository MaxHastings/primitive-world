use super::*;

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
    let source = |text: &str| {
        text.replace(
            "COALESCED_INPUTS:bool=true",
            &format!("COALESCED_INPUTS:bool={coalesced}"),
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
                            // Hold starting capacity fixed for this execution probe.
                            a.next_birth = MAX_WORLD_TICKS;
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
