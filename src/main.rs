#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]

mod brain;
mod controls;
mod environment;
mod evolution;
mod experiments;
mod family_observer;
mod founders;
mod headless;
mod inspection;
mod journey_observer;
mod model;
mod play_files;
mod playback;
mod renderer;
mod session;
mod simulation;
mod survivor_observer;
mod travel_observer;
mod ui;
mod ui_details;
#[cfg(windows)]
mod windows_platform;

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use egui_wgpu::ScreenDescriptor;
use renderer::{Lens, Renderer};
use simulation::{MAX_AGENTS, SelectionOutput, Simulation, WORLD_SIZE};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, KeyEvent, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, ModifiersState, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

struct App {
    window: Option<Arc<Window>>,
    state: Option<AppState>,
}

struct AppState {
    wallpaper: bool,
    wallpaper_size: Option<[f32; 2]>,
    assisted: bool,
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
    food_eaten: u64,
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
    scheduler: playback::Scheduler,
    render_hz: u32,
    compute_budget: f32,
    next_frame: Instant,
    last_metrics: Instant,
    last_inspection: Instant,
    occluded: bool,
    batch_readback: wgpu::Buffer,
    pending_batch: Option<playback::PendingBatch>,
    gpu_tick_ms: Option<f32>,
    gpu_timing: Option<GpuTiming>,
    last_autosave: Instant,
    modifiers: ModifiersState,
    last_desktop_check: Instant,
    #[cfg(windows)]
    tray: Option<windows_platform::Tray>,
}

struct GpuTiming {
    query_set: wgpu::QuerySet,
    resolve_buffer: wgpu::Buffer,
    timestamp_period_ns: f32,
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
                        label: Some("simulation batch timestamps"),
                        ty: wgpu::QueryType::Timestamp,
                        count: 2,
                    }),
                    resolve_buffer: device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("timestamp resolve buffer"),
                        size: 4 * std::mem::size_of::<u64>() as u64,
                        usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                        mapped_at_creation: false,
                    }),
                    timestamp_period_ns,
                })
            } else {
                None
            };

        let args = launch_arguments();
        let wallpaper = args.iter().any(|a| a == "--wallpaper");
        let wallpaper_size = wallpaper.then(|| {
            let size = window.inner_size();
            [size.width as f32, size.height as f32]
        });
        let mut simulation = Simulation::new(&device, &queue, 1);
        headless::configure(&mut simulation, &args).expect("Invalid command line");
        if let Some([width, height]) = wallpaper_size {
            simulation.settings.habitat_width = width;
            simulation.settings.habitat_height = height;
            if !args.iter().any(|arg| arg == "--load-game")
                && !args.iter().any(|arg| arg == "--population")
            {
                let area_ratio =
                    f64::from(width) * f64::from(height) / f64::from(WORLD_SIZE * WORLD_SIZE);
                simulation.settings.population =
                    (f64::from(simulation.settings.population) * area_ratio)
                        .round()
                        .clamp(1.0, f64::from(MAX_AGENTS)) as u32;
            }
            simulation
                .settings
                .validate()
                .expect("monitor habitat dimensions must be valid");
        }
        let command_line_world = args.iter().skip(1).any(|arg| arg != "--viewer");
        if command_line_world {
            simulation.reset(&queue);
        }
        let speed_index = args
            .iter()
            .position(|a| a == "--view-speed")
            .map(|i| {
                playback::SPEED_LABELS
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

        let batch_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("asynchronous batch telemetry"),
            size: playback::READBACK_SIZE,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut state = Self {
            wallpaper,
            wallpaper_size,
            assisted: false,
            ui: ui::UiState::new(command_line_world),
            experiment: None,
            world_revision: 0,
            saved_revision: None,
            window: window.clone(),
            surface,
            device,
            queue,
            config,
            simulation,
            renderer,
            egui_context,
            egui_state,
            egui_renderer,
            paused: !command_line_world,
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
            scheduler: playback::Scheduler::new(Instant::now()),
            render_hz: args
                .iter()
                .position(|a| a == "--view-fps")
                .map(|i| args[i + 1].parse().expect("validated FPS"))
                .unwrap_or_else(|| {
                    if wallpaper {
                        window
                            .current_monitor()
                            .and_then(|monitor| monitor.refresh_rate_millihertz())
                            .map(|hz| (hz / 1000).clamp(30, 240))
                            .unwrap_or(60)
                    } else {
                        30
                    }
                }),
            compute_budget: args
                .iter()
                .position(|a| a == "--compute-budget")
                .map(|i| args[i + 1].parse::<f32>().expect("validated budget") / 100.0)
                .unwrap_or(1.0),
            next_frame: Instant::now(),
            last_metrics: Instant::now(),
            last_inspection: Instant::now(),
            occluded: false,
            batch_readback,
            pending_batch: None,
            gpu_tick_ms: None,
            gpu_timing,
            last_autosave: Instant::now(),
            modifiers: ModifiersState::default(),
            last_desktop_check: Instant::now(),
            #[cfg(windows)]
            tray: if wallpaper {
                match windows_platform::Tray::new() {
                    Ok(tray) => Some(tray),
                    Err(error) => {
                        eprintln!("Wallpaper tray unavailable: {error}");
                        None
                    }
                }
            } else {
                None
            },
        };
        state.refresh_saves();
        if state.ui.has_world
            && let Err(error) = state.start_command_line_world(&args)
        {
            state.ui.has_world = false;
            state.ui.screen = ui::Screen::Home;
            state.paused = true;
            state.file_status = error;
        }
        state.sync_world_view();
        state
    }

    fn sync_world_view(&mut self) {
        self.assisted = self.simulation.assisted;
        self.renderer.set_world(
            self.simulation.settings.habitat_width,
            self.simulation.settings.habitat_height,
        );
        if self.wallpaper {
            self.renderer.camera.zoom = 1.0;
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

    fn handle_click(&mut self, point: egui::Pos2) {
        let world = controls::world_position(
            self.ui.world_rect,
            self.renderer.camera.center,
            self.renderer.camera.zoom,
            point,
            [
                self.simulation.settings.habitat_width,
                self.simulation.settings.habitat_height,
            ],
        );
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

    fn handle_food_click(&mut self, point: egui::Pos2) {
        self.ui.food_brush.end();
        self.paint_food(point);
    }

    fn paint_food(&mut self, point: egui::Pos2) {
        let controls = &self.ui.wallpaper_controls;
        if !self.ui.has_world
            || !self.ui.food_brush.enabled
            || !self.ui.world_rect.contains(point)
            || controls.menu.is_some()
            || controls.hud_rect.contains(point)
        {
            self.ui.food_brush.end();
            return;
        }
        let world = controls::world_position(
            self.ui.world_rect,
            self.renderer.camera.center,
            self.renderer.camera.zoom,
            point,
            [
                self.simulation.settings.habitat_width,
                self.simulation.settings.habitat_height,
            ],
        );
        let stamps = self.ui.food_brush.stamps(egui::pos2(world[0], world[1]));
        if stamps.is_empty() {
            return;
        }
        for stamp in stamps {
            self.simulation.paint_food(
                &self.device,
                &self.queue,
                [stamp.x, stamp.y],
                self.ui.food_brush.radius,
                2.0 * self.ui.food_brush.density,
            );
        }
        self.assisted = true;
        self.world_revision = self.world_revision.saturating_add(1);
        self.file_status = format!("Food patch added at {:.0}, {:.0}", world[0], world[1]);
    }

    #[cfg(windows)]
    fn handle_wallpaper_desktop_click(&mut self, x: i32, y: i32, can_paint: bool) {
        self.ui.food_brush.end();
        let point = egui::pos2(
            x as f32 / self.egui_context.pixels_per_point(),
            y as f32 / self.egui_context.pixels_per_point(),
        );
        let controls = &mut self.ui.wallpaper_controls;
        controls.brush_sizing = None;
        if controls.paint_button.contains(point) {
            self.ui.food_brush.toggle();
            return;
        }
        if self.ui.food_brush.enabled && controls.brush_size_rect.contains(point) {
            controls.brush_sizing = Some(ui::BrushAdjustment::Size);
            self.ui.food_brush.slider(point.x, controls.brush_size_rect);
            return;
        }
        if self.ui.food_brush.enabled && controls.brush_density_rect.contains(point) {
            controls.brush_sizing = Some(ui::BrushAdjustment::Density);
            self.ui
                .food_brush
                .density_slider(point.x, controls.brush_density_rect);
            return;
        }
        for (rect, menu) in [
            (controls.lens_button, ui::WallpaperMenu::View),
            (controls.speed_button, ui::WallpaperMenu::Speed),
            (controls.details_button, ui::WallpaperMenu::Details),
        ] {
            if rect.contains(point) {
                controls.toggle(menu);
                return;
            }
        }
        if let Some(menu) = controls.menu {
            match menu {
                ui::WallpaperMenu::View => {
                    if let Some(lens) = controls
                        .lens_options
                        .iter()
                        .position(|rect| rect.contains(point))
                    {
                        self.renderer.camera.lens = lens as u32;
                        controls.menu = None;
                    }
                }
                ui::WallpaperMenu::Speed => {
                    if let Some(speed) = controls
                        .speed_buttons
                        .iter()
                        .position(|rect| rect.contains(point))
                    {
                        self.speed_index = speed;
                        controls.menu = None;
                    }
                }
                ui::WallpaperMenu::Details => {}
            }
            if !controls.popup_rect.contains(point) {
                controls.menu = None;
            }
            // An outside click dismisses the menu without adding food.
            return;
        }
        if controls.hud_rect.contains(point) {
            return;
        }
        if can_paint && self.ui.world_rect.contains(point) {
            self.handle_food_click(point);
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
            "Primitive World {} | {} / {} living | {:.1} FPS | World {} | {} target | {:.1}x actual{}",
            env!("CARGO_PKG_VERSION"),
            self.living_agents,
            MAX_AGENTS,
            self.render_fps,
            self.simulation.progress.world,
            playback::SPEED_LABELS[self.speed_index],
            self.ticks_last_second as f32 / playback::BASE_TPS as f32,
            if self.paused { " | PAUSED" } else { "" }
        ));
    }

    fn update_selection_highlight(&mut self) {
        (
            self.renderer.camera.selected_id,
            self.renderer.camera.selected_generation,
        ) = self.inspection.highlight();
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.poll_saves();
        let now = Instant::now();
        let output = self.surface.get_current_texture()?;
        self.next_frame = now + self.frame_interval();
        let raw_input = self.egui_state.take_egui_input(&self.window);
        let context = self.egui_context.clone();
        let mut command = controls::Command::None;
        let full_output = context.run(raw_input, |ctx| {
            command = ui::draw(ctx, self);
        });
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("viewer frame"),
            });
        let rect = self.ui.world_rect;
        self.renderer.camera.aspect = rect.width().max(1.0) / rect.height().max(1.0);
        if self.wallpaper {
            self.renderer.camera.zoom = (self.renderer.camera.aspect
                * self.simulation.settings.habitat_height
                / self.simulation.settings.habitat_width)
                .min(1.0);
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
        self.queue.submit(Some(encoder.finish()));
        self.window.pre_present_notify();
        output.present();
        self.frame_count += 1;
        if now.duration_since(self.fps_timer) >= Duration::from_secs(1) {
            let seconds = now.duration_since(self.fps_timer).as_secs_f32();
            self.render_fps = self.frame_count as f32 / seconds;
            self.ticks_last_second = (self.ticks_window_accumulated as f32 / seconds) as u32;
            self.frame_count = 0;
            self.ticks_window_accumulated = 0;
            self.fps_timer = now;
            self.update_title();
        }
        if !matches!(
            command,
            controls::Command::None | controls::Command::Pan(_) | controls::Command::Zoom(_)
        ) {
            self.complete_batch(true);
        }
        controls::apply(self, command);
        Ok(())
    }

    fn clear_world_observers(&mut self) {
        // World-local identities and counters expire, presentation/session state does not.
        self.scheduler.reset(Instant::now());
        self.inspection = inspection::Inspection::default();
        self.history.clear();
        self.recent_events.clear();
        self.evolution_snapshot = None;
        self.assisted = false;
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

    fn close_requested(&mut self) -> bool {
        self.complete_batch(true);
        if self.experiment.is_some()
            && let Err(error) = self.save_experiment()
        {
            self.paused = true;
            self.file_status = format!("Save failed; window kept open so you can retry: {error}");
            return false;
        }
        true
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
    match action {
        model::SIGNAL_OBSERVED => "signal observed",
        model::MEMORY_SAMPLE => "memory sample",
        _ => model::ACTION_NAMES
            .get(action as usize)
            .copied()
            .unwrap_or("unknown"),
    }
}
impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let wallpaper = launch_arguments().iter().any(|arg| arg == "--wallpaper");
        let mut attributes = WindowAttributes::default()
            .with_visible(false)
            .with_title(format!("Primitive World {}", env!("CARGO_PKG_VERSION")))
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 820.0))
            .with_min_inner_size(winit::dpi::LogicalSize::new(900.0, 620.0));
        if wallpaper {
            attributes = attributes.with_decorations(false).with_resizable(false);
        }
        let window = Arc::new(
            event_loop
                .create_window(attributes)
                .expect("window creation failed"),
        );
        #[cfg(windows)]
        if wallpaper && let Err(error) = windows_platform::attach_to_desktop(&window) {
            eprintln!("Wallpaper desktop hosting unavailable: {error}");
            windows_platform::show_error(&format!(
                "Could not attach wallpaper to the desktop.\n\n{error}"
            ));
            // Wallpaper mode must never fall back to an ordinary visible
            // window: that would cover the user's desktop and icons.
            event_loop.exit();
            return;
        }
        let state = pollster::block_on(AppState::new(window.clone()));
        #[cfg(windows)]
        if wallpaper && (!state.ui.has_world || state.tray.is_none()) {
            let message = if state.tray.is_none() {
                "The wallpaper tray controls could not start. See %LOCALAPPDATA%\\PrimitiveWorld\\logs\\wallpaper.log for details."
            } else {
                &state.file_status
            };
            windows_platform::show_error(&format!("Wallpaper could not start.\n\n{message}"));
            event_loop.exit();
            return;
        }
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
            WindowEvent::Destroyed if state.wallpaper => {
                // Explorer owns our parent. If it exits, the child cannot be
                // reattached or rendered; save and release the hook/singleton.
                if !state.close_requested() {
                    eprintln!("Wallpaper host disappeared: {}", state.file_status);
                }
                event_loop.exit();
            }
            WindowEvent::CloseRequested => {
                if state.close_requested() {
                    event_loop.exit();
                }
            }
            WindowEvent::Resized(size) => {
                state.occluded = size.width == 0 || size.height == 0;
                state.resize(size.width, size.height);
            }
            WindowEvent::Occluded(hidden) => state.occluded = hidden,
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
                    KeyCode::Escape if !state.wallpaper => state.open_menu(),
                    KeyCode::Escape if state.wallpaper => {
                        if state.close_requested() {
                            event_loop.exit();
                        }
                    }
                    KeyCode::KeyQ
                        if state.wallpaper
                            && state.modifiers.control_key()
                            && state.modifiers.shift_key() =>
                    {
                        if state.close_requested() {
                            event_loop.exit();
                        }
                    }
                    KeyCode::Space => state.paused = !state.paused,
                    KeyCode::KeyL => {
                        state.renderer.camera.lens =
                            Lens::from_u32(state.renderer.camera.lens).next() as u32
                    }
                    KeyCode::Home if !state.wallpaper => {
                        state.renderer.camera.center = [
                            state.simulation.settings.habitat_width * 0.5,
                            state.simulation.settings.habitat_height * 0.5,
                        ];
                        state.renderer.camera.zoom = 1.0;
                    }
                    KeyCode::ArrowUp | KeyCode::KeyW if !state.wallpaper => {
                        state.renderer.camera.center[1] -= pan
                    }
                    KeyCode::ArrowDown | KeyCode::KeyS if !state.wallpaper => {
                        state.renderer.camera.center[1] += pan
                    }
                    KeyCode::ArrowLeft | KeyCode::KeyA if !state.wallpaper => {
                        state.renderer.camera.center[0] -= pan
                    }
                    KeyCode::ArrowRight | KeyCode::KeyD if !state.wallpaper => {
                        state.renderer.camera.center[0] += pan
                    }
                    KeyCode::Digit1 => state.speed_index = 0,
                    KeyCode::Digit2 => state.speed_index = 1,
                    KeyCode::Digit4 => state.speed_index = 2,
                    KeyCode::Digit8 => state.speed_index = 3,
                    KeyCode::Digit6 => state.speed_index = 4,
                    KeyCode::KeyM => state.speed_index = playback::SPEED_LABELS.len() - 1,
                    _ => {}
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => state.modifiers = modifiers.state(),
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

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(state) = &mut self.state {
            #[cfg(windows)]
            for _ in 0..64 {
                let Some(action) = state.tray.as_ref().and_then(|tray| tray.take_action()) else {
                    break;
                };
                match action {
                    windows_platform::TrayAction::TogglePause => state.paused = !state.paused,
                    windows_platform::TrayAction::DesktopClick { x, y, can_paint } => {
                        state.handle_wallpaper_desktop_click(x, y, can_paint)
                    }
                    windows_platform::TrayAction::DesktopDrag { x, y } => {
                        let scale = state.egui_context.pixels_per_point();
                        let point = egui::pos2(x as f32 / scale, y as f32 / scale);
                        state.ui.brush_cursor = Some(point);
                        if let Some(adjustment) = state.ui.wallpaper_controls.brush_sizing {
                            match adjustment {
                                ui::BrushAdjustment::Size => state
                                    .ui
                                    .food_brush
                                    .slider(point.x, state.ui.wallpaper_controls.brush_size_rect),
                                ui::BrushAdjustment::Density => state.ui.food_brush.density_slider(
                                    point.x,
                                    state.ui.wallpaper_controls.brush_density_rect,
                                ),
                            }
                        } else if state.ui.food_brush.active() {
                            state.paint_food(point);
                        }
                    }
                    windows_platform::TrayAction::DesktopHover { x, y } => {
                        let scale = state.egui_context.pixels_per_point();
                        state.ui.brush_cursor =
                            Some(egui::pos2(x as f32 / scale, y as f32 / scale));
                    }
                    windows_platform::TrayAction::DesktopRelease => {
                        state.ui.food_brush.end();
                        state.ui.wallpaper_controls.brush_sizing = None;
                    }
                    windows_platform::TrayAction::DesktopLeave => {
                        state.ui.food_brush.end();
                        state.ui.wallpaper_controls.brush_sizing = None;
                        state.ui.brush_cursor = None;
                    }
                    windows_platform::TrayAction::DesktopWheel { x, y, notches } => {
                        let scale = state.egui_context.pixels_per_point();
                        let point = egui::pos2(x as f32 / scale, y as f32 / scale);
                        let controls = &state.ui.wallpaper_controls;
                        if state.ui.food_brush.enabled
                            && state.ui.world_rect.contains(point)
                            && !controls.hud_rect.contains(point)
                            && controls.menu.is_none()
                        {
                            state.ui.food_brush.resize(notches);
                        }
                    }
                    windows_platform::TrayAction::SetStartup(enabled) => {
                        let result = if enabled {
                            windows_platform::install_startup()
                        } else {
                            windows_platform::uninstall_startup()
                        };
                        if let Err(error) = result {
                            windows_platform::show_error(&error);
                        }
                    }
                    windows_platform::TrayAction::Save => match state.save_experiment() {
                        Ok(message) => {
                            if let Some(tray) = &state.tray {
                                tray.notify(&message);
                            }
                            state.file_status = message;
                        }
                        Err(error) => windows_platform::show_error(&error),
                    },
                    windows_platform::TrayAction::OpenSaves => {
                        if let Err(error) = windows_platform::open_folder(&experiments::save_root())
                        {
                            windows_platform::show_error(&error);
                        }
                    }
                    windows_platform::TrayAction::Quit => {
                        if state.close_requested() {
                            event_loop.exit();
                            return;
                        }
                        windows_platform::show_error(&state.file_status);
                    }
                }
            }
            state.pump_simulation();
            #[cfg(windows)]
            if let Some(tray) = &mut state.tray {
                tray.set_paused(state.paused);
                windows_platform::set_paint_enabled(state.ui.food_brush.enabled);
            }
            let now = Instant::now();
            #[cfg(windows)]
            if state.wallpaper
                && now.duration_since(state.last_desktop_check) >= Duration::from_secs(3)
            {
                if let Some(tray) = &state.tray {
                    tray.refresh();
                }
                state.last_desktop_check = now;
            }
            if !state.occluded && now >= state.next_frame {
                state.window.request_redraw();
            }
            event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                state.next_wake(now),
            ));
        }
    }
}

fn launch_arguments() -> Vec<String> {
    normalize_launch_arguments(std::env::args().collect(), cfg!(windows))
}

fn normalize_launch_arguments(mut args: Vec<String>, windows: bool) -> Vec<String> {
    if windows && args.len() == 1 {
        args.extend(["--wallpaper".into(), "--resume".into()]);
    }
    args
}

fn main() {
    let options = launch_arguments();
    #[cfg(windows)]
    windows_platform::prepare_console(&options);
    if options.iter().any(|x| x == "--prune-saves") {
        if options.len() != 2 {
            eprintln!("Use --prune-saves by itself");
            std::process::exit(2);
        }
        match experiments::prune(&experiments::save_root()) {
            Ok(report) => println!("{}", report.message()),
            Err(error) => {
                eprintln!("Could not prune saves: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    #[cfg(windows)]
    {
        if options.iter().any(|x| x == "--stop-wallpaper") {
            if options.len() != 2 {
                eprintln!("Use --stop-wallpaper by itself");
                std::process::exit(2);
            }
            match windows_platform::stop_wallpaper() {
                Ok(message) => println!("{message}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
            return;
        }
        if options.iter().any(|x| x == "--install-startup")
            || options.iter().any(|x| x == "--uninstall-startup")
        {
            if options.iter().any(|x| x == "--install-startup")
                && options.iter().any(|x| x == "--uninstall-startup")
            {
                eprintln!("Choose either --install-startup or --uninstall-startup");
                std::process::exit(2);
            }
            let result = if options.iter().any(|x| x == "--install-startup") {
                windows_platform::install_startup()
            } else {
                windows_platform::uninstall_startup()
            };
            match result {
                Ok(message) => println!("{message}"),
                Err(error) => {
                    eprintln!("{error}");
                    std::process::exit(1);
                }
            }
            return;
        }
    }
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
        #[cfg(windows)]
        if options.iter().any(|arg| arg == "--wallpaper")
            && !options.iter().any(|arg| arg == "--headless")
        {
            windows_platform::show_error(&e);
        }
        std::process::exit(2);
    }
    #[cfg(windows)]
    let _single_instance = match if options.iter().any(|a| a == "--wallpaper") {
        windows_platform::SingleInstance::acquire()
    } else {
        Ok(None)
    } {
        Ok(Some(guard)) => Some(guard),
        Ok(None) if !options.iter().any(|a| a == "--wallpaper") => None,
        Ok(None) => {
            windows_platform::show_info(
                "Primitive World wallpaper is already running. Use its icon in the Windows notification area to pause, save, or quit.",
            );
            return;
        }
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    };
    #[cfg(windows)]
    let _wallpaper_log = if options.iter().any(|a| a == "--wallpaper") {
        match windows_platform::open_wallpaper_log() {
            Ok(file) => Some(file),
            Err(error) => {
                windows_platform::show_error(&error);
                return;
            }
        }
    } else {
        None
    };
    #[cfg(windows)]
    if !options.iter().any(|a| a == "--headless") {
        windows_platform::install_graphical_panic_hook();
    }
    let args = launch_arguments();
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

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn double_click_resumes_wallpaper_but_explicit_modes_are_preserved() {
        assert_eq!(
            normalize_launch_arguments(vec!["app".into()], true),
            vec!["app", "--wallpaper", "--resume"]
        );
        for args in [
            vec!["app", "--viewer"],
            vec!["app", "--headless", "--ticks", "1"],
            vec!["app", "--wallpaper", "--seed", "42"],
        ] {
            let args: Vec<String> = args.into_iter().map(str::to_owned).collect();
            assert_eq!(normalize_launch_arguments(args.clone(), true), args);
        }
        assert_eq!(
            normalize_launch_arguments(vec!["app".into()], false),
            vec!["app"]
        );
    }
}
