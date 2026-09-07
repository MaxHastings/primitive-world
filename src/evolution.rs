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
    pub descendant_founders: u32,
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
            descendant_founders: 0,
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
            || self
                .descendant_founders
                .saturating_add(self.mutated_founders)
                .saturating_add(self.random_founders)
                > population
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
            (Phase::Incumbent, None)
                if self.descendant_founders == 0
                    && self.mutated_founders == 0
                    && self.random_founders == 0 => {}
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
fn proposal(
    incumbent: &[f32],
    descendants: &[f32],
    settings: &SimSettings,
    p: &mut Progress,
) -> Vec<f32> {
    let population = settings.population as usize;
    debug_assert_eq!(descendants.len() % GENOME_SIZE, 0);
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
    // A completed world's descendants are sampled without an individual score.
    // They occupy the ordinary 5% variation budget, so a candidate still changes
    // only a sparse part of the current population.
    let variation = (population / 20).max(1).min(population - random);
    let descendant = descendants.len() / GENOME_SIZE;
    let carried = descendant.min(variation);
    let mutated = if settings.mutation_probability > 0.0 && settings.mutation_magnitude > 0.0 {
        variation - carried
    } else {
        0
    };
    p.random_founders = random as u32;
    p.descendant_founders = carried as u32;
    p.mutated_founders = mutated as u32;
    for &slot in slots.iter().take(random) {
        result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
            .copy_from_slice(&brain::random_genome(&mut p.rng));
    }
    for (&slot, genome) in slots
        .iter()
        .skip(random)
        .take(carried)
        .zip(descendants.chunks_exact(GENOME_SIZE))
    {
        result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE].copy_from_slice(genome);
    }
    for &slot in slots.iter().skip(random + carried).take(mutated) {
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
    /// Uniformly sample terminal descendants before reset. This is deliberately
    /// not a survivor, birth, or behavior ranking: every terminal descendant
    /// slot has the same chance to become a founder in the next challenger.
    fn sample_terminal_descendants(
        &self,
        d: &wgpu::Device,
        q: &wgpu::Queue,
        limit: usize,
        rng: &mut u32,
    ) -> Result<Vec<f32>, String> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let agents = self.agent_snapshot(d, q)?;
        let mut slots: Vec<_> = agents
            .iter()
            .enumerate()
            .filter_map(|(slot, agent)| (agent.ancestry_depth > 0).then_some(slot))
            .collect();
        let count = limit.min(slots.len());
        for i in 0..count {
            let j = i + ((u64::from(next(rng)) * (slots.len() - i) as u64) >> 32) as usize;
            slots.swap(i, j);
        }
        slots.truncate(count);
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        let genome_bytes = (slots.len() * GENOME_SIZE * std::mem::size_of::<f32>()) as u64;
        let staging = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("terminal descendant genomes"),
            size: genome_bytes,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = d.create_command_encoder(&Default::default());
        for (destination, slot) in slots.into_iter().enumerate() {
            encoder.copy_buffer_to_buffer(
                &self.genome_buffer,
                (slot * GENOME_SIZE * std::mem::size_of::<f32>()) as u64,
                &staging,
                (destination * GENOME_SIZE * std::mem::size_of::<f32>()) as u64,
                GENOME_SIZE as u64 * std::mem::size_of::<f32>() as u64,
            );
        }
        q.submit(Some(encoder.finish()));
        let (tx, rx) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        d.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| e.to_string())?
            .map_err(|e| e.to_string())?;
        let genomes =
            bytemuck::cast_slice::<u8, f32>(&staging.slice(..).get_mapped_range()).to_vec();
        staging.unmap();
        for genome in genomes.chunks_exact(GENOME_SIZE) {
            brain::validate(genome)?;
        }
        Ok(genomes)
    }
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
        let population = self.settings.population as usize;
        let random = if p.comparison.is_multiple_of(4) {
            (population / 100).max(1)
        } else {
            0
        };
        let descendant_limit = (population / 20)
            .max(1)
            .min(population.saturating_sub(random));
        let descendants = if p.phase == Phase::Incumbent {
            self.sample_terminal_descendants(d, q, descendant_limit, &mut p.rng)?
        } else {
            Vec::new()
        };
        let outcome = p.completed.take().ok_or("Missing world outcome")?;
        let mut search = std::mem::take(&mut self.search);
        match p.phase {
            Phase::Incumbent => {
                search.challenger =
                    proposal(&search.incumbent, &descendants, &self.settings, &mut p);
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
                p.descendant_founders = 0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenger_can_carry_a_terminal_descendant_without_an_individual_score() {
        let mut settings = SimSettings {
            population: 20,
            ..Default::default()
        };
        settings.mutation_probability = 0.0;
        settings.mutation_magnitude = 0.0;
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let descendants = vec![1.0; GENOME_SIZE];
        let mut progress = Progress::initial(7);

        let challenger = proposal(&incumbent, &descendants, &settings, &mut progress);

        assert_eq!(progress.descendant_founders, 1);
        assert_eq!(progress.mutated_founders, 0);
        assert_eq!(progress.random_founders, 0);
        assert_eq!(
            challenger
                .chunks_exact(GENOME_SIZE)
                .filter(|genome| *genome == descendants.as_slice())
                .count(),
            1
        );
    }

    #[test]
    fn challenger_uses_mutation_when_no_descendant_is_available() {
        let settings = SimSettings {
            population: 20,
            mutation_probability: 1.0,
            mutation_magnitude: 0.5,
            ..Default::default()
        };
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let mut progress = Progress::initial(7);

        let challenger = proposal(&incumbent, &[], &settings, &mut progress);

        assert_eq!(progress.descendant_founders, 0);
        assert_eq!(progress.mutated_founders, 1);
        assert_ne!(challenger, incumbent);
    }
}
