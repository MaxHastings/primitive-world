//! Repeated matched trials with scheduled, behavior-independent pool renewal.
use crate::{
    candidate_pool::{Intake, Pool, Source},
    headless,
    neural_selector::Learner,
    reproduction_archive::{CAPACITY, Snapshot},
    save_files::write_new,
    selector_comparison::{CandidatePool, Comparison, PausedWorld, run_world},
    simulation::{SimSettings, Simulation},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Plan {
    pub rounds: u32,
    pub batches: u32,
    pub retention: u32,
    pub compositions: usize,
    pub seeds: Vec<u32>,
    pub bootstrap_seed: u32,
    pub settings: SimSettings,
}
impl Plan {
    pub fn validate(&self) -> Result<(), String> {
        self.settings.validate()?;
        let seeds: std::collections::HashSet<_> = self.seeds.iter().collect();
        if self.settings.population == 0
            || self.rounds > 100_000
            || !(1..=1000).contains(&self.batches)
            || !(1..=16).contains(&self.retention)
            || !(2..=32).contains(&self.compositions)
            || !(2..=32).contains(&self.seeds.len())
            || seeds.len() != self.seeds.len()
        {
            return Err("Rounds: use 1..100000 rounds, 1..1000 batches, 1..16 retention, 2..32 populations and distinct seeds".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Training {
    pub version: u32,
    pub plan: Plan,
    /// One-based coordinates. A completed run retains its final trained batch.
    pub round: u32,
    pub batch: u32,
    pub completed: bool,
    pub comparison: Option<Comparison>,
    /// Present only while waiting for the first unselected world to go extinct.
    pub bootstrap_learner: Option<Learner>,
    pub bootstrap_paused: Option<PausedWorld>,
    pub bootstrap_duration: Option<u32>,
    pub intake: Intake,
    pub executed_ticks: u64,
}
impl Training {
    pub fn accept_world(&mut self, archive: &Snapshot) -> Result<(), String> {
        if let Some(c) = &mut self.comparison {
            let (population, index) = c.next().ok_or("No outstanding trial")?;
            let seed = c.seeds[index];
            if c.paused.is_some() || seed != archive.source_seed {
                return Err("Completion does not match the active trial".into());
            }
            self.intake.observe(
                archive,
                Source {
                    round: self.round,
                    batch: self.batch,
                    population,
                    seed,
                },
            )?;
            c.durations[population][index] =
                Some(archive.extinction_tick.ok_or("No natural extinction")?);
        } else {
            if self.bootstrap_paused.is_some() || archive.source_seed != self.plan.bootstrap_seed {
                return Err("Completion does not match the initial world".into());
            }
            let pool = Pool::from_archive(archive)?;
            self.bootstrap_duration = archive.extinction_tick;
            let learner = self
                .bootstrap_learner
                .take()
                .ok_or("Missing initial selector")?;
            self.start(pool, learner)?;
        }
        self.validate()
    }

    pub fn world(&self) -> Result<(SimSettings, u32), String> {
        if let Some(c) = &self.comparison {
            let (population, index) = c.next().unwrap_or((c.actions.len() - 1, c.seeds.len() - 1));
            Ok((c.world_settings(population, c.seeds[index]), c.seeds[index]))
        } else {
            Ok((self.plan.settings.clone(), self.plan.bootstrap_seed))
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        self.plan.validate()?;
        self.intake.validate()?;
        if self.version != 2
            || (self.round == 0 || (self.plan.rounds != 0 && self.round > self.plan.rounds))
            || !(1..=self.plan.batches).contains(&self.batch)
            || self.intake.limit != CAPACITY.div_ceil(self.plan.retention as usize)
        {
            return Err("Invalid round-training format or coordinates".into());
        }
        if let Some(c) = &self.comparison {
            c.validate()?;
            let CandidatePool::Retained(pool) = &c.pool else {
                return Err("Rounds require a retained pool".into());
            };
            if self.bootstrap_learner.is_some()
                || self.bootstrap_paused.is_some()
                || c.seeds != self.plan.seeds
                || c.actions.len() != self.plan.compositions
                || serde_json::to_value(&c.settings).map_err(|e| e.to_string())?
                    != serde_json::to_value(&self.plan.settings).map_err(|e| e.to_string())?
                || pool.entries.iter().any(|p| {
                    p.admitted_round > self.round
                        || self.round - p.admitted_round >= self.plan.retention
                })
                || (self.completed
                    && (!c.completed
                        || self.round != self.plan.rounds
                        || self.batch != self.plan.batches))
                || self.intake.sources().any(|s| {
                    s.round != self.round
                        || s.batch == 0
                        || s.batch > self.batch
                        || s.population >= self.plan.compositions
                        || !self.plan.seeds.contains(&s.seed)
                        || (s.batch == self.batch
                            && c.durations[s.population][self
                                .plan
                                .seeds
                                .iter()
                                .position(|&seed| seed == s.seed)
                                .unwrap()]
                            .is_none())
                })
            {
                return Err(
                    "Round state disagrees with pool, batch, or incoming candidates".into(),
                );
            }
        } else {
            let learner = self
                .bootstrap_learner
                .as_ref()
                .ok_or("Missing initial selector")?;
            learner.validate()?;
            if self.round != 1
                || self.batch != 1
                || self.completed
                || self.intake.len() != 0
                || self.bootstrap_duration.is_some()
            {
                return Err("Invalid bootstrap state".into());
            }
            if let Some(p) = &self.bootstrap_paused {
                p.archive.validate()?;
                if p.archive.source_seed != self.plan.bootstrap_seed
                    || p.archive.extinction_tick.is_some()
                {
                    return Err("Bootstrap checkpoint belongs to another world".into());
                }
            }
        }
        Ok(())
    }

    pub fn start(&mut self, pool: Pool, learner: Learner) -> Result<(), String> {
        self.comparison = Some(Comparison::new(
            CandidatePool::Retained(pool),
            self.plan.settings.clone(),
            self.plan.seeds.clone(),
            learner,
            self.plan.compositions,
        )?);
        self.bootstrap_learner = None;
        Ok(())
    }

    /// Called after the complete batch receipt is durable. Also handles resuming
    /// that receipt, without crediting its rewards or refreshing its pool twice.
    pub fn advance(&mut self) -> Result<(), String> {
        let c = self.comparison.as_ref().ok_or("No batch to advance")?;
        if !c.completed {
            return Err("Cannot advance an unfinished batch".into());
        }
        if self.batch == self.plan.batches && self.round == self.plan.rounds {
            self.completed = true;
            return self.validate();
        }
        let c = self.comparison.take().unwrap();
        let CandidatePool::Retained(mut pool) = c.pool else {
            return Err("Missing round pool".into());
        };
        if self.batch == self.plan.batches {
            self.round = self.round.checked_add(1).ok_or("Round capacity reached")?;
            self.batch = 1;
            pool.refresh(&mut self.intake, self.round, self.plan.retention)?;
        } else {
            self.batch += 1;
        }
        self.start(pool, c.learner)?;
        self.validate()
    }

    pub fn learner(&self) -> &Learner {
        self.comparison
            .as_ref()
            .map_or_else(|| self.bootstrap_learner.as_ref().unwrap(), |c| &c.learner)
    }
}

fn number<T: std::str::FromStr>(
    options: &HashMap<String, String>,
    key: &str,
    default: T,
) -> Result<T, String> {
    options.get(key).map_or(Ok(default), |s| {
        s.parse().map_err(|_| format!("Invalid {key}"))
    })
}

#[cfg(test)]
#[path = "selector_round_tests.rs"]
mod tests;

pub fn run(options: &HashMap<String, String>, args: &[String]) -> Result<(), String> {
    let ticks: u64 = number(options, "--ticks", 2000)?;
    if ticks == 0 {
        return Err("Training tick budget must be positive".into());
    }
    let root = Path::new(
        options
            .get("--train-loop")
            .ok_or("Missing output directory")?,
    );
    if root.exists() {
        return Err("Use a new round-training output directory".into());
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).ok_or("No GPU")?;
    let hardware = adapter.get_info();
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("round-based selector training"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| e.to_string())?;
    let mut sim = Simulation::new(&d, &q, 1);
    let mut state = if let Some(path) = options.get("--round-resume") {
        let saved: Training =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("Expected round-training format 2: {e}"))?;
        saved.validate()?;
        if saved.completed {
            return Err("These training rounds are already complete".into());
        }
        saved
    } else {
        headless::configure(&mut sim, args)?;
        let seeds = options
            .get("--comparison-seeds")
            .map_or("11,22", String::as_str)
            .split(',')
            .map(|s| {
                s.parse()
                    .map_err(|_| "Use comma-separated u32 comparison seeds".to_string())
            })
            .collect::<Result<Vec<_>, _>>()?;
        let plan = Plan {
            rounds: number(options, "--rounds", 8)?,
            batches: number(options, "--batches-per-round", 3)?,
            retention: number(options, "--pool-retention", 4)?,
            compositions: number(options, "--compositions", 3)?,
            seeds,
            bootstrap_seed: sim.seed,
            settings: sim.settings.clone(),
        };
        plan.validate()?;
        let learner = crate::selector_run::configure_selector(options, sim.seed)?;
        let intake = Intake::new(
            CAPACITY.div_ceil(plan.retention as usize),
            u64::from(sim.seed) ^ 0x504f4f4c,
        );
        let mut state = Training {
            version: 2,
            plan,
            round: 1,
            batch: 1,
            completed: false,
            comparison: None,
            bootstrap_learner: Some(learner),
            bootstrap_paused: None,
            bootstrap_duration: None,
            intake,
            executed_ticks: 0,
        };
        if let Some(path) = options.get("--candidate-pool") {
            let archive: Snapshot =
                serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                    .map_err(|e| format!("Expected a completed life-record archive: {e}"))?;
            let pool = Pool::from_archive(&archive)?;
            let learner = state.bootstrap_learner.take().unwrap();
            state.start(pool, learner)?;
        }
        state
    };
    state.validate()?;
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    write_new(&root.join("initial-round-state.json"), &state)?;
    let started = std::time::Instant::now();
    let mut work = 0;
    loop {
        // Finish durable transitions even if the last world used the remaining budget.
        if let Some(c) = &mut state.comparison
            && c.next().is_none()
            && !state.completed
        {
            c.complete()?;
            let label = format!("round-{}-batch-{}", state.round, state.batch);
            write_new(&root.join(format!("{label}.json")), &c.report())?;
            write_new(&root.join(format!("{label}-complete.state.json")), &state)?;
            state.advance()?;
            write_new(&root.join(format!("{label}-advanced.state.json")), &state)?;
        }
        if state.completed || work >= ticks {
            break;
        }
        let (settings, seed, paused, label) = if let Some(c) = &mut state.comparison {
            let (population, index) = c.next().ok_or("Missing next trial")?;
            let seed = c.seeds[index];
            (
                c.world_settings(population, seed),
                seed,
                c.paused.take(),
                format!(
                    "round-{}-batch-{}-population-{population}-seed-{seed}",
                    state.round, state.batch
                ),
            )
        } else {
            (
                state.plan.settings.clone(),
                state.plan.bootstrap_seed,
                state.bootstrap_paused.take(),
                "bootstrap".into(),
            )
        };
        let result = run_world(
            &mut sim,
            &d,
            &q,
            settings,
            seed,
            paused,
            ticks - work,
            &root,
            &label,
        )?;
        work += result.work;
        state.executed_ticks = state
            .executed_ticks
            .checked_add(result.work)
            .ok_or("Training tick counter overflow")?;
        let complete = result.paused.is_none();
        if let Some(c) = &mut state.comparison {
            c.paused = result.paused;
        } else {
            state.bootstrap_paused = result.paused;
        }
        if complete {
            state.accept_world(&result.archive)?;
            write_new(&root.join(format!("{label}.archive.json")), &result.archive)?;
            eprintln!(
                "{label}: extinct at {} ticks",
                result.archive.extinction_tick.unwrap()
            );
        }
        state.validate()?;
        write_new(&root.join(format!("work-{work}.state.json")), &state)?;
    }
    state.validate()?;
    let resume = root.join("round-state.json");
    write_new(&resume, &state)?;
    write_new(&root.join("selector.json"), state.learner())?;
    let report = serde_json::json!({
        "build":env!("CARGO_PKG_VERSION"), "model":crate::model::MODEL_ID,
        "gpu":hardware.name, "backend":format!("{:?}", hardware.backend), "os":std::env::consts::OS,
        "completed":state.completed, "round":state.round, "batch":state.batch, "plan":state.plan,
        "completed_rounds":if state.completed {state.round} else {state.round - 1},
        "updates":state.learner().updates, "policy":state.learner().policy, "frozen":state.learner().frozen,
        "executed_ticks":work, "total_executed_ticks":state.executed_ticks, "wall_seconds":started.elapsed().as_secs_f64(),
        "bootstrap_duration":state.bootstrap_duration, "bootstrap_pending":state.comparison.is_none(),
        "current_batch":state.comparison.as_ref().map(Comparison::report),
        "pool_size":state.comparison.as_ref().map(|c| c.pool.records().len()), "incoming_candidates":state.intake.len(),
        "resume_state":resume,
        "scope":"Round training on fixed training seeds; comparisons within each batch share one pool and environments. Only complete extinction batches update weights. Pool refresh is independent of reward, with equal-world sampling and age-limited records. Bootstrap is uncredited. Pauses receive no reward. Improvement and generalization require separate equal-budget controls and unseen pools/seeds."
    });
    write_new(&root.join("round-training.json"), &report)?;
    eprintln!("Round training state: {}", resume.display());
    Ok(())
}
