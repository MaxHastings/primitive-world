//! Offline founder preparation. Only bodies that actually survived/reproduced
//! contribute genomes; no action reward, population target or observer feedback.
use crate::simulation::{AgentGpu, GENOME_SIZE, Simulation, observability::read_buffer};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct FounderBank {
    pub version: u32,
    pub model: String,
    pub name: String,
    pub source_seed: u32,
    pub source_tick: u32,
    pub genomes: Vec<Vec<f32>>,
}

/// Frozen unprepared development bank. No v1 genes are silently reinterpreted.
/// Mutable starting dispositions plus declared standing noise; not blank brains.
pub fn bundled() -> &'static FounderBank {
    static BANK: std::sync::OnceLock<FounderBank> = std::sync::OnceLock::new();
    BANK.get_or_init(|| {
        let initial = crate::model::bootstrap_genome();
        let mut rng = 0x184a2321u32;
        let genomes = (0..128)
            .map(|_| {
                initial
                    .iter()
                    .map(|&g| {
                        rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                        g + ((rng >> 8) as f32 / 16_777_215.0 - 0.5) * 0.02
                    })
                    .collect()
            })
            .collect();
        FounderBank {
            version: 3,
            model: crate::model::MODEL_ID.into(),
            name: "physiology-v2-unprepared-frozen-bootstrap-128".into(),
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
        return Err("Invalid physiology-v2 founder genomes".into());
    }
    Ok(())
}

impl Simulation {
    pub fn use_bootstrap_founders(&mut self) {
        self.settings.founder_genomes.clear();
        self.settings.founder_name = "physiology-v2-unprepared-bootstrap".into();
    }
    pub fn load_founders(&mut self, path: &Path) -> Result<(), String> {
        let bank: FounderBank =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        if bank.version != 3 || bank.model != crate::model::MODEL_ID || bank.genomes.is_empty() {
            return Err("Expected nonempty physiology-v2 founder bank".into());
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
            version: 3,
            model: crate::model::MODEL_ID.into(),
            name: format!(
                "physiology-v2-descendants-seed{}-tick{}",
                self.seed, self.tick
            ),
            source_seed: self.seed,
            source_tick: self.tick,
            genomes: descendants
                .into_iter()
                .take(128)
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
