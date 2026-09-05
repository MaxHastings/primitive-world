//! Measured behavior only. No genome, identity, architecture, or authored fitness.
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

pub const FEATURES: usize = 36;

/// GPU storage contract, accumulated for every individual regardless of archival selection.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeRecord {
    pub ticks: u32,
    pub birth_tick: u32,
    pub death_tick: u32,
    /// 0 = unobserved, 1 = energy depleted, 2 = age limit, 3 = external removal.
    pub death_cause: u32,
    pub actions: [u32; 6],
    pub invalid_decisions: u32,
    pub transfers: u32,
    pub forces: u32,
    pub emissions: u32,
    pub offspring: u32,
    pub start_energy: f32,
    pub start_food: f32,
    pub end_energy: f32,
    pub end_food: f32,
    pub energy_sum: f32,
    pub food_sum: f32,
    pub collected: f32,
    pub consumed: f32,
    pub given: f32,
    pub received: f32,
    pub spent: f32,
    pub distance: f32,
    pub displaced: f32,
    pub local_food_sum: f32,
    pub nearby_sum: f32,
    pub first_observed_tick: u32,
    pub initialized: u32,
}
impl LifeRecord {
    pub fn initial(energy: f32, food: f32, birth_tick: u32, observed_tick: u32) -> Self {
        Self {
            start_energy: energy,
            end_energy: energy,
            start_food: food,
            end_food: food,
            birth_tick,
            first_observed_tick: observed_tick,
            initialized: 1,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.initialized > 1
            || self.death_cause > 3
            || self.actions.iter().map(|&n| u64::from(n)).sum::<u64>() != u64::from(self.ticks)
            || self.features().iter().any(|v| !v.is_finite())
            || (self.death_cause != 0 && self.death_tick < self.first_observed_tick)
        {
            return Err("Invalid factual life record".into());
        }
        Ok(())
    }
    /// Fixed schema; categorical causes and missing/censored observations are explicit.
    pub fn features(&self) -> [f64; FEATURES] {
        let n = f64::from(self.ticks.max(1));
        let mut x = [0.0; FEATURES];
        x[..4].copy_from_slice(&[
            self.ticks as f64,
            self.birth_tick as f64,
            self.death_tick as f64,
            self.first_observed_tick as f64,
        ]);
        for (dst, &src) in x[4..10].iter_mut().zip(&self.actions) {
            *dst = src as f64;
        }
        x[10..30].copy_from_slice(&[
            self.invalid_decisions as f64,
            self.transfers as f64,
            self.forces as f64,
            self.emissions as f64,
            self.offspring as f64,
            self.start_energy as f64,
            self.start_food as f64,
            self.end_energy as f64,
            self.end_food as f64,
            self.energy_sum as f64 / n,
            self.food_sum as f64 / n,
            self.collected as f64,
            self.consumed as f64,
            self.given as f64,
            self.received as f64,
            self.spent as f64,
            self.distance as f64,
            self.displaced as f64,
            self.local_food_sum as f64 / n,
            self.nearby_sum as f64 / n,
        ]);
        for cause in 0..4 {
            x[30 + cause] = f64::from(self.death_cause as usize == cause);
        }
        x[34] = f64::from(self.initialized == 0 || self.first_observed_tick > self.birth_tick);
        x[35] = f64::from(self.ticks == 0); // Means are unavailable, rather than measured zeros.
        x
    }
}
