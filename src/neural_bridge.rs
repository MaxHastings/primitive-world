//! Persistent JSON-lines training transport. Physics runs in the actual GPU world.
use super::*;
use serde_json::{Value, json};
use std::io::{BufRead, Write};

fn paint_patch(resources: &mut [u32], center: [f32; 2], radius: f32) {
    let low = center.map(|v| ((v - radius) / 4.).clamp(0., 511.) as usize);
    let high = center.map(|v| ((v + radius) / 4.).clamp(0., 511.) as usize);
    for y in low[1]..=high[1] {
        for x in low[0]..=high[0] {
            let dx = x as f32 * 4. + 2. - center[0];
            let dy = y as f32 * 4. + 2. - center[1];
            if dx * dx + dy * dy <= radius * radius {
                resources[y * RESOURCE_GRID as usize + x] = 2000;
            }
        }
    }
}

pub fn read_items<T: Pod>(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &wgpu::Buffer,
    start: usize,
    count: usize,
) -> Result<Vec<T>, String> {
    let size = (count * std::mem::size_of::<T>()) as u64;
    if size == 0 {
        return Ok(Vec::new());
    }
    let staging = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("bounded neural readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_buffer_to_buffer(
        buffer,
        (start * std::mem::size_of::<T>()) as u64,
        &staging,
        0,
        size,
    );
    queue.submit(Some(encoder.finish()));
    let (tx, rx) = mpsc::channel();
    staging.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    device.poll(wgpu::Maintain::Wait);
    rx.recv()
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    let out = staging
        .slice(..)
        .get_mapped_range()
        .chunks_exact(std::mem::size_of::<T>())
        .map(bytemuck::pod_read_unaligned)
        .collect();
    staging.unmap();
    Ok(out)
}
impl Simulation {
    pub fn neural_inspect(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
    ) -> Result<NeuralState, String> {
        if slot >= MAX_AGENTS as usize {
            return Err("Invalid agent slot".into());
        }
        Ok(read_items(device, queue, &self.neural_state_buffer, slot, 1)?[0])
    }
    /// Reset the same world used by the interactive application. The bridge
    /// adds readback and diagnostics; it does not substitute a training map.
    pub fn neural_world(
        &mut self,
        queue: &wgpu::Queue,
        n: usize,
        seed: u32,
        blind: bool,
        reset_memory: bool,
        greedy: bool,
        neural: bool,
        regeneration: Option<f32>,
        no_social: bool,
        no_force: bool,
        static_landscape: bool,
    ) -> Result<(), String> {
        if n == 0 || n > 256 {
            return Err("Bridge population must be 1..256".into());
        }
        let name = self.settings.neural_model.clone();
        self.seed = seed;
        self.settings = SimSettings::default();
        self.settings.neural_model = name;
        self.settings.population = n as u32;
        if let Some(value) = regeneration {
            if !value.is_finite() || value < 0.0 {
                return Err("Regeneration must be finite and nonnegative".into());
            }
            self.settings.resource_regeneration = value;
        }
        if no_social {
            self.settings.social_access = 0.0;
            self.settings.social_concern = 0.0;
            self.settings.reciprocity = 0.0;
            self.settings.communication_enabled = false;
        }
        if no_force {
            self.settings.force_enabled = false;
        }
        if static_landscape {
            self.settings.evolving_landscape = false;
        }
        self.settings.neural_policy = neural;
        self.settings.neural_greedy = greedy;
        self.settings.neural_flags = u32::from(reset_memory) | if blind { 2 } else { 0 };
        self.reset(queue);
        Ok(())
    }
    pub fn neural_frame(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        n: usize,
    ) -> Result<Value, String> {
        let start = self.tick;
        let before: Vec<AgentGpu> = read_items(
            device,
            queue,
            &self.agent_buffers[self.current_buffer],
            0,
            n,
        )?;
        let mut encoder = device.create_command_encoder(&Default::default());
        self.encode_ticks(&mut encoder, device, queue, crate::neural::INTERVAL);
        queue.submit(Some(encoder.finish()));
        let trace: Vec<NeuralState> = read_items(device, queue, &self.neural_state_buffer, 0, n)?;
        let after: Vec<AgentGpu> = read_items(
            device,
            queue,
            &self.agent_buffers[self.current_buffer],
            0,
            n,
        )?;
        let rows: Vec<_> = (0..n)
            .map(|i| {
                let a = after[i];
                let b = before[i];
                let generation_changed = a.generation != b.generation;
                let died = a.alive == 0 || generation_changed;
                let starvation = a.alive == 0 && a.energy <= 0.0001;
                let old_age = a.alive == 0 && a.age >= a.max_age;
                // Energy-equivalent reserves: eating is a conversion, not new intake.
                let conversion = self.settings.conversion_efficiency.max(0.001);
                let delta =
                    (a.energy + conversion * a.food - b.energy - conversion * b.food) / conversion;
                json!({
                    "slot": i,
                    "generation": b.generation,
                    "valid": b.alive != 0,
                    "trace": trace[i],
                    "done": died,
                    "death_cause": if starvation { "starvation" } else if old_age { "age" } else { "none" },
                    "had_carried_food_at_start": b.food > 0.001,
                    "ground_food_observed": trace[i].observation[2],
                    "energy_before": b.energy,
                    "food_before": b.food,
                    "reward_physiology": if b.alive != 0 { delta - if died { 2. } else { 0. } } else { 0. },
                    "reward_survival": if b.alive != 0 { if died { -1. } else { 0.01 } } else { 0. },
                    "energy": a.energy,
                    "food": a.food,
                    "position": a.position,
                    "alive": a.alive,
                    "executed_action": a.action,
                })
            })
            .collect();
        Ok(json!({"tick":start,"elapsed_ticks":crate::neural::INTERVAL,"rows":rows}))
    }
    pub fn neural_bridge(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<(), String> {
        let mut n = 64usize;
        let stdin = std::io::stdin();
        let mut out = std::io::BufWriter::new(std::io::stdout().lock());
        writeln!(out,"{}",json!({"ready":true,"schema":crate::neural::VERSION,"observations":crate::neural::OBSERVATIONS,"hidden":crate::neural::HIDDEN,"actions":crate::neural::ACTIONS,"interval":crate::neural::INTERVAL})).map_err(|e|e.to_string())?;
        out.flush().map_err(|e| e.to_string())?;
        for line in stdin.lock().lines() {
            let cmd: Value = serde_json::from_str(&line.map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
            let response: Result<Value, String> = (|| match cmd["op"].as_str().unwrap_or("") {
                "reset" => {
                    n = cmd["population"].as_u64().unwrap_or(64) as usize;
                    self.neural_world(
                        queue,
                        n,
                        cmd["seed"].as_u64().unwrap_or(1) as u32,
                        cmd["blind"].as_bool().unwrap_or(false),
                        cmd["memory_reset"].as_bool().unwrap_or(false),
                        cmd["greedy"].as_bool().unwrap_or(false),
                        cmd["neural"].as_bool().unwrap_or(true),
                        cmd["regeneration"].as_f64().map(|v| v as f32),
                        cmd["no_social"].as_bool().unwrap_or(false),
                        cmd["no_force"].as_bool().unwrap_or(false),
                        cmd["static_landscape"].as_bool().unwrap_or(false),
                    )?;
                    Ok(json!({"reset":true,"tick":self.tick}))
                }
                "weights" => {
                    let w: NeuralWeights = serde_json::from_value(cmd["weights"].clone())
                        .map_err(|e| e.to_string())?;
                    self.set_neural_weights(queue, &w)?;
                    Ok(json!({"loaded":w.name}))
                }
                "step" => self.neural_frame(device, queue, n),
                "forget" => {
                    self.reset_neural_memory(queue);
                    Ok(json!({"forgotten":true,"tick":self.tick}))
                }
                "patches" => {
                    // Observer intervention only. Targets never enter policy inputs.
                    let centers: Vec<[f32; 2]> = serde_json::from_value(cmd["centers"].clone())
                        .map_err(|e| e.to_string())?;
                    if centers.len() > 256
                        || centers
                            .iter()
                            .flatten()
                            .any(|v| !v.is_finite() || !(0.0..=WORLD_SIZE).contains(v))
                    {
                        return Err("Invalid patch centers".into());
                    }
                    let mut resources = vec![0u32; (RESOURCE_GRID * RESOURCE_GRID) as usize];
                    for center in centers {
                        paint_patch(&mut resources, center, 8.0);
                    }
                    queue.write_buffer(&self.resource_buffer, 0, bytemuck::cast_slice(&resources));
                    queue.write_buffer(
                        &self.resource_display_buffer,
                        0,
                        bytemuck::cast_slice(&resources),
                    );
                    queue.write_buffer(
                        &self.ground_buffer,
                        0,
                        &vec![0u8; self.ground_buffer.size() as usize],
                    );
                    Ok(json!({"replaced":true,"tick":self.tick}))
                }
                "save" => {
                    self.save_checkpoint(
                        device,
                        queue,
                        std::path::Path::new(cmd["path"].as_str().ok_or("Missing path")?),
                    )?;
                    Ok(json!({"saved":true}))
                }
                "load" => {
                    self.load_checkpoint(
                        queue,
                        std::path::Path::new(cmd["path"].as_str().ok_or("Missing path")?),
                    )?;
                    Ok(json!({"loaded":true}))
                }
                "quit" => Ok(json!({"bye":true})),
                _ => Err("Unknown bridge operation".into()),
            })();
            writeln!(out, "{}", response.unwrap_or_else(|e| json!({"error":e})))
                .map_err(|e| e.to_string())?;
            out.flush().map_err(|e| e.to_string())?;
            if cmd["op"] == "quit" {
                break;
            }
        }
        Ok(())
    }
}
