use super::*;
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
    pub reports: u32,
    pub strong_ties: u64,
    pub nearby_strong_ties: u64,
    pub mean_tie_distance: f64,
    pub stocked_agents: u64,
    pub hungry_agents: u64,
    pub moving_agents: u64,
    pub eating_agents: u64,
    pub harvested: f64,
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
            reports: bytemuck::cast_slice::<u8, u32>(&events)[9],
            strong_ties: total[8],
            nearby_strong_ties: total[9],
            mean_tie_distance: total[10] as f64 / (10.0 * total[8].max(1) as f64),
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
        ];
        let data: Vec<_> = buffers
            .iter()
            .map(|b| read_buffer(device, queue, b))
            .collect::<Result<_, _>>()?;
        let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        file.write_all(b"PRIMWORLD006").map_err(|e| e.to_string())?;
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
        // Version 3 has identical size with zero padding where goal_score now lives.
        if ![
            b"PRIMWORLD003",
            b"PRIMWORLD004",
            b"PRIMWORLD005",
            b"PRIMWORLD006",
        ]
        .contains(&&magic)
        {
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
        let buffers = [
            &self.agent_buffers[0],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.social_memory_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
        ];
        let mut data = Vec::new();
        for (index, buffer) in buffers.iter().enumerate() {
            let mut length = [0; 8];
            file.read_exact(&mut length).map_err(|e| e.to_string())?;
            let legacy_relations = index == 4 && &magic != b"PRIMWORLD006";
            let legacy_agents = index == 0 && &magic != b"PRIMWORLD006";
            let expected = if legacy_relations {
                MAX_AGENTS as u64 * 8 * 40
            } else if legacy_agents {
                MAX_AGENTS as u64 * 248
            } else {
                buffer.size()
            };
            if u64::from_le_bytes(length) != expected {
                return Err("Checkpoint layout mismatch".into());
            }
            let mut bytes = vec![0; expected as usize];
            file.read_exact(&mut bytes).map_err(|e| e.to_string())?;
            if legacy_relations {
                let mut upgraded = vec![0; buffer.size() as usize];
                for (old, new) in bytes.chunks_exact(40).zip(upgraded.chunks_exact_mut(48)) {
                    new[..40].copy_from_slice(old);
                }
                bytes = upgraded;
            }
            if legacy_agents {
                let mut upgraded = vec![0; buffer.size() as usize];
                for (old, new) in bytes.chunks_exact(248).zip(upgraded.chunks_exact_mut(256)) {
                    new[..120].copy_from_slice(&old[..120]);
                    new[120..128].copy_from_slice(&old[48..56]);
                    new[128..].copy_from_slice(&old[120..]);
                }
                bytes = upgraded;
            }
            data.push(bytes);
        }
        if &magic == b"PRIMWORLD003" || &magic == b"PRIMWORLD004" {
            // Preserve old worlds' diffuse geography until an explicit reset.
            for cell in data[3].chunks_exact_mut(32) {
                cell[24..28].copy_from_slice(&1.0f32.to_le_bytes());
                cell[28..32].copy_from_slice(&1.0f32.to_le_bytes());
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
        let start = total.saturating_sub(65536);
        Ok((start..total).map(|n| ring[n as usize % 65536]).collect())
    }
    pub fn shuffle_relationships(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        let data = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let agents = bytemuck::cast_slice::<u8, AgentGpu>(&data);
        let live: Vec<_> = agents
            .iter()
            .enumerate()
            .filter(|(_, a)| a.alive != 0)
            .map(|(i, _)| i)
            .collect();
        if live.len() < 2 {
            return Ok(());
        }
        let data = read_buffer(device, queue, &self.social_memory_buffer)?;
        let mut relations = bytemuck::cast_slice::<u8, SocialRelationGpu>(&data).to_vec();
        let mut rng = self.seed ^ self.tick ^ 0x931dead;
        for (i, table) in relations.chunks_exact_mut(8).enumerate() {
            for r in table {
                if r.target_slot >= MAX_AGENTS {
                    continue;
                }
                let mut target =
                    live[(random01(&mut rng) * live.len() as f32) as usize % live.len()];
                if target == i {
                    target =
                        live[(live.iter().position(|v| *v == target).unwrap() + 1) % live.len()];
                }
                r.target_slot = target as u32;
                r.target_generation = agents[target].generation;
            }
        }
        queue.write_buffer(
            &self.social_memory_buffer,
            0,
            bytemuck::cast_slice(&relations),
        );
        Ok(())
    }
}
