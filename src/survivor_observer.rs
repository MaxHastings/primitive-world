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
    observe_cached(latest, sim, device, queue, None)
}

pub fn observe_cached(
    latest: &mut Option<SurvivorSample>,
    sim: &Simulation,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    previous: Option<&SurvivorSample>,
) -> Result<(), String> {
    let agents = sim.agent_snapshot(device, queue)?;
    let chosen = slots(&agents, sim.seed, sim.tick);
    if chosen.is_empty() {
        return Ok(());
    } // Extinction must not erase the archive.
    // Structure/weights are fixed during life. Match identity, never a reused
    // slot, and never reuse a cache across worlds.
    let cached: std::collections::HashMap<_, _> = previous
        .filter(|p| p.bank.source_seed == sim.seed)
        .into_iter()
        .flat_map(|p| p.bodies.iter().zip(&p.bank.genomes))
        .map(|(body, genome)| (body.lineage_id, genome))
        .collect();
    let missing: Vec<_> = chosen
        .iter()
        .copied()
        .filter(|&slot| !cached.contains_key(&agents[slot].lineage_id))
        .collect();
    let mut genomes = Vec::new();
    if !missing.is_empty() {
        let stride = (GENOME_SIZE * std::mem::size_of::<f32>()) as u64;
        let packed = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("read-only survivor genomes"),
            size: stride * missing.len() as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        for (out, &slot) in missing.iter().enumerate() {
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
        genomes = genes
            .chunks_exact(GENOME_SIZE)
            .map(<[f32]>::to_vec)
            .collect();
    }
    let mut fresh = genomes.into_iter();
    let genomes: Vec<_> = chosen
        .iter()
        .map(|&slot| {
            cached.get(&agents[slot].lineage_id).map_or_else(
                || fresh.next().expect("packed missing genome"),
                |g| (*g).clone(),
            )
        })
        .collect();
    crate::founders::validate_genomes(&genomes)?;
    *latest = Some(SurvivorSample {
        bank: FounderBank {
            version: crate::model::FOUNDER_BANK_VERSION,
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
