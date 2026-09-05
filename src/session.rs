//! Desktop experiment lifecycle, separate from simulation rules and diagnostics.
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
                        "Skipped {invalid} unreadable save receipts; earlier complete saves are shown."
                    )
                } else {
                    String::new()
                };
            }
            Err(error) => self.ui.library_notice = format!("Could not read experiments: {error}"),
        }
    }

    pub(crate) fn save_experiment(&mut self) -> Result<String, String> {
        if self.experiment.is_none() {
            return self.save_new_checkpoint();
        }
        if self.saved_revision == Some(self.world_revision) {
            return Ok("All progress saved".into());
        }
        // Refresh the rolling archive. Extinction preserves its last contents.
        let snapshot = if let Some(trial) = &mut self.visible_trial {
            trial.observe(&self.simulation, &self.device, &self.queue)?;
            Some(trial.snapshot()?)
        } else {
            None
        };
        let experiment = self.experiment.as_ref().expect("managed experiment");
        let path = experiment.save(&self.simulation, &self.device, &self.queue, snapshot)?;
        self.checkpoint_path = path.to_string_lossy().into_owned();
        self.last_autosave = Instant::now();
        self.saved_revision = Some(self.world_revision);
        Ok(format!(
            "Saved {} · world {} · tick {}",
            experiment.name,
            self.visible_trial
                .as_ref()
                .map_or(1, |trial| trial.world_number),
            self.simulation.tick
        ))
    }

    pub(crate) fn open_menu(&mut self) {
        self.paused = true;
        self.step_requested = false;
        if self.ui.has_world && self.ui.screen == ui::Screen::Play {
            match self.save_experiment() {
                Ok(message) => self.file_status = message,
                Err(error) => {
                    self.file_status = format!("Could not save; your world is still open: {error}");
                    return;
                }
            }
        }
        self.ui.screen = ui::Screen::Home;
        self.shock_mode = ShockMode::Select;
        // The current world already supplies Continue. Opening Load Game (or
        // explicit Refresh) requests a background scan; menu navigation doesn't.
    }

    fn prepare_replacement(&mut self) -> Result<(), String> {
        // Menu navigation already saved progress. Save again if invoked from
        // an active world so a failed import can always return to the library.
        if self.ui.has_world && self.ui.screen == ui::Screen::Play {
            self.save_experiment()?;
        }
        self.paused = true;
        self.step_requested = false;
        Ok(())
    }

    fn clear_replaced_session(&mut self) {
        self.ui.has_world = false;
        self.experiment = None;
        self.visible_trial = None;
        self.autosaved_state = None;
        self.world_revision = 0;
        self.saved_revision = None;
        self.clear_world_observers();
    }

    fn activate_experiment(
        &mut self,
        experiment: experiments::Experiment,
        snapshot: Option<visible_trial::LoopSnapshot>,
        continuous: bool,
    ) -> Result<(), String> {
        self.visible_trial = if let Some(snapshot) = snapshot {
            Some(visible_trial::VisibleTrial::resume_loop(
                &experiment.next_session()?,
                snapshot,
                &self.simulation,
            )?)
        } else if continuous {
            Some(visible_trial::VisibleTrial::new_loop(
                &experiment.next_session()?,
                &self.simulation,
                &self.device,
                &self.queue,
            )?)
        } else {
            None
        };
        self.experiment = Some(experiment);
        self.clear_world_observers();
        self.refresh_metrics()?;
        self.shock_mode = ShockMode::Select;
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
        self.food_eaten = m.events[0];
        self.interaction_stats = m.events[4..8].try_into().unwrap();
        self.history.push_back(m);
        Ok(())
    }

    pub(crate) fn new_experiment(&mut self) -> Result<(), String> {
        self.ui.setup.validate()?;
        if self.ui.setup.population == 0 {
            return Err("Start with at least one body.".into());
        }
        let experiment = experiments::create(&self.ui.name, "Random, untrained brains")?;
        self.prepare_replacement()?;
        self.clear_replaced_session();
        self.simulation.settings = self.ui.setup.clone();
        self.simulation.use_random_founders();
        self.simulation.seed = self.ui.seed;
        self.simulation.reset(&self.queue);
        self.activate_experiment(experiment, None, self.ui.continuous)?;
        // “Start evolution” should begin a watchable world immediately; loaded
        // worlds remain paused so resuming them is always deliberate.
        self.paused = false;
        self.file_status = "A fresh world is running. Space pauses it.".into();
        Ok(())
    }

    pub(crate) fn load_experiment(
        &mut self,
        saved: experiments::SavedExperiment,
        brains_only: bool,
    ) -> Result<(), String> {
        if let Some(snapshot) = &saved.record.evolution {
            snapshot.validate()?;
        }
        self.prepare_replacement()?;
        self.simulation.load_checkpoint_checked(
            &self.queue,
            std::fs::File::open(saved.checkpoint()).map_err(|e| e.to_string())?,
            Some((saved.record.seed, saved.record.tick, saved.record.living)),
        )?;
        self.clear_replaced_session();
        if brains_only {
            let experiment = experiments::create(
                &format!(
                    "{} — new line",
                    saved.record.name.chars().take(65).collect::<String>()
                ),
                &format!(
                    "Brains from {} · world {} · tick {}",
                    saved.record.name,
                    saved.world_number(),
                    saved.record.tick
                ),
            )?;
            self.use_current_brains(saved.record.evolution.as_ref())?;
            self.activate_experiment(experiment, None, true)
        } else {
            let continuous = saved.record.evolution.is_some();
            self.activate_experiment(saved.experiment(), saved.record.evolution, continuous)
        }
    }

    fn use_current_brains(
        &mut self,
        fallback: Option<&visible_trial::LoopSnapshot>,
    ) -> Result<(), String> {
        let mut latest = None;
        survivor_observer::observe(&mut latest, &self.simulation, &self.device, &self.queue)?;
        let sample = latest
            .as_ref()
            .or_else(|| fallback.map(|x| &x.latest))
            .ok_or("This world has no living brains or saved survivor archive to inherit.")?;
        self.simulation.settings.founder_genomes = sample.bank.genomes.clone();
        self.simulation.settings.founder_name = format!(
            "Inherited from seed {} tick {}",
            sample.bank.source_seed, sample.bank.source_tick
        );
        self.simulation.seed = experiments::stamp()? as u32;
        self.simulation.reset(&self.queue);
        Ok(())
    }

    pub(crate) fn import_checkpoint(&mut self, path: &Path) -> Result<(), String> {
        // A receipt imports a whole experiment, preserving its archive while
        // creating a separate local copy. A raw checkpoint starts a new record.
        let receipt = if path.extension().is_some_and(|x| x == "json") {
            Some(experiments::read_record(path)?)
        } else {
            None
        };
        self.prepare_replacement()?;
        let checkpoint = receipt
            .as_ref()
            .map_or_else(|| path.to_path_buf(), |x| x.checkpoint());
        self.simulation.load_checkpoint_checked(
            &self.queue,
            std::fs::File::open(&checkpoint).map_err(|e| e.to_string())?,
            receipt
                .as_ref()
                .map(|r| (r.record.seed, r.record.tick, r.record.living)),
        )?;
        self.clear_replaced_session();
        let name = receipt.as_ref().map_or_else(
            || {
                path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .chars()
                    .take(70)
                    .collect::<String>()
            },
            |x| x.record.name.clone(),
        );
        let mut experiment = experiments::create(&name, "Imported save")?;
        if let Some(receipt) = receipt {
            experiment.total_ticks = receipt.record.total_ticks;
            let continuous = receipt.record.evolution.is_some();
            self.activate_experiment(experiment, receipt.record.evolution, continuous)
        } else {
            let continuous = self.simulation.metrics(&self.device, &self.queue)?.living > 0;
            self.activate_experiment(experiment, None, continuous)
        }
    }
}
