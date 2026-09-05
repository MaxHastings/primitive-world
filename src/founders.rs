//! Random founder initialization, bank loading, and living-descendant export.
//! Genomes receive no action reward, population target, or observer feedback.
use crate::simulation::{AgentGpu, GENOME_SIZE, Simulation, observability::read_buffer};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub struct FounderBank {
    pub version: u32,
    pub model: String,
    pub name: String,
    pub source_seed: u32,
    pub source_tick: u32,
    pub genomes: Vec<Vec<f32>>,
}

impl FounderBank {
    pub fn validate(&self) -> Result<(), String> {
        let compatible_model = self.model == crate::model::MODEL_ID;
        if self.version != 6 || !compatible_model || self.genomes.is_empty() {
            return Err("Expected a nonempty Primitive World founder bank in format 6".into());
        }
        validate_genomes(&self.genomes)
    }
}

/// Reproducible random founder bank shared by default worlds.
/// Random weights with no hand-written behavior; no claim of established viability.
pub fn bundled() -> &'static FounderBank {
    static BANK: std::sync::OnceLock<FounderBank> = std::sync::OnceLock::new();
    BANK.get_or_init(|| {
        let mut rng = 0x184a2321u32;
        let genomes = (0..256)
            .map(|_| crate::model::random_genome(&mut rng).to_vec())
            .collect();
        FounderBank {
            version: 6,
            model: crate::model::MODEL_ID.into(),
            name: "primitive-world-random-256".into(),
            source_seed: 0,
            source_tick: 0,
            genomes,
        }
    })
}

pub fn validate_genomes(genomes: &[Vec<f32>]) -> Result<(), String> {
    if genomes.len() > 256
        || genomes
            .iter()
            .any(|g| g.len() != GENOME_SIZE || g.iter().any(|v| !v.is_finite() || v.abs() > 4.0))
    {
        return Err("Invalid primitive-world founder genomes".into());
    }
    Ok(())
}

impl Simulation {
    pub fn use_random_founders(&mut self) {
        self.settings.founder_genomes.clear();
        self.settings.founder_name = "primitive-world-random".into();
    }
    pub fn load_founders(&mut self, path: &Path) -> Result<(), String> {
        let bank: FounderBank =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        bank.validate()?;
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
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let gene_bytes = read_buffer(device, queue, &self.genome_buffer)?;
        let genes: &[f32] = bytemuck::cast_slice(&gene_bytes);
        let mut descendants: Vec<_> = bytemuck::cast_slice::<u8, AgentGpu>(&bytes)
            .iter()
            .enumerate()
            .filter(|(_, a)| a.alive != 0 && a.ancestry_depth > 0)
            .collect();
        if descendants.is_empty() {
            return Err("No living descendants: founder bank was not exported".into());
        }
        // Stable hash ordering samples bodies independently of slot allocation or
        // action history. Large surviving families occur at their actual frequency.
        descendants.sort_by_key(|(_, a)| {
            let mut x = a.lineage_id ^ self.seed;
            x = x.wrapping_mul(0x9e3779b9);
            x ^ (x >> 16)
        });
        let bank = FounderBank {
            version: 6,
            model: crate::model::MODEL_ID.into(),
            name: format!(
                "primitive-world-descendants-seed{}-tick{}",
                self.seed, self.tick
            ),
            source_seed: self.seed,
            source_tick: self.tick,
            genomes: descendants
                .into_iter()
                .take(256)
                .map(|(i, _)| genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE].to_vec())
                .collect(),
        };
        use std::io::Write;
        let bytes = serde_json::to_vec(&bank).map_err(|e| e.to_string())?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| format!("{}: {e}; choose a new bank path", path.display()))?;
        file.write_all(&bytes).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bank(model: &str) -> FounderBank {
        serde_json::from_value(serde_json::json!({
            "version": 6, "model": model, "name": "test-pool",
            "source_seed": 42, "source_tick": 128,
            "genomes": [vec![0.125; GENOME_SIZE]]
        }))
        .unwrap()
    }

    #[test]
    fn current_bank_preserves_genomes_and_provenance() {
        let bank = bank(crate::model::MODEL_ID);
        let before = serde_json::to_value(&bank).unwrap();
        bank.validate().unwrap();
        assert_eq!(serde_json::to_value(&bank).unwrap(), before);
        assert_eq!(bank.genomes, vec![vec![0.125; GENOME_SIZE]]);
        assert_eq!(bundled().model, crate::model::MODEL_ID);
    }

    #[test]
    fn bank_validation_rejects_unknown_models_formats_and_invalid_genomes() {
        assert!(bank("unrelated-model").validate().is_err());
        assert!(bank("primitive-v3").validate().is_err());
        assert!(bank("primitive-v4").validate().is_err());
        assert!(bank("primitive-world").validate().is_err());
        let mut bank = bank(crate::model::MODEL_ID);
        bank.version = 0;
        assert!(bank.validate().is_err());
        bank.version = 6;
        bank.genomes[0].pop();
        assert!(bank.validate().is_err());
        bank.genomes = vec![vec![5.0; GENOME_SIZE]];
        assert!(bank.validate().is_err());
        bank.genomes = vec![vec![f32::NAN; GENOME_SIZE]];
        assert!(bank.validate().is_err());
        bank.genomes.clear();
        assert!(bank.validate().is_err());
    }
}
