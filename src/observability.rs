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
    /// Reproduction attempts: immature, energy, cooldown, requested, eligible, resolved.
    pub birth_gates: [u32; 6],
    pub action_ticks: [u32; 6],
    pub invalid_outputs: u32,
    pub force_attempts: u32,
    pub force_energy_spent: f64,
    pub forced_distance: f64,
}

/// A read-only population snapshot for studying evolution. Nothing in this
/// structure is uploaded to the GPU or consulted by the decision pipeline.
#[derive(Clone, Debug, Default, serde::Serialize)]
pub struct EvolutionSnapshot {
    pub tick: u32,
    pub living: u64,
    pub unique_lineages: u64,
    pub parent_lineages_present: u64,
    pub maximum_ancestry_depth: u32,
    pub mean_ancestry_depth: f64,
    pub mean_genome: Vec<f64>,
    pub genome_variance: Vec<f64>,
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
    /// Vegetation only, for optional read-only journey diagnostics.
    pub fn vegetation_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<u32>, String> {
        let bytes = read_buffer(device, queue, &self.resource_buffer)?;
        Ok(bytes
            .chunks_exact(4)
            .map(bytemuck::pod_read_unaligned::<u32>)
            .collect())
    }

    /// Read the current body buffer without changing simulation state.
    pub fn agent_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<Vec<AgentGpu>, String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        Ok(bytes
            .chunks_exact(std::mem::size_of::<AgentGpu>())
            .map(bytemuck::pod_read_unaligned::<AgentGpu>)
            .collect())
    }

    pub fn evolution_snapshot(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<EvolutionSnapshot, String> {
        let bytes = read_buffer(device, queue, &self.agent_buffers[self.current_buffer])?;
        let agents = bytemuck::cast_slice::<u8, AgentGpu>(&bytes);
        let gene_bytes = read_buffer(device, queue, &self.genome_buffer)?;
        let genes: &[f32] = bytemuck::cast_slice(&gene_bytes);
        let mut lineages = HashSet::new();
        let mut parent_lineages = HashSet::new();
        let mut snapshot = EvolutionSnapshot {
            tick: self.tick,
            mean_genome: vec![0.0; GENOME_SIZE],
            genome_variance: vec![0.0; GENOME_SIZE],
            ..Default::default()
        };
        let mut genome_squares = [0.0; crate::simulation::GENOME_SIZE];
        for (slot, agent) in agents
            .iter()
            .enumerate()
            .filter(|(_, agent)| agent.alive != 0)
        {
            snapshot.living += 1;
            lineages.insert(agent.lineage_id);
            if agent.parent_lineage != 0 {
                parent_lineages.insert(agent.parent_lineage);
            }
            snapshot.maximum_ancestry_depth =
                snapshot.maximum_ancestry_depth.max(agent.ancestry_depth);
            snapshot.mean_ancestry_depth += agent.ancestry_depth as f64;
            for (index, gene) in genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
                .iter()
                .enumerate()
            {
                snapshot.mean_genome[index] += *gene as f64;
                genome_squares[index] += (*gene as f64) * (*gene as f64);
            }
        }
        snapshot.unique_lineages = lineages.len() as u64;
        snapshot.parent_lineages_present = parent_lineages.len() as u64;
        if snapshot.living > 0 {
            let count = snapshot.living as f64;
            snapshot.mean_ancestry_depth /= count;
            for (index, square) in genome_squares.iter().enumerate() {
                snapshot.mean_genome[index] /= count;
                snapshot.genome_variance[index] =
                    (square / count - snapshot.mean_genome[index].powi(2)).max(0.0);
            }
        }
        Ok(snapshot)
    }

    pub fn metrics(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<WorldMetrics, String> {
        let mut encoder = device.create_command_encoder(&Default::default());
        self.dispatch(&mut encoder, "summary", self.current_buffer, 64, 1);
        queue.submit(Some(encoder.finish()));
        let bytes = read_buffer(device, queue, &self.summary_buffer)?;
        let mut total = [0u64; 16];
        for chunk in bytemuck::cast_slice::<u8, u32>(&bytes).chunks_exact(16) {
            for i in 0..16 {
                total[i] += chunk[i] as u64;
            }
        }
        let events = read_buffer(device, queue, &self.death_stats_buffer)?;
        let counters: &[u32] = bytemuck::cast_slice(&events);
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
            birth_gates: [
                counters[16],
                counters[17],
                counters[19],
                counters[20],
                counters[21],
                counters[22],
            ],
            action_ticks: counters[24..30]
                .try_into()
                .map_err(|_| "Invalid action counters")?,
            invalid_outputs: counters[31],
            force_attempts: counters[12],
            force_energy_spent: (counters[13] as f64 + counters[14] as f64) / 1000.0,
            forced_distance: counters[15] as f64 / 1000.0,
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
            &self.death_stats_buffer,
            &self.event_buffer,
            &self.perception_buffer,
            &self.decision_buffer,
            &self.genome_buffer,
        ];
        let data: Vec<_> = buffers
            .iter()
            .map(|b| read_buffer(device, queue, b))
            .collect::<Result<_, _>>()?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|e| {
                format!(
                    "{}: {e}; preserve or rename the previous save first",
                    path.display()
                )
            })?;
        file.write_all(b"PRIMWORLD016").map_err(|e| e.to_string())?;
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
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        self.load_checkpoint_reader(queue, file)
    }

    pub fn load_checkpoint_reader(
        &mut self,
        queue: &wgpu::Queue,
        file: impl Read,
    ) -> Result<(), String> {
        self.load_checkpoint_checked(queue, file, None)
    }

    pub fn load_checkpoint_checked(
        &mut self,
        queue: &wgpu::Queue,
        mut file: impl Read,
        expected: Option<(u32, u32, u32)>,
    ) -> Result<(), String> {
        let mut magic = [0; 12];
        file.read_exact(&mut magic).map_err(|e| e.to_string())?;
        if &magic != b"PRIMWORLD016" {
            return Err("Unsupported checkpoint: expected format 16".into());
        }
        let mut fields = [0; 12];
        file.read_exact(&mut fields).map_err(|e| e.to_string())?;
        let seed = u32::from_le_bytes(fields[0..4].try_into().unwrap());
        let tick = u32::from_le_bytes(fields[4..8].try_into().unwrap());
        let settings_len = u32::from_le_bytes(fields[8..12].try_into().unwrap()) as usize;
        if settings_len > 16_777_216 {
            return Err("Invalid settings length".into());
        }
        let mut json = vec![0; settings_len];
        file.read_exact(&mut json).map_err(|e| e.to_string())?;
        let settings: SimSettings = serde_json::from_slice(&json).map_err(|e| e.to_string())?;
        settings.validate()?;
        let mut buffers = vec![
            &self.agent_buffers[0],
            &self.resource_buffer,
            &self.fertility_buffer,
            &self.ground_buffer,
            &self.death_stats_buffer,
            &self.event_buffer,
        ];
        buffers.extend([
            &self.perception_buffer,
            &self.decision_buffer,
            &self.genome_buffer,
        ]);
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
        for a in data[0]
            .chunks_exact(std::mem::size_of::<AgentGpu>())
            .map(bytemuck::pod_read_unaligned::<AgentGpu>)
        {
            if a.alive > 1
                || a.action > 5
                || a.position
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=WORLD_SIZE).contains(v))
                || [
                    a.energy,
                    a.age,
                    a.food,
                    a.max_speed,
                    a.sensor_radius,
                    a.max_age,
                    a.body_padding,
                    a.signal_payload,
                    a.mutation_probability,
                    a.mutation_magnitude,
                    a.collected,
                    a.ingested,
                    a.spent,
                    a.received,
                ]
                .iter()
                .chain(&a.hidden)
                .chain(&a.velocity)
                .chain(&a.moved)
                .any(|v| !v.is_finite())
                || !(0.0..=1.0).contains(&a.mutation_probability)
                || !(0.0..=8.0).contains(&a.mutation_magnitude)
                || a.food < 0.0
                || a.food > 8.001
                || a.energy < 0.0
                || a.energy > 100.001
                || a.age < 0.0
                || a.max_age < 1.0
                || a.max_age > 11000.0
                || a.max_speed < 0.0
                || a.max_speed > 1.2
                || a.hidden.iter().any(|v| v.abs() > 1.0)
                || a.sensor_radius < 4.0
                || a.sensor_radius > 48.0
            {
                return Err("Invalid primitive-world body checkpoint".into());
            }
        }
        if bytemuck::cast_slice::<u8, f32>(&data[8])
            .iter()
            .any(|x| !x.is_finite() || x.abs() > 4.0)
        {
            return Err("Invalid checkpoint genome".into());
        }
        if data[2]
            .chunks_exact(4)
            .map(|v| f32::from_le_bytes(v.try_into().unwrap()))
            .any(|v| !v.is_finite() || !(0.0..=1.0).contains(&v))
        {
            return Err("Invalid checkpoint soil".into());
        }
        for cell in data[3].chunks_exact(32) {
            let value = |i| f32::from_le_bytes(cell[i..i + 4].try_into().unwrap());
            if [value(8), value(24), value(28)]
                .iter()
                .any(|v| !v.is_finite() || *v < 0.0)
                || value(8) >= 1.0
                || value(24) > 1.0
                || value(28) > 1000.0
            {
                return Err("Invalid checkpoint ground".into());
            }
        }
        for p in data[6]
            .chunks_exact(std::mem::size_of::<PerceptionGpu>())
            .map(bytemuck::pod_read_unaligned::<PerceptionGpu>)
        {
            if !p.resource_here.is_finite()
                || !p.local_count.is_finite()
                || p.samples
                    .iter()
                    .any(|s| !s.food.is_finite() || s.offset.iter().any(|v| !v.is_finite()))
                || p.bodies.iter().any(|b| {
                    !b.food.is_finite()
                        || !b.signal.is_finite()
                        || b.offset.iter().chain(&b.velocity).any(|v| !v.is_finite())
                        || b.slot > MAX_AGENTS
                })
            {
                return Err("Invalid checkpoint perception".into());
            }
        }
        for d in data[7]
            .chunks_exact(std::mem::size_of::<DecisionGpu>())
            .map(bytemuck::pod_read_unaligned::<DecisionGpu>)
        {
            if d.selected_action > 5
                || d.invalid > 1
                || d.target > MAX_AGENTS
                || !(0.0..=1.0).contains(&d.mutation_probability)
                || !(0.0..=8.0).contains(&d.mutation_magnitude)
                || [d.amount, d.payload]
                    .iter()
                    .chain(&d.movement)
                    .chain(&d.force)
                    .chain(&d.scores)
                    .chain(&d.hidden)
                    .chain(&d.inputs)
                    .any(|v| !v.is_finite())
            {
                return Err("Invalid checkpoint decision".into());
            }
        }
        // Reject trailing payloads before mutating live state.
        let mut trailing = [0u8; 1];
        if file.read(&mut trailing).map_err(|e| e.to_string())? != 0 {
            return Err("Trailing checkpoint data".into());
        }
        // Apply only after the entire checkpoint has been read and checked.
        if let Some((expected_seed, expected_tick, expected_living)) = expected {
            let living = data[0]
                .chunks_exact(std::mem::size_of::<AgentGpu>())
                .map(bytemuck::pod_read_unaligned::<AgentGpu>)
                .filter(|a| a.alive != 0)
                .count() as u32;
            if (seed, tick, living) != (expected_seed, expected_tick, expected_living) {
                return Err("The experiment receipt and checkpoint do not match".into());
            }
        }
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
