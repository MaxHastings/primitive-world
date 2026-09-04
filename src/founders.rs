//! Offline founder preparation. Only bodies that actually survived/reproduced
//! contribute genomes; no action reward, population target or observer feedback.
use crate::simulation::{AgentGpu, GENOME_SIZE, Simulation, observability::read_buffer};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct FounderBank {
    pub version: u32,
    pub name: String,
    pub source_seed: u32,
    pub source_tick: u32,
    pub genomes: Vec<Vec<f32>>,
}

pub fn bundled() -> FounderBank {
    let bank: FounderBank = serde_json::from_str(include_str!("../policies/ancestor-v1.json"))
        .expect("bundled founder schema");
    assert_eq!(bank.version, 1);
    validate_genomes(&bank.genomes).expect("bundled founder weights");
    assert!(!bank.genomes.is_empty());
    bank
}

pub fn validate_genomes(genomes: &[Vec<f32>]) -> Result<(), String> {
    if genomes.len() > 256
        || genomes.iter().any(|g| {
            g.len() != GENOME_SIZE
                || g.iter()
                    .enumerate()
                    .any(|(i, v)| !v.is_finite() || v.abs() > if i < 16 { 1.0 } else { 4.0 })
        })
    {
        return Err("Invalid candidate-v1 founder genomes".into());
    }
    Ok(())
}

impl Simulation {
    pub fn use_bootstrap_founders(&mut self) {
        self.settings.founder_genomes.clear();
        self.settings.founder_name = "candidate-v1-bootstrap".into();
    }
    pub fn load_founders(&mut self, path: &Path) -> Result<(), String> {
        let bank: FounderBank =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if bank.version != 1 || bank.genomes.is_empty() {
            return Err("Expected nonempty candidate-v1 founder bank".into());
        }
        validate_genomes(&bank.genomes)?;
        self.settings.founder_name = bank.name;
        self.settings.founder_genomes = bank.genomes;
        Ok(())
    }

    pub fn export_founders(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<(), String> {
        if self.settings.neural_policy || self.settings.legacy_controller {
            return Err("Export requires candidate-v1 controller".into());
        }
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let mut descendants: Vec<_> = bytemuck::cast_slice::<u8, AgentGpu>(&bytes)
            .iter()
            .filter(|a| a.alive != 0 && a.ancestry_depth > 0)
            .collect();
        if descendants.is_empty() {
            return Err("No living descendants: founder bank was not exported".into());
        }
        // Stable hash ordering samples bodies independently of slot allocation or
        // action history. Large surviving families occur at their actual frequency.
        descendants.sort_by_key(|a| {
            let mut x = a.lineage_id ^ self.seed;
            x = x.wrapping_mul(0x9e3779b9);
            x ^ (x >> 16)
        });
        let bank = FounderBank {
            version: 1,
            name: format!(
                "candidate-v1-descendants-seed{}-tick{}",
                self.seed, self.tick
            ),
            source_seed: self.seed,
            source_tick: self.tick,
            genomes: descendants
                .into_iter()
                .take(128)
                .map(|a| a.genome.to_vec())
                .collect(),
        };
        std::fs::write(path, serde_json::to_vec(&bank).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
}
