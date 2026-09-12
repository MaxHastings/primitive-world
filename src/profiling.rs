//! Manual, read-only checkpoint profiling. No production biological changes.
use super::*;
use serde_json::{Value, json};
use std::{cell::RefCell, path::PathBuf, rc::Rc, time::Instant};

fn load(s: &mut Simulation, q: &wgpu::Queue, receipt: &PathBuf) {
    let record: crate::experiments::SaveRecord =
        serde_json::from_slice(&std::fs::read(receipt).unwrap()).unwrap();
    s.load_game_checkpoint(
        q,
        std::fs::File::open(receipt.parent().unwrap().join(record.checkpoint)).unwrap(),
        (record.seed, record.tick, record.living),
        record.world,
    )
    .unwrap();
}

fn population(s: &Simulation, d: &wgpu::Device, q: &wgpu::Queue) -> Value {
    let bodies = s.agent_snapshot(d, q).unwrap();
    let mut units = [0u32; 17];
    for a in bodies.iter().filter(|a| a.alive == 1) {
        units[a.active_mask.count_ones() as usize] += 1;
    }
    json!({"tick": s.tick, "world": s.progress.world,
        "organisms": bodies.iter().filter(|a| a.alive == 1).count(),
        "packets": bodies.iter().filter(|a| a.alive == 2).count(),
        "expressed_units_histogram": units})
}

#[test]
#[ignore = "explicit copied receipt and output required; observer and durable save timings"]
fn profile_checkpoint_observers() {
    let receipt = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_RECEIPT").unwrap());
    let output = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_OUTPUT").unwrap());
    assert!(!output.exists());
    let (d, q) = gpu();
    let mut s = Simulation::new(&d, &q, 42);
    let saved: crate::experiments::SaveRecord =
        serde_json::from_slice(&std::fs::read(&receipt).unwrap()).unwrap();
    let dir = output.with_extension("saves").join("experiment");
    std::fs::create_dir_all(&dir).unwrap();
    let experiment = crate::experiments::Experiment {
        directory: dir,
        name: "Isolated profiling copy".into(),
        origin: "Read-only source checkpoint profiling".into(),
        total_ticks: saved.total_ticks,
    };
    let mut rows = Vec::new();
    for repeat in 0..5 {
        let at = Instant::now();
        load(&mut s, &q, &receipt);
        // write_buffer uploads are submitted lazily; polling alone does not flush them.
        let upload = q.submit(std::iter::empty());
        d.poll(wgpu::Maintain::WaitForSubmissionIndex(upload));
        rows.push(json!({"repeat":repeat,"name":"checkpoint_load_and_upload","ms":at.elapsed().as_secs_f64()*1000.0}));
        for _ in 0..4 {
            step(&mut s, &d, &q, 32);
        }
        let at = Instant::now();
        let metrics = s.metrics(&d, &q).unwrap();
        rows.push(json!({"repeat":repeat,"name":"metrics","ms":at.elapsed().as_secs_f64()*1000.0}));
        let at = Instant::now();
        let evolution = s.evolution_snapshot(&d, &q).unwrap();
        rows.push(json!({"repeat":repeat,"name":"evolution_snapshot","ms":at.elapsed().as_secs_f64()*1000.0}));
        let at = Instant::now();
        let search = s.search_snapshot(&d, &q, &mut None).unwrap();
        rows.push(json!({"repeat":repeat,"name":"search_snapshot","ms":at.elapsed().as_secs_f64()*1000.0}));
        let at = Instant::now();
        let serialized =
            serde_json::to_vec(&json!({"metrics":metrics,"evolution":evolution,"search":search}))
                .unwrap();
        rows.push(json!({"repeat":repeat,"name":"sample_json_serialization","ms":at.elapsed().as_secs_f64()*1000.0,"bytes":serialized.len()}));
        let at = Instant::now();
        experiment.save(&s, &d, &q).unwrap();
        rows.push(json!({"repeat":repeat,"name":"durable_experiment_save","ms":at.elapsed().as_secs_f64()*1000.0}));
        for (name, buffer) in [
            ("bodies", &s.agent_buffers[s.current_buffer]),
            ("resources", &s.resource_buffer),
            ("fertility", &s.fertility_buffer),
            ("ground", &s.ground_buffer),
            ("counters", &s.death_stats_buffer),
            ("events", &s.event_buffer),
            ("perception", &s.perception_buffer),
            ("decisions", &s.decision_buffer),
            ("genomes_0", &s.genome_buffers[0]),
            ("genomes_1", &s.genome_buffers[1]),
            ("learned_weights_0", &s.fast_weight_buffers[0]),
            ("learned_weights_1", &s.fast_weight_buffers[1]),
            ("traces", &s.trace_buffer),
            ("reservoir_genomes_0", &s.reservoir_genome_buffers[0]),
            ("reservoir_genomes_1", &s.reservoir_genome_buffers[1]),
            ("reservoir_traits", &s.reservoir_traits_buffer),
            ("reservoir_rng", &s.reservoir_rng_buffer),
            ("ecology", &s.ecology_buffer),
        ] {
            let at = Instant::now();
            let data = observability::read_buffer(&d, &q, buffer).unwrap();
            rows.push(json!({"repeat":repeat,"name":format!("readback_{name}"),"ms":at.elapsed().as_secs_f64()*1000.0,"bytes":data.len()}));
        }
    }
    std::fs::write(output,serde_json::to_vec_pretty(&json!({"schema":1,"rows":rows,"notes":"Copied checkpoint warmed for 128 ticks, then frozen during observer/save probes. Load timing flushes queued uploads. Readbacks are separate probes, not instrumentation of the durable save. JSON serialization is one sample, not a full run report."})).unwrap()).unwrap();
}

#[test]
#[ignore = "explicit copied receipt and output required; low-density dispatch timestamps"]
fn profile_checkpoint_selective() {
    let receipt = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_RECEIPT").unwrap());
    let output = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_OUTPUT").unwrap());
    assert!(!output.exists());
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("selective checkpoint profiler"),
            required_features: wgpu::Features::TIMESTAMP_QUERY
                | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
                | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES,
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap();
    let mut s = Simulation::new(&d, &q, 42);
    let trace = Rc::new(RefCell::new(DispatchTrace {
        queries: d.create_query_set(&wgpu::QuerySetDescriptor {
            label: None,
            ty: wgpu::QueryType::Timestamp,
            count: 4096,
        }),
        capacity: 4096,
        labels: Vec::new(),
    }));
    let resolved = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 32768,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mapped = readback(&d, 32768);
    let names = [
        "baseline",
        "perceive_live",
        "inherit_fusion",
        "resource",
        "plastic",
        "decide_live",
        "reset_cognitive_birth_state",
        "inherit_genomes",
        "update_reservoir",
        "birth",
        "linked_clear",
        "interact_propose",
        "interact_resolve",
        "linked_link",
        "body_live",
        "observe_signals",
        "observe_memory",
        "consume",
        "interact_production",
        "birth_blocks",
        "fusion",
        "free_blocks",
        "free",
        "claim_reservoir",
        "birth_sums",
        "release",
        "free_compact",
        "birth_add",
        "free_sums",
        "birth_compact",
        "interact_clear",
        "free_add",
        "advance_reservoir",
        "alive",
    ];
    let mut captures = Vec::new();
    for repeat in 0..3 {
        let order: Vec<_> = if repeat % 2 == 0 {
            names.to_vec()
        } else {
            names.iter().rev().copied().collect()
        };
        for name in order {
            load(&mut s, &q, &receipt);
            for _ in 0..4 {
                step(&mut s, &d, &q, 32);
            }
            if name != "baseline" {
                s.passes.get_mut(name).unwrap().dispatch_trace =
                    Some((name.to_string(), trace.clone()));
            }
            let mut group_gpu = 0.0;
            let mut group_kernel = 0.0;
            for sample in 0..4 {
                trace.borrow_mut().labels.clear();
                let at = Instant::now();
                let mut e = d.create_command_encoder(&Default::default());
                e.write_timestamp(&trace.borrow().queries, 0);
                s.encode_ticks(&mut e, &d, &q, 32);
                let tr = trace.borrow();
                e.write_timestamp(&tr.queries, 1);
                let count = 2 + tr.labels.len() as u32 * 2;
                e.resolve_query_set(&tr.queries, 0..count, &resolved, 0);
                e.copy_buffer_to_buffer(&resolved, 0, &mapped, 0, u64::from(count) * 8);
                q.submit(Some(e.finish()));
                let (tx, rx) = mpsc::channel();
                mapped
                    .slice(..u64::from(count) * 8)
                    .map_async(wgpu::MapMode::Read, move |r| {
                        tx.send(r).unwrap();
                    });
                d.poll(wgpu::Maintain::Wait);
                rx.recv().unwrap().unwrap();
                let wall_us = at.elapsed().as_secs_f64() * 1e6;
                let range = mapped.slice(..u64::from(count) * 8).get_mapped_range();
                let times: &[u64] = bytemuck::cast_slice(&range);
                let scale = f64::from(q.get_timestamp_period()) / 1000.0;
                let durations: Vec<f64> = (0..tr.labels.len())
                    .map(|i| (times[3 + i * 2] - times[2 + i * 2]) as f64 * scale)
                    .collect();
                let gpu_us = (times[1] - times[0]) as f64 * scale;
                group_gpu += gpu_us;
                group_kernel += durations.iter().sum::<f64>();
                captures.push(json!({"name":name,"repeat":repeat,"sample":sample,"ticks":32,"gpu_us":gpu_us,"wall_us":wall_us,"dispatch_us":durations}));
                drop(range);
                mapped.unmap();
            }
            if name != "baseline" {
                s.passes.get_mut(name).unwrap().dispatch_trace = None;
            }
            eprintln!(
                "selective {name} repeat {repeat}: {:.2} us/tick, batch GPU {:.2} us/tick",
                group_kernel / 128.0,
                group_gpu / 128.0
            );
        }
    }
    std::fs::write(&output, serde_json::to_vec_pretty(&json!({"schema":1,"receipt":receipt,"adapter":format!("{:?}",adapter.get_info()),"captures":captures,
        "notes":"Each run times only one selected kernel, with production grouping unchanged. Durations from different runs are diagnostic estimates, not an additive accounting identity."})).unwrap()).unwrap();
}

#[test]
#[ignore = "explicit copied receipt and output required; layered GPU/host profile"]
fn profile_checkpoint_layers() {
    let receipt = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_RECEIPT").unwrap());
    let output = PathBuf::from(std::env::var("PRIMITIVE_PROFILE_OUTPUT").unwrap());
    assert!(!output.exists(), "Use a new output filename");
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let features = wgpu::Features::TIMESTAMP_QUERY
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS
        | wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
    assert!(
        adapter.features().contains(features),
        "Inside-pass timestamps unavailable"
    );
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("checkpoint layered profiler"),
            required_features: features,
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .unwrap();
    let mut s = Simulation::new(&d, &q, 42);
    load(&mut s, &q, &receipt);
    let initial = population(&s, &d, &q);
    eprintln!("Adapter {:?}; checkpoint {initial}", adapter.get_info());
    let queries = d.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("whole batch"),
        ty: wgpu::QueryType::Timestamp,
        count: 2,
    });
    let resolved = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("profile resolve"),
        size: 65536,
        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mapped = readback(&d, 65536);
    let telemetry = readback(&d, crate::playback::READBACK_SIZE);
    let mut runs = Vec::new();
    let modes = [
        "baseline",
        "gpu_batch",
        "telemetry",
        "poll_1ms",
        "metrics_1s",
        "observation_1024",
        "batch_1",
    ];
    for repeat in 0..5 {
        let order: Vec<_> = if repeat % 2 == 0 {
            modes.to_vec()
        } else {
            modes.iter().rev().copied().collect()
        };
        for mode in order {
            load(&mut s, &q, &receipt);
            for _ in 0..4 {
                step(&mut s, &d, &q, 32);
            }
            let before = population(&s, &d, &q);
            let ticks = if mode == "batch_1" { 512 } else { 4096 };
            let batch = if mode == "batch_1" { 1 } else { 32 };
            let timestamp = mode != "baseline" && mode != "batch_1";
            let with_telemetry = matches!(
                mode,
                "telemetry" | "poll_1ms" | "metrics_1s" | "observation_1024"
            );
            let polling = mode == "poll_1ms" || mode == "metrics_1s";
            let mut enc_us = 0.0;
            let mut submit_us = 0.0;
            let mut wait_us = 0.0;
            let mut gpu_us = 0.0;
            let mut observation_us = 0.0;
            let mut metrics_count = 0;
            let mut last_metrics = Instant::now();
            let mut batches = Vec::new();
            let start = Instant::now();
            for elapsed in (0..ticks).step_by(batch as usize) {
                let batch_at = Instant::now();
                let mut e = d.create_command_encoder(&Default::default());
                if timestamp {
                    e.write_timestamp(&queries, 0);
                }
                s.encode_ticks(&mut e, &d, &q, batch);
                if timestamp {
                    e.write_timestamp(&queries, 1);
                }
                if with_telemetry {
                    s.encode_telemetry(&mut e, &telemetry);
                }
                let metrics = mode == "metrics_1s" && last_metrics.elapsed().as_secs_f64() >= 1.0;
                if metrics {
                    s.encode_metrics(&mut e, &telemetry, crate::playback::METRICS_OFFSET);
                    metrics_count += 1;
                }
                if timestamp {
                    e.resolve_query_set(&queries, 0..2, &resolved, 0);
                    e.copy_buffer_to_buffer(&resolved, 0, &mapped, 0, 16);
                }
                let commands = e.finish();
                enc_us += batch_at.elapsed().as_secs_f64() * 1e6;
                let at = Instant::now();
                let submission = q.submit(Some(commands));
                submit_us += at.elapsed().as_secs_f64() * 1e6;
                let at = Instant::now();
                if timestamp {
                    let (tx, rx) = mpsc::channel();
                    mapped.slice(..16).map_async(wgpu::MapMode::Read, move |r| {
                        tx.send(r).unwrap();
                    });
                    // Map the actual telemetry too, as the viewer does.
                    let telemetry_rx = if with_telemetry {
                        let (tx, rx) = mpsc::channel();
                        telemetry
                            .slice(..)
                            .map_async(wgpu::MapMode::Read, move |r| {
                                tx.send(r).unwrap();
                            });
                        Some(rx)
                    } else {
                        None
                    };
                    if polling {
                        loop {
                            d.poll(wgpu::Maintain::Poll);
                            match rx.try_recv() {
                                Ok(r) => {
                                    r.unwrap();
                                    break;
                                }
                                Err(mpsc::TryRecvError::Empty) => {
                                    std::thread::sleep(std::time::Duration::from_millis(1))
                                }
                                Err(e) => panic!("{e}"),
                            }
                        }
                    } else {
                        d.poll(wgpu::Maintain::Wait);
                        rx.recv().unwrap().unwrap();
                    }
                    let range = mapped.slice(..16).get_mapped_range();
                    let times: &[u64] = bytemuck::cast_slice(&range);
                    let batch_gpu =
                        (times[1] - times[0]) as f64 * f64::from(q.get_timestamp_period()) / 1000.0;
                    gpu_us += batch_gpu;
                    drop(range);
                    mapped.unmap();
                    if let Some(rx) = telemetry_rx {
                        d.poll(wgpu::Maintain::Wait);
                        rx.recv().unwrap().unwrap();
                        if metrics {
                            let data = telemetry.slice(..).get_mapped_range();
                            s.decode_metrics(
                                &data[crate::playback::METRICS_OFFSET as usize..],
                                &data[4..crate::playback::TELEMETRY_SIZE as usize],
                            )
                            .unwrap();
                            last_metrics = Instant::now();
                        }
                        telemetry.unmap();
                    }
                } else {
                    d.poll(wgpu::Maintain::WaitForSubmissionIndex(submission));
                }
                wait_us += at.elapsed().as_secs_f64() * 1e6;
                if mode == "observation_1024" && (elapsed + batch) % 1024 == 0 {
                    let at = Instant::now();
                    s.metrics(&d, &q).unwrap();
                    s.evolution_snapshot(&d, &q).unwrap();
                    s.search_snapshot(&d, &q, &mut None).unwrap();
                    observation_us += at.elapsed().as_secs_f64() * 1e6;
                }
                batches.push(batch_at.elapsed().as_secs_f64() * 1e6);
            }
            let seconds = start.elapsed().as_secs_f64();
            let row = json!({"mode":mode,"repeat":repeat,"ticks":ticks,"batch":batch,
                "seconds":seconds,"tps": f64::from(ticks)/seconds,
                "encode_us_per_tick":enc_us/f64::from(ticks),"submit_us_per_tick":submit_us/f64::from(ticks),
                "completion_us_per_tick":wait_us/f64::from(ticks),"gpu_us_per_tick":gpu_us/f64::from(ticks),
                "observation_us":observation_us,"metrics_count":metrics_count,"batch_wall_us":batches,
                "before":before,"after":population(&s,&d,&q)});
            eprintln!(
                "{mode} repeat {repeat}: {:.1} ticks/s",
                f64::from(ticks) / seconds
            );
            runs.push(row);
        }
    }
    let trace = Rc::new(RefCell::new(DispatchTrace {
        capacity: 4096,
        queries: d.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("every dispatch occurrence"),
            ty: wgpu::QueryType::Timestamp,
            count: 4096,
        }),
        labels: Vec::new(),
    }));
    let mut captures = Vec::new();
    for repeat in 0..5 {
        load(&mut s, &q, &receipt);
        for _ in 0..4 {
            step(&mut s, &d, &q, 32);
        }
        for (name, pass) in &mut s.passes {
            pass.dispatch_trace = Some((name.clone(), trace.clone()));
        }
        for sample in 0..4 {
            trace.borrow_mut().labels.clear();
            let at = Instant::now();
            let mut e = d.create_command_encoder(&Default::default());
            e.write_timestamp(&trace.borrow().queries, 0);
            s.encode_ticks(&mut e, &d, &q, 32);
            let trace_ref = trace.borrow();
            e.write_timestamp(&trace_ref.queries, 1);
            let count = 2 + trace_ref.labels.len() as u32 * 2;
            e.resolve_query_set(&trace_ref.queries, 0..count, &resolved, 0);
            e.copy_buffer_to_buffer(&resolved, 0, &mapped, 0, u64::from(count) * 8);
            q.submit(Some(e.finish()));
            let (tx, rx) = mpsc::channel();
            mapped
                .slice(..u64::from(count) * 8)
                .map_async(wgpu::MapMode::Read, move |r| {
                    tx.send(r).unwrap();
                });
            d.poll(wgpu::Maintain::Wait);
            rx.recv().unwrap().unwrap();
            let wall_us = at.elapsed().as_secs_f64() * 1e6;
            let range = mapped.slice(..u64::from(count) * 8).get_mapped_range();
            let times: &[u64] = bytemuck::cast_slice(&range);
            let scale = f64::from(q.get_timestamp_period()) / 1000.0;
            let rows: Vec<_>=trace_ref.labels.iter().enumerate().map(|(i,name)| {
                assert!(times[3+i*2]>=times[2+i*2]);
                json!({"name":name,"start_us":times[2+i*2].saturating_sub(times[0]) as f64*scale,
                    "duration_us":(times[3+i*2]-times[2+i*2]) as f64*scale})
            }).collect();
            assert_eq!(
                trace_ref
                    .labels
                    .iter()
                    .filter(|n| *n == "linked_link")
                    .count(),
                64
            );
            captures.push(
                json!({"repeat":repeat,"sample":sample,"ticks":32,"wall_us":wall_us,
                "gpu_us":(times[1]-times[0]) as f64*scale,"dispatches":rows}),
            );
            drop(range);
            mapped.unmap();
        }
        for pass in s.passes.values_mut() {
            pass.dispatch_trace = None;
        }
    }
    load(&mut s, &q, &receipt);
    let checkpoint_output = output.with_extension("save-probe.checkpoint");
    assert!(!checkpoint_output.exists());
    let at = Instant::now();
    s.save_checkpoint(&d, &q, &checkpoint_output).unwrap();
    let save_seconds = at.elapsed().as_secs_f64();
    let result = json!({"schema":1,"adapter":format!("{:?}",adapter.get_info()),"model":MODEL_ID,
        "receipt":receipt,"warmup_ticks":128,"initial":initial,"runs":runs,"captures":captures,
        "checkpoint_save_seconds":save_seconds,"checkpoint_bytes":std::fs::metadata(checkpoint_output).unwrap().len(),
        "notes":["Completion time includes GPU execution; do not add it to GPU time.",
        "Per-dispatch timestamps retain grouped passes but still perturb execution.",
        "Observation probe runs metrics/evolution/search every 1024 ticks; no report serialization.",
        "Polling is a headless approximation; excludes rendering and actual event-loop pacing."]});
    std::fs::write(&output, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
    eprintln!("Profile written to {}", output.display());
}
