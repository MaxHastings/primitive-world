//! UI commands are applied once, after egui finishes its layout passes and the
//! frame has been presented. Widgets never submit GPU work while drawing.
use crate::*;

pub enum Command {
    None,
    Home,
    NewGame,
    LoadGame,
    Create,
    Open(Box<experiments::SavedExperiment>),
    Import,
    Save,
    Pause,
    Step,
    WorldClick(egui::Pos2),
    FoodClick(egui::Pos2),
    FoodDrag(egui::Pos2),
    FoodEnd,
    BrushSize(f32),
    Pan(egui::Vec2),
    Zoom(f32),
    InspectEvolution,
    RefreshEvents,
    ExportFounders,
    ExportHistory,
}

pub fn world_position(
    rect: egui::Rect,
    center: [f32; 2],
    zoom: f32,
    point: egui::Pos2,
    world_size: [f32; 2],
) -> [f32; 2] {
    let delta = (point - rect.center()) * world_size[1] / (rect.height().max(1.0) * zoom);
    [center[0] + delta.x, center[1] + delta.y]
}

pub fn apply(state: &mut AppState, command: Command) {
    let result = match command {
        Command::None => return,
        Command::Home => {
            state.open_menu();
            return;
        }
        Command::NewGame => {
            state.ui.screen = ui::Screen::NewGame;
            state.file_status.clear();
            return;
        }
        Command::LoadGame => {
            state.refresh_saves();
            state.ui.screen = ui::Screen::LoadGame;
            return;
        }
        Command::Create => state.new_experiment(),
        Command::Open(saved) => state.load_experiment(*saved),
        Command::Import => import(state),
        Command::Save => state
            .save_experiment()
            .map(|message| state.file_status = message),
        Command::Pause => {
            state.paused = !state.paused;
            return;
        }
        Command::Step => {
            state.paused = true;
            state.step_requested = true;
            return;
        }
        Command::WorldClick(point) => {
            state.handle_click(point);
            return;
        }
        Command::FoodClick(point) => {
            state.handle_food_click(point);
            state.ui.food_brush.end();
            return;
        }
        Command::FoodDrag(point) => {
            state.paint_food(point);
            return;
        }
        Command::FoodEnd => {
            state.ui.food_brush.end();
            return;
        }
        Command::BrushSize(notches) => {
            if state.ui.food_brush.enabled {
                state.ui.food_brush.resize(notches);
            }
            return;
        }
        Command::Pan(delta) => {
            let delta = delta * state.simulation.settings.habitat_height
                / (state.ui.world_rect.height().max(1.0) * state.renderer.camera.zoom);
            state.renderer.camera.center[0] -= delta.x;
            state.renderer.camera.center[1] -= delta.y;
            return;
        }
        Command::Zoom(scroll) => {
            state.renderer.camera.zoom =
                (state.renderer.camera.zoom * (scroll * 0.003).exp()).clamp(0.15, 80.0);
            return;
        }
        Command::InspectEvolution => state
            .simulation
            .evolution_snapshot(&state.device, &state.queue)
            .map(|x| state.evolution_snapshot = Some(x)),
        Command::RefreshEvents => state
            .simulation
            .recent_events(&state.device, &state.queue)
            .map(|x| state.recent_events = x),
        Command::ExportFounders => experiments::stamp().and_then(|stamp| {
            let path = std::path::PathBuf::from(format!(
                "reports/play-founders-{}-{}-{stamp}.json",
                state.simulation.seed, state.simulation.tick
            ));
            std::fs::create_dir_all("reports").map_err(|e| e.to_string())?;
            state
                .simulation
                .export_founders(&state.device, &state.queue, &path)?;
            state.file_status = format!("Exported {}", path.display());
            Ok(())
        }),
        Command::ExportHistory => serde_json::to_vec_pretty(&state.history)
            .map_err(|e| e.to_string())
            .and_then(|b| {
                play_files::export_history(state.simulation.seed, state.simulation.tick, &b)
            })
            .map(|p| state.file_status = format!("Exported {}", p.display())),
    };
    if let Err(error) = result {
        state.paused = true;
        state.file_status = error;
        if !state.ui.has_world && state.ui.screen == ui::Screen::Play {
            state.ui.screen = ui::Screen::LoadGame;
        }
    }
}

#[cfg(windows)]
fn import(state: &mut AppState) -> Result<(), String> {
    if let Some(path) = rfd::FileDialog::new()
        .add_filter("Primitive World save", &["json"])
        .pick_file()
    {
        state.import_checkpoint(&path)?;
    }
    Ok(())
}

#[cfg(not(windows))]
fn import(state: &mut AppState) -> Result<(), String> {
    state.import_checkpoint(&std::path::PathBuf::from(state.ui.import_path.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::WORLD_SIZE;
    #[test]
    fn picks_and_brushes_use_the_world_viewport_at_every_zoom() {
        for rect in [
            egui::Rect::from_min_size(egui::pos2(0.0, 56.0), egui::vec2(920.0, 734.0)),
            egui::Rect::from_min_size(egui::pos2(0.0, 64.0), egui::vec2(530.0, 486.0)),
        ] {
            for zoom in [0.15, 1.0, 8.0, 80.0] {
                let center = [1200.0, 800.0];
                assert_eq!(
                    world_position(rect, center, zoom, rect.center(), [WORLD_SIZE, WORLD_SIZE]),
                    center
                );
                let point = rect.center() + egui::vec2(27.0, -16.0);
                let result = world_position(rect, center, zoom, point, [WORLD_SIZE, WORLD_SIZE]);
                let roundtrip = rect.center()
                    + egui::vec2(result[0] - center[0], result[1] - center[1])
                        * rect.height()
                        * zoom
                        / WORLD_SIZE;
                assert!((roundtrip - point).length() < 0.002);
            }
        }
    }
}

/// Distance-spaced stamps make a drag independent of event/frame frequency.
/// UI-only state: never serialized into an experiment or exposed to agents.
pub struct FoodBrush {
    pub radius: f32,
    pub density: f32,
    pub enabled: bool,
    previous: Option<egui::Pos2>,
    remaining: f32,
}
impl Default for FoodBrush {
    fn default() -> Self {
        Self {
            radius: 36.0,
            density: 1.0,
            enabled: false,
            previous: None,
            remaining: 0.0,
        }
    }
}
impl FoodBrush {
    pub fn toggle(&mut self) {
        self.enabled = !self.enabled;
        self.end();
    }
    pub fn density_slider(&mut self, x: f32, rect: egui::Rect) {
        let track = rect.shrink2(egui::vec2(8.0, 0.0));
        let t = ((x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
        self.density = 0.1 + t * 3.9;
    }
    pub fn slider(&mut self, x: f32, rect: egui::Rect) {
        let track = rect.shrink2(egui::vec2(8.0, 0.0));
        let t = ((x - track.left()) / track.width().max(1.0)).clamp(0.0, 1.0);
        self.radius = 8.0 + t * 232.0;
        self.remaining = self.remaining.min(self.radius * 0.35);
    }
    pub fn end(&mut self) {
        self.previous = None;
        self.remaining = 0.0;
    }
    pub fn active(&self) -> bool {
        self.previous.is_some()
    }
    pub fn resize(&mut self, notches: f32) {
        if notches.is_finite() {
            self.radius =
                (self.radius * 1.2_f32.powf(notches.clamp(-20.0, 20.0))).clamp(8.0, 240.0);
            self.remaining = self.remaining.min(self.radius * 0.35);
        }
    }
    pub fn stamps(&mut self, point: egui::Pos2) -> Vec<egui::Pos2> {
        let Some(previous) = self.previous.replace(point) else {
            self.remaining = self.radius * 0.35;
            return vec![point];
        };
        let delta = point - previous;
        let distance = delta.length();
        if distance <= f32::EPSILON {
            return Vec::new();
        }
        let mut stamps = Vec::new();
        let mut along = self.remaining;
        while along <= distance {
            stamps.push(previous + delta * (along / distance));
            along += self.radius * 0.35;
        }
        self.remaining = along - distance;
        stamps
    }
}

#[cfg(test)]
mod food_brush_tests {
    use super::*;
    #[test]
    fn drag_stamps_are_independent_of_mouse_event_frequency() {
        let stroke = |points: Vec<f32>| {
            let mut brush = FoodBrush::default();
            points
                .into_iter()
                .flat_map(|x| brush.stamps(egui::pos2(x, 20.0)))
                .collect::<Vec<_>>()
        };
        let fast = stroke(vec![0.0, 300.0]);
        let slow = stroke((0..=300).map(|x| x as f32).collect());
        assert_eq!(fast.len(), slow.len());
        for (a, b) in fast.iter().zip(slow) {
            assert!((*a - b).length() < 0.001);
        }
        assert!(fast.windows(2).all(|p| (p[1] - p[0]).length() < 13.0));
    }
    #[test]
    fn toggle_release_and_size_controls_do_not_leave_a_stale_stroke() {
        let mut brush = FoodBrush::default();
        assert!(!brush.enabled);
        brush.toggle();
        assert!(brush.enabled);
        assert_eq!(brush.stamps(egui::pos2(0.0, 0.0)).len(), 1);
        assert!(brush.stamps(egui::pos2(0.0, 0.0)).is_empty());
        brush.toggle();
        assert!(!brush.active());
        brush.resize(1.0);
        assert!((brush.radius - 43.2).abs() < 0.001);
        brush.resize(-1.0);
        assert!((brush.radius - 36.0).abs() < 0.001);
        let rect = egui::Rect::from_min_size(egui::pos2(20.0, 0.0), egui::vec2(160.0, 24.0));
        brush.slider(-100.0, rect);
        assert_eq!(brush.radius, 8.0);
        brush.slider(999.0, rect);
        assert_eq!(brush.radius, 240.0);
        brush.slider(100.0, rect);
        assert_eq!(brush.radius, 124.0);
        brush.density_slider(-100.0, rect);
        assert_eq!(brush.density, 0.1);
        brush.density_slider(999.0, rect);
        assert_eq!(brush.density, 4.0);
        assert_eq!(
            brush.radius, 124.0,
            "Density changes must not resize the brush"
        );
        brush.end();
        assert_eq!(brush.stamps(egui::pos2(1000.0, 0.0)).len(), 1);
    }
}
