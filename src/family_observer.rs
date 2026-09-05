//! Per-tick family outcomes survive extinction; never reward or steer live bodies.
use crate::{
    model::*,
    simulation::{Simulation, observability::read_buffer},
};
use wgpu::util::DeviceExt;

pub struct FamilyObserver {
    counters: wgpu::Buffer,
    pipeline: wgpu::ComputePipeline,
    groups: [wgpu::BindGroup; 2],
    initial_counts: Vec<u32>,
    horizon: u32,
}

#[derive(serde::Serialize)]
pub struct FamilyOutcome {
    pub family: usize,
    pub initial_founders: u32,
    pub founder_body_ticks: u32,
    pub descendant_body_ticks: u32,
    pub late_descendant_body_ticks: u32,
    pub mature_descendant_body_ticks: u32,
    pub births: u32,
    pub maximum_depth: u32,
    pub last_alive_tick: u32,
    pub matured_descendants: u32,
    pub juvenile_starvation_deaths: u32,
    pub adult_descendant_starvation_deaths: u32,
    pub descendant_age_deaths: u32,
    pub descendant_other_deaths: u32,
    pub births_to_descendant_parents: u32,
    pub births_below_stationary_maturity_energy: u32,
    pub birth_energy_milli: u64,
    pub juvenile_collected_milli: u64,
    pub juvenile_ingested_milli: u64,
    pub collected_milli: u64,
    pub ingested_milli: u64,
    pub descendant_spent_milli: u64,
    pub juvenile_collect_action_ticks: u32,
    pub juvenile_processed_ticks: u32,
    pub energy_at_maturity_milli: u64,
    pub juvenile_food_present_ticks: u32,
    pub juvenile_food_present_collect_ticks: u32,
}

#[derive(serde::Serialize)]
pub struct FamilyReport {
    pub schema: u32,
    pub requested_horizon: u32,
    pub late_window_start: u32,
    pub families: Vec<FamilyOutcome>,
    pub scope: &'static str,
}

impl FamilyObserver {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        sim: &Simulation,
        horizon: u32,
    ) -> Result<Self, String> {
        // Even an entire capacity-sized family for every tick stays below u32::MAX.
        if sim.tick != 0 || horizon == 0 || horizon > 200_000 {
            return Err("Family evaluation requires a fresh world and 1..=200000 ticks".into());
        }
        let initial = sim.agent_snapshot(device, queue)?;
        let count = initial
            .iter()
            .filter(|a| a.alive != 0)
            .map(|a| a.founder_family + 1)
            .max()
            .unwrap_or(0) as usize;
        if count > MAX_AGENTS as usize {
            return Err("Invalid founder family".into());
        }
        let mut initial_counts = vec![0; count];
        for a in initial.iter().filter(|a| a.alive != 0) {
            initial_counts[a.founder_family as usize] += 1;
        }
        let counters = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("observer family counters"),
            size: MAX_AGENTS as u64 * 128,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let window = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("observer family window"),
            contents: bytemuck::cast_slice(&[0u32, horizon / 2, horizon, count as u32]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("family observer"),
            source: wgpu::ShaderSource::Wgsl(
                crate::simulation::shader_source(include_str!("../shaders/observe_families.wgsl"))
                    .into(),
            ),
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("family observer"),
            layout: None,
            module: &shader,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        let layout = pipeline.get_bind_group_layout(0);
        let groups = std::array::from_fn(|i| {
            let buffers = [
                &sim.agent_buffers[i],
                &counters,
                &sim.params_buffer,
                &window,
                &sim.agent_buffers[1 - i],
                &sim.perception_buffer,
            ];
            let entries: Vec<_> = buffers
                .iter()
                .enumerate()
                .map(|(binding, buffer)| wgpu::BindGroupEntry {
                    binding: binding as u32,
                    resource: buffer.as_entire_binding(),
                })
                .collect();
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("family observer"),
                layout: &layout,
                entries: &entries,
            })
        });
        Ok(Self {
            counters,
            pipeline,
            groups,
            initial_counts,
            horizon,
        })
    }

    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder, source: usize) {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("observe families"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[source], &[]);
        pass.dispatch_workgroups(MAX_AGENTS.div_ceil(64), 1, 1);
    }

    pub fn report(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Result<FamilyReport, String> {
        let bytes = read_buffer(device, queue, &self.counters)?;
        let rows: &[[u32; 32]] = bytemuck::cast_slice(&bytes);
        let families = self
            .initial_counts
            .iter()
            .enumerate()
            .map(|(family, count)| {
                let r = rows[family];
                let total = |i: usize| u64::from(r[i]) + (u64::from(r[i + 1]) << 32);
                FamilyOutcome {
                    family,
                    initial_founders: *count,
                    founder_body_ticks: r[0],
                    descendant_body_ticks: r[1],
                    late_descendant_body_ticks: r[2],
                    mature_descendant_body_ticks: r[3],
                    births: r[4],
                    maximum_depth: r[5],
                    last_alive_tick: r[6],
                    matured_descendants: r[7],
                    juvenile_starvation_deaths: r[8],
                    adult_descendant_starvation_deaths: r[9],
                    descendant_age_deaths: r[10],
                    descendant_other_deaths: r[11],
                    births_to_descendant_parents: r[12],
                    births_below_stationary_maturity_energy: r[13],
                    birth_energy_milli: total(14),
                    juvenile_collected_milli: total(16),
                    juvenile_ingested_milli: total(18),
                    collected_milli: total(20),
                    ingested_milli: total(22),
                    descendant_spent_milli: total(24),
                    juvenile_collect_action_ticks: r[26],
                    juvenile_processed_ticks: r[27],
                    energy_at_maturity_milli: total(28),
                    juvenile_food_present_ticks: r[30],
                    juvenile_food_present_collect_ticks: r[31],
                }
            })
            .collect();
        Ok(FamilyReport {
            schema: 2,
            requested_horizon: self.horizon,
            late_window_start: self.horizon / 2,
            families,
            scope: "Per-tick post-step counts plus terminal transitions before dead-slot reuse. Food/energy sums round each contribution to thousandths with 64-bit carry. Juvenile feeding uses age at tick start; death class uses terminal age. Food-present means local vegetation >=.001 before collection, not guaranteed available after competition. Family identifies initial genome slot, not an unchanged descendant genotype. Diagnostics do not select genomes or enter controller inputs.",
        })
    }
}
