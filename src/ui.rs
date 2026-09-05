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
    pub continuous: bool,
    pub has_world: bool,
    pub saves: Vec<experiments::SavedExperiment>,
    pub library_notice: String,
    pub library_scan: experiments::LibraryScan,
    pub world_rect: egui::Rect,
    #[cfg(not(windows))]
    pub import_path: String,
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
            continuous: true,
            has_world: command_line_world,
            saves: Vec::new(),
            library_notice: String::new(),
            library_scan: experiments::LibraryScan::default(),
            world_rect: egui::Rect::NOTHING,
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
    match state.ui.screen {
        Screen::Home => draw_home(ctx, state, &mut action),
        Screen::NewGame => draw_new(ctx, state, &mut action),
        Screen::LoadGame => draw_load(ctx, state, &mut action),
        Screen::Play => draw_play(ctx, state, &mut action),
    }
    action
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
                            *action = Action::Open(Box::new(saved.clone()), false);
                        }
                        if large_button(ui, "New Game", "Start with random, untrained brains") {
                            *action = Action::NewGame;
                        }
                        if large_button(
                            ui,
                            "Load Game",
                            "Continue or branch from your saved experiments",
                        ) {
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
                ui.checkbox(
                    &mut state.ui.continuous,
                    "At extinction, seed the next world from late survivors",
                );
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
                            .text("Metabolic cost"),
                    );
                    ui.checkbox(&mut state.ui.setup.evolving_landscape, "Evolving geography");
                    ui.checkbox(&mut state.ui.setup.force_enabled, "Contact force available");
                    ui.checkbox(
                        &mut state.ui.setup.communication_enabled,
                        "Local signals available",
                    );
                });
                ui.add_space(16.0);
                if ui
                    .add(
                        egui::Button::new("Start evolution")
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
            heading(
                ui,
                "Load Game",
                "Continue a saved evolutionary line or branch from its brains.",
            );
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
                                    *action = Action::Open(Box::new(saved.clone()), false);
                                }
                                if (saved.record.living > 0 || saved.record.evolution.is_some())
                                    && ui.button("Use brains in a new world").clicked()
                                {
                                    *action = Action::Open(Box::new(saved.clone()), true);
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
                if let Some(trial) = &state.visible_trial {
                    ui.small(format!("World {}", trial.world_number));
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Save").clicked() {
                        *action = Action::Save;
                    }
                    egui::ComboBox::from_id_salt("speed")
                        .selected_text(["1x", "2x", "4x", "8x", "16x", "MAX"][state.speed_index])
                        .show_ui(ui, |ui| {
                            for (i, label) in
                                ["1x", "2x", "4x", "8x", "16x", "MAX"].iter().enumerate()
                            {
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
                if state.shock_mode == ShockMode::Select {
                    *action = Action::Pan(world.drag_delta());
                } else if let Some(point) = world.interact_pointer_pos() {
                    *action = Action::WorldClick(point);
                }
            }
            if world.hovered() {
                let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                if scroll != 0.0 {
                    *action = Action::Zoom(scroll);
                }
                if state.shock_mode != ShockMode::Select
                    && let Some(point) = ui.input(|i| i.pointer.hover_pos())
                {
                    let radius = state.shock_radius * state.ui.world_rect.height() / WORLD_SIZE;
                    ui.painter().circle_stroke(
                        point,
                        radius,
                        egui::Stroke::new(1.5, egui::Color32::YELLOW),
                    );
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
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
                                if state.shock_mode != ShockMode::Select {
                                    ui.colored_label(
                                        egui::Color32::YELLOW,
                                        state.shock_mode.name(),
                                    );
                                    if ui.button("Done").clicked() {
                                        state.shock_mode = ShockMode::Select;
                                    }
                                }
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
            ui.label("Tick");
            ui.strong(state.simulation.tick.to_string());
            ui.end_row();
            ui.label("Ticks / second");
            ui.strong(state.ticks_last_second.to_string());
            ui.end_row();
        });
    ui_details::stats(ui, state);
    ui_details::history(ui, state);
    if ui.button("Inspect evolution").clicked() {
        *action = Action::InspectEvolution;
    }
    if let Some(x) = &state.evolution_snapshot {
        ui.small(format!(
            "{} lineages · maximum ancestry {} · mean {:.2}",
            x.unique_lineages, x.maximum_ancestry_depth, x.mean_ancestry_depth
        ));
    }
}

fn experiment(ui: &mut egui::Ui, state: &mut AppState, action: &mut Action) {
    heading(
        ui,
        "Change the conditions",
        "Live controls and deliberate interventions.",
    );
    ui_details::physics(ui, state);
    ui.collapsing("Interventions", |ui| {
        for (mode, label) in [
            (ShockMode::Select, "Inspect"),
            (ShockMode::RemoveResource, "Remove food"),
            (ShockMode::AddResource, "Add food"),
            (ShockMode::KillAgents, "Remove agents"),
        ] {
            ui.selectable_value(&mut state.shock_mode, mode, label);
        }
        if state.shock_mode != ShockMode::Select {
            ui.add(egui::Slider::new(&mut state.shock_radius, 4.0..=400.0).text("Brush radius"));
            ui.colored_label(
                egui::Color32::YELLOW,
                "Clicking the world now applies this intervention.",
            );
        }
    });
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
