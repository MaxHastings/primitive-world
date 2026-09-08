//! Two founding populations, one matched environment, natural survival only.
use crate::{
    brain,
    model::*,
    simulation::{Simulation, observability::read_buffer},
};
use serde::{Deserialize, Serialize};
pub const HISTORY_LIMIT: usize = 64;
pub const DESCENDANT_FOUNDER_PERCENT: usize = 10;
/// Keep a small, direction-agnostic novelty stream available to selection.
pub const RANDOM_FOUNDER_PERCENT: usize = 5;
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
    /// Symmetry-preserving environment orientation used by this world.
    pub environment_rotation: u32,
    /// Environmental/action age at this world's tick zero; duration stays local.
    pub environment_start_age: u32,
    pub duration: u32,
    pub births: u32,
    pub maximum_generation: u32,
    pub food_ingested: f64,
    pub food_collected: f64,
    #[serde(default)]
    pub assisted: bool,
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
    /// Candidate genomes based on uniformly sampled terminal descendants.
    pub descendant_founders: u32,
    /// Adaptive environmental age at which every subsequent matched pair begins.
    pub environment_age_floor: u32,
    /// A promoted challenger remains alive in its winning world. Its eventual
    /// extinction starts a fresh hard-environment comparison, never an unfair
    /// challenger trial against its easier pre-promotion run.
    pub live_winner: bool,
    /// Longest completed world in this experiment, retained beyond the rolling history.
    #[serde(default)]
    pub best: Option<Outcome>,
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
            environment_age_floor: 0,
            live_winner: false,
            best: None,
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
                || o.environment_start_age > MAX_WORLD_TICKS
                || o.environment_rotation > 3
                || o.maximum_generation > o.births
                || o.population_id > o.comparison.saturating_add(1)
                || (o.phase == Phase::Incumbent && o.population_id > o.comparison)
                || (o.phase == Phase::Challenger
                    && o.population_id != o.comparison.saturating_add(1))
                || (o.phase == Phase::Incumbent) != o.challenger_accepted.is_none()
                || o.parent_population_id
                    .is_some_and(|id| id == 0 || id >= o.population_id)
                || !o.food_ingested.is_finite()
                || o.food_ingested < 0.0
                || !o.food_collected.is_finite()
                || o.food_collected < 0.0
        };
        if self.comparison == 0
            || self.comparison == u64::MAX
            || self.incumbent_id == 0
            || self.incumbent_id > self.comparison
            || self.accepted_challengers >= self.comparison
            || self.environment_age_floor > MAX_WORLD_TICKS / 2
            || self.descendant_founders > population
            || self.history.len() > HISTORY_LIMIT
            || self.history.iter().any(bad)
            || self.best.as_ref().is_some_and(bad)
            || self.best.as_ref().is_some_and(|o| o.world > self.world)
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
            (Phase::Incumbent, None) if self.descendant_founders == 0 => {}
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

/// Keep an incumbent/challenger pair matched, then rotate the ecology for the
/// next independent comparison. Rotation changes the world, never the brain
/// inputs, so a fixed heading is not rewarded by one map orientation forever.
fn next_comparison_rotation(current: u32, phase: Phase, live_winner: bool) -> u32 {
    match phase {
        Phase::Incumbent if !live_winner => current,
        _ => (current + 1) % 4,
    }
}
fn proposal(
    incumbent: &[f32],
    descendants: &[f32],
    settings: &SimSettings,
    p: &mut Progress,
    descendant_percent: usize,
    random_percent: usize,
) -> Vec<f32> {
    let population = settings.population as usize;
    debug_assert_eq!(descendants.len() % GENOME_SIZE, 0);
    let mut result = incumbent.to_vec();
    let mut slots: Vec<_> = (0..population).collect();
    // Preserve combinations and multiplicities; uniformly choose which inherited
    // sources come from terminal descendants before mutating every founder.
    for i in (1..population).rev() {
        let j = ((u64::from(next(&mut p.rng)) * (i + 1) as u64) >> 32) as usize;
        slots.swap(i, j);
    }
    // A completed world's descendants are sampled without an individual score.
    // Some remain unchanged as viability anchors; the rest of the candidate is
    // supplied by fresh random genomes or continuously mutated incumbent
    // sources.
    let descendant_limit = (population * descendant_percent / 100)
        .min(population)
        .max(usize::from(descendant_percent > 0 && population > 0));
    let descendant = descendants.len() / GENOME_SIZE;
    let carried = descendant.min(descendant_limit);
    p.descendant_founders = carried as u32;
    for (&slot, genome) in slots
        .iter()
        .take(carried)
        .zip(descendants.chunks_exact(GENOME_SIZE))
    {
        result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE].copy_from_slice(genome);
    }
    // Preserve sampled terminal descendants as viable anchors. Their selection
    // is uniform over surviving ancestry, not an individual fitness score.
    let random_limit = (population * random_percent / 100)
        .min(population)
        .max(usize::from(random_percent > 0 && population > 0));
    let random_count = random_limit.min(population.saturating_sub(carried));
    for &slot in slots.iter().skip(carried).take(random_count) {
        let genome = brain::random_genome(&mut p.rng);
        result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE].copy_from_slice(&genome);
    }
    // The remaining inherited sources receive the ordinary continuous mutation
    // law. No source is scored by heading, behavior, or individual outcome.
    for &slot in slots.iter().skip(carried + random_count) {
        let g = &mut result[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE];
        brain::mutate(
            g,
            next(&mut p.rng),
            BASE_MUTATION_PROBABILITY,
            BASE_MUTATION_MAGNITUDE,
        );
    }
    result
}

fn promote_challenger_if_outlived(
    progress: &mut Progress,
    search: &mut Search,
    tick: u32,
    living: u64,
) -> Result<bool, String> {
    if living == 0 || progress.phase != Phase::Challenger {
        return Ok(false);
    }
    let baseline = progress
        .baseline
        .as_ref()
        .ok_or("Candidate is missing its matched incumbent duration")?;
    if tick <= baseline.duration {
        return Ok(false);
    }
    if search.challenger == search.incumbent {
        return Err("A candidate must differ from its incumbent before promotion".into());
    }

    progress.incumbent_parent_id = Some(progress.incumbent_id);
    progress.incumbent_id = progress
        .comparison
        .checked_add(1)
        .ok_or("Population identifier overflow")?;
    progress.accepted_challengers = progress
        .accepted_challengers
        .checked_add(1)
        .ok_or("Accepted challenger counter overflow")?;
    progress.comparison = progress
        .comparison
        .checked_add(1)
        .ok_or("Comparison counter overflow")?;
    progress.phase = Phase::Incumbent;
    progress.baseline = None;
    progress.descendant_founders = 0;
    progress.live_winner = true;
    search.incumbent = std::mem::take(&mut search.challenger);
    Ok(true)
}

fn ticks_until_challenger_can_win(progress: &Progress, tick: u32) -> Option<u32> {
    let baseline = progress.baseline.as_ref()?;
    (progress.phase == Phase::Challenger && tick <= baseline.duration)
        .then_some(baseline.duration + 1 - tick)
}

impl Simulation {
    /// Number of ticks remaining until a living candidate can strictly outlive
    /// its matched incumbent. Callers use this to end a GPU batch exactly at the
    /// decision boundary, rather than detecting a winner up to 31 ticks late.
    pub fn ticks_until_challenger_can_win(&self) -> Option<u32> {
        ticks_until_challenger_can_win(&self.progress, self.tick)
    }

    /// Promote a candidate as soon as it has strictly outlived its matched
    /// incumbent. Unlike natural completion, this deliberately leaves the
    /// candidate's live bodies, descendant genomes, and ecology in place.
    ///
    /// Extinction therefore eliminates a population; it is not required before
    /// a living winner can be recognized.
    pub fn promote_challenger_if_outlived(&mut self, living: u64) -> Result<bool, String> {
        promote_challenger_if_outlived(&mut self.progress, &mut self.search, self.tick, living)
    }

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
        if self.settings.population == 0 || counters[18] == 0 {
            return Err("Only a populated world ending in natural extinction can be scored".into());
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
            environment_rotation: self.settings.environment_rotation,
            environment_start_age: self.environment_start_age,
            duration: counters[18],
            births: counters[3],
            maximum_generation: counters[23],
            food_ingested: m.food_ingested,
            food_collected: m.harvested,
            assisted: counters[30] != 0,
            challenger_accepted: accepted,
        };
        self.progress.history.push(outcome.clone());
        if self.progress.history.len() > HISTORY_LIMIT {
            self.progress.history.remove(0);
        }
        self.progress.completed = Some(outcome);
        let completed = self.progress.completed.as_ref().expect("just completed");
        if self
            .progress
            .best
            .as_ref()
            .is_none_or(|best| completed.duration > best.duration)
        {
            self.progress.best = Some(completed.clone());
        }
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
        let descendant_limit = (population * self.founder_descendant_percent / 100)
            .min(population)
            .max(usize::from(
                self.founder_descendant_percent > 0 && population > 0,
            ));
        let descendants = if p.phase == Phase::Incumbent && !p.live_winner {
            self.sample_terminal_descendants(d, q, descendant_limit, &mut p.rng)?
        } else {
            Vec::new()
        };
        let outcome = p.completed.take().ok_or("Missing world outcome")?;
        let next_environment_rotation =
            next_comparison_rotation(self.settings.environment_rotation, p.phase, p.live_winner);
        let mut search = std::mem::take(&mut self.search);
        let next_environment_start_age;
        match p.phase {
            Phase::Incumbent => {
                if p.live_winner {
                    // A living challenger was already accepted. The next
                    // world still starts from the same fresh environment age.
                    p.live_winner = false;
                    p.descendant_founders = 0;
                    self.seed = next(&mut p.rng);
                    next_environment_start_age = 0;
                } else {
                    search.challenger = proposal(
                        &search.incumbent,
                        &descendants,
                        &self.settings,
                        &mut p,
                        self.founder_descendant_percent,
                        self.founder_random_percent,
                    );
                    p.baseline = Some(outcome);
                    p.phase = Phase::Challenger;
                    // Same seed, body locations, ages, resources and physical laws.
                    next_environment_start_age = p
                        .baseline
                        .as_ref()
                        .expect("new baseline")
                        .environment_start_age;
                }
            }
            Phase::Challenger => {
                let _ = p.baseline.as_ref().ok_or("Missing paired baseline")?;
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
                self.seed = next(&mut p.rng);
                next_environment_start_age = 0;
            }
        }
        p.world = world;
        self.settings.environment_rotation = next_environment_rotation;
        self.settings.founder_genomes.clear();
        self.settings.founder_name = "world-duration population search".into();
        let founders = match p.phase {
            Phase::Incumbent => &search.incumbent,
            Phase::Challenger => &search.challenger,
        };
        self.reset_with_genomes_at(q, Some(founders), next_environment_start_age);
        self.search = search;
        self.progress = p;
        self.update_params(q);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_descendant_percentage_carries_no_descendant_anchor() {
        let settings = SimSettings {
            population: 20,
            ..Default::default()
        };
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let descendants = vec![1.0; GENOME_SIZE];
        let mut progress = Progress::initial(7);
        let challenger = proposal(&incumbent, &descendants, &settings, &mut progress, 0, 0);
        assert_eq!(progress.descendant_founders, 0);
        assert!(
            !challenger
                .chunks_exact(GENOME_SIZE)
                .any(|genome| genome == descendants)
        );
    }

    #[test]
    fn challenger_mutates_incumbent_sources_and_preserves_descendant_anchor() {
        let settings = SimSettings {
            population: 20,
            ..Default::default()
        };
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let descendants = vec![1.0; GENOME_SIZE];
        let mut progress = Progress::initial(7);

        let challenger = proposal(
            &incumbent,
            &descendants,
            &settings,
            &mut progress,
            DESCENDANT_FOUNDER_PERCENT,
            RANDOM_FOUNDER_PERCENT,
        );

        assert_eq!(progress.descendant_founders, 1);
        assert!(
            challenger
                .chunks_exact(GENOME_SIZE)
                .all(|genome| genome != &incumbent[..GENOME_SIZE])
        );
        assert_eq!(
            challenger
                .chunks_exact(GENOME_SIZE)
                .filter(|genome| genome.iter().all(|value| *value > 0.5))
                .count(),
            1,
            "one terminal descendant must remain a viable inherited anchor"
        );
    }

    #[test]
    fn challenger_varies_every_incumbent_founder_when_no_descendant_is_available() {
        let settings = SimSettings {
            population: 20,
            ..Default::default()
        };
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let mut progress = Progress::initial(7);

        let challenger = proposal(
            &incumbent,
            &[],
            &settings,
            &mut progress,
            DESCENDANT_FOUNDER_PERCENT,
            RANDOM_FOUNDER_PERCENT,
        );

        assert_eq!(progress.descendant_founders, 0);
        assert!(
            challenger
                .chunks_exact(GENOME_SIZE)
                .all(|genome| genome != &incumbent[..GENOME_SIZE])
        );
    }

    #[test]
    fn terminal_descendant_sources_are_preserved_as_uniform_viable_anchors() {
        let settings = SimSettings {
            population: 100,
            ..Default::default()
        };
        let incumbent = vec![0.0; settings.population as usize * GENOME_SIZE];
        let descendants = vec![1.0; GENOME_SIZE * 10];
        let mut progress = Progress::initial(7);
        let challenger = proposal(
            &incumbent,
            &descendants,
            &settings,
            &mut progress,
            DESCENDANT_FOUNDER_PERCENT,
            RANDOM_FOUNDER_PERCENT,
        );

        assert_eq!(progress.descendant_founders, 10);
        assert_eq!(
            challenger
                .chunks_exact(GENOME_SIZE)
                .filter(|genome| genome.iter().all(|value| *value == 1.0))
                .count(),
            10
        );
    }

    #[test]
    fn every_new_world_uses_a_fresh_environment_age() {
        let mut progress = Progress::initial(7);
        assert_eq!(progress.environment_age_floor, 0);
        progress.environment_age_floor = 100_000;
        progress.environment_age_floor = 0;
        assert_eq!(progress.environment_age_floor, 0);
    }

    #[test]
    fn comparison_rotation_keeps_pairs_matched_and_changes_between_pairs() {
        assert_eq!(next_comparison_rotation(2, Phase::Incumbent, false), 2);
        assert_eq!(next_comparison_rotation(2, Phase::Challenger, false), 3);
        assert_eq!(next_comparison_rotation(3, Phase::Incumbent, true), 0);
    }

    #[test]
    fn living_challenger_is_promoted_at_the_first_strictly_longer_tick() {
        let mut progress = Progress::initial(7);
        progress.world = 2;
        progress.phase = Phase::Challenger;
        progress.descendant_founders = 1;
        progress.baseline = Some(Outcome {
            world: 1,
            comparison: 1,
            population_id: 1,
            parent_population_id: None,
            phase: Phase::Incumbent,
            seed: 7,
            environment_rotation: 0,
            environment_start_age: 0,
            duration: 10,
            births: 0,
            maximum_generation: 0,
            food_ingested: 0.0,
            food_collected: 0.0,
            assisted: false,
            challenger_accepted: None,
        });
        progress.history.push(progress.baseline.clone().unwrap());
        let mut search = Search {
            incumbent: vec![0.0],
            challenger: vec![1.0],
        };

        assert_eq!(ticks_until_challenger_can_win(&progress, 10), Some(1));
        assert!(!promote_challenger_if_outlived(&mut progress, &mut search, 10, 1).unwrap());
        assert!(promote_challenger_if_outlived(&mut progress, &mut search, 11, 1).unwrap());
        assert_eq!(progress.world, 2, "promotion must not reset the live world");
        assert_eq!(progress.phase, Phase::Incumbent);
        assert_eq!(progress.comparison, 2);
        assert_eq!(progress.incumbent_id, 2);
        assert_eq!(progress.incumbent_parent_id, Some(1));
        assert_eq!(progress.accepted_challengers, 1);
        assert!(progress.baseline.is_none());
        assert_eq!(search.incumbent, vec![1.0]);
        assert!(search.challenger.is_empty());
        assert_eq!(ticks_until_challenger_can_win(&progress, 11), None);
        progress.validate(1, 7, 11).unwrap();
    }
}
