//! Random founder initialization, bank loading, and living-descendant export.
//! Genomes receive no action reward, population target, or observer feedback.
use crate::simulation::{
    AgentGpu, CognitiveTraits, FOUNDER_BANK_VERSION, GENOME_SIZE, MAX_AGENTS, Simulation,
    observability::read_buffer,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FounderBank {
    pub version: u32,
    pub model: String,
    pub name: String,
    pub source_seed: u32,
    pub source_tick: u32,
    pub genomes: Vec<Vec<f32>>,
    pub traits: Vec<CognitiveTraits>,
}

impl FounderBank {
    pub fn validate(&self) -> Result<(), String> {
        let compatible_model = self.model == crate::model::MODEL_ID;
        if self.version != FOUNDER_BANK_VERSION || !compatible_model || self.genomes.is_empty() {
            return Err(format!(
                "Expected a nonempty Primitive World founder bank in format {FOUNDER_BANK_VERSION}; noncurrent banks are rejected"
            ));
        }
        validate_genomes(&self.genomes)?;
        if self.traits.len() != self.genomes.len() || self.traits.iter().any(|t| !t.validate()) {
            return Err("Founder bank cognitive traits do not match genomes".into());
        }
        Ok(())
    }
}

/// Reproducible random fixture used only by existing wiring checks.
/// Random weights with no hand-written behavior; no claim of established viability.
#[cfg(test)]
pub fn bundled() -> &'static FounderBank {
    static BANK: std::sync::OnceLock<FounderBank> = std::sync::OnceLock::new();
    BANK.get_or_init(|| {
        let mut rng = 0x184a2321u32;
        let mut genomes = Vec::new();
        let mut traits = Vec::new();
        for _ in 0..256 {
            genomes.push(crate::model::random_genome(&mut rng).to_vec());
            let (plasticity_rate, trace_retention, learned_weight_retention, mutation_scale) =
                crate::brain::random_plasticity(&mut rng);
            traits.push(CognitiveTraits {
                active_mask: crate::brain::random_active_mask(&mut rng),
                padding: [0; 3],
                plasticity_rate,
                trace_retention,
                learned_weight_retention,
                mutation_scale,
            });
        }
        FounderBank {
            version: FOUNDER_BANK_VERSION,
            model: crate::model::MODEL_ID.into(),
            name: "primitive-world-random-256".into(),
            source_seed: 0,
            source_tick: 0,
            genomes,
            traits,
        }
    })
}

pub fn validate_genomes(genomes: &[Vec<f32>]) -> Result<(), String> {
    if genomes.len() > 256 {
        return Err("Invalid primitive-world founder genomes".into());
    }
    for g in genomes {
        crate::brain::validate(g)?;
    }
    Ok(())
}

impl Simulation {
    pub fn use_random_founders(&mut self) {
        self.settings.founder_genomes.clear();
        self.settings.founder_traits.clear();
        self.settings.founder_name = "primitive-world-random".into();
    }
    pub fn load_founders(&mut self, path: &Path) -> Result<(), String> {
        let bank: FounderBank =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        bank.validate()?;
        self.settings.founder_name = bank.name;
        self.settings.founder_genomes = bank.genomes;
        self.settings.founder_traits = bank.traits;
        Ok(())
    }

    pub fn export_founders(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &Path,
    ) -> Result<(), String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let genes = self.read_genomes(device, queue, MAX_AGENTS as usize)?;
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
        let selected: Vec<_> = descendants.into_iter().take(256).collect();
        let bank = FounderBank {
            version: FOUNDER_BANK_VERSION,
            model: crate::model::MODEL_ID.into(),
            name: format!(
                "primitive-world-descendants-seed{}-tick{}",
                self.seed, self.tick
            ),
            source_seed: self.seed,
            source_tick: self.tick,
            genomes: selected
                .iter()
                .map(|(i, _)| genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE].to_vec())
                .collect(),
            traits: selected
                .iter()
                .map(|(_, a)| CognitiveTraits {
                    active_mask: a.active_mask,
                    padding: [0; 3],
                    plasticity_rate: a.plasticity_rate,
                    trace_retention: a.trace_retention,
                    learned_weight_retention: a.learned_weight_retention,
                    mutation_scale: a.mutation_scale,
                })
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
            "version": FOUNDER_BANK_VERSION, "model": model, "name": "test-pool",
            "source_seed": 42, "source_tick": 128,
            "genomes": [crate::brain::blank().to_vec()],
            "traits": [{"active_mask":1,"padding":[0,0,0],"plasticity_rate":vec![0.0; crate::model::HIDDEN],"trace_retention":0.9,"learned_weight_retention":0.99,"mutation_scale":1.0}]
        }))
        .unwrap()
    }

    #[test]
    fn current_bank_preserves_genomes_and_provenance() {
        let bank = bank(crate::model::MODEL_ID);
        let before = serde_json::to_value(&bank).unwrap();
        bank.validate().unwrap();
        assert_eq!(serde_json::to_value(&bank).unwrap(), before);
        assert_eq!(bank.genomes, vec![crate::brain::blank().to_vec()]);
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
        bank.version = FOUNDER_BANK_VERSION;
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
