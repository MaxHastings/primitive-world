//! Read-only, latest-nonempty population sample for external serial transfer.
//! This never supplies information or changes weights inside a running world.
use crate::{
    founders::FounderBank,
    simulation::{AgentGpu, GENOME_SIZE, Simulation, observability::read_buffer},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SampledBody {
    pub slot: usize,
    pub lineage_id: u32,
    pub parent_lineage: u32,
    pub ancestry_depth: u32,
    pub founder_family: u32,
    pub age: f32,
    pub energy: f32,
    pub food: f32,
    pub mutation_probability: f32,
    pub mutation_magnitude: f32,
    /// None means the source did not record an individual observation tick.
    pub observed_tick: Option<u32>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SurvivorSample {
    #[serde(flatten)]
    pub bank: FounderBank,
    pub source_population: usize,
    pub bodies: Vec<SampledBody>,
    pub selection: String,
}

impl SurvivorSample {
    /// Current living bodies take priority; retain earlier bodies to fill vacant
    /// archive entries. Reobserving one individual never gives it another entry.
    pub fn retain_previous(&mut self, previous: &Self) {
        if self.bank.source_seed != previous.bank.source_seed {
            return;
        }
        let mut identities: std::collections::HashSet<_> =
            self.bodies.iter().map(|b| b.lineage_id).collect();
        for (body, genome) in previous.bodies.iter().zip(&previous.bank.genomes) {
            if self.bodies.len() == 64 {
                break;
            }
            if identities.insert(body.lineage_id) {
                self.bodies.push(body.clone());
                self.bank.genomes.push(genome.clone());
            }
        }
        self.selection = "Rolling archive of up to 64 distinct bodies: current sampled survivors first, then previously observed bodies to fill remaining entries. Each body retains its own genome and mutation requests from its recorded observation tick. Source tick/population describe the latest live observation, not all archived bodies.".into();
    }
}

fn slots(agents: &[AgentGpu], seed: u32, tick: u32) -> Vec<usize> {
    let mut indices: Vec<_> = agents
        .iter()
        .enumerate()
        .filter(|(_, a)| a.alive != 0)
        .map(|(i, _)| i)
        .collect();
    // Hash ordering does not inspect food, energy, action, family or ancestry.
    indices.sort_unstable_by_key(|&i| {
        let mut x = (i as u64) ^ ((seed as u64) << 32) ^ tick as u64;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    });
    indices.truncate(64);
    indices
}

pub fn observe(
    latest: &mut Option<SurvivorSample>,
    sim: &Simulation,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<(), String> {
    let agents = sim.agent_snapshot(device, queue)?;
    let chosen = slots(&agents, sim.seed, sim.tick);
    if chosen.is_empty() {
        return Ok(());
    } // Extinction must not erase the archive.
    let stride = (GENOME_SIZE * std::mem::size_of::<f32>()) as u64;
    let packed = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("read-only survivor genomes"),
        size: stride * chosen.len() as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    for (out, &slot) in chosen.iter().enumerate() {
        encoder.copy_buffer_to_buffer(
            &sim.genome_buffer,
            slot as u64 * stride,
            &packed,
            out as u64 * stride,
            stride,
        );
    }
    queue.submit(Some(encoder.finish()));
    let bytes = read_buffer(device, queue, &packed)?;
    let genes: &[f32] = bytemuck::cast_slice(&bytes);
    let genomes: Vec<_> = genes
        .chunks_exact(GENOME_SIZE)
        .map(<[f32]>::to_vec)
        .collect();
    crate::founders::validate_genomes(&genomes)?;
    *latest = Some(SurvivorSample {
        bank: FounderBank {
            version: 6,
            model: crate::model::MODEL_ID.into(),
            name: format!("survivors-seed{}-tick{}", sim.seed, sim.tick),
            source_seed: sim.seed,
            source_tick: sim.tick,
            genomes,
        },
        source_population: agents.iter().filter(|a| a.alive != 0).count(),
        bodies: chosen
            .into_iter()
            .map(|slot| {
                let a = agents[slot];
                SampledBody {
                    slot,
                    lineage_id: a.lineage_id,
                    parent_lineage: a.parent_lineage,
                    ancestry_depth: a.ancestry_depth,
                    founder_family: a.founder_family,
                    age: a.age,
                    energy: a.energy,
                    food: a.food,
                    mutation_probability: a.mutation_probability,
                    mutation_magnitude: a.mutation_magnitude,
                    observed_tick: Some(sim.tick),
                }
            })
            .collect(),
        selection: "Up to 64 hash-sampled living bodies at latest nonempty observation; founders and descendants eligible; current slot genomes, never ancestor substitution.".into(),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selection_ignores_behavior_energy_and_family() {
        let mut agents = vec![
            AgentGpu {
                alive: 1,
                ..Default::default()
            };
            200
        ];
        agents[5].alive = 0;
        let before = slots(&agents, 123, 128);
        assert_eq!(before.len(), 64);
        assert!(!before.contains(&5));
        for (i, a) in agents.iter_mut().enumerate() {
            a.food = i as f32;
            a.energy = (200 - i) as f32;
            a.action = i as u32 % 6;
            a.ancestry_depth = i as u32;
            a.founder_family = i as u32;
        }
        assert_eq!(before, slots(&agents, 123, 128));
        assert_ne!(before, slots(&agents, 124, 128));
    }
}
