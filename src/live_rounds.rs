//! The viewer runs the same round state machine as headless training, one live
//! world at a time. A save embeds its entire selector state and current archive.
use crate::{
    candidate_pool::Intake,
    neural_selector::Learner,
    reproduction_archive::{Archive, CAPACITY, Snapshot},
    selector_rounds::{Plan, Training},
    simulation::{SimSettings, Simulation},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub batches: u32,
    pub compositions: usize,
    pub environments: usize,
    pub retention: u32,
}

#[cfg(test)]
pub(crate) fn test_fixture(seed: u32, tick: u32, population: usize) -> Viewer {
    Viewer {
        version: 1,
        training: Training {
            version: 2,
            plan: Plan {
                rounds: 0,
                batches: 3,
                retention: 4,
                compositions: 3,
                seeds: vec![11, 22],
                bootstrap_seed: seed,
                settings: SimSettings {
                    population: population as u32,
                    ..SimSettings::default()
                },
            },
            round: 1,
            batch: 1,
            completed: false,
            comparison: None,
            bootstrap_learner: Some(Learner::new(1)),
            bootstrap_paused: None,
            bootstrap_duration: None,
            intake: Intake::new(64, 1),
            executed_ticks: u64::from(tick),
        },
        archive: crate::candidate_pool::tests::archive(seed, population, tick),
        last_batch: None,
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            batches: 3,
            compositions: 3,
            environments: 2,
            retention: 4,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Viewer {
    pub version: u32,
    pub training: Training,
    pub archive: Snapshot,
    pub last_batch: Option<serde_json::Value>,
}
impl Viewer {
    pub fn new(
        sim: &mut Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
        config: &Config,
    ) -> Result<Self, String> {
        if sim.tick != 0 {
            return Err("New evolution starts with a fresh world".into());
        }
        let settings = sim.settings.clone();
        let seed = sim.seed;
        if !(2..=32).contains(&config.environments) || !(1..=16).contains(&config.retention) {
            return Err("Use 2..32 environments and 1..16 retention rounds".into());
        }
        let plan = Plan {
            rounds: 0,
            batches: config.batches,
            compositions: config.compositions,
            retention: config.retention,
            settings,
            bootstrap_seed: seed,
            seeds: (1..=config.environments)
                .map(|i| seed.wrapping_add((i as u32).wrapping_mul(0x9e3779b9)))
                .collect(),
        };
        plan.validate()?;
        let training = Training {
            version: 2,
            plan,
            round: 1,
            batch: 1,
            completed: false,
            comparison: None,
            bootstrap_learner: Some(Learner::new(u64::from(seed) ^ 0x53454c454354)),
            bootstrap_paused: None,
            bootstrap_duration: None,
            intake: Intake::new(
                CAPACITY.div_ceil(config.retention as usize),
                u64::from(seed) ^ 0x504f4f4c,
            ),
            executed_ticks: 0,
        };
        let archive = Snapshot::initial(sim, d, q)?;
        sim.reproduction_archive = Some(Archive::new(d, q, sim, &archive)?);
        let state = Self {
            version: 1,
            training,
            archive,
            last_batch: None,
        };
        state.validate_world(sim.seed, sim.tick)?;
        Ok(state)
    }

    pub fn validate_world(&self, seed: u32, tick: u32) -> Result<(), String> {
        self.training.validate()?;
        self.archive.validate()?;
        let (_, expected_seed) = self.training.world()?;
        if self.version != 1
            || seed != expected_seed
            || self.archive.source_seed != seed
            || self.archive.source_tick != tick
            || self.training.bootstrap_paused.is_some()
            || self
                .training
                .comparison
                .as_ref()
                .is_some_and(|c| c.paused.is_some())
        {
            return Err("Round save does not match its current world".into());
        }
        Ok(())
    }

    pub fn expected_settings(&self) -> Result<SimSettings, String> {
        Ok(self.training.world()?.0)
    }

    pub fn snapshot(
        &self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<Self, String> {
        let mut state = self.clone();
        state.archive = sim
            .reproduction_archive
            .as_ref()
            .ok_or("Missing current life archive")?
            .snapshot(sim, d, q)?;
        state.validate_world(sim.seed, sim.tick)?;
        if serde_json::to_value(&state.expected_settings()?).map_err(|e| e.to_string())?
            != serde_json::to_value(&sim.settings).map_err(|e| e.to_string())?
        {
            return Err("World rules no longer match the saved round experiment".into());
        }
        Ok(state)
    }

    pub fn restore(
        self,
        sim: &mut Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<Self, String> {
        self.validate_world(sim.seed, sim.tick)?;
        if serde_json::to_value(&self.expected_settings()?).map_err(|e| e.to_string())?
            != serde_json::to_value(&sim.settings).map_err(|e| e.to_string())?
        {
            return Err("Checkpoint settings disagree with the round save".into());
        }
        sim.reproduction_archive = Some(Archive::new(d, q, sim, &self.archive)?);
        Ok(self)
    }

    /// The frontend durably saves the extinct world before this transition and
    /// saves its successor afterward. All learning and pool changes are staged.
    pub fn advance(
        &mut self,
        sim: &mut Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<(), String> {
        let mut next = self.snapshot(sim, d, q)?;
        if next.training.completed {
            return Ok(());
        }
        next.training.accept_world(&next.archive)?;
        if let Some(c) = &mut next.training.comparison
            && c.next().is_none()
        {
            c.complete()?;
            next.last_batch = Some(c.report());
            next.training.advance()?;
        }
        if !next.training.completed {
            let (settings, seed) = next.training.world()?;
            sim.settings = settings;
            sim.seed = seed;
            sim.reset(q);
            next.archive = Snapshot::initial(sim, d, q)?;
            sim.reproduction_archive = Some(Archive::new(d, q, sim, &next.archive)?);
        }
        *self = next;
        Ok(())
    }

    pub fn world_number(&self) -> u64 {
        let t = &self.training;
        let Some(c) = &t.comparison else {
            return 1;
        };
        let (p, e) = c.next().unwrap_or((c.actions.len() - 1, c.seeds.len() - 1));
        2 + (u64::from(t.round - 1) * u64::from(t.plan.batches) + u64::from(t.batch - 1))
            * t.plan.compositions as u64
            * t.plan.seeds.len() as u64
            + (p * t.plan.seeds.len() + e) as u64
    }

    pub fn position(&self) -> String {
        let t = &self.training;
        let Some(c) = &t.comparison else {
            return "Initial world · collecting the first pool".into();
        };
        let (p, e) = c.next().unwrap_or((c.actions.len() - 1, c.seeds.len() - 1));
        format!(
            "Round {} · batch {}/{} · population {}/{} · environment {}/{}",
            t.round,
            t.batch,
            t.plan.batches,
            p + 1,
            c.actions.len(),
            e + 1,
            c.seeds.len()
        )
    }
    pub fn pool_size(&self) -> usize {
        self.training
            .comparison
            .as_ref()
            .map_or(0, |c| c.pool.records().len())
    }
}
