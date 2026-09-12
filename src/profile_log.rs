//! Opt-in viewer measurements, emitted once per display statistics window.
use std::io::Write;

pub struct ProfileLog {
    file: std::fs::File,
    pub gpu_ms: f64,
    pub ticks: u64,
    pub batches: u64,
    pub batch_wall_ms: f64,
    pub encode_submit_ms: f64,
    pub render_cpu_ms: f64,
    pub surface_wait_ms: f64,
    render: Option<RenderProbe>,
    world_gpu_ms: f64,
    ui_gpu_ms: f64,
    render_samples: u64,
}

struct RenderProbe {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    pending: Option<std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>>,
    period: f64,
}

impl ProfileLog {
    pub fn from_env(device: &wgpu::Device, period: f32) -> Option<Self> {
        let path = std::env::var_os("PRIMITIVE_PROFILE_VIEWER")?;
        let file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
        {
            Ok(file) => file,
            Err(error) => {
                eprintln!("Viewer profiling disabled: {error}");
                return None;
            }
        };
        Some(Self {
            file,
            gpu_ms: 0.0,
            ticks: 0,
            batches: 0,
            batch_wall_ms: 0.0,
            encode_submit_ms: 0.0,
            render_cpu_ms: 0.0,
            surface_wait_ms: 0.0,
            world_gpu_ms: 0.0,
            ui_gpu_ms: 0.0,
            render_samples: 0,
            render: device
                .features()
                .contains(wgpu::Features::TIMESTAMP_QUERY)
                .then(|| RenderProbe {
                    queries: device.create_query_set(&wgpu::QuerySetDescriptor {
                        label: Some("viewer render profile"),
                        ty: wgpu::QueryType::Timestamp,
                        count: 4,
                    }),
                    resolve: device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: 32,
                        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }),
                    readback: device.create_buffer(&wgpu::BufferDescriptor {
                        label: None,
                        size: 32,
                        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    pending: None,
                    period: f64::from(period),
                }),
        })
    }

    pub fn prepare_render(&mut self) -> bool {
        let Some(render) = &mut self.render else {
            return false;
        };
        if let Some(rx) = &render.pending {
            match rx.try_recv() {
                Ok(Ok(())) => {
                    let data = render.readback.slice(..).get_mapped_range();
                    let times: &[u64] = bytemuck::cast_slice(&data);
                    self.world_gpu_ms +=
                        times[1].saturating_sub(times[0]) as f64 * render.period / 1e6;
                    self.ui_gpu_ms +=
                        times[3].saturating_sub(times[2]) as f64 * render.period / 1e6;
                    self.render_samples += 1;
                    drop(data);
                    render.readback.unmap();
                    render.pending = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => return false,
                _ => {
                    self.render = None;
                    return false;
                }
            }
        }
        true
    }

    pub fn timestamps(&self, start: u32) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        self.render
            .as_ref()
            .map(|r| wgpu::RenderPassTimestampWrites {
                query_set: &r.queries,
                beginning_of_pass_write_index: Some(start),
                end_of_pass_write_index: Some(start + 1),
            })
    }

    pub fn resolve_render(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(r) = &self.render {
            encoder.resolve_query_set(&r.queries, 0..4, &r.resolve, 0);
            encoder.copy_buffer_to_buffer(&r.resolve, 0, &r.readback, 0, 32);
        }
    }

    pub fn map_render(&mut self) {
        if let Some(r) = &mut self.render {
            let (tx, rx) = std::sync::mpsc::channel();
            r.readback
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |result| {
                    let _ = tx.send(result);
                });
            r.pending = Some(rx);
        }
    }

    pub fn emit(&mut self, mut row: serde_json::Value) {
        row["gpu_ms"] = self.gpu_ms.into();
        row["completed_ticks"] = self.ticks.into();
        row["batches"] = self.batches.into();
        row["batch_wall_ms"] = self.batch_wall_ms.into();
        row["encode_submit_ms"] = self.encode_submit_ms.into();
        row["render_cpu_ms"] = self.render_cpu_ms.into();
        row["surface_wait_ms"] = self.surface_wait_ms.into();
        row["world_gpu_ms"] = self.world_gpu_ms.into();
        row["ui_gpu_ms"] = self.ui_gpu_ms.into();
        row["render_samples"] = self.render_samples.into();
        if let Err(error) = writeln!(self.file, "{row}") {
            eprintln!("Viewer profile write failed: {error}");
        }
        self.gpu_ms = 0.0;
        self.ticks = 0;
        self.batches = 0;
        self.batch_wall_ms = 0.0;
        self.encode_submit_ms = 0.0;
        self.render_cpu_ms = 0.0;
        self.surface_wait_ms = 0.0;
        self.world_gpu_ms = 0.0;
        self.ui_gpu_ms = 0.0;
        self.render_samples = 0;
    }
}
