//! UI commands are applied once, after egui finishes its layout passes and the
//! frame has been presented. Widgets never submit GPU work while drawing.
use crate::*;

pub enum Command {
    None,
    Home,
    NewGame,
    LoadGame,
    Create,
    Open(Box<experiments::SavedExperiment>, bool),
    Import,
    Save,
    Pause,
    Step,
    WorldClick(egui::Pos2),
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
) -> [f32; 2] {
    let delta = (point - rect.center()) * WORLD_SIZE / (rect.height().max(1.0) * zoom);
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
        Command::Open(saved, brains) => state.load_experiment(*saved, brains),
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
            if state.shock_mode != ShockMode::Select && state.paused {
                state.refresh_metrics()
            } else {
                return;
            }
        }
        Command::Pan(delta) => {
            let delta = delta * WORLD_SIZE
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
        .add_filter("Primitive World save", &["json", "checkpoint"])
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
    #[test]
    fn picks_and_brushes_use_the_world_viewport_at_every_zoom() {
        for rect in [
            egui::Rect::from_min_size(egui::pos2(0.0, 56.0), egui::vec2(920.0, 734.0)),
            egui::Rect::from_min_size(egui::pos2(0.0, 64.0), egui::vec2(530.0, 486.0)),
        ] {
            for zoom in [0.15, 1.0, 8.0, 80.0] {
                let center = [1200.0, 800.0];
                assert_eq!(world_position(rect, center, zoom, rect.center()), center);
                let point = rect.center() + egui::vec2(27.0, -16.0);
                let result = world_position(rect, center, zoom, point);
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
