//! Desktop lifecycle for the fixed-brain evolutionary loop.
use crate::*;
use std::path::Path;

impl AppState {
    pub(crate) fn refresh_saves(&mut self) {
        self.ui.library_scan.request(experiments::save_root());
    }
    pub(crate) fn poll_saves(&mut self) {
        let Some(result) = self.ui.library_scan.poll() else {
            return;
        };
        match result {
            Ok((saves, invalid)) => {
                self.ui.saves = saves;
                self.ui.library_notice = if invalid > 0 {
                    format!(
                        "Skipped {invalid} incompatible or incomplete saves. Only current-format saves are listed."
                    )
                } else {
                    String::new()
                };
            }
            Err(e) => self.ui.library_notice = format!("Could not read saves: {e}"),
        }
    }
    pub(crate) fn save_experiment(&mut self) -> Result<String, String> {
        self.complete_batch(true);
        if self.experiment.is_none() {
            return Err("No active world to save".into());
        }
        if self.saved_revision == Some(self.world_revision) {
            return Ok("All progress saved".into());
        }
        let experiment = self.experiment.as_ref().unwrap();
        let path = experiment.save(&self.simulation, &self.device, &self.queue)?;
        self.checkpoint_path = path.to_string_lossy().into_owned();
        self.last_autosave = Instant::now();
        self.saved_revision = Some(self.world_revision);
        Ok(format!(
            "Saved {} at tick {}",
            experiment.name, self.simulation.tick
        ))
    }
    pub(crate) fn open_menu(&mut self) {
        self.paused = true;
        self.step_requested = false;
        if self.ui.has_world && self.ui.screen == ui::Screen::Play {
            match self.save_experiment() {
                Ok(message) => self.file_status = message,
                Err(e) => {
                    self.file_status = format!("Save failed; your world is still open: {e}");
                    return;
                }
            }
        }
        self.ui.screen = ui::Screen::Home;
    }
    fn prepare_replacement(&mut self) -> Result<(), String> {
        if self.ui.has_world && self.ui.screen == ui::Screen::Play {
            self.save_experiment()?;
        }
        self.paused = true;
        self.step_requested = false;
        Ok(())
    }
    fn activate_experiment(&mut self, experiment: experiments::Experiment) -> Result<(), String> {
        self.experiment = Some(experiment);
        self.world_revision = 0;
        self.saved_revision = None;
        self.clear_world_observers();
        self.refresh_metrics()?;
        self.renderer.camera.center = [WORLD_SIZE * 0.5; 2];
        self.renderer.camera.zoom = 1.0;
        self.renderer.camera.lens = Lens::Normal as u32;
        self.ui.tab = ui::Tab::Overview;
        self.ui.has_world = true;
        self.ui.screen = ui::Screen::Play;
        self.paused = true;
        self.file_status = self.save_experiment()?;
        Ok(())
    }
    pub(crate) fn refresh_metrics(&mut self) -> Result<(), String> {
        let m = self.simulation.metrics(&self.device, &self.queue)?;
        self.living_agents = m.living as u32;
        self.births = m.events[3];
        self.starvation_deaths = m.events[1];
        self.age_deaths = m.events[2];
        self.food_eaten = (m.food_ingested * 1000.0) as u64;
        self.interaction_stats = m.events[4..8].try_into().unwrap();
        self.history.push_back(m);
        Ok(())
    }
    pub(crate) fn new_experiment(&mut self) -> Result<(), String> {
        self.ui.setup.validate()?;
        if self.ui.setup.population == 0 {
            return Err("Start with at least one body".into());
        }
        self.prepare_replacement()?;
        let experiment = experiments::create(
            &self.ui.name,
            "World-duration population search · random, untrained brains",
        )?;
        self.simulation.settings = self.ui.setup.clone();
        self.simulation.use_random_founders();
        self.simulation.seed = self.ui.seed;
        self.simulation.reset(&self.queue);
        self.activate_experiment(experiment)?;
        self.paused = false;
        self.file_status =
            "Incumbent world running. Founding populations compete on completed world duration."
                .into();
        Ok(())
    }
    pub(crate) fn load_experiment(
        &mut self,
        saved: experiments::SavedExperiment,
    ) -> Result<(), String> {
        self.prepare_replacement()?;
        self.simulation.load_game_checkpoint(
            &self.queue,
            std::fs::File::open(saved.checkpoint()).map_err(|e| e.to_string())?,
            (saved.record.seed, saved.record.tick, saved.record.living),
            saved.record.world,
        )?;
        let experiment = saved.experiment();
        self.activate_experiment(experiment)
    }
    pub(crate) fn import_checkpoint(&mut self, path: &Path) -> Result<(), String> {
        self.load_experiment(experiments::read_record(path)?)
    }
    pub(crate) fn start_command_line_world(&mut self, args: &[String]) -> Result<(), String> {
        if let Some(i) = args.iter().position(|a| a == "--load-game") {
            return self.import_checkpoint(Path::new(&args[i + 1]));
        }
        let experiment = experiments::create(
            "New evolution",
            "World-duration population search · command line",
        )?;
        self.activate_experiment(experiment)?;
        self.paused = false;
        Ok(())
    }
}
