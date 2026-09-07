//! Two founding populations, one matched environment, completed world duration only.
use crate::{
    brain,
    model::*,
    simulation::{Simulation, observability::read_buffer},
};
use serde::{Deserialize, Serialize};
pub const HISTORY_LIMIT: usize = 64;
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Incumbent,
    Challenger,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub world: u64,
    pub comparison: u64,
    pub population_id: u64,
    pub parent_population_id: Option<u64>,
    pub phase: Phase,
    pub seed: u32,
    pub duration: u32,
    pub births: u32,
    pub maximum_generation: u32,
    pub food_ingested: f64,
    pub food_collected: f64,
    pub challenger_accepted: Option<bool>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub world: u64,
    pub comparison: u64,
    pub rng: u32,
    pub phase: Phase,
    pub incumbent_id: u64,
    pub incumbent_parent_id: Option<u64>,
    pub accepted_challengers: u64,
    pub mutated_founders: u32,
    pub random_founders: u32,
    pub baseline: Option<Outcome>,
    pub completed: Option<Outcome>,
    pub history: Vec<Outcome>,
}
impl Progress {
    pub fn initial(seed: u32) -> Self {
        Self {
            world: 1,
            comparison: 1,
            rng: seed ^ 0x61c88647,
            phase: Phase::Incumbent,
            incumbent_id: 1,
            incumbent_parent_id: None,
            accepted_challengers: 0,
            mutated_founders: 0,
            random_founders: 0,
            baseline: None,
            completed: None,
            history: Vec::new(),
        }
    }
    pub fn population_id(&self) -> u64 {
        match self.phase {
            Phase::Incumbent => self.incumbent_id,
            Phase::Challenger => self.comparison + 1,
        }
    }
    pub fn parent_id(&self) -> Option<u64> {
        match self.phase {
            Phase::Incumbent => self.incumbent_parent_id,
            Phase::Challenger => Some(self.incumbent_id),
        }
    }
    pub fn validate(&self, population: u32, seed: u32, tick: u32) -> Result<(), String> {
        let bad = |o: &Outcome| {
            o.world == 0
                || o.comparison == 0
                || o.population_id == 0
                || o.duration == 0
                || o.duration > MAX_WORLD_TICKS
                || o.maximum_generation > o.births
                || o.comparison != o.world / 2 + o.world % 2
                || o.population_id > o.comparison.saturating_add(1)
                || (o.phase == Phase::Incumbent && o.population_id > o.comparison)
                || (o.phase == Phase::Challenger
                    && o.population_id != o.comparison.saturating_add(1))
                || (o.phase == Phase::Incumbent) != (o.world % 2 == 1)
                || (o.phase == Phase::Incumbent) != o.challenger_accepted.is_none()
                || o.parent_population_id
                    .is_some_and(|id| id == 0 || id >= o.population_id)
                || !o.food_ingested.is_finite()
                || o.food_ingested < 0.0
                || !o.food_collected.is_finite()
                || o.food_collected < 0.0
        };
        let expected = self
            .comparison
            .checked_mul(2)
            .and_then(|w| w.checked_sub(u64::from(self.phase == Phase::Incumbent)));
        if self.comparison == 0
            || self.comparison == u64::MAX
            || expected != Some(self.world)
            || self.incumbent_id == 0
            || self.incumbent_id > self.comparison
            || self.accepted_challengers >= self.comparison
            || self.mutated_founders.saturating_add(self.random_founders) > population
            || self.history.len() > HISTORY_LIMIT
            || self.history.iter().any(bad)
            || self.history.windows(2).any(|w| w[0].world >= w[1].world)
            || self.history.iter().any(|o| o.world > self.world)
            || self
                .incumbent_parent_id
                .is_some_and(|id| id == 0 || id >= self.incumbent_id)
        {
            return Err("Invalid population search progress".into());
        }
        let last_world = self.world - u64::from(self.completed.is_none());
        if self.history.len() != last_world.min(HISTORY_LIMIT as u64) as usize
            || self
                .history
                .iter()
                .enumerate()
                .any(|(i, o)| o.world != last_world - self.history.len() as u64 + i as u64 + 1)
        {
            return Err("Incomplete population search history".into());
        }
        match (&self.phase, &self.baseline) {
            (Phase::Incumbent, None) if self.mutated_founders == 0 && self.random_founders == 0 => {
            }
            (Phase::Challenger, Some(b))
                if !bad(b)
                    && b.seed == seed
                    && b.phase == Phase::Incumbent
                    && b.comparison == self.comparison
                    && b.world.checked_add(1) == Some(self.world)
                    && b.population_id == self.incumbent_id
                    && b.parent_population_id == self.incumbent_parent_id
                    && self.history.iter().find(|o| o.world == b.world) == Some(b) => {}
            _ => return Err("Missing or mismatched paired baseline".into()),
        }
        if let Some(o) = &self.completed {
            if bad(o)
                || o.world != self.world
                || o.seed != seed
                || o.duration > tick
                || o.comparison != self.comparison
                || o.phase != self.phase
                || o.population_id != self.population_id()
                || o.parent_population_id != self.parent_id()
                || self.history.last() != Some(o)
            {
                return Err("Invalid completed world outcome".into());
            }
        } else if self.history.last().is_some_and(|o| o.world == self.world) {
            return Err("Current world outcome missing".into());
        }
        Ok(())
    }
}
/// CPU snapshots of founding groups only; no per-tick copies or population readbacks.
#[derive(Default)]
pub struct Search {
    pub incumbent: Vec<f32>,
    pub challenger: Vec<f32>,
}
impl Search {
    pub fn validate(&self, population: u32, phase: Phase) -> Result<(), String> {
        let n = population as usize * GENOME_SIZE;
        if self.incumbent.len() != n
            || self.challenger.len() != if phase == Phase::Challenger { n } else { 0 }
        {
            return Err("Population search genome length mismatch".into());
        }
        for g in self
            .incumbent
            .chunks_exact(GENOME_SIZE)
            .chain(self.challenger.chunks_exact(GENOME_SIZE))
        {
            brain::validate(g)?;
        }
        Ok(())
    }
}
fn next(rng: &mut u32) -> u32 {
    *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *rng
}
fn proposal(incumbent: &[f32], settings: &SimSettings, p: &mut Progress) -> Vec<f32> {
    let population = settings.population as usize;
    let mut result = incumbent.to_vec();
    let mut slots: Vec<_> = (0..population).collect();
    // Preserve combinations and multiplicities; perturb uniformly chosen positions.
    for i in (1..population).rev() {
        let j = ((u64::from(next(&mut p.rng)) * (i + 1) as u64) >> 32) as usize;
        slots.swap(i, j);
    }
    let random = if p.comparison.is_multiple_of(4) {
        (population / 100).max(1)
    } else {
        0
    };
    let mutated = if settings.mutation_probability > 0.0 && settings.mutation_magnitude > 0.0 {
        (population / 20).max(1).min(population - random)
    } else {
        0
    };
    p.random_founders = random as u32;
    p.mutated_founders = mutated as u32;
    for &slot in slots.iter().take(random) {
        result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
            .copy_from_slice(&brain::random_genome(&mut p.rng));
    }
    for &slot in slots.iter().skip(random).take(mutated) {
        let g = &mut result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE];
        let original = &incumbent[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE];
        brain::mutate(g, next(&mut p.rng), settings);
        // A proposal labelled mutated must differ, including near parameter bounds.
        if g == original {
            let start = next(&mut p.rng) as usize % GENOME_SIZE;
            for offset in 0..GENOME_SIZE {
                let k = (start + offset) % GENOME_SIZE;
                let step = settings.mutation_magnitude;
                let changed = (g[k] + if g[k] > 0.0 { -step } else { step }).clamp(-4.0, 4.0);
                if changed != g[k] {
                    g[k] = changed;
                    break;
                }
            }
            if g == original {
                p.mutated_founders -= 1;
            }
        }
    }
    result
}
impl Simulation {
    /// Idempotent natural completion. A pause or tick budget cannot complete a living world.
    pub fn complete_world(&mut self, d: &wgpu::Device, q: &wgpu::Queue) -> Result<(), String> {
        if self.progress.completed.is_some() {
            return Ok(());
        }
        let m = self.metrics(d, q)?;
        if m.living != 0 {
            return Err("The world is still populated; no completed score".into());
        }
        let bytes = read_buffer(d, q, &self.death_stats_buffer)?;
        let counters: &[u32] = bytemuck::cast_slice(&bytes);
        if self.settings.population == 0 || counters[18] == 0 || counters[30] != 0 {
            return Err(
                "Only an unmodified world ending in natural extinction can be compared".into(),
            );
        }
        let accepted =
            self.progress.baseline.as_ref().map(|b| {
                counters[18] > b.duration && self.search.challenger != self.search.incumbent
            });
        let outcome = Outcome {
            world: self.progress.world,
            comparison: self.progress.comparison,
            population_id: self.progress.population_id(),
            parent_population_id: self.progress.parent_id(),
            phase: self.progress.phase,
            seed: self.seed,
            duration: counters[18],
            births: counters[3],
            maximum_generation: counters[23],
            food_ingested: m.food_ingested,
            food_collected: m.harvested,
            challenger_accepted: accepted,
        };
        self.progress.history.push(outcome.clone());
        if self.progress.history.len() > HISTORY_LIMIT {
            self.progress.history.remove(0);
        }
        self.progress.completed = Some(outcome);
        Ok(())
    }
    pub fn advance_world(&mut self, d: &wgpu::Device, q: &wgpu::Queue) -> Result<(), String> {
        self.complete_world(d, q)?;
        let world = self
            .progress
            .world
            .checked_add(1)
            .ok_or("World counter overflow")?;
        let comparison = self
            .progress
            .comparison
            .checked_add(1)
            .filter(|x| *x <= u64::MAX / 2)
            .ok_or("Comparison counter overflow")?;
        let mut p = self.progress.clone();
        let outcome = p.completed.take().ok_or("Missing world outcome")?;
        let mut search = std::mem::take(&mut self.search);
        match p.phase {
            Phase::Incumbent => {
                search.challenger = proposal(&search.incumbent, &self.settings, &mut p);
                p.baseline = Some(outcome);
                p.phase = Phase::Challenger;
                // Same seed, body locations, ages, resources and physical laws.
            }
            Phase::Challenger => {
                if outcome.challenger_accepted == Some(true) {
                    search.incumbent = std::mem::take(&mut search.challenger);
                    p.incumbent_parent_id = Some(p.incumbent_id);
                    p.incumbent_id = p.comparison + 1;
                    p.accepted_challengers += 1;
                } else {
                    search.challenger.clear();
                }
                p.comparison = comparison;
                p.phase = Phase::Incumbent;
                p.baseline = None;
                p.mutated_founders = 0;
                p.random_founders = 0;
                self.seed = next(&mut p.rng);
            }
        }
        p.world = world;
        self.settings.founder_genomes.clear();
        self.settings.founder_name = "world-duration population search".into();
        let founders = match p.phase {
            Phase::Incumbent => &search.incumbent,
            Phase::Challenger => &search.challenger,
        };
        self.reset_with_genomes(q, Some(founders));
        self.search = search;
        self.progress = p;
        Ok(())
    }
}
