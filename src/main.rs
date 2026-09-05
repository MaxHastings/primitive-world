mod environment;
mod founders;
mod headless;
mod inspection;
mod journey_observer;
mod model;
mod play_files;
mod renderer;
mod simulation;
mod travel_observer;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use bytemuck::cast_slice;
use egui_wgpu::ScreenDescriptor;
use renderer::{Lens, Renderer};
use simulation::{MAX_AGENTS, SelectionOutput, Simulation, WORLD_SIZE};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalPosition,
    event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    state: Option<AppState>,
}

struct AppState {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    simulation: Simulation,
    renderer: Renderer,
    egui_context: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
    paused: bool,
    step_requested: bool,
    speed_index: usize,
    fps_timer: Instant,
    frame_count: u32,
    render_fps: f32,
    ticks_last_second: u32,
    ticks_window_accumulated: u32,
    living_agents: u32,
    food_eaten: u32,
    starvation_deaths: u32,
    age_deaths: u32,
    births: u32,
    interaction_stats: [u32; 4],
    history: std::collections::VecDeque<simulation::observability::WorldMetrics>,
    file_status: String,
    founder_path: String,
    checkpoint_path: String,
    recent_events: Vec<simulation::observability::InteractionEvent>,
    evolution_snapshot: Option<simulation::observability::EvolutionSnapshot>,
    cursor_position: PhysicalPosition<f64>,
    inspection: inspection::Inspection,
    seed_input: u32,
    shock_mode: ShockMode,
    shock_radius: f32,
    last_submit_ms: f32,
    gpu_sim_ms: Option<f32>,
    gpu_render_ms: Option<f32>,
    gpu_timing: Option<GpuTiming>,
}

struct GpuTiming {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    readback: wgpu::Buffer,
    timestamp_period_ns: f32,
}

impl GpuTiming {
    fn read(&self, device: &wgpu::Device) -> Option<(f32, f32)> {
        let slice = self.readback.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        receiver.recv().ok()?.ok()?;
        let mapped = slice.get_mapped_range();
        let values: &[u64] = cast_slice(&mapped);
        if values.len() < 4 {
            drop(mapped);
            self.readback.unmap();
            return None;
        }
        let tick_to_ms = self.timestamp_period_ns / 1_000_000.0;
        let sim_ms = values[1].saturating_sub(values[0]) as f32 * tick_to_ms;
        let render_ms = values[3].saturating_sub(values[2]) as f32 * tick_to_ms;
        drop(mapped);
        self.readback.unmap();
        Some((sim_ms, render_ms))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ShockMode {
    Select,
    AddResource,
    RemoveResource,
    KillAgents,
}

impl AppState {
    async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let surface = instance
            .create_surface(window.clone())
            .expect("surface creation failed");
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no compatible GPU adapter found");
        let mut required_features = wgpu::Features::empty();
        if adapter
            .features()
            .contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS)
        {
            required_features |=
                wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("primitive world device"),
                    required_features,
                    required_limits: adapter.limits(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("GPU device creation failed");
        let capabilities = surface.get_capabilities(&adapter);
        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(capabilities.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: capabilities
                .present_modes
                .iter()
                .copied()
                .find(|mode| *mode == wgpu::PresentMode::AutoVsync)
                .unwrap_or(capabilities.present_modes[0]),
            alpha_mode: capabilities.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);
        let timestamp_period_ns = queue.get_timestamp_period();

        let gpu_timing =
            if required_features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS) {
                Some(GpuTiming {
                    query_set: device.create_query_set(&wgpu::QuerySetDescriptor {
                        label: Some("simulation and render timestamps"),
                        ty: wgpu::QueryType::Timestamp,
                        count: 4,
                    }),
                    resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("timestamp resolve buffer"),
                        size: 4 * std::mem::size_of::<u64>() as u64,
                        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }),
                    readback: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("timestamp readback"),
                        size: 4 * std::mem::size_of::<u64>() as u64,
                        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    timestamp_period_ns,
                })
            } else {
                None
            };

        let mut simulation = Simulation::new(&device, &queue, 1);
        let args: Vec<_> = std::env::args().collect();
        headless::configure(&mut simulation, &args).expect("Invalid command line");
        simulation.reset(&queue);
        if let Some(i) = args.iter().position(|a| a == "--checkpoint") {
            simulation
                .load_checkpoint(&queue, std::path::Path::new(&args[i + 1]))
                .expect("Invalid checkpoint");
        }
        let renderer = Renderer::new(
            &device,
            config.format,
            &simulation,
            config.width,
            config.height,
        );
        let egui_context = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_context.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(&device, config.format, None, 1, false);
        let initial_population = simulation.settings.population;
        let initial_seed = simulation.seed;

        Self {
            window,
            surface,
            device,
            queue,
            config,
            simulation,
            renderer,
            egui_context,
            egui_state,
            egui_renderer,
            paused: false,
            step_requested: false,
            speed_index: 0,
            fps_timer: Instant::now(),
            frame_count: 0,
            render_fps: 0.0,
            ticks_last_second: 0,
            ticks_window_accumulated: 0,
            living_agents: initial_population,
            food_eaten: 0,
            starvation_deaths: 0,
            age_deaths: 0,
            births: 0,
            interaction_stats: [0; 4],
            history: Default::default(),
            file_status: String::new(),
            founder_path: String::new(),
            checkpoint_path: "recurrent-world.checkpoint".into(),
            recent_events: Vec::new(),
            evolution_snapshot: None,
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            inspection: inspection::Inspection::default(),
            seed_input: initial_seed,
            shock_mode: ShockMode::Select,
            shock_radius: 45.0,
            last_submit_ms: 0.0,
            gpu_sim_ms: None,
            gpu_render_ms: None,
            gpu_timing,
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.renderer.resize(width, height);
    }

    fn screen_to_world(&self, position: PhysicalPosition<f64>) -> [f32; 2] {
        let x = position.x as f32 / self.config.width.max(1) as f32 * 2.0 - 1.0;
        let y = 1.0 - position.y as f32 / self.config.height.max(1) as f32 * 2.0;
        [
            self.renderer.camera.center[0]
                + x * self.renderer.camera.aspect * WORLD_SIZE / (2.0 * self.renderer.camera.zoom),
            self.renderer.camera.center[1] - y * WORLD_SIZE / (2.0 * self.renderer.camera.zoom),
        ]
    }

    fn handle_click(&mut self) {
        let world = self.screen_to_world(self.cursor_position);
        match self.shock_mode {
            ShockMode::Select => {
                let selected = self.simulation.select_agent(
                    &self.device,
                    &self.queue,
                    world,
                    14.0 / self.renderer.camera.zoom,
                );
                self.inspection.select(selected, self.simulation.tick);
                self.update_selection_highlight();
            }
            ShockMode::AddResource => self.simulation.apply_resource_shock(
                &self.device,
                &self.queue,
                world,
                self.shock_radius / self.renderer.camera.zoom,
                0.45,
            ),
            ShockMode::RemoveResource => self.simulation.apply_resource_shock(
                &self.device,
                &self.queue,
                world,
                self.shock_radius / self.renderer.camera.zoom,
                -0.65,
            ),
            ShockMode::KillAgents => self.simulation.kill_agents_in_region(
                &self.device,
                &self.queue,
                world,
                self.shock_radius / self.renderer.camera.zoom,
            ),
        }
    }

    fn tick_count_for_frame(&mut self) -> u32 {
        if self.step_requested {
            self.step_requested = false;
            1
        } else if self.paused {
            0
        } else {
            [1, 2, 4, 8, 16, 32][self.speed_index]
        }
    }

    fn update_title(&self) {
        self.window.set_title(&format!(
            "Primitive World {} / physiology-v2 / checkpoint 14 | {} / {} living | {:.1} FPS",
            env!("CARGO_PKG_VERSION"),
            self.living_agents,
            MAX_AGENTS,
            self.render_fps
        ));
    }

    fn save_new_checkpoint(&mut self) -> Result<String, String> {
        let path = play_files::new_checkpoint_path(self.simulation.seed, self.simulation.tick)?;
        std::fs::create_dir_all(path.parent().expect("checkpoint folder"))
            .map_err(|e| e.to_string())?;
        self.simulation
            .save_checkpoint(&self.device, &self.queue, &path)?;
        self.checkpoint_path = path.to_string_lossy().into_owned();
        Ok(format!("Saved {}", self.checkpoint_path))
    }

    fn update_selection_highlight(&mut self) {
        (
            self.renderer.camera.selected_id,
            self.renderer.camera.selected_generation,
        ) = self.inspection.highlight();
    }

    fn refresh_inspection(&mut self) {
        if self.inspection.following
            && let Some(previous) = self.inspection.snapshot
        {
            let result =
                self.simulation
                    .refresh_selected_agent(&self.device, &self.queue, &previous);
            self.inspection.refresh(result, self.simulation.tick);
        }
        self.update_selection_highlight();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let now = Instant::now();
        let output = self.surface.get_current_texture()?;
        // Read the last completed simulation batch before building this frame's UI.
        // The label states its tick; the world advances by this frame's batch next.
        self.refresh_inspection();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let context = self.egui_context.clone();
        let full_output = context.run(raw_input, |ctx| draw_ui(ctx, self));
        let ticks = self.tick_count_for_frame();
        let submit_start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("simulation and frame encoder"),
            });
        if let Some(timing) = &self.gpu_timing
            && ticks > 0
        {
            encoder.write_timestamp(&timing.query_set, 0);
        }
        self.simulation
            .encode_ticks(&mut encoder, &self.device, &self.queue, ticks);
        if let Some(timing) = &self.gpu_timing {
            if ticks > 0 {
                encoder.write_timestamp(&timing.query_set, 1);
            }
            encoder.write_timestamp(&timing.query_set, 2);
        }
        self.renderer.update_camera(&self.queue);
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("world and agents"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.005,
                            g: 0.008,
                            b: 0.014,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.renderer.draw(&mut pass, &self.simulation);
        }
        if let Some(timing) = &self.gpu_timing {
            encoder.write_timestamp(&timing.query_set, 3);
        }

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);
        for (texture_id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *texture_id, image_delta);
        }
        let paint_jobs = context.tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &paint_jobs,
            &screen_descriptor,
        );
        {
            let pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui overlay"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            self.egui_renderer
                .render(&mut pass.forget_lifetime(), &paint_jobs, &screen_descriptor);
        }
        for texture_id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(texture_id);
        }
        let sample_alive = ticks > 0 && self.frame_count.is_multiple_of(30);
        let sample_gpu =
            self.gpu_timing.is_some() && ticks > 0 && self.frame_count.is_multiple_of(30);
        if sample_alive {
            self.simulation.copy_alive_count(&mut encoder);
            self.simulation.copy_death_stats(&mut encoder);
        }
        if let Some(timing) = &self.gpu_timing {
            encoder.resolve_query_set(&timing.query_set, 0..4, &timing.resolve_buffer, 0);
            if sample_gpu {
                encoder.copy_buffer_to_buffer(
                    &timing.resolve_buffer,
                    0,
                    &timing.readback,
                    0,
                    4 * std::mem::size_of::<u64>() as u64,
                );
            }
        }
        self.queue.submit(Some(encoder.finish()));
        self.last_submit_ms = submit_start.elapsed().as_secs_f32() * 1000.0;
        output.present();
        if sample_alive {
            if let Ok(metrics) = self.simulation.metrics(&self.device, &self.queue) {
                if self.history.len() >= 400 {
                    self.history.pop_front();
                }
                self.history.push_back(metrics);
            }
            if let Some(count) = self.simulation.read_alive_count(&self.device) {
                self.living_agents = count;
            }
            if let Some(
                [
                    food_eaten,
                    starvation_deaths,
                    age_deaths,
                    births,
                    transfers,
                    force,
                    transferred_matter,
                    force_deaths,
                    ..,
                ],
            ) = self.simulation.read_death_stats(&self.device)
            {
                self.food_eaten = food_eaten;
                self.starvation_deaths = starvation_deaths;
                self.age_deaths = age_deaths;
                self.births = births;
                self.interaction_stats = [transfers, force, transferred_matter, force_deaths];
            }
        }
        if sample_gpu
            && let Some(timing) = &self.gpu_timing
            && let Some((sim_ms, render_ms)) = timing.read(&self.device)
        {
            self.gpu_sim_ms = Some(sim_ms);
            self.gpu_render_ms = Some(render_ms);
        }
        self.frame_count += 1;
        self.ticks_window_accumulated = self.ticks_window_accumulated.saturating_add(ticks);
        if now.duration_since(self.fps_timer) >= Duration::from_secs(1) {
            let seconds = now.duration_since(self.fps_timer).as_secs_f32();
            self.render_fps = self.frame_count as f32 / seconds;
            self.ticks_last_second = (self.ticks_window_accumulated as f32 / seconds) as u32;
            self.frame_count = 0;
            self.ticks_window_accumulated = 0;
            self.fps_timer = now;
            self.update_title();
        }
        Ok(())
    }
}

trait SelectedAgentId {
    fn agent_id(&self) -> u32;
}

impl SelectedAgentId for SelectionOutput {
    fn agent_id(&self) -> u32 {
        self.selected - 1
    }
}

impl Lens {
    fn from_u32(value: u32) -> Self {
        match value {
            1 => Self::ResourceDensity,
            2 => Self::AgentDensity,
            3 => Self::Energy,
            4 => Self::Movement,
            5 => Self::Age,
            6 => Self::Digestion,
            7 => Self::CarriedFood,
            8 => Self::Action,
            9 => Self::Fertility,
            _ => Self::Normal,
        }
    }
}

fn action_name(action: u32) -> &'static str {
    model::ACTION_NAMES[action.min(6) as usize]
}
fn draw_ui(ctx: &egui::Context, state: &mut AppState) {
    let mut reset = false;
    egui::Window::new("Primitive World — physiology-v2")
        .default_pos([16.0, 16.0])
        .default_width(430.0)
        .vscroll(true)
        .show(ctx, |ui| draw_inspector(ui, state, &mut reset));
    if reset {
        state.simulation.seed = state.seed_input;
        state.simulation.reset(&state.queue);
        state.inspection = inspection::Inspection::default();
        state.history.clear();
        state.recent_events.clear();
        state.evolution_snapshot = None;
        state.renderer.camera.selected_id = u32::MAX;
        state.living_agents = state.simulation.settings.population;
        state.births = 0;
        state.starvation_deaths = 0;
        state.age_deaths = 0;
        state.food_eaten = 0;
        state.interaction_stats = [0; 4];
    }
}
fn draw_inspector(ui: &mut egui::Ui, state: &mut AppState, reset: &mut bool) {
    ui.small(format!(
        "Environment orientation: {}°",
        state.simulation.settings.environment_rotation * 90
    ));
    ui.label(format!(
        "Build {} · physiology-v2 · checkpoint 14",
        env!("CARGO_PKG_VERSION")
    ));
    ui.small(format!(
        "Loaded founder bank: {}",
        state.simulation.settings.founder_name
    ));
    ui.small("weights fixed during life; inherited at birth");
    ui.small(format!(
        "Current costs · metabolic {:.4} · movement {:.4}",
        state.simulation.settings.metabolic_cost, state.simulation.settings.movement_energy_cost
    ));
    ui.small(format!(
        "Motor response gain: {:.3}",
        state.simulation.settings.motor_response_gain
    ));
    ui.horizontal(|ui| {
        if ui
            .button(if state.paused { "Resume" } else { "Pause" })
            .clicked()
        {
            state.paused = !state.paused;
        }
        if ui.button("Step").clicked() {
            state.paused = true;
            state.step_requested = true;
        }
        for (i, label) in ["1x", "2x", "4x", "8x", "16x", "MAX"].iter().enumerate() {
            if ui
                .selectable_label(state.speed_index == i, *label)
                .clicked()
            {
                state.speed_index = i;
            }
        }
    });
    ui.label(format!(
        "Tick {} · {} living · {} births · {:.1} FPS · {} ticks/s",
        state.simulation.tick,
        state.living_agents,
        state.births,
        state.render_fps,
        state.ticks_last_second
    ));
    ui.label(format!(
        "Deaths: {} starvation / {} age",
        state.starvation_deaths, state.age_deaths
    ));
    let capacity_percent = 100.0 * state.living_agents as f32 / MAX_AGENTS as f32;
    ui.small(format!(
        "Slots: {} / {} ({capacity_percent:.1}%)",
        state.living_agents, MAX_AGENTS
    ));
    if u64::from(state.living_agents) * 100 >= u64::from(MAX_AGENTS) * 95 {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Capacity warning: slots nearly full; births need free slots.",
        );
    }
    if let Some(m) = state.history.back() {
        ui.label(format!(
            "Food: {:.1} vegetation / {:.1} dropped / {:.1} carried",
            m.vegetation, m.dropped_food, m.carried_food
        ));
        ui.small(format!(
            "Invalid outputs: {} · force: {} · signals: {}",
            m.invalid_outputs, m.events[5], m.signals
        ));
        ui.collapsing("Reproduction resolution", |ui| {
            for (name, count) in [
                "Immature attempts",
                "Insufficient energy",
                "Insufficient inventory",
                "Recovery active",
                "Requested",
                "Eligible before interactions",
                "Resolved",
            ]
            .iter()
            .zip(m.birth_gates)
            {
                ui.label(format!("{name}: {count}"));
            }
        });
    }
    ui.label("Checkpoint path to load:");
    ui.text_edit_singleline(&mut state.checkpoint_path);
    ui.small("Save creates a new file in reports/checkpoints and fills this path. Existing saves are kept.");
    ui.horizontal(|ui| {
        if ui.button("Save new checkpoint").clicked() {
            state.file_status = state.save_new_checkpoint().unwrap_or_else(|e| e);
        }
        if ui.button("Load checkpoint").clicked() {
            state.file_status = match state.simulation.load_checkpoint(
                &state.queue,
                std::path::Path::new(state.checkpoint_path.trim()),
            ) {
                Ok(()) => {
                    state.inspection = inspection::Inspection::default();
                    state.renderer.camera.selected_id = u32::MAX;
                    state.history.clear();
                    state.recent_events.clear();
                    state.evolution_snapshot = None;
                    state.paused = true;
                    state.seed_input = state.simulation.seed;
                    if let Ok(m) = state.simulation.metrics(&state.device, &state.queue) {
                        state.living_agents = m.living as u32;
                        state.births = m.events[3];
                        state.starvation_deaths = m.events[1];
                        state.age_deaths = m.events[2];
                        state.food_eaten = m.events[0];
                        state.interaction_stats = m.events[4..8].try_into().unwrap();
                        state.history.push_back(m);
                    }
                    "Loaded (paused)".into()
                }
                Err(e) => e,
            };
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Inspect evolution").clicked() {
            match state
                .simulation
                .evolution_snapshot(&state.device, &state.queue)
            {
                Ok(x) => state.evolution_snapshot = Some(x),
                Err(e) => state.file_status = e,
            }
        }
        if ui.button("Export history").clicked() {
            state.file_status = serde_json::to_vec_pretty(&state.history)
                .map_err(|e| e.to_string())
                .and_then(|b| {
                    std::fs::write("recurrent-history.json", b).map_err(|e| e.to_string())
                })
                .map(|_| "Exported recurrent-history.json".into())
                .unwrap_or_else(|e| e);
        }
    });
    ui.collapsing("Founder banks", |ui| {
        if ui.button("Export living descendants").clicked() {
            let path = std::path::PathBuf::from(format!(
                "reports/play-founders-{}-{}.json",
                state.simulation.seed, state.simulation.tick
            ));
            state.file_status = std::fs::create_dir_all("reports")
                .map_err(|e| e.to_string())
                .and_then(|()| {
                    state
                        .simulation
                        .export_founders(&state.device, &state.queue, &path)
                })
                .map(|()| {
                    format!(
                        "Exported {}; not automatically loaded or validated.",
                        path.display()
                    )
                })
                .unwrap_or_else(|e| format!("Founder export failed: {e}"));
        }
        ui.small("Export creates a new file; existing files are never overwritten.");
        ui.label("Founder bank path");
        ui.add(
            egui::TextEdit::singleline(&mut state.founder_path)
                .hint_text("Path to founder bank JSON")
                .desired_width(f32::INFINITY),
        );
        ui.small("Loading starts a new world and clears experience and history.");
        ui.small("Banks carry weights only; current physical settings stay in use.");
        if ui.button("Load bank and new world").clicked() {
            let path = state.founder_path.trim();
            if path.is_empty() {
                state.file_status = "Enter a founder bank path before loading.".into();
            } else {
                state.file_status = match state.simulation.load_founders(std::path::Path::new(path))
                {
                    Ok(()) => {
                        *reset = true;
                        format!(
                            "Loaded {}; new world clears experience and history.",
                            state.simulation.settings.founder_name
                        )
                    }
                    Err(e) => format!("Founder load failed: {e}"),
                };
            }
        }
    });
    if let Some(x) = &state.evolution_snapshot {
        ui.small(format!(
            "Lineages {} · max ancestry {} · mean ancestry {:.2}",
            x.unique_lineages, x.maximum_ancestry_depth, x.mean_ancestry_depth
        ));
    }
    ui.collapsing("Population history", |ui| {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(340.0, 80.0), egui::Sense::hover());
        let peak = state
            .history
            .iter()
            .map(|m| m.living)
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        let points: Vec<_> = state
            .history
            .iter()
            .enumerate()
            .map(|(i, m)| {
                egui::pos2(
                    rect.left() + rect.width() * i as f32 / (state.history.len().max(2) - 1) as f32,
                    rect.bottom() - rect.height() * m.living as f32 / peak,
                )
            })
            .collect();
        if points.len() > 1 {
            ui.painter().add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5, egui::Color32::LIGHT_GREEN),
            ));
        }
    });
    ui.collapsing("Physical settings", |ui| {
        ui.add(egui::DragValue::new(&mut state.seed_input).prefix("Seed "));
        ui.add(
            egui::Slider::new(&mut state.simulation.settings.population, 0..=MAX_AGENTS)
                .text("Initial bodies"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.simulation.settings.resource_regeneration,
                0.0..=0.1,
            )
            .text("Regeneration"),
        );
        ui.add(
            egui::Slider::new(&mut state.simulation.settings.metabolic_cost, 0.0..=0.2)
                .text("Metabolic cost"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.simulation.settings.movement_energy_cost,
                0.0..=0.1,
            )
            .text("Movement cost"),
        );
        ui.add(
            egui::Slider::new(
                &mut state.simulation.settings.motor_response_gain,
                0.1..=32.0,
            )
            .logarithmic(true)
            .text("Motor response gain"),
        );
        ui.small("Zero movement intent remains zero; maximum speed is unchanged.");
        ui.checkbox(
            &mut state.simulation.settings.evolving_landscape,
            "Evolving geography",
        );
        ui.checkbox(
            &mut state.simulation.settings.force_enabled,
            "Contact force available",
        );
        ui.checkbox(
            &mut state.simulation.settings.communication_enabled,
            "Local signals available",
        );
        ui.small("Reset uses the loaded founder bank shown above.");
        ui.small("New world clears experience (recurrent state) and history.");
        if ui.button("Reset / new world").clicked() {
            *reset = true;
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Next lens").clicked() {
            state.renderer.camera.lens = Lens::from_u32(state.renderer.camera.lens).next() as u32;
        }
        ui.label(Lens::from_u32(state.renderer.camera.lens).name());
    });
    ui.horizontal(|ui| {
        for (mode, label) in [
            (ShockMode::Select, "Select"),
            (ShockMode::RemoveResource, "Remove food"),
            (ShockMode::AddResource, "Add food"),
            (ShockMode::KillAgents, "Kill"),
        ] {
            if ui
                .selectable_label(state.shock_mode == mode, label)
                .clicked()
            {
                state.shock_mode = mode;
            }
        }
    });
    ui.add(egui::Slider::new(&mut state.shock_radius, 4.0..=400.0).text("Intervention radius"));
    ui.collapsing("Recent physical events", |ui| {
        if ui.button("Refresh events").clicked() {
            match state.simulation.recent_events(&state.device, &state.queue) {
                Ok(x) => state.recent_events = x,
                Err(e) => state.file_status = e,
            }
        }
        for e in state.recent_events.iter().rev().take(30) {
            ui.small(format!(
                "{}: {} {} → {} ({:.3})",
                e.tick,
                e.actor,
                action_name(e.action),
                e.other,
                e.amount
            ));
        }
    });
    if let Some(s) = state.inspection.snapshot {
        ui.separator();
        ui.small(format!(
            "Inspector snapshot: tick {} (before this frame's steps)",
            state.inspection.tick
        ));
        if !state.inspection.notice.is_empty() {
            ui.label(&state.inspection.notice);
        }
        ui.label(format!(
            "Agent {} · incarnation {} · ancestry {}",
            s.agent_id(),
            s.agent.generation,
            s.agent.ancestry_depth
        ));
        ui.label(format!(
            "Energy {:.2} · inventory {:.3} · age {:.0}",
            s.agent.energy, s.agent.food, s.agent.age
        ));
        ui.label(format!(
            "Position {:.1}, {:.1} · fixed compass sensors",
            s.agent.position[0], s.agent.position[1]
        ));
        ui.collapsing("Actual feedback (last body update)",|ui|{ui.label(format!("Collected {:.3} · ingested {:.3} · spent {:.3} · received {:.3} · displacement {:?}",s.agent.collected,s.agent.ingested,s.agent.spent,s.agent.received,s.agent.moved));});
        if state.inspection.has_decision_trace() {
            ui.small(
                "Inputs were sensed before the last step's movement; body feedback is after it.",
            );
            ui.label(format!(
                "Last body action: {} · requested movement {:.2}, {:.2}",
                action_name(s.agent.action),
                s.decision.movement[0],
                s.decision.movement[1]
            ));
            ui.small(format!(
                "Requested amount {:.3} · signal {:.3} · target {}:{}",
                s.decision.amount,
                s.decision.payload,
                s.decision.target,
                s.decision.target_generation
            ));
            ui.collapsing("Local food samples", |ui| {
                ui.label(format!("Underfoot {:.3}", s.perception.resource_here));
                for p in s.perception.samples {
                    ui.small(format!(
                        "Offset {:.2}, {:.2}: food {:.3}",
                        p.offset[0], p.offset[1], p.food
                    ));
                }
            });
            ui.collapsing("Observed bodies", |ui| {
                for b in s.perception.bodies.iter().filter(|b| b.slot < MAX_AGENTS) {
                    ui.small(format!(
                        "{}:{} offset {:?} · food {:.3} · event {:.3}",
                        b.slot, b.generation, b.offset, b.food, b.event
                    ));
                }
            });
            ui.collapsing("Action logits (not rewards)", |ui| {
                for (name, v) in model::ACTION_NAMES.iter().zip(s.decision.scores) {
                    ui.label(format!("{name}: {v:.3}"));
                }
            });
            ui.collapsing("Internal recurrent state", |ui| {
                for (i, v) in s.agent.hidden.iter().enumerate() {
                    ui.small(format!("{i}: {v:.4}"));
                }
                ui.small("Numeric state has no assigned semantic labels.");
            });
            ui.collapsing("Raw controller input vector", |ui| {
                for (i, v) in s.decision.inputs.iter().enumerate() {
                    ui.small(format!("{i}: {v:.4}"));
                }
            });
        } else if s.agent.alive == 0 {
            ui.small("Terminal body data only: decision/perception buffers may have been cleared after death.");
        } else {
            ui.small("No controller decision yet for this body; first update pending.");
        }
    }
    if !state.file_status.is_empty() {
        ui.small(&state.file_status);
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_title(format!(
                            "Primitive World {} — physiology-v2 / checkpoint 14",
                            env!("CARGO_PKG_VERSION")
                        ))
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 820.0)),
                )
                .expect("window creation failed"),
        );
        let state = pollster::block_on(AppState::new(window.clone()));
        self.window = Some(window);
        self.state = Some(state);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let Some(state) = self.state.as_mut() else {
            return;
        };
        let egui_response = state.egui_state.on_window_event(&state.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                state.resize(size.width, size.height);
            }
            WindowEvent::CursorMoved { position, .. } => state.cursor_position = position,
            WindowEvent::MouseWheel { delta, .. } if !egui_response.consumed => {
                let amount = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(position) => position.y as f32 / 80.0,
                };
                state.renderer.camera.zoom =
                    (state.renderer.camera.zoom * (1.0 + amount * 0.12)).clamp(0.15, 80.0);
            }
            WindowEvent::MouseInput {
                state: ElementState::Released,
                button: MouseButton::Left,
                ..
            } => {
                if !egui_response.consumed {
                    state.handle_click();
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: ElementState::Pressed,
                        repeat: false,
                        ..
                    },
                ..
            } if !egui_response.consumed => {
                let pan = 80.0 / state.renderer.camera.zoom;
                match code {
                    KeyCode::Space => state.paused = !state.paused,
                    KeyCode::KeyL => {
                        state.renderer.camera.lens =
                            Lens::from_u32(state.renderer.camera.lens).next() as u32
                    }
                    KeyCode::Home => {
                        state.renderer.camera.center = [WORLD_SIZE * 0.5, WORLD_SIZE * 0.5];
                        state.renderer.camera.zoom = 1.0;
                    }
                    KeyCode::ArrowUp | KeyCode::KeyW => state.renderer.camera.center[1] -= pan,
                    KeyCode::ArrowDown | KeyCode::KeyS => state.renderer.camera.center[1] += pan,
                    KeyCode::ArrowLeft | KeyCode::KeyA => state.renderer.camera.center[0] -= pan,
                    KeyCode::ArrowRight | KeyCode::KeyD => state.renderer.camera.center[0] += pan,
                    KeyCode::Digit1 => state.speed_index = 0,
                    KeyCode::Digit2 => state.speed_index = 1,
                    KeyCode::Digit4 => state.speed_index = 2,
                    KeyCode::Digit8 => state.speed_index = 3,
                    KeyCode::Digit6 => state.speed_index = 4,
                    KeyCode::KeyM => state.speed_index = 5,
                    _ => {}
                }
            }
            WindowEvent::RedrawRequested => match state.render() {
                Ok(()) => {}
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    state.resize(state.config.width, state.config.height)
                }
                Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                Err(wgpu::SurfaceError::Timeout) => {}
                Err(wgpu::SurfaceError::Other) => {}
            },
            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn main() {
    let options: Vec<_> = std::env::args().collect();
    if options.iter().any(|x| x == "--help") {
        println!("{}", headless::HELP);
        return;
    }
    if options.iter().any(|x| x == "--version") {
        println!(
            "Primitive World {} / physiology-v2 / checkpoint 14",
            env!("CARGO_PKG_VERSION")
        );
        return;
    }
    if let Err(e) = headless::arguments(&options) {
        eprintln!("{e}");
        std::process::exit(2);
    }
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless") {
        if let Err(error) = headless::run(&args) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return;
    }
    let event_loop = EventLoop::new().expect("event loop creation failed");
    event_loop
        .run_app(&mut App {
            window: None,
            state: None,
        })
        .expect("event loop failed");
}
