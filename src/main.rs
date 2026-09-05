mod controls;
mod environment;
mod experiments;
mod family_observer;
mod founders;
mod headless;
mod inspection;
mod journey_observer;
mod model;
mod play_files;
mod renderer;
mod session;
mod simulation;
mod survivor_observer;
mod travel_observer;
mod ui;
mod ui_details;
mod visible_trial;

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
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    state: Option<AppState>,
}

struct AppState {
    ui: ui::UiState,
    experiment: Option<experiments::Experiment>,
    world_revision: u64,
    saved_revision: Option<u64>,
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
    checkpoint_path: String,
    recent_events: Vec<simulation::observability::InteractionEvent>,
    evolution_snapshot: Option<simulation::observability::EvolutionSnapshot>,
    inspection: inspection::Inspection,
    seed_input: u32,
    shock_mode: ShockMode,
    shock_radius: f32,
    last_submit_ms: f32,
    gpu_sim_ms: Option<f32>,
    gpu_render_ms: Option<f32>,
    gpu_timing: Option<GpuTiming>,
    visible_trial: Option<visible_trial::VisibleTrial>,
    last_autosave: Instant,
    autosaved_state: Option<(u32, u32)>,
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

impl ShockMode {
    fn name(self) -> &'static str {
        match self {
            Self::Select => "Inspect",
            Self::AddResource => "Add food",
            Self::RemoveResource => "Remove food",
            Self::KillAgents => "Remove agents",
        }
    }
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
        if let Some(i) = args.iter().position(|a| a == "--checkpoint") {
            simulation
                .load_checkpoint(&queue, std::path::Path::new(&args[i + 1]))
                .expect("Invalid checkpoint");
        } else if args.len() > 1 {
            simulation.reset(&queue);
        }
        let visible_trial = args.iter().position(|a| a == "--watch-loop").map(|i| {
            visible_trial::VisibleTrial::new_loop(
                std::path::Path::new(&args[i + 1]),
                &simulation,
                &device,
                &queue,
            )
            .expect("Could not initialize native survivor loop")
        });
        let speed_index = args
            .iter()
            .position(|a| a == "--view-speed")
            .map(|i| {
                ["1x", "2x", "4x", "8x", "16x", "MAX"]
                    .iter()
                    .position(|s| *s == args[i + 1])
                    .expect("validated speed")
            })
            .unwrap_or(0);
        let renderer = Renderer::new(
            &device,
            config.format,
            &simulation,
            config.width,
            config.height,
        );
        let egui_context = egui::Context::default();
        ui::style(&egui_context);
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

        let mut state = Self {
            ui: ui::UiState::new(args.len() > 1),
            experiment: None,
            world_revision: 0,
            saved_revision: None,
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
            paused: args.len() == 1,
            step_requested: false,
            speed_index,
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
            checkpoint_path: "world.checkpoint".into(),
            recent_events: Vec::new(),
            evolution_snapshot: None,
            inspection: inspection::Inspection::default(),
            seed_input: initial_seed,
            shock_mode: ShockMode::Select,
            shock_radius: 45.0,
            last_submit_ms: 0.0,
            gpu_sim_ms: None,
            gpu_render_ms: None,
            gpu_timing,
            visible_trial,
            last_autosave: Instant::now(),
            autosaved_state: None,
        };
        state.refresh_saves();
        if state.ui.has_world
            && let Err(error) = state.refresh_metrics()
        {
            state.file_status = error;
        }
        state
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

    fn handle_click(&mut self, point: egui::Pos2) {
        if self.shock_mode != ShockMode::Select {
            self.world_revision = self.world_revision.saturating_add(1);
        }
        let world = controls::world_position(
            self.ui.world_rect,
            self.renderer.camera.center,
            self.renderer.camera.zoom,
            point,
        );
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
                self.ui.tab = ui::Tab::Agent;
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
        if self.ui.screen != ui::Screen::Play {
            self.step_requested = false;
            return 0;
        }
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
        if self.ui.screen != ui::Screen::Play {
            self.window.set_title(&format!(
                "Primitive World {} | Main menu",
                env!("CARGO_PKG_VERSION")
            ));
            return;
        }
        self.window.set_title(&format!(
            "Primitive World {} | {} / {} living | {:.1} FPS | World {} | {}{}",
            env!("CARGO_PKG_VERSION"),
            self.living_agents,
            MAX_AGENTS,
            self.render_fps,
            self.visible_trial
                .as_ref()
                .map_or(1, |trial| trial.world_number),
            ["1x", "2x", "4x", "8x", "16x", "MAX"][self.speed_index],
            if self.paused { " | PAUSED" } else { "" }
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
        if self.ui.screen == ui::Screen::Play {
            self.refresh_inspection();
        }
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let context = self.egui_context.clone();
        let mut command = controls::Command::None;
        let full_output = context.run(raw_input, |ctx| {
            command = ui::draw(ctx, self);
        });
        let mut ticks = self.tick_count_for_frame();
        if self.visible_trial.is_some() {
            ticks = ticks.min(128 - self.simulation.tick % 128);
        }
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
        let rect = self.ui.world_rect;
        self.renderer.camera.aspect = rect.width().max(1.0) / rect.height().max(1.0);
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
            if self.ui.screen == ui::Screen::Play && rect.is_positive() {
                let scale = full_output.pixels_per_point;
                let x =
                    (rect.left() * scale).clamp(0.0, self.config.width.saturating_sub(1) as f32);
                let y =
                    (rect.top() * scale).clamp(0.0, self.config.height.saturating_sub(1) as f32);
                let width = (rect.width() * scale)
                    .min(self.config.width as f32 - x)
                    .max(1.0);
                let height = (rect.height() * scale)
                    .min(self.config.height as f32 - y)
                    .max(1.0);
                pass.set_viewport(x, y, width, height, 0.0, 1.0);
                pass.set_scissor_rect(x as u32, y as u32, width as u32, height as u32);
                self.renderer.draw(&mut pass, &self.simulation);
            }
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
        let watch_alive = ticks > 0 && self.visible_trial.is_some();
        let sample_gpu =
            self.gpu_timing.is_some() && ticks > 0 && self.frame_count.is_multiple_of(30);
        if sample_alive || watch_alive {
            self.simulation.copy_alive_count(&mut encoder);
        }
        if sample_alive {
            self.simulation.copy_death_stats(&mut encoder);
        }
        // A paused startup has never written simulation queries 0 and 1.
        // Resolving them asks Vulkan to wait forever for unavailable results.
        // Resolve only a sampled frame that actually wrote all four queries.
        if let Some(timing) = &self.gpu_timing
            && sample_gpu
        {
            encoder.resolve_query_set(&timing.query_set, 0..4, &timing.resolve_buffer, 0);
            encoder.copy_buffer_to_buffer(
                &timing.resolve_buffer,
                0,
                &timing.readback,
                0,
                4 * std::mem::size_of::<u64>() as u64,
            );
        }
        self.queue.submit(Some(encoder.finish()));
        self.last_submit_ms = submit_start.elapsed().as_secs_f32() * 1000.0;
        self.window.pre_present_notify();
        output.present();
        if ticks > 0 {
            self.world_revision = self.world_revision.saturating_add(1);
        }
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
        if watch_alive {
            let result = (|| -> Result<(), String> {
                let count = self
                    .simulation
                    .read_alive_count(&self.device)
                    .ok_or("Could not verify living population; visible loop paused")?;
                self.living_agents = count;
                let trial = self.visible_trial.as_mut().expect("visible trial");
                if count == 0 {
                    if trial.is_loop() {
                        let cohort = trial.transfer_cohort_size().unwrap_or(0);
                        trial.advance(&mut self.simulation, &self.device, &self.queue)?;
                        self.clear_world_observers();
                        self.file_status = format!(
                            "World ended; continued automatically from {cohort} archived survivor(s)."
                        );
                    } else {
                        trial.finish(&self.simulation, &self.device, &self.queue, false)?;
                    }
                } else if count <= 64 || self.simulation.tick.is_multiple_of(128) {
                    trial.observe(&self.simulation, &self.device, &self.queue)?;
                }
                Ok(())
            })();
            if let Err(error) = result {
                self.paused = true;
                self.file_status = format!("Visible loop stopped safely: {error}");
                eprintln!("{}", self.file_status);
            }
        }
        if self.experiment.is_none()
            && let Some(trial) = &self.visible_trial
            && trial.is_loop()
            && (self.autosaved_state.is_none()
                || self.last_autosave.elapsed() >= Duration::from_secs(300))
        {
            let current = (self.simulation.seed, self.simulation.tick);
            if self.autosaved_state != Some(current) {
                match trial.autosave(&self.simulation, &self.device, &self.queue) {
                    Ok(path) => {
                        self.checkpoint_path = path.to_string_lossy().into_owned();
                        self.file_status = format!("Autosaved {}", self.checkpoint_path);
                        self.autosaved_state = Some(current);
                    }
                    Err(error) => {
                        self.paused = true;
                        // Avoid retrying a failed disk write on every frame.
                        self.autosaved_state = Some(current);
                        self.file_status =
                            format!("Autosave failed; paused to protect progress: {error}");
                        eprintln!("{}", self.file_status);
                    }
                }
            }
            self.last_autosave = Instant::now();
        }
        if let Some(experiment) = &mut self.experiment {
            experiment.total_ticks = experiment.total_ticks.saturating_add(ticks as u64);
        }
        if self.experiment.is_some()
            && self.ui.screen == ui::Screen::Play
            && self.last_autosave.elapsed() >= Duration::from_secs(300)
        {
            self.file_status = match self.save_experiment() {
                Ok(message) => message,
                Err(error) => {
                    self.paused = true;
                    format!("Autosave failed; paused: {error}")
                }
            };
            self.last_autosave = Instant::now();
        }
        if sample_alive
            && self.experiment.is_some()
            && self.visible_trial.is_none()
            && self.living_agents == 0
        {
            self.paused = true;
            self.file_status =
                "This world has ended. Open the menu to start another evolutionary line.".into();
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
        controls::apply(self, command);
        Ok(())
    }

    fn clear_world_observers(&mut self) {
        // World-local identities and counters expire, presentation/session state does not.
        self.inspection = inspection::Inspection::default();
        self.history.clear();
        self.recent_events.clear();
        self.evolution_snapshot = None;
        self.renderer.camera.selected_id = u32::MAX;
        self.renderer.camera.selected_generation = 0;
        self.seed_input = self.simulation.seed;
        self.living_agents = self.simulation.settings.population;
        self.births = 0;
        self.starvation_deaths = 0;
        self.age_deaths = 0;
        self.food_eaten = 0;
        self.interaction_stats = [0; 4];
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
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = Arc::new(
            event_loop
                .create_window(
                    WindowAttributes::default()
                        .with_visible(false)
                        .with_title(format!("Primitive World {}", env!("CARGO_PKG_VERSION")))
                        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 820.0))
                        .with_min_inner_size(winit::dpi::LogicalSize::new(900.0, 620.0)),
                )
                .expect("window creation failed"),
        );
        let state = pollster::block_on(AppState::new(window.clone()));
        window.set_visible(true);
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
            WindowEvent::CloseRequested => {
                if state.experiment.is_some() {
                    if let Err(error) = state.save_experiment() {
                        state.paused = true;
                        state.file_status =
                            format!("Save failed; window kept open so you can retry: {error}");
                        return;
                    }
                } else if let Some(trial) = &mut state.visible_trial
                    && let Err(error) =
                        trial.finish(&state.simulation, &state.device, &state.queue, true)
                {
                    eprintln!(
                        "Could not finish visible-world save: {error}. Existing artifacts retained."
                    );
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                state.resize(size.width, size.height);
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
            } if !egui_response.consumed && state.ui.screen == ui::Screen::Play => {
                let pan = 80.0 / state.renderer.camera.zoom;
                match code {
                    KeyCode::Escape => {
                        if state.shock_mode == ShockMode::Select {
                            state.open_menu();
                        } else {
                            state.shock_mode = ShockMode::Select;
                        }
                    }
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
                Ok(()) => {
                    if state
                        .visible_trial
                        .as_ref()
                        .is_some_and(|trial| trial.finished && !trial.is_loop())
                    {
                        event_loop.exit();
                    }
                }
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
        println!("Primitive World {}", env!("CARGO_PKG_VERSION"));
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
