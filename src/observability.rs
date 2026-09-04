use super::*;
use std::collections::HashSet;
use std::io::{Read, Write};

#[derive(Clone, Copy, Debug, Default, serde::Serialize)]
pub struct WorldMetrics {
    pub tick: u32,
    pub living: u64,
    pub juveniles: u64,
    pub carried_food: f64,
    pub energy: f64,
    pub vegetation: f64,
    pub dropped_food: f64,
    pub regenerated: f64,
    pub weather_loss: f64,
    pub events: [u32; 8],
    pub signals: u32,
    pub stocked_agents: u64,
    pub hungry_agents: u64,
    pub moving_agents: u64,
    pub eating_agents: u64,
    pub harvested: f64,
}

/// A read-only population snapshot for studying evolution. Nothing in this
/// structure is uploaded to the GPU or consulted by the decision pipeline.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct EvolutionSnapshot {
    pub tick: u32,
    pub living: u64,
    pub unique_lineages: u64,
    pub parent_lineages_present: u64,
    pub maximum_generation: u32,
    pub mean_generation: f64,
    pub mean_genome: [f64; crate::simulation::GENOME_SIZE],
    pub genome_variance: [f64; crate::simulation::GENOME_SIZE],
    pub mean_copy_fidelity: f64,
}

pub fn read_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
) -> Result<Vec<u8>, String> {
    let size = buffer.size();
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("state readback"),
        size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, size);
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let bytes = staging.slice(..).get_mapped_range().to_vec();
    staging.unmap();
    Ok(bytes)
}

impl Simulation {
    pub fn evolution_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<EvolutionSnapshot, String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let agents = bytemuck::cast_slice::<u8, AgentGpu>(&bytes);
        let mut lineages = HashSet::new();
        let mut parent_lineages = HashSet::new();
        let mut snapshot = EvolutionSnapshot {
            tick: self.tick,
            ..Default::default()
        };
        let mut genome_squares = [0.0; crate::simulation::GENOME_SIZE];
        for agent in agents.iter().filter(|agent| agent.alive != 0) {
            snapshot.living += 1;
            lineages.insert(agent.lineage_id);
            if agent.parent_lineage != 0 {
                parent_lineages.insert(agent.parent_lineage);
            }
            snapshot.maximum_generation = snapshot.maximum_generation.max(agent.generation);
            snapshot.mean_generation += agent.generation as f64;
            for (index, gene) in agent.genome.iter().enumerate() {
                snapshot.mean_genome[index] += *gene as f64;
                genome_squares[index] += (*gene as f64) * (*gene as f64);
            }
        }
        snapshot.unique_lineages = lineages.len() as u64;
        snapshot.parent_lineages_present = parent_lineages.len() as u64;
        if snapshot.living > 0 {
            let count = snapshot.living as f64;
            snapshot.mean_generation /= count;
            for index in 0..crate::simulation::GENOME_SIZE {
                snapshot.mean_genome[index] /= count;
                snapshot.genome_variance[index] =
                    (genome_squares[index] / count - snapshot.mean_genome[index].powi(2)).max(0.0);
            }
            snapshot.mean_copy_fidelity = (snapshot.mean_genome[7] + 1.0) * 0.5;
        }
        Ok(snapshot)
    }

    pub fn metrics(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<WorldMetrics, String> {
        let mut encoder = device.create_command_encoder(&Default::default());
        {
            let mut pass = encoder.begin_compute_pass(&Default::default());
            pass.set_pipeline(&self.summary_pipeline);
            pass.set_bind_group(0, &self.summary_bind_groups[self.current_buffer], &[]);
            pass.dispatch_workgroups(64, 1, 1);
        }
        queue.submit(Some(encoder.finish()));
        let bytes = read_buffer(device, queue, &self.summary_buffer)?;
        let mut total = [0u64; 16];
        for chunk in bytemuck::cast_slice::<u8, u32>(&bytes).chunks_exact(16) {
            for i in 0..16 {
                total[i] += chunk[i] as u64;
            }
        }
        let events = read_buffer(device, queue, &self.death_stats_buffer)?;
        Ok(WorldMetrics {
            tick: self.tick,
            living: total[0],
            juveniles: total[1],
            carried_food: total[2] as f64 / 1000.0,
            energy: total[3] as f64 / 1000.0,
            vegetation: total[4] as f64 / 1000.0,
            dropped_food: total[5] as f64 / 1000.0,
            regenerated: total[6] as f64 / 1000.0,
            weather_loss: total[7] as f64 / 1000.0,
            events: bytemuck::cast_slice::<u8, u32>(&events)[..8]
                .try_into()
                .map_err(|_| "invalid event buffer")?,
            signals: bytemuck::cast_slice::<u8, u32>(&events)[9],
            stocked_agents: total[11],
            hungry_agents: total[12],
            moving_agents: total[13],
            harvested: total[14] as f64 / 1000.0,
            eating_agents: total[15],
        })
    }

    pub fn save_checkpoint(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), String> {
        // Capture before opening the destination, so a GPU read failure cannot truncate a save.
        let settings = serde_json::to_vec(&self.settings).map_err(|e| e.to_string())?;
        let buffers = [
            &self.agent_buffers[self.current_buffer],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.social_memory_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
            &self.neural_state_buffer,
            &self.neural_weights_buffer,
        ];
        let data: Vec<_> = buffers
            .iter()
            .map(|b| read_buffer(device, queue, b))
            .collect::<Result<_, _>>()?;
        let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        file.write_all(b"PRIMWORLD010").map_err(|e| e.to_string())?;
        for n in [self.seed, self.tick, settings.len() as u32] {
            file.write_all(&n.to_le_bytes())
                .map_err(|e| e.to_string())?;
        }
        file.write_all(&settings).map_err(|e| e.to_string())?;
        for bytes in data {
            file.write_all(&(bytes.len() as u64).to_le_bytes())
                .map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn load_checkpoint(
        &mut self,
        queue: &wgpu::Queue,
        path: &std::path::Path,
    ) -> Result<(), String> {
        let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        let mut magic = [0; 12];
        file.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != b"PRIMWORLD010" {
            return Err("Unsupported checkpoint version".into());
        }
        let mut fields = [0; 12];
        file.read_exact(&mut fields).map_err(|e| e.to_string())?;
        let seed = u32::from_le_bytes(fields[0..4].try_into().unwrap());
        let tick = u32::from_le_bytes(fields[4..8].try_into().unwrap());
        let settings_len = u32::from_le_bytes(fields[8..12].try_into().unwrap()) as usize;
        if settings_len > 65536 {
            return Err("Invalid settings length".into());
        }
        let mut json = vec![0; settings_len];
        file.read_exact(&mut json).map_err(|e| e.to_string())?;
        let settings: SimSettings = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
        let mut buffers = vec![
            &self.agent_buffers[0],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.social_memory_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
        ];
        buffers.extend([&self.neural_state_buffer, &self.neural_weights_buffer]);
        let mut data = Vec::new();
        for buffer in &buffers {
            let mut length = [0; 8];
            file.read_exact(&mut length).map_err(|e| e.to_string())?;
            let expected = buffer.size();
            if u64::from_le_bytes(length) != expected {
                return Err("Checkpoint layout mismatch".into());
            }
            let mut bytes = vec![0; expected as usize];
            file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
            data.push(bytes);
        }
        // Validate neural payloads before changing any live buffer.
        NeuralWeights::from_flat(bytemuck::cast_slice(&data[8]))?;
        for st in data[7]
            .chunks_exact(std::mem::size_of::<NeuralState>())
            .map(bytemuck::pod_read_unaligned::<NeuralState>)
        {
            if st.choice >= crate::neural::ACTIONS as u32
                || st.valid > 1
                || !st.energy.is_finite()
                || !st.food.is_finite()
                || st
                    .hidden
                    .iter()
                    .chain(&st.before)
                    .chain(&st.after)
                    .chain(&st.observation)
                    .chain(&st.logits)
                    .chain(&st.mask)
                    .chain(&st.probabilities)
                    .any(|v| !v.is_finite())
            {
                return Err("Invalid recurrent checkpoint state".into());
            }
        }
        // Apply only after the entire checkpoint has been read and checked.
        for (buffer, bytes) in buffers.iter().zip(&data) {
            queue.write_buffer(buffer, 0, bytes);
        }
        queue.write_buffer(&self.agent_buffers[1], 0, &data[0]);
        queue.write_buffer(&self.resource_display_buffer, 0, &data[1]);
        self.settings = settings;
        self.seed = seed;
        self.tick = tick;
        self.current_buffer = 0;
        self.terrain_epoch = u32::MAX;
        self.update_params(queue);
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, serde::Serialize)]
pub struct InteractionEvent {
    pub tick: u32,
    pub actor: u32,
    pub other: u32,
    pub action: u32,
    pub amount: f32,
    pub sequence: u32,
    pub actor_lineage: u32,
    pub other_lineage: u32,
    pub position: [f32; 2],
}
impl Simulation {
    pub fn recent_events(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<InteractionEvent>, String> {
        let stats = read_buffer(device, queue, &self.death_stats_buffer)?;
        let total = bytemuck::cast_slice::<u8, u32>(&stats)[8];
        let data = read_buffer(device, queue, &self.event_buffer)?;
        let ring = bytemuck::cast_slice::<u8, InteractionEvent>(&data);
        let start = total.saturating_sub(crate::simulation::EVENT_RING_SIZE);
        Ok((start..total)
            .map(|n| ring[n as usize % crate::simulation::EVENT_RING_SIZE as usize])
            .collect())
    }
}
