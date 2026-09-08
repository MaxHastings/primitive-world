//! Blind continuity through one fixed rolling hereditary reservoir.
//! Observers and completed-world reports have no causal path into heredity.
use crate::{
    model::*,
    simulation::{Simulation, observability::read_buffer},
};
use serde::{Deserialize, Serialize};

pub const HISTORY_LIMIT: usize = 64;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Outcome {
    pub world: u64,
    pub seed: u32,
    pub duration: u32,
    pub births: u32,
    pub maximum_generation: u32,
    pub food_ingested: f64,
    pub food_collected: f64,
    pub energy: f64,
    pub assisted: bool,
    pub reservoir_occupancy: u32,
    pub engine_saturated: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub world: u64,
    /// CPU RNG used solely to draw fresh founders from the reservoir.
    pub rng: u32,
    pub completed: Option<Outcome>,
    pub history: Vec<Outcome>,
    /// Capacity exhaustion pauses the engine; it is never an extinction.
    pub engine_saturated: bool,
}

impl Progress {
    pub fn initial(seed: u32) -> Self {
        Self {
            world: 1,
            rng: seed ^ 0x61c8_8647,
            completed: None,
            history: vec![],
            engine_saturated: false,
        }
    }
    pub fn validate(&self, _population: u32, seed: u32, tick: u32) -> Result<(), String> {
        let bad = |o: &Outcome| {
            o.world == 0
                || o.duration == 0
                || o.duration > MAX_WORLD_TICKS
                || o.maximum_generation > o.births
                || o.reservoir_occupancy != HEREDITARY_RESERVOIR_SIZE
                || !o.food_ingested.is_finite()
                || !o.food_collected.is_finite()
                || !o.energy.is_finite()
                || o.food_ingested < 0.0
                || o.food_collected < 0.0
                || o.energy < 0.0
                || o.engine_saturated
        };
        if self.world == 0
            || self.history.len() > HISTORY_LIMIT
            || self.history.iter().any(bad)
            || self.history.last().is_some_and(|o| {
                o.world > self.world || (o.world == self.world && self.completed.is_none())
            })
            || self.history.windows(2).any(|w| w[0].world >= w[1].world)
        {
            return Err("Invalid hereditary-reservoir progress".into());
        }
        if let Some(o) = &self.completed
            && (bad(o)
                || o.duration > tick
                || o.seed != seed
                || o.world != self.world
                || self.history.last() != Some(o))
        {
            return Err("Invalid completed-world observation".into());
        }
        Ok(())
    }
}

fn next(r: &mut u32) -> u32 {
    *r = r.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    *r
}

impl Simulation {
    pub fn complete_world(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        if self.refresh_engine_status(device, queue)? {
            return Err(
                "Engine capacity reached; this world cannot be completed or reseeded".into(),
            );
        }
        if self.progress.completed.is_some() {
            return Ok(());
        }
        let metrics = self.metrics(device, queue)?;
        if metrics.living != 0 {
            return Err("The world is still populated".into());
        }
        let bytes = read_buffer(device, queue, &self.death_stats_buffer)?;
        let counters: &[u32] = bytemuck::cast_slice(&bytes);
        if self.settings.population == 0 || counters[18] == 0 {
            return Err("Only natural extinction completes a world".into());
        }
        let outcome = Outcome {
            world: self.progress.world,
            seed: self.seed,
            duration: counters[18],
            births: counters[3],
            maximum_generation: counters[23],
            food_ingested: metrics.food_ingested,
            food_collected: metrics.harvested,
            energy: metrics.energy,
            assisted: counters[30] != 0,
            reservoir_occupancy: HEREDITARY_RESERVOIR_SIZE,
            engine_saturated: false,
        };
        self.progress.history.push(outcome.clone());
        if self.progress.history.len() > HISTORY_LIMIT {
            self.progress.history.remove(0);
        }
        self.progress.completed = Some(outcome);
        Ok(())
    }

    /// Start a fresh ecology with uniformly sampled, unchanged reservoir entries.
    pub fn advance_world(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        self.complete_world(device, queue)?;
        let count = self.settings.population as usize;
        let bank0 = read_buffer(device, queue, &self.reservoir_genome_buffers[0])?;
        let bank1 = read_buffer(device, queue, &self.reservoir_genome_buffers[1])?;
        let stored_traits = read_buffer(device, queue, &self.reservoir_traits_buffer)?;
        let bank0: &[f32] = bytemuck::cast_slice(&bank0);
        let bank1: &[f32] = bytemuck::cast_slice(&bank1);
        let stored_traits: &[CognitiveTraits] = bytemuck::cast_slice(&stored_traits);
        let mut genomes = vec![0.0; count * GENOME_SIZE];
        let mut traits = Vec::with_capacity(count);
        for row in 0..count {
            let slot = (next(&mut self.progress.rng) >> 20) as usize;
            genomes[row * GENOME_SIZE..row * GENOME_SIZE + GENOME_BANK_STRIDE].copy_from_slice(
                &bank0[slot * GENOME_BANK_STRIDE..(slot + 1) * GENOME_BANK_STRIDE],
            );
            genomes[row * GENOME_SIZE + GENOME_BANK_STRIDE..(row + 1) * GENOME_SIZE]
                .copy_from_slice(
                    &bank1[slot * GENOME_BANK_STRIDE
                        ..slot * GENOME_BANK_STRIDE + GENOME_SIZE - GENOME_BANK_STRIDE],
                );
            crate::brain::validate(&genomes[row * GENOME_SIZE..(row + 1) * GENOME_SIZE])?;
            if !stored_traits[slot].validate() {
                return Err("Invalid hereditary reservoir traits".into());
            }
            traits.push(stored_traits[slot]);
        }
        self.progress.world = self
            .progress
            .world
            .checked_add(1)
            .ok_or("World counter overflow")?;
        self.progress.completed = None;
        self.progress.engine_saturated = false;
        self.seed = next(&mut self.progress.rng);
        self.reset_live_world(queue, &genomes, &traits);
        Ok(())
    }

    pub fn refresh_engine_status(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<bool, String> {
        let bytes = read_buffer(device, queue, &self.death_stats_buffer)?;
        let counters: &[u32] = bytemuck::cast_slice(&bytes);
        self.progress.engine_saturated |= counters[36] != 0 || self.tick >= MAX_WORLD_TICKS;
        Ok(self.progress.engine_saturated)
    }

    pub fn record_engine_saturation(&mut self) {
        self.progress.engine_saturated = true;
    }

    #[cfg(test)]
    pub(crate) fn reservoir_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(Vec<f32>, Vec<CognitiveTraits>, u32), String> {
        let mut genomes = vec![0.0; HEREDITARY_RESERVOIR_SIZE as usize * GENOME_SIZE];
        let b0 = read_buffer(device, queue, &self.reservoir_genome_buffers[0])?;
        let b1 = read_buffer(device, queue, &self.reservoir_genome_buffers[1])?;
        let b0: &[f32] = bytemuck::cast_slice(&b0);
        let b1: &[f32] = bytemuck::cast_slice(&b1);
        for row in 0..HEREDITARY_RESERVOIR_SIZE as usize {
            genomes[row * GENOME_SIZE..row * GENOME_SIZE + GENOME_BANK_STRIDE]
                .copy_from_slice(&b0[row * GENOME_BANK_STRIDE..(row + 1) * GENOME_BANK_STRIDE]);
            genomes[row * GENOME_SIZE + GENOME_BANK_STRIDE..(row + 1) * GENOME_SIZE]
                .copy_from_slice(
                    &b1[row * GENOME_BANK_STRIDE
                        ..row * GENOME_BANK_STRIDE + GENOME_SIZE - GENOME_BANK_STRIDE],
                );
        }
        let trait_bytes = read_buffer(device, queue, &self.reservoir_traits_buffer)?;
        let rng_bytes = read_buffer(device, queue, &self.reservoir_rng_buffer)?;
        Ok((
            genomes,
            bytemuck::cast_slice::<u8, CognitiveTraits>(&trait_bytes).to_vec(),
            *bytemuck::from_bytes::<u32>(&rng_bytes),
        ))
    }
}
