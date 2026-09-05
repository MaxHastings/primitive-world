//! Fixed-pool, matched-environment experiments with resumable, extinction-only credit.
use crate::{
    headless,
    neural_selector::{Action, Learner},
    reproduction_archive::{Archive, Snapshot},
    save_files::write_new,
    simulation::{SimSettings, Simulation},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PausedWorld {
    pub checkpoint: PathBuf,
    pub archive: Snapshot,
}
/// The World representation preserves existing single-pool comparison saves.
#[derive(Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub(crate) enum CandidatePool {
    World(Snapshot),
    Retained(crate::candidate_pool::Pool),
}
impl CandidatePool {
    fn validate(&self) -> Result<(), String> {
        match self {
            Self::World(p) => p.validate(),
            Self::Retained(p) => p.validate(),
        }
    }
    pub fn records(&self) -> Vec<crate::life_record::LifeRecord> {
        match self {
            Self::World(p) => p.records(),
            Self::Retained(p) => p.records(),
        }
    }
    fn genomes(&self) -> Vec<Vec<f32>> {
        match self {
            Self::World(p) => p.sample().map_or_else(Vec::new, |s| s.bank.genomes),
            Self::Retained(p) => p.genomes(),
        }
    }
    fn len(&self) -> usize {
        match self {
            Self::World(p) => p.entries.len(),
            Self::Retained(p) => p.entries.len(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Comparison {
    version: u32,
    pub pool: CandidatePool,
    pub settings: SimSettings,
    pub seeds: Vec<u32>,
    pub learner: Learner,
    pub actions: Vec<Action>,
    /// Row = population; column = environment seed. None is unobserved, never a reward.
    pub durations: Vec<Vec<Option<u32>>>,
    pub paused: Option<PausedWorld>,
    pub completed: bool,
}
impl Comparison {
    pub fn new(
        pool: CandidatePool,
        settings: SimSettings,
        seeds: Vec<u32>,
        mut learner: Learner,
        count: usize,
    ) -> Result<Self, String> {
        let actions =
            learner.propose_comparison(&pool.records(), settings.population as usize, count)?;
        let state = Self {
            version: 1,
            pool,
            settings,
            durations: vec![vec![None; seeds.len()]; count],
            seeds,
            learner,
            actions,
            paused: None,
            completed: false,
        };
        state.validate()?;
        Ok(state)
    }

    pub fn complete(&mut self) -> Result<(), String> {
        self.validate()?;
        if self.completed {
            return Ok(());
        }
        if self.next().is_some() || self.paused.is_some() {
            return Err("Cannot train on an unfinished batch".into());
        }
        let ticks = self
            .durations
            .iter()
            .map(|r| r.iter().map(|t| t.unwrap()).collect())
            .collect::<Vec<_>>();
        self.learner.complete_comparison(&self.actions, &ticks)?;
        self.completed = true;
        self.validate()
    }

    pub fn next(&self) -> Option<(usize, usize)> {
        self.durations
            .iter()
            .enumerate()
            .find_map(|(i, row)| row.iter().position(Option::is_none).map(|j| (i, j)))
    }
    pub fn validate(&self) -> Result<(), String> {
        self.pool.validate()?;
        self.settings.validate()?;
        self.learner.validate()?;
        let mut seeds = std::collections::HashSet::new();
        if self.version != 1
            || self.seeds.len() < 2
            || self.seeds.len() > 32
            || !self.seeds.iter().all(|s| seeds.insert(s))
            || !(2..=32).contains(&self.actions.len())
            || self.durations.len() != self.actions.len()
            || self.durations.iter().any(|r| r.len() != self.seeds.len())
            || self.actions.iter().any(|a| {
                a.selected.len() != self.settings.population as usize
                    || a.selected.iter().any(|&i| i >= self.pool.len())
                    || a.gradient.len() != self.learner.weights.len()
                    || a.gradient.iter().any(|g| !g.is_finite())
                    || a.policy_version
                        + u64::from(
                            self.completed
                                && !self.learner.frozen
                                && self.learner.policy != crate::neural_selector::Policy::Uniform,
                        )
                        != self.learner.updates
            })
            || (self.completed && self.next().is_some())
        {
            return Err("Invalid matched-comparison state".into());
        }
        if let Some(paused) = &self.paused {
            paused.archive.validate()?;
            let (_, seed) = self.next().ok_or("Completed batch has a paused world")?;
            if paused.archive.source_seed != self.seeds[seed]
                || paused.archive.extinction_tick.is_some()
            {
                return Err("Paused comparison world does not match pending trial".into());
            }
        }
        Ok(())
    }
    pub fn world_settings(&self, population: usize, seed: u32) -> SimSettings {
        let mut settings = self.settings.clone();
        settings.founder_genomes = self.pool.genomes();
        settings.founder_slots = self.actions[population]
            .selected
            .iter()
            .map(|&i| i as u32)
            .collect();
        let mut rng = u64::from(seed) ^ 0x504c414345;
        for i in (1..settings.founder_slots.len()).rev() {
            let j = crate::neural_selector::random(&mut rng) as usize % (i + 1);
            settings.founder_slots.swap(i, j);
        }
        settings.founder_name = "Matched life-record comparison".into();
        settings
    }
    pub fn report(&self) -> serde_json::Value {
        let complete = self.durations.iter().all(|r| r.iter().all(Option::is_some));
        let means: Option<Vec<f64>> = complete.then(|| {
            self.durations
                .iter()
                .map(|r| r.iter().map(|t| f64::from(t.unwrap())).sum::<f64>() / r.len() as f64)
                .collect()
        });
        let contrasts = means.as_ref().map(|means| {
            (1..means.len()).map(|i| {
                let differences: Vec<_> = self.durations[i].iter().zip(&self.durations[0])
                    .map(|(a,b)| f64::from(a.unwrap())-f64::from(b.unwrap())).collect();
                let mean = differences.iter().sum::<f64>() / differences.len() as f64;
                let variance = differences.iter().map(|x| (x-mean).powi(2)).sum::<f64>() / (differences.len()-1) as f64;
                serde_json::json!({"population": i, "reference_population":0,
                    "mean_difference_ticks":mean, "paired_standard_error":(variance/differences.len() as f64).sqrt(),
                    "per_seed_differences":differences})
            }).collect::<Vec<_>>()
        });
        let copies: Vec<_> = self
            .actions
            .iter()
            .map(|a| {
                let mut counts = vec![0; self.pool.len()];
                for &i in &a.selected {
                    counts[i] += 1;
                }
                counts
            })
            .collect();
        serde_json::json!({"completed":self.completed,"seeds":self.seeds,"duration_ticks":self.durations,
            "mean_ticks":means,"paired_contrasts":contrasts,"copy_counts":copies,
            "updates":self.learner.updates,"frozen":self.learner.frozen,"policy":self.learner.policy,
            "scope":"One fixed pool; each population is tested on identical environment seeds. Paired contrasts describe this batch, not generalization. Null durations are unfinished and excluded from training; no partial-batch update. Use unseen pools and disjoint seeds for frozen evaluation."})
    }
}

pub fn run(options: &HashMap<String, String>, args: &[String]) -> Result<(), String> {
    let ticks: u64 = options
        .get("--ticks")
        .map_or(Ok(2000), |s| s.parse().map_err(|_| "Invalid tick budget"))?;
    if ticks == 0 {
        return Err("Tick budget must be positive".into());
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter =
        pollster::block_on(instance.request_adapter(&Default::default())).ok_or("No GPU")?;
    let hardware = adapter.get_info();
    let (d, q) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("matched selector experiments"),
            required_features: wgpu::Features::empty(),
            required_limits: adapter.limits(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .map_err(|e| e.to_string())?;
    let mut sim = Simulation::new(&d, &q, 1);
    let mut state = if let Some(path) = options.get("--comparison-resume") {
        let state: Comparison =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        state.validate()?;
        if state.completed {
            return Err(
                "This comparison is already complete; start a new pool/seed experiment".into(),
            );
        }
        state
    } else {
        headless::configure(&mut sim, args)?;
        let path = options
            .get("--candidate-pool")
            .ok_or("Matched comparisons require --candidate-pool ARCHIVE")?;
        let pool: Snapshot =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        pool.validate()?;
        let seeds: Vec<u32> = options
            .get("--comparison-seeds")
            .ok_or("Missing comparison seeds")?
            .split(',')
            .map(|s| {
                s.parse()
                    .map_err(|_| "Use comma-separated u32 comparison seeds")
            })
            .collect::<Result<_, _>>()?;
        let count: usize = options.get("--compositions").map_or(Ok(8), |s| {
            s.parse().map_err(|_| "Invalid composition count")
        })?;
        let learner = crate::selector_run::configure_selector(options, sim.seed)?;
        Comparison::new(
            CandidatePool::World(pool),
            sim.settings.clone(),
            seeds,
            learner,
            count,
        )?
    };
    state.validate()?;
    let root = Path::new(
        options
            .get("--train-loop")
            .ok_or("Missing output directory")?,
    );
    if root.exists() {
        return Err("Use a new comparison output directory".into());
    }
    std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let root = root.canonicalize().map_err(|e| e.to_string())?;
    write_new(&root.join("initial-comparison.json"), &state)?;
    let started = std::time::Instant::now();
    let mut work = 0u64;
    while let Some((population, seed_index)) = state.next() {
        if work >= ticks {
            break;
        }
        let seed = state.seeds[seed_index];
        let settings = state.world_settings(population, seed);
        let result = run_world(
            &mut sim,
            &d,
            &q,
            settings,
            seed,
            state.paused.take(),
            ticks - work,
            &root,
            &format!("population-{population}-seed-{seed}"),
        )?;
        work += result.work;
        let archive = result.archive;
        state.paused = result.paused;
        if state.paused.is_none() {
            let duration = archive
                .extinction_tick
                .ok_or("Missing exact extinction tick")?;
            state.durations[population][seed_index] = Some(duration);
            write_new(
                &root.join(format!("population-{population}-seed-{seed}.archive.json")),
                &archive,
            )?;
            eprintln!("Population {population}, seed {seed}: extinct at {duration} ticks");
            write_new(
                &root.join(format!("progress-{population}-{seed_index}.json")),
                &state,
            )?;
        }
        write_new(&root.join(format!("work-{work}.json")), &state)?;
    }
    if state.next().is_none() {
        state.complete()?;
    }
    state.validate()?;
    write_new(&root.join("comparison-state.json"), &state)?;
    write_new(&root.join("selector.json"), &state.learner)?;
    let mut report = state.report();
    report["build"] = env!("CARGO_PKG_VERSION").into();
    report["model"] = crate::model::MODEL_ID.into();
    report["gpu"] = hardware.name.into();
    report["backend"] = format!("{:?}", hardware.backend).into();
    report["os"] = std::env::consts::OS.into();
    report["executed_ticks"] = work.into();
    report["wall_seconds"] = started.elapsed().as_secs_f64().into();
    report["resume_state"] = root
        .join("comparison-state.json")
        .display()
        .to_string()
        .into();
    write_new(&root.join("comparison.json"), &report)?;
    eprintln!(
        "Comparison state: {}",
        root.join("comparison-state.json").display()
    );
    Ok(())
}

pub(crate) struct WorldResult {
    pub archive: Snapshot,
    pub paused: Option<PausedWorld>,
    pub work: u64,
}

/// Run until natural extinction, the compute budget, or a five-minute save point.
/// The caller saves the matching experiment state before continuing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_world(
    sim: &mut Simulation,
    d: &wgpu::Device,
    q: &wgpu::Queue,
    settings: SimSettings,
    seed: u32,
    paused: Option<PausedWorld>,
    budget: u64,
    root: &Path,
    label: &str,
) -> Result<WorldResult, String> {
    if budget == 0 {
        return Err("World needs a positive tick budget".into());
    }
    if let Some(paused) = paused {
        sim.load_checkpoint(q, &paused.checkpoint)?;
        if sim.seed != seed
            || sim.tick != paused.archive.source_tick
            || serde_json::to_value(&sim.settings).map_err(|e| e.to_string())?
                != serde_json::to_value(&settings).map_err(|e| e.to_string())?
        {
            return Err("Paused checkpoint does not match experiment settings/seed/tick".into());
        }
        sim.reproduction_archive = Some(Archive::new(d, q, sim, &paused.archive)?);
    } else {
        sim.settings = settings;
        sim.seed = seed;
        sim.reset(q);
        let archive = Snapshot::initial(sim, d, q)?;
        sim.reproduction_archive = Some(Archive::new(d, q, sim, &archive)?);
    }
    let started = std::time::Instant::now();
    let mut work = 0;
    let mut living = 1;
    while work < budget && living != 0 && started.elapsed().as_secs() < 300 {
        if sim.tick == u32::MAX {
            return Err("World tick capacity reached without extinction".into());
        }
        // Align readbacks to world ticks so budget pauses preserve final archive
        // observation ticks and charged work when the experiment resumes.
        let n = (budget - work)
            .min(u64::from(32 - sim.tick % 32))
            .min(u64::from(u32::MAX - sim.tick)) as u32;
        let mut e = d.create_command_encoder(&Default::default());
        sim.encode_ticks(&mut e, d, q, n);
        sim.copy_alive_count(&mut e);
        q.submit(Some(e.finish()));
        work += u64::from(n);
        living = sim
            .read_alive_count(d)
            .ok_or("Failed population readback; extinction cannot be inferred")?;
    }
    let archive = sim
        .reproduction_archive
        .as_ref()
        .unwrap()
        .snapshot(sim, d, q)?;
    let paused = if living == 0 {
        if archive.extinction_tick.is_none() {
            return Err("Missing exact extinction tick".into());
        }
        None
    } else {
        let checkpoint = root.join(format!("{label}-tick-{}.checkpoint", sim.tick));
        let partial = checkpoint.with_extension("partial");
        sim.save_checkpoint(d, q, &partial)?;
        std::fs::rename(&partial, &checkpoint).map_err(|e| e.to_string())?;
        Some(PausedWorld {
            checkpoint,
            archive: archive.clone(),
        })
    };
    Ok(WorldResult {
        archive,
        paused,
        work,
    })
}
