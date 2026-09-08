use crate::controls::Command as Action;
use crate::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    NewGame,
    LoadGame,
    Play,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Overview,
    Agent,
    Experiment,
}

pub struct UiState {
    pub screen: Screen,
    pub tab: Tab,
    pub name: String,
    pub seed: u32,
    pub setup: simulation::SimSettings,
    pub has_world: bool,
    pub saves: Vec<experiments::SavedExperiment>,
    pub library_notice: String,
    pub library_scan: experiments::LibraryScan,
    pub world_rect: egui::Rect,
    pub wallpaper_controls: WallpaperControls,
    #[cfg(not(windows))]
    pub import_path: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WallpaperMenu {
    View,
    Speed,
    Details,
}

pub struct WallpaperControls {
    pub hud_rect: egui::Rect,
    pub popup_rect: egui::Rect,
    pub lens_button: egui::Rect,
    pub speed_button: egui::Rect,
    pub details_button: egui::Rect,
    pub lens_options: [egui::Rect; 10],
    pub speed_buttons: [egui::Rect; 6],
    pub menu: Option<WallpaperMenu>,
}

impl WallpaperControls {
    pub fn toggle(&mut self, menu: WallpaperMenu) {
        self.menu = if self.menu == Some(menu) {
            None
        } else {
            Some(menu)
        };
    }
}

impl Default for WallpaperControls {
    fn default() -> Self {
        Self {
            hud_rect: egui::Rect::NOTHING,
            popup_rect: egui::Rect::NOTHING,
            lens_button: egui::Rect::NOTHING,
            speed_button: egui::Rect::NOTHING,
            details_button: egui::Rect::NOTHING,
            lens_options: [egui::Rect::NOTHING; 10],
            speed_buttons: [egui::Rect::NOTHING; 6],
            menu: None,
        }
    }
}

impl UiState {
    pub fn new(command_line_world: bool) -> Self {
        Self {
            screen: if command_line_world {
                Screen::Play
            } else {
                Screen::Home
            },
            tab: Tab::Overview,
            name: "My first evolution".into(),
            seed: 1,
            setup: simulation::SimSettings::default(),
            has_world: command_line_world,
            saves: Vec::new(),
            library_notice: String::new(),
            library_scan: experiments::LibraryScan::default(),
            world_rect: egui::Rect::NOTHING,
            wallpaper_controls: WallpaperControls::default(),
            #[cfg(not(windows))]
            import_path: String::new(),
        }
    }
}

pub fn style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::from_rgb(21, 29, 25);
    visuals.window_fill = egui::Color32::from_rgb(25, 36, 30);
    visuals.extreme_bg_color = egui::Color32::from_rgb(13, 19, 16);
    visuals.selection.bg_fill = egui::Color32::from_rgb(58, 115, 78);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(64, 126, 86);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(45, 66, 54);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(34, 48, 40);
    ctx.set_visuals(visuals);
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(9.0, 9.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.wrap_mode = Some(egui::TextWrapMode::Wrap);
    style
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(16.0));
    style
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(15.0));
    style
        .text_styles
        .insert(egui::TextStyle::Small, egui::FontId::proportional(13.0));
    style
        .text_styles
        .insert(egui::TextStyle::Heading, egui::FontId::proportional(24.0));
    ctx.set_style(style);
}

fn heading(ui: &mut egui::Ui, text: &str, subtitle: &str) {
    ui.heading(text);
    ui.label(egui::RichText::new(subtitle).color(egui::Color32::from_rgb(167, 187, 174)));
    ui.add_space(16.0);
}

fn large_button(ui: &mut egui::Ui, title: &str, body: &str) -> bool {
    let width = ui.available_width().min(560.0);
    ui.add_sized(
        [width, 72.0],
        egui::Button::new(egui::RichText::new(format!("{title}\n{body}")).size(16.0)),
    )
    .clicked()
}

fn menu_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(21, 29, 25))
        .inner_margin(egui::Margin::same(28))
        .corner_radius(12.0)
}

pub fn draw(ctx: &egui::Context, state: &mut AppState) -> Action {
    let mut action = Action::None;
    if state.wallpaper {
        draw_wallpaper(ctx, state, &mut action);
        return action;
    }
    match state.ui.screen {
        Screen::Home => draw_home(ctx, state, &mut action),
        Screen::NewGame => draw_new(ctx, state, &mut action),
        Screen::LoadGame => draw_load(ctx, state, &mut action),
        Screen::Play => draw_play(ctx, state, &mut action),
    }
    action
}

fn draw_wallpaper(ctx: &egui::Context, state: &mut AppState, action: &mut Action) {
    state.ui.wallpaper_controls.speed_buttons = [egui::Rect::NOTHING; 6];
    state.ui.wallpaper_controls.lens_options = [egui::Rect::NOTHING; 10];
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            state.ui.world_rect = ui.max_rect();
            let world = ui.allocate_rect(state.ui.world_rect, egui::Sense::click());
            if world.clicked()
                && let Some(point) = world.interact_pointer_pos()
            {
                *action = Action::FoodClick(point);
            }
        });

    let hud = egui::Area::new(egui::Id::new("wallpaper_hud_minimal"))
        .anchor(egui::Align2::RIGHT_TOP, [-18.0, 38.0])
        .movable(false)
        .show(ctx, |ui| {
            wallpaper_style(ui);
            wallpaper_frame()
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // Let the strip grow to fit long view names and Details.
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Extend);
                        ui.label(
                            egui::RichText::new(format!(
                                "World {}",
                                state.simulation.progress.world
                            ))
                            .strong(),
                        );
                        ui.label(format!("{} alive", state.living_agents));
                        ui.separator();
                        let view = ui.button(Lens::from_u32(state.renderer.camera.lens).name());
                        state.ui.wallpaper_controls.lens_button = view.rect;
                        if view.clicked() {
                            state.ui.wallpaper_controls.toggle(WallpaperMenu::View);
                        }
                        let speed = ui.button(playback::SPEED_LABELS[state.speed_index]);
                        state.ui.wallpaper_controls.speed_button = speed.rect;
                        if speed.clicked() {
                            state.ui.wallpaper_controls.toggle(WallpaperMenu::Speed);
                        }
                        let details = ui.button("Details");
                        state.ui.wallpaper_controls.details_button = details.rect;
                        if details.clicked() {
                            state.ui.wallpaper_controls.toggle(WallpaperMenu::Details);
                        }
                    });
                    if !state.ui.has_world {
                        ui.colored_label(egui::Color32::LIGHT_RED, "World did not start");
                        ui.small(&state.file_status);
                    }
                })
                .response
                .rect
        })
        .inner;
    state.ui.wallpaper_controls.hud_rect = hud;
    state.ui.wallpaper_controls.popup_rect = egui::Rect::NOTHING;

    if let Some(menu) = state.ui.wallpaper_controls.menu {
        let popup = egui::Area::new(egui::Id::new("wallpaper_menu"))
            .order(egui::Order::Foreground)
            .pivot(egui::Align2::RIGHT_TOP)
            .fixed_pos(egui::pos2(hud.right(), hud.bottom() + 6.0))
            .movable(false)
            .show(ctx, |ui| {
                wallpaper_style(ui);
                wallpaper_frame()
                    .show(ui, |ui| {
                        ui.set_width(210.0);
                        match menu {
                            WallpaperMenu::View => {
                                ui.weak("View");
                                for value in 0..=9 {
                                    let choice = ui.add_sized(
                                        [210.0, 26.0],
                                        egui::Button::new(Lens::from_u32(value).name())
                                            .selected(state.renderer.camera.lens == value),
                                    );
                                    state.ui.wallpaper_controls.lens_options[value as usize] =
                                        choice.rect;
                                    if choice.clicked() {
                                        state.renderer.camera.lens = value;
                                        state.ui.wallpaper_controls.menu = None;
                                    }
                                }
                            }
                            WallpaperMenu::Speed => {
                                ui.weak("Playback speed");
                                for (index, label) in playback::SPEED_LABELS[..6].iter().enumerate()
                                {
                                    let choice = ui.add_sized(
                                        [210.0, 26.0],
                                        egui::Button::new(*label)
                                            .selected(state.speed_index == index),
                                    );
                                    state.ui.wallpaper_controls.speed_buttons[index] = choice.rect;
                                    if choice.clicked() {
                                        state.speed_index = index;
                                        state.ui.wallpaper_controls.menu = None;
                                    }
                                }
                            }
                            WallpaperMenu::Details => {
                                ui.weak("World details");
                                ui.label(format!("Age: {} ticks", state.simulation.tick));
                                if state.assisted {
                                    ui.label("Food added by you");
                                }
                                if let Some(record) = state.simulation.progress.best.as_ref() {
                                    ui.label(format!(
                                        "Best: world {} / {} ticks",
                                        record.world, record.duration
                                    ));
                                    if state.simulation.tick > record.duration {
                                        ui.label("New survival record");
                                    }
                                } else {
                                    ui.label("No completed worlds yet");
                                }
                                ui.add_space(4.0);
                                ui.weak("Population history");
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(210.0, 38.0),
                                    egui::Sense::hover(),
                                );
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
                                    .map(|(index, metrics)| {
                                        egui::pos2(
                                            rect.left()
                                                + rect.width() * index as f32
                                                    / (state.history.len().max(2) - 1) as f32,
                                            rect.bottom()
                                                - rect.height() * metrics.living as f32 / peak,
                                        )
                                    })
                                    .collect();
                                if points.len() > 1 {
                                    ui.painter().add(egui::Shape::line(
                                        points,
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(129, 163, 142),
                                        ),
                                    ));
                                }
                                ui.weak(format!(
                                    "{} Hz / {:.1}x actual",
                                    state.render_hz,
                                    state.ticks_last_second as f32 / playback::BASE_TPS as f32
                                ));
                            }
                        }
                    })
                    .response
                    .rect
            })
            .inner;
        state.ui.wallpaper_controls.popup_rect = popup;
    }
}

fn wallpaper_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(10, 16, 22, 235))
        .inner_margin(8)
        .corner_radius(8.0)
}

fn wallpaper_style(ui: &mut egui::Ui) {
    let style = ui.style_mut();
    style.override_font_id = Some(egui::FontId::proportional(13.0));
    style.spacing.item_spacing = egui::vec2(8.0, 4.0);
    style.spacing.button_padding = egui::vec2(7.0, 4.0);
    style.visuals.override_text_color = Some(egui::Color32::from_rgb(185, 197, 202));
    style.visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
    style.visuals.selection.bg_fill = egui::Color32::from_rgb(39, 62, 53);
}

fn draw_home(ctx: &egui::Context, state: &mut AppState, action: &mut Action) {
    state.ui.world_rect = egui::Rect::NOTHING;
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_rgb(12, 18, 15)))
        .show(ctx, |ui| {
            let width = ui.available_width().min(620.0);
            ui.add_space((ui.available_height() * 0.12).min(90.0));
            ui.horizontal(|ui| {
                ui.add_space(((ui.available_width() - width) * 0.5).max(0.0));
                ui.allocate_ui_with_layout(
                    egui::vec2(width, ui.available_height()),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.label(
                            egui::RichText::new("PRIMITIVE WORLD")
                                .size(13.0)
                                .color(egui::Color32::from_rgb(130, 190, 148)),
                        );
                        heading(
                            ui,
                            "Watch life find its way",
                            "Begin a new evolutionary line or return to one already in motion.",
                        );
                        if state.ui.has_world {
                            let name = state
                                .experiment
                                .as_ref()
                                .map_or("Current world", |x| x.name.as_str());
                            if large_button(ui, "Continue", name) {
                                state.ui.screen = Screen::Play;
                            }
                        } else if let Some(saved) = state.ui.saves.first()
                            && large_button(
                                ui,
                                "Continue",
                                &format!(
                                    "{} · world {} · tick {}",
                                    saved.record.name,
                                    saved.world_number(),
                                    saved.record.tick
                                ),
                            )
                        {
                            *action = Action::Open(Box::new(saved.clone()));
                        }
                        if large_button(ui, "New Game", "Start with random, untrained brains") {
                            *action = Action::NewGame;
                        }
                        if large_button(ui, "Load Game", "Resume your saved world") {
                            *action = Action::LoadGame;
                        }
                        ui.add_space(14.0);
                        ui.small(format!("Primitive World {}", env!("CARGO_PKG_VERSION")));
                        if state.ui.library_scan.busy() {
                            ui.small("Checking saved experiments in the background…");
                        }
                        if !state.file_status.is_empty() {
                            ui.label(&state.file_status);
                        }
                    },
                );
            });
        });
}

fn draw_new(ctx: &egui::Context, state: &mut AppState, action: &mut Action) {
    state.ui.world_rect = egui::Rect::NOTHING;
    egui::CentralPanel::default().show(ctx, |ui| {
        egui::ScrollArea::vertical().show(ui, |ui| {
            menu_frame().show(ui, |ui| {
                if ui.button("Main menu").clicked() {
                    *action = Action::Home;
                }
                ui.add_space(18.0);
                heading(ui, "New Game", "Fresh brains. Your own evolutionary line.");
                ui.label("Experiment name");
                ui.add(egui::TextEdit::singleline(&mut state.ui.name).desired_width(380.0));
                ui.label("Founding populations compete on how long their worlds stay populated.");
                ui.small("Current population and candidate face the same environment. Living worlds are never cut short.");
                egui::CollapsingHeader::new("World setup").show(ui, |ui| {
                    ui.add(egui::DragValue::new(&mut state.ui.seed).prefix("Seed "));
                    ui.add(
                        egui::Slider::new(&mut state.ui.setup.population, 1..=MAX_AGENTS)
                            .text("Initial bodies"),
                    );
                    ui.add(
                        egui::Slider::new(&mut state.ui.setup.resource_regeneration, 0.0..=0.1)
                            .text("Food regeneration"),
                    );
                    ui.add(
                        egui::Slider::new(&mut state.ui.setup.metabolic_cost, 0.0..=0.2)
                            .text("Metabolic cap"),
                    );
                    ui.small("Metabolism rises from .01; food patches recede to normal size over 50,000 ticks.");
                    ui.checkbox(&mut state.ui.setup.evolving_landscape, "Evolving geography");
                    ui.checkbox(
                        &mut state.ui.setup.social_actions_enabled,
                        "Unlock social actions after the 50,000-tick survival bootstrap",
                    );
                    ui.checkbox(&mut state.ui.setup.force_enabled, "Contact force available");
                    ui.checkbox(
                        &mut state.ui.setup.communication_enabled,
                        "Local signals available",
                    );
                });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new("Start world")
                            .fill(egui::Color32::from_rgb(57, 117, 78)),
                    )
                    .clicked()
                {
                    *action = Action::Create;
                }
                if !state.file_status.is_empty() {
                    ui.colored_label(egui::Color32::LIGHT_RED, &state.file_status);
                }
            });
        });
    });
}

fn draw_load(ctx: &egui::Context, state: &mut AppState, action: &mut Action) {
    state.ui.world_rect = egui::Rect::NOTHING;
    egui::CentralPanel::default().show(ctx, |ui| {
        menu_frame().show(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Main menu").clicked() {
                    *action = Action::Home;
                }
                if ui.button("Refresh").clicked() {
                    state.refresh_saves();
                }
                if ui.button("Import save…").clicked() {
                    *action = Action::Import;
                }
            });
            #[cfg(not(windows))]
            {
                ui.label("Save path");
                ui.text_edit_singleline(&mut state.ui.import_path);
            }
            ui.add_space(18.0);
            heading(ui, "Load Game", "Continue your saved world.");
            if !state.ui.library_notice.is_empty() {
                ui.colored_label(egui::Color32::YELLOW, &state.ui.library_notice);
            }
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Your experiments");
                if state.ui.library_scan.busy() {
                    ui.label("Refreshing saved experiments… You can keep using the menu.");
                } else if state.ui.saves.is_empty() {
                    ui.label("No saved experiments yet.");
                }
                for saved in &state.ui.saves {
                    egui::Frame::group(ui.style())
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.strong(&saved.record.name);
                            ui.label(format!(
                                "World {} · tick {} · {} living",
                                saved.world_number(),
                                saved.record.tick,
                                saved.record.living
                            ));
                            ui.small(format!(
                                "{} · {} ticks in your experiment",
                                saved.record.origin, saved.record.total_ticks
                            ));
                            ui.horizontal(|ui| {
                                if ui.button("Continue").clicked() {
                                    *action = Action::Open(Box::new(saved.clone()));
                                }
                            });
                        });
                    ui.add_space(8.0);
                }
            });
            if !state.file_status.is_empty() {
                ui.label(&state.file_status);
            }
        })
    });
}

fn draw_play(ctx: &egui::Context, state: &mut AppState, action: &mut Action) {
    egui::TopBottomPanel::top("game_bar")
        .exact_height(56.0)
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                if ui.button("Menu").clicked() {
                    *action = Action::Home;
                }
                ui.separator();
                let name = state
                    .experiment
                    .as_ref()
                    .map_or("Primitive World", |x| x.name.as_str());
                ui.add_sized(
                    [(ui.available_width() - 440.0).max(80.0), 24.0],
                    egui::Label::new(egui::RichText::new(name).strong()).truncate(),
                )
                .on_hover_text(name);

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        *action = Action::Save;
                    }
                    egui::ComboBox::from_id_salt("speed")
                        .selected_text(playback::SPEED_LABELS[state.speed_index])
                        .show_ui(ui, |ui| {
                            for (i, label) in playback::SPEED_LABELS.iter().enumerate() {
                                ui.selectable_value(&mut state.speed_index, i, *label);
                            }
                        });
                    if ui
                        .button(if state.paused { "Resume" } else { "Pause" })
                        .clicked()
                    {
                        *action = Action::Pause;
                    }
                    if ui.button("Step").clicked() {
                        *action = Action::Step;
                    }
                });
            });
        });
    egui::TopBottomPanel::bottom("status_bar")
        .exact_height(30.0)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                let status = if state.file_status.is_empty() {
                    "Space pause · WASD pan · wheel zoom · Home fit · Esc menu"
                } else {
                    &state.file_status
                };
                ui.add(egui::Label::new(egui::RichText::new(status).small()).truncate())
                    .on_hover_text(status);
            });
        });
    egui::SidePanel::right("inspector")
        .default_width(360.0)
        .width_range(320.0..=480.0)
        .resizable(true)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (tab, label) in [
                    (Tab::Overview, "Overview"),
                    (Tab::Agent, "Agent"),
                    (Tab::Experiment, "Experiment"),
                ] {
                    ui.selectable_value(&mut state.ui.tab, tab, label);
                }
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .min_scrolled_width(0.0)
                .auto_shrink([false, false])
                .show(ui, |ui| match state.ui.tab {
                    Tab::Overview => overview(ui, state, action),
                    Tab::Agent => ui_details::agent(ui, state),
                    Tab::Experiment => experiment(ui, state, action),
                });
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::NONE)
        .show(ctx, |ui| {
            state.ui.world_rect = ui.max_rect();
            let world = ui.allocate_rect(state.ui.world_rect, egui::Sense::click_and_drag());
            if world.clicked()
                && let Some(point) = world.interact_pointer_pos()
            {
                *action = Action::WorldClick(point);
            }
            if world.dragged() && world.drag_delta() != egui::Vec2::ZERO {
                *action = Action::Pan(world.drag_delta());
            }
            if world.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    *action = Action::Zoom(scroll);
                }
            }
            egui::Area::new(egui::Id::new("world_controls"))
                .anchor(egui::Align2::LEFT_BOTTOM, [14.0, -44.0])
                .show(ctx, |ui| {
                    egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(190))
                        .inner_margin(8.0)
                        .corner_radius(7.0)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("lens")
                                    .selected_text(
                                        Lens::from_u32(state.renderer.camera.lens).name(),
                                    )
                                    .show_ui(ui, |ui| {
                                        for value in 0..=9 {
                                            let lens = Lens::from_u32(value);
                                            ui.selectable_value(
                                                &mut state.renderer.camera.lens,
                                                value,
                                                lens.name(),
                                            );
                                        }
                                    });
                            });
                        });
                });
        });
}

fn overview(ui: &mut egui::Ui, state: &mut AppState, action: &mut Action) {
    heading(
        ui,
        "Life in this world",
        "Population and history at a glance.",
    );

    let p = &state.simulation.progress;
    ui.strong(format!("World {}", p.world));
    let evaluating = match p.phase {
        evolution::Phase::Incumbent => "Current population",
        evolution::Phase::Challenger => "Candidate",
    };
    ui.label(format!(
        "Comparison {} · {evaluating} #{}",
        p.comparison,
        p.population_id()
    ));
    if let Some(b) = &p.baseline {
        ui.small(format!(
            "Current population lasted {} ticks on this same seed. A living candidate wins immediately when it outlasts that.",
            b.duration
        ));
        ui.small(format!(
            "Candidate founders: all {} uniquely varied; {} use terminal-descendant genomes as their inherited source",
            state.simulation.settings.population, p.descendant_founders
        ));
    } else {
        ui.small(
            "Establishing the current population's duration on this comparison's environment.",
        );
    }
    if let Some(o) = &p.completed {
        ui.small(format!("Natural extinction after {} ticks", o.duration));
    } else {
        ui.small("World still in progress. Biological evolution continues; a candidate is promoted as soon as it outlives its incumbent.");
    }
    ui.small(format!(
        "{} candidates accepted · natural survival duration decides",
        p.accepted_challengers
    ));
    ui.collapsing("Completed worlds", |ui| {
        ui.small(
            "Recent 64 worlds. Feeding, births and generations are observations, not rewards.",
        );
        for o in p.history.iter().rev() {
            let result = match o.challenger_accepted {
                Some(true) => " · candidate accepted",
                Some(false) => " · current population kept",
                None => " · baseline",
            };
            let assisted = if o.assisted { " · assisted" } else { "" };
            ui.small(format!(
                "World {} · population #{} · seed {} · {} ticks{}{}",
                o.world, o.population_id, o.seed, o.duration, result, assisted
            ));
            ui.small(format!(
                "{} births · generation {} · {:.1} food collected / {:.1} digested",
                o.births, o.maximum_generation, o.food_collected, o.food_ingested
            ));
        }
    });
    egui::Grid::new("world_stats")
        .num_columns(2)
        .striped(true)
        .show(ui, |ui| {
            ui.label("Living");
            ui.strong(state.living_agents.to_string());
            ui.end_row();
            ui.label("Births");
            ui.strong(state.births.to_string());
            ui.end_row();

            ui.label("World tick");
            ui.strong(state.simulation.tick.to_string());
            ui.end_row();
            ui.label("Ticks / second");
            ui.strong(state.ticks_last_second.to_string());
            ui.end_row();
        });
    ui.small(format!(
        "Actual {:.1}x · target {} (1x = 60 ticks/s)",
        state.ticks_last_second as f32 / playback::BASE_TPS as f32,
        playback::SPEED_LABELS[state.speed_index]
    ));
    if let Some(ms) = state.gpu_tick_ms {
        ui.small(format!("Simulation GPU time: {ms:.3} ms/tick"));
    }
    ui_details::stats(ui, state);
    ui_details::history(ui, state);
    if ui.button("Inspect evolution").clicked() {
        *action = Action::InspectEvolution;
    }
    if let Some(x) = &state.evolution_snapshot {
        ui.small(format!(
            "{} individual identities (not genetic diversity) · maximum ancestry {} · mean {:.2}",
            x.individual_identities, x.maximum_ancestry_depth, x.mean_ancestry_depth
        ));
    }
}

fn experiment(ui: &mut egui::Ui, state: &mut AppState, action: &mut Action) {
    heading(
        ui,
        "Experiment controls",
        "Playback, fixed world rules, and exports.",
    );
    ui.collapsing("Playback and compute", |ui| {
        ui.label("Simulation speed and viewer refresh are independent.");
        egui::ComboBox::from_id_salt("view_refresh").selected_text(format!("{} FPS",state.render_hz)).show_ui(ui,|ui| {
            for hz in [10,30,60] { ui.selectable_value(&mut state.render_hz,hz,format!("{hz} FPS")); }
        });
        let mut percent = state.compute_budget * 100.0;
        ui.add(egui::Slider::new(&mut percent,10.0..=100.0).suffix("%").text("Compute duty budget"));
        state.compute_budget = percent / 100.0;
        ui.small("Lower budgets insert idle time between batches. This is not a GPU utilization or wattage limit.");
        ui.small("MAX pursues available throughput; other speeds idle once their target is met.");
    });
    ui_details::physics(ui, state);
    ui_details::events(ui, state, action);
    ui.collapsing("Export & diagnostics", |ui| {
        if ui.button("Export living descendants").clicked() {
            *action = Action::ExportFounders;
        }
        if ui.button("Export recent history").clicked() {
            *action = Action::ExportHistory;
        }
    });
}
