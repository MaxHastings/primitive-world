//! Native, single-window, extinction-only survivor evolution and recovery saves.
use crate::{
    simulation::{SimSettings, Simulation},
    survivor_observer::{self, SurvivorSample},
};
use std::{
    io::Write,
    path::{Path, PathBuf},
};

pub struct VisibleTrial {
    pub directory: PathBuf,
    pub finished: bool,
    latest: Option<SurvivorSample>,
    initial_tick: u32,
    initial_settings: SimSettings,
    root: Option<PathBuf>,
    pub world_number: u64,
}

/// The between-world survivor archive is part of an experiment save, even when
/// the current world is extinct. A body checkpoint alone cannot restore it.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct LoopSnapshot {
    pub world_number: u64,
    pub latest: SurvivorSample,
}

impl LoopSnapshot {
    pub fn validate(&self) -> Result<(), String> {
        self.latest.bank.validate()?;
        if self.world_number == 0
            || self.latest.bank.genomes.len() > 64
            || self.latest.bank.genomes.len() != self.latest.bodies.len()
            || self.latest.source_population == 0
            || self.latest.source_population > crate::model::MAX_AGENTS as usize
            || self.latest.bodies.iter().any(|b| {
                !(0.0..=1.0).contains(&b.mutation_probability)
                    || !(0.0..=8.0).contains(&b.mutation_magnitude)
                    || b.observed_tick
                        .is_some_and(|tick| tick > self.latest.bank.source_tick)
            })
        {
            return Err("Invalid saved evolution archive".into());
        }
        Ok(())
    }
}

// Explicit between-world variation, with a versioned native PRNG.
// No action priors or within-life weight updates.
fn random_u64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut x = *state;
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}
fn random_unit(state: &mut u64) -> f32 {
    (random_u64(state) >> 40) as f32 / 16777216.0
}

fn replicate(
    sample: &SurvivorSample,
    seed: u64,
) -> Result<(crate::founders::FounderBank, serde_json::Value), String> {
    let parents = &sample.bank.genomes;
    sample.bank.validate()?;
    if sample.bodies.iter().any(|b| {
        !(0.0..=1.0).contains(&b.mutation_probability)
            || !(0.0..=8.0).contains(&b.mutation_magnitude)
    }) {
        return Err("Invalid survivor mutation requests".into());
    }
    if parents.is_empty() || parents.len() > 64 || parents.len() != sample.bodies.len() {
        return Err("Invalid survivor population; refusing a random fallback".into());
    }
    let mut genomes = parents.clone();
    let mut provenance: Vec<_> = (0..parents.len())
        .map(|i| {
            serde_json::json!({
        "parent":i,"kind":"exact","changed_weights":0})
        })
        .collect();
    let mut rng = seed;
    let mut order: Vec<_> = (0..parents.len()).collect();
    while genomes.len() < 256 {
        for i in (1..order.len()).rev() {
            let j = random_u64(&mut rng) as usize % (i + 1);
            order.swap(i, j);
        }
        for &i in &order {
            if genomes.len() == 256 {
                break;
            }
            let probability = sample.bodies[i].mutation_probability;
            let magnitude = sample.bodies[i].mutation_magnitude;
            let child: Vec<_> = parents[i]
                .iter()
                .map(|&v| {
                    if random_unit(&mut rng) < probability {
                        (v + (random_unit(&mut rng) * 2.0 - 1.0) * magnitude).clamp(-4.0, 4.0)
                    } else {
                        v
                    }
                })
                .collect();
            let changes = child
                .iter()
                .zip(&parents[i])
                .filter(|(a, b)| a != b)
                .count();
            genomes.push(child);
            provenance.push(
                serde_json::json!({"parent":i,"kind":"offspring_replica","changed_weights":changes,"mutation_probability":probability,"mutation_magnitude":magnitude}),
            );
        }
    }
    Ok((
        crate::founders::FounderBank {
            version: 6,
            model: crate::model::MODEL_ID.into(),
            name: format!(
                "native-survivors-seed{}-tick{}",
                sample.bank.source_seed, sample.bank.source_tick
            ),
            source_seed: sample.bank.source_seed,
            source_tick: sample.bank.source_tick,
            genomes,
        },
        serde_json::json!({"algorithm":"splitmix64-f32-v2-parent-controls","seed":seed,
            "mutation_controls":"Most recent sampled controller requests; zero before first decision","provenance":provenance,"source_bodies":sample.bodies}),
    ))
}

fn write_new(path: &Path, value: &impl serde::Serialize) -> Result<(), String> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    file.write_all(&serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}

impl VisibleTrial {
    pub fn new(
        directory: &Path,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<Self, String> {
        std::fs::create_dir(directory).map_err(|e| {
            format!(
                "{}: {e}; choose a new visible-world directory",
                directory.display()
            )
        })?;
        let mut trial = Self {
            directory: directory.into(),
            finished: false,
            latest: None,
            initial_tick: sim.tick,
            initial_settings: sim.settings.clone(),
            root: None,
            world_number: 1,
        };
        trial.observe(sim, d, q)?;
        if trial.latest.is_none() {
            return Err("Visible survivor worlds need an initial living population".into());
        }
        write_new(
            &directory.join("ready.json"),
            &serde_json::json!({
            "model":crate::model::MODEL_ID,"seed":sim.seed,"initial_tick":sim.tick,
            "ending":"extinction_only_or_user_close","tick_limit":null}),
        )?;
        Ok(trial)
    }

    pub fn new_loop(
        root: &Path,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<Self, String> {
        if let Some(parent) = root.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir(root)
            .map_err(|e| format!("{}: {e}; choose a new loop directory", root.display()))?;
        let mut trial = Self::new(&root.join("world-000001"), sim, d, q)?;
        write_new(
            &root.join("registration.json"),
            &serde_json::json!({
            "mode":"native_single_window_survivor_loop","model":crate::model::MODEL_ID,
            "build":env!("CARGO_PKG_VERSION"),"initial_seed":sim.seed,"initial_tick":sim.tick,
            "initial_settings":sim.settings,"tick_limit":null,"round_limit":null,
            "variation":"splitmix64-f32-v2-parent-controls; each sampled parent's latest mutation probability and magnitude",
            "selection":"rolling archive of up to 64 distinct observed bodies; current survivors first, earlier entries retained to fill vacancies",
            "ending":"extinction advances in place; user close saves and stops"}),
        )?;
        trial.root = Some(root.into());
        Ok(trial)
    }

    pub fn is_loop(&self) -> bool {
        self.root.is_some()
    }

    pub fn transfer_cohort_size(&self) -> Option<usize> {
        self.latest.as_ref().map(|sample| sample.bodies.len())
    }

    pub fn snapshot(&self) -> Result<LoopSnapshot, String> {
        let snapshot = LoopSnapshot {
            world_number: self.world_number,
            latest: self.latest.clone().ok_or("No saved survivor archive")?,
        };
        snapshot.validate()?;
        Ok(snapshot)
    }

    /// Each resume owns a new output session. Existing world reports are kept;
    /// the evolutionary world number and actual late survivors carry forward.
    pub fn resume_loop(
        root: &Path,
        snapshot: LoopSnapshot,
        sim: &Simulation,
    ) -> Result<Self, String> {
        snapshot.validate()?;
        if snapshot.latest.bank.source_seed != sim.seed
            || snapshot.latest.bank.source_tick > sim.tick
        {
            return Err("The saved survivor archive does not belong to this world".into());
        }
        if let Some(parent) = root.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::create_dir(root).map_err(|e| e.to_string())?;
        let directory = root.join(format!("world-{:06}", snapshot.world_number));
        std::fs::create_dir(&directory).map_err(|e| e.to_string())?;
        write_new(
            &directory.join("ready.json"),
            &serde_json::json!({
                "model": crate::model::MODEL_ID, "seed": sim.seed, "initial_tick": sim.tick,
                "world_number": snapshot.world_number, "resumed": true
            }),
        )?;
        Ok(Self {
            directory,
            finished: false,
            latest: Some(snapshot.latest),
            initial_tick: sim.tick,
            initial_settings: sim.settings.clone(),
            root: Some(root.into()),
            world_number: snapshot.world_number,
        })
    }

    pub fn autosave(
        &self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<PathBuf, String> {
        let root = self
            .root
            .as_ref()
            .ok_or("Autosave requires a native loop")?;
        let folder = root.join("checkpoints");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let stem = format!(
            "world-{:06}-seed{}-tick{}-{stamp}",
            self.world_number, sim.seed, sim.tick
        );
        let pending = folder.join(format!("{stem}.partial"));
        let complete = folder.join(format!("{stem}.checkpoint"));
        sim.save_checkpoint(d, q, &pending)?;
        // Only completed files get the loadable suffix. Interrupted writes remain
        // explicitly partial and never replace the previous successful autosave.
        std::fs::OpenOptions::new()
            .write(true)
            .open(&pending)
            .map_err(|e| e.to_string())?
            .sync_all()
            .map_err(|e| e.to_string())?;
        std::fs::rename(&pending, &complete).map_err(|e| e.to_string())?;
        Ok(complete)
    }

    /// Reuse this Simulation and its GPU buffers. Never reconstruct the window,
    /// renderer, camera, speed controls, or other presentation state.
    pub fn advance(
        &mut self,
        sim: &mut Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<(), String> {
        let root = self.root.clone().ok_or("Not an in-place loop")?;
        if sim.metrics(d, q)?.living != 0 {
            return Err("Cannot advance a living world".into());
        }
        self.finish(sim, d, q, false)?;
        if sim.settings.population == 0 {
            return Err(
                "Initial bodies is zero; loop paused instead of spawning empty worlds".into(),
            );
        }
        let next_number = self
            .world_number
            .checked_add(1)
            .ok_or("World counter overflow")?;
        let next_dir = root.join(format!("world-{next_number:06}"));
        if next_dir.exists() {
            return Err("Next world directory exists; refusing to overwrite".into());
        }
        let sample = self
            .latest
            .as_ref()
            .ok_or("Missing actual survivor sample")?;
        let seed = ((sim.seed as u64) << 32) ^ sim.tick as u64 ^ next_number;
        let (bank, mut transfer) = replicate(sample, seed)?;
        transfer["source_directory"] = self.directory.to_string_lossy().to_string().into();
        write_new(&self.directory.join("next.bank.json"), &bank)?;
        write_new(&self.directory.join("transfer.json"), &transfer)?;
        let mut settings = sim.settings.clone();
        settings.founder_genomes = bank.genomes;
        settings.founder_name = bank.name;
        settings.validate()?;
        // Bijective progression gives distinct world seeds until the u32 seed space cycles.
        let next_seed = sim.seed.wrapping_add(0x9e3779b9);
        std::fs::create_dir(&next_dir).map_err(|e| e.to_string())?;
        write_new(
            &next_dir.join("ready.json"),
            &serde_json::json!({
            "model":crate::model::MODEL_ID,"seed":next_seed,"initial_tick":0,
            "ending":"extinction_only_or_user_close","tick_limit":null}),
        )?;
        // Complete fallible file preparation before touching the live world. Install
        // new observer ownership before readback so even a readback error can be saved.
        sim.settings = settings.clone();
        sim.seed = next_seed;
        sim.reset(q);
        *self = Self {
            directory: next_dir,
            finished: false,
            latest: None,
            initial_tick: 0,
            initial_settings: settings,
            root: Some(root),
            world_number: next_number,
        };
        self.observe(sim, d, q)
    }

    pub fn observe(
        &mut self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<(), String> {
        let mut current = None;
        survivor_observer::observe(&mut current, sim, d, q)?;
        if let Some(mut sample) = current {
            if let Some(previous) = &self.latest {
                sample.retain_previous(previous);
            }
            self.latest = Some(sample);
        }
        Ok(())
    }

    pub fn finish(
        &mut self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
        user_closed: bool,
    ) -> Result<(), String> {
        if self.finished {
            return Ok(());
        }
        let metrics = sim.metrics(d, q)?;
        if !user_closed && metrics.living != 0 {
            return Err("Refusing premature world termination: agents are still alive".into());
        }
        self.observe(sim, d, q)?;
        let sample = self.latest.as_ref().ok_or("No survivor archive")?;
        // Publish report last: it is the completion receipt, never a guess after a crash.
        write_new(&self.directory.join("survivors.bank.json"), sample)?;
        if user_closed {
            sim.save_checkpoint(d, q, &self.directory.join("paused.checkpoint"))?;
        }
        write_new(
            &self.directory.join("report.json"),
            &serde_json::json!({
            "model":crate::model::MODEL_ID,"checkpoint_version":16,"seed":sim.seed,
            "initial_tick":self.initial_tick,"elapsed_ticks":sim.tick-self.initial_tick,
            "tick_limit":null,"termination_reason":if user_closed {"user_closed"} else {"extinction"},
            "initial_settings":self.initial_settings,"final_settings":sim.settings,"end":metrics,
            "survivor_tick":sample.bank.source_tick,"sampled_bodies":sample.bodies.len(),
            "scope":"Interactive play, not a controlled benchmark. User physical interventions permitted. Closing stops the outer loop; only extinction authorizes transfer."}),
        )?;
        self.finished = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(probability: f32, magnitude: f32) -> SurvivorSample {
        let genome = vec![0.0; crate::model::GENOME_SIZE];
        SurvivorSample {
            bank: crate::founders::FounderBank {
                version: 6,
                model: crate::model::MODEL_ID.into(),
                name: "fixture".into(),
                source_seed: 1,
                source_tick: 2,
                genomes: vec![genome],
            },
            source_population: 1,
            bodies: vec![crate::survivor_observer::SampledBody {
                slot: 0,
                lineage_id: 1,
                parent_lineage: 0,
                ancestry_depth: 0,
                founder_family: 0,
                age: 1.0,
                energy: 1.0,
                food: 0.0,
                mutation_probability: probability,
                mutation_magnitude: magnitude,
                observed_tick: Some(2),
            }],
            selection: "fixture".into(),
        }
    }

    #[test]
    fn survivor_transfer_honors_parent_mutation_controls() {
        let (exact, _) = replicate(&sample(0.0, 8.0), 42).unwrap();
        assert!(
            exact
                .genomes
                .iter()
                .all(|genome| genome.iter().all(|&v| v == 0.0))
        );

        let (varied, transfer) = replicate(&sample(1.0, 0.5), 42).unwrap();
        assert!(
            varied
                .genomes
                .iter()
                .skip(1)
                .flatten()
                .all(|&v| (-0.5..=0.5).contains(&v))
        );
        assert!(
            varied
                .genomes
                .iter()
                .skip(1)
                .any(|genome| genome.iter().any(|&v| v != 0.0))
        );
        assert_eq!(transfer["algorithm"], "splitmix64-f32-v2-parent-controls");
    }
}
