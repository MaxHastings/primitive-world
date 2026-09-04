mod renderer;
mod simulation;

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
    cursor_position: PhysicalPosition<f64>,
    selected: Option<SelectionOutput>,
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

        let simulation = Simulation::new(&device, &queue, 1);
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
            cursor_position: PhysicalPosition::new(0.0, 0.0),
            selected: None,
            seed_input: 1,
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
                self.selected = self.simulation.select_agent(
                    &self.device,
                    &self.queue,
                    world,
                    14.0 / self.renderer.camera.zoom,
                );
                self.renderer.camera.selected_id = self
                    .selected
                    .as_ref()
                    .map(|selected| selected.agent_id())
                    .unwrap_or(u32::MAX);
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
        let lens = Lens::from_u32(self.renderer.camera.lens).name();
        self.window.set_title(&format!(
            "Primitive World  |  {:>6} / {} living  |  {:>5.1} FPS  |  {}",
            self.living_agents, MAX_AGENTS, self.render_fps, lens
        ));
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let now = Instant::now();
        let ticks = self.tick_count_for_frame();
        let submit_start = Instant::now();
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("simulation and frame encoder"),
            });
        if let Some(timing) = &self.gpu_timing {
            if ticks > 0 {
                encoder.write_timestamp(&timing.query_set, 0);
            }
        }
        self.simulation
            .encode_ticks(&mut encoder, &self.queue, ticks);
        if let Some(timing) = &self.gpu_timing {
            if ticks > 0 {
                encoder.write_timestamp(&timing.query_set, 1);
            }
            encoder.write_timestamp(&timing.query_set, 2);
        }
        self.renderer.update_camera(&self.queue);
        let output = self.surface.get_current_texture()?;
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

        let raw_input = self.egui_state.take_egui_input(&self.window);
        let context = self.egui_context.clone();
        let full_output = context.run(raw_input, |ctx| draw_ui(ctx, self));
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
        let sample_alive = ticks > 0 && self.frame_count % 30 == 0;
        let sample_gpu = self.gpu_timing.is_some() && ticks > 0 && self.frame_count % 30 == 0;
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
            if let Some(count) = self.simulation.read_alive_count(&self.device) {
                self.living_agents = count;
            }
            if let Some([food_eaten, starvation_deaths, age_deaths, births]) =
                self.simulation.read_death_stats(&self.device)
            {
                self.food_eaten = food_eaten;
                self.starvation_deaths = starvation_deaths;
                self.age_deaths = age_deaths;
                self.births = births;
            }
        }
        if sample_gpu {
            if let Some(timing) = &self.gpu_timing {
                if let Some((sim_ms, render_ms)) = timing.read(&self.device) {
                    self.gpu_sim_ms = Some(sim_ms);
                    self.gpu_render_ms = Some(render_ms);
                }
            }
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
            6 => Self::Gradient,
            _ => Self::Normal,
        }
    }
}

fn draw_ui(ctx: &egui::Context, state: &mut AppState) {
    let mut reset = false;
    let mut population_changed = false;
    let mut parameters_changed = false;
    egui::Window::new("Primitive World")
        .default_pos([16.0, 16.0])
        .resizable(true)
        .show(ctx, |ui| {
            ui.label("GPU-first local-rule experiment");
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(if state.paused { "Resume" } else { "Pause" })
                    .clicked()
                {
                    state.paused = !state.paused;
                }
                for (index, label) in ["1x", "2x", "4x", "8x", "16x", "MAX"].iter().enumerate() {
                    if ui
                        .selectable_label(state.speed_index == index, *label)
                        .clicked()
                    {
                        state.speed_index = index;
                    }
                }
                if ui.button("Step").clicked() {
                    state.paused = true;
                    state.speed_index = 0;
                    state.step_requested = true;
                }
            });
            ui.label(format!(
                "Tick: {}    render: {:.1} FPS    simulation dispatch: {} ticks/s",
                state.simulation.tick, state.render_fps, state.ticks_last_second
            ));
            ui.label(format!(
                "Living agents: {} / {}    CPU submit path: {:.2} ms",
                state.living_agents, MAX_AGENTS, state.last_submit_ms
            ));
            ui.label(format!(
                "Food eaten: {} units    births: {}    deaths: {} starvation / {} old age",
                state.food_eaten, state.births, state.starvation_deaths, state.age_deaths
            ));
            if let (Some(sim_ms), Some(render_ms)) = (state.gpu_sim_ms, state.gpu_render_ms) {
                ui.label(format!(
                    "GPU simulation: {:.3} ms    GPU world render: {:.3} ms",
                    sim_ms, render_ms
                ));
            } else {
                ui.label("GPU timestamps: unavailable on this adapter");
            }
            ui.separator();
            ui.label("Primitive parameters");
            if ui
                .add(
                    egui::Slider::new(
                        &mut state.simulation.settings.population,
                        1_000..=MAX_AGENTS,
                    )
                    .text("initial population"),
                )
                .changed()
            {
                population_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(
                        &mut state.simulation.settings.resource_regeneration,
                        0.0..=2.0,
                    )
                    .text("resource regeneration"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(
                        &mut state.simulation.settings.movement_energy_cost,
                        0.0..=0.04,
                    )
                    .text("movement cost"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.metabolic_cost, 0.0..=0.06)
                        .text("metabolic cost"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.sensor_radius, 2.0..=40.0)
                        .text("sensory radius"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.exploration_noise, 0.0..=2.0)
                        .text("exploration noise"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.consume_amount, 1.0..=100.0)
                        .text("consume amount"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(
                        &mut state.simulation.settings.reproduction_threshold,
                        40.0..=100.0,
                    )
                    .text("reproduction threshold"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.reproduction_cost, 5.0..=50.0)
                        .text("reproduction cost"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.maturity_age, 50.0..=1_500.0)
                        .text("maturity age"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            if ui
                .add(
                    egui::Slider::new(&mut state.simulation.settings.heterogeneity, 0.0..=1.0)
                        .text("resource heterogeneity"),
                )
                .changed()
            {
                parameters_changed = true;
            }
            ui.horizontal(|ui| {
                ui.label("seed");
                ui.add(egui::DragValue::new(&mut state.seed_input).range(1..=u32::MAX));
                if ui.button("Reset world").clicked() {
                    state.simulation.seed = state.seed_input;
                    reset = true;
                }
            });
            if population_changed {
                reset = true;
            } else if parameters_changed {
                state.simulation.update_params(&state.queue);
            }
            if reset {
                state.simulation.reset(&state.queue);
                state.living_agents = state.simulation.settings.population;
                state.food_eaten = 0;
                state.starvation_deaths = 0;
                state.age_deaths = 0;
                state.births = 0;
                state.selected = None;
                state.renderer.camera.selected_id = u32::MAX;
            }
            ui.separator();
            ui.label("Raw visualization lens");
            egui::ComboBox::from_id_salt("lens")
                .selected_text(Lens::from_u32(state.renderer.camera.lens).name())
                .show_ui(ui, |ui| {
                    for lens in [
                        Lens::Normal,
                        Lens::ResourceDensity,
                        Lens::AgentDensity,
                        Lens::Energy,
                        Lens::Movement,
                        Lens::Age,
                        Lens::Gradient,
                    ] {
                        ui.selectable_value(
                            &mut state.renderer.camera.lens,
                            lens as u32,
                            lens.name(),
                        );
                    }
                });
            match Lens::from_u32(state.renderer.camera.lens) {
                Lens::Normal => ui.label("Background: regenerating local resource field"),
                Lens::ResourceDensity => {
                    ui.label("Heatmap: low resource blue → high resource yellow")
                }
                Lens::AgentDensity => ui.label("Heatmap: low occupancy blue → high occupancy red"),
                Lens::Energy => ui.label("Particles: low energy red → high energy green"),
                Lens::Movement => ui.label("Particles: low movement blue → high movement yellow"),
                Lens::Age => ui.label("Particles: younger cyan → older magenta"),
                Lens::Gradient => ui.label("Particles: local resource-gradient magnitude"),
            };
            ui.separator();
            ui.label("World intervention (click the world)");
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut state.shock_mode, ShockMode::Select, "select");
                ui.selectable_value(&mut state.shock_mode, ShockMode::AddResource, "+ resource");
                ui.selectable_value(
                    &mut state.shock_mode,
                    ShockMode::RemoveResource,
                    "- resource",
                );
                ui.selectable_value(&mut state.shock_mode, ShockMode::KillAgents, "kill agents");
            });
            ui.add(egui::Slider::new(&mut state.shock_radius, 10.0..=160.0).text("brush radius"));
            ui.label("Pan: WASD / arrows    Zoom: wheel    Lens: L    Camera reset: Home");
            if let Some(selected) = state.selected {
                ui.separator();
                ui.label(format!("Agent {}", selected.agent_id()));
                ui.label(format!(
                    "Age: {:.0} ticks    Energy: {:.2}",
                    selected.agent.age, selected.agent.energy
                ));
                ui.label(format!(
                    "Position: {:.2}, {:.2}",
                    selected.agent.position[0], selected.agent.position[1]
                ));
                ui.label(format!(
                    "Sensory radius: {:.2}    max movement: {:.2}",
                    selected.agent.sensor_radius, selected.agent.max_speed
                ));
                ui.label(format!(
                    "Exploration: {:.3}    attraction: {:.3}",
                    selected.agent.exploration, selected.agent.resource_attraction
                ));
                ui.label(format!(
                    "Persistence: {:.3}    risk: {:.3}",
                    selected.agent.persistence, selected.agent.risk
                ));
                ui.collapsing("Current perception", |ui| {
                    ui.label(format!(
                        "here {:.3}  north {:.3}  east {:.3}",
                        selected.perception.resource_here,
                        selected.perception.resource_north,
                        selected.perception.resource_east
                    ));
                    ui.label(format!(
                        "south {:.3}  west {:.3}  density {:.3}",
                        selected.perception.resource_south,
                        selected.perception.resource_west,
                        selected.perception.local_density
                    ));
                    ui.label(format!(
                        "gradient {:.3}, {:.3}",
                        selected.perception.gradient[0], selected.perception.gradient[1]
                    ));
                });
                ui.collapsing("Candidate action scores", |ui| {
                    for (name, score) in ["remain", "north", "east", "south", "west"]
                        .into_iter()
                        .zip(selected.decision.scores)
                    {
                        ui.label(format!("{name:>6}  {score:>8.3}"));
                    }
                    ui.label(format!(
                        "selected: {}",
                        ["remain", "north", "east", "south", "west"]
                            [selected.decision.selected_action.min(4) as usize]
                    ));
                });
            } else {
                ui.label(
                    "Click an agent to inspect actual state, local perception, and action scores.",
                );
            }
        });
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
                        .with_title("Primitive World")
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
        if egui_response.consumed { /* UI gets first chance to consume text and sliders. */ }
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::ScaleFactorChanged { .. } => {
                let size = state.window.inner_size();
                state.resize(size.width, size.height);
            }
            WindowEvent::CursorMoved { position, .. } => state.cursor_position = position,
            WindowEvent::MouseWheel { delta, .. } => {
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
            } => {
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
    let event_loop = EventLoop::new().expect("event loop creation failed");
    event_loop
        .run_app(&mut App {
            window: None,
            state: None,
        })
        .expect("event loop failed");
}
