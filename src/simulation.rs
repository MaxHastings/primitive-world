pub use crate::model::*;
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, sync::mpsc};
use wgpu::util::DeviceExt;
const RESOURCE_SCALE: f32 = 1000.0;

struct Compute {
    pipeline: wgpu::ComputePipeline,
    groups: Vec<wgpu::BindGroup>,
}
fn pair<'a>(f: impl Fn(usize) -> Vec<&'a wgpu::Buffer>) -> Vec<Vec<&'a wgpu::Buffer>> {
    (0..2).map(f).collect()
}
impl Compute {
    fn new(
        device: &wgpu::Device,
        name: &str,
        source: &str,
        entry: &str,
        kinds: &str,
        buffers: Vec<Vec<&wgpu::Buffer>>,
    ) -> Self {
        let entries: Vec<_> = kinds
            .chars()
            .enumerate()
            .map(|(i, k)| wgpu::BindGroupLayoutEntry {
                binding: i as u32,
                visibility: wgpu::ShaderStages::COMPUTE,
                count: None,
                ty: wgpu::BindingType::Buffer {
                    ty: if k == 'u' {
                        wgpu::BufferBindingType::Uniform
                    } else {
                        wgpu::BufferBindingType::Storage {
                            read_only: k == 'r',
                        }
                    },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
            })
            .collect();
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(name),
            entries: &entries,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(shader_source(source).into()),
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(name),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(&pl),
            module: &shader,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            cache: None,
        });
        let groups = buffers
            .into_iter()
            .map(|bs| {
                assert_eq!(bs.len(), entries.len());
                let es: Vec<_> = bs
                    .into_iter()
                    .enumerate()
                    .map(|(i, b)| wgpu::BindGroupEntry {
                        binding: i as u32,
                        resource: b.as_entire_binding(),
                    })
                    .collect();
                device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(name),
                    layout: &layout,
                    entries: &es,
                })
            })
            .collect();
        Self { pipeline, groups }
    }
    fn dispatch(&self, encoder: &mut wgpu::CommandEncoder, group: usize, x: u32, y: u32) {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[group], &[]);
        pass.dispatch_workgroups(x, y, 1);
    }
}
fn buffer(device: &wgpu::Device, name: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(name),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
fn uniform(device: &wgpu::Device, name: &str, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(name),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
fn readback(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
pub struct Simulation {
    pub settings: SimSettings,
    pub seed: u32,
    pub tick: u32,
    pub current_buffer: usize,
    pub(crate) genome_buffer: wgpu::Buffer,
    pub agent_buffers: [wgpu::Buffer; 2],
    pub resource_buffer: wgpu::Buffer,
    pub resource_display_buffer: wgpu::Buffer,
    pub ground_buffer: wgpu::Buffer,
    pub perception_buffer: wgpu::Buffer,
    pub occupancy_buffer: wgpu::Buffer,
    pub params_buffer: wgpu::Buffer,
    pub alive_count_buffer: wgpu::Buffer,
    decision_buffer: wgpu::Buffer,
    fertility_buffer: wgpu::Buffer,
    terrain_buffer: wgpu::Buffer,
    terrain_epoch: u32,
    death_stats_buffer: wgpu::Buffer,
    event_buffer: wgpu::Buffer,
    summary_buffer: wgpu::Buffer,
    tick_params_buffer: wgpu::Buffer,
    alive_count_readback: wgpu::Buffer,
    death_stats_readback: wgpu::Buffer,
    selection_params_buffer: wgpu::Buffer,
    selection_key_buffer: wgpu::Buffer,
    selection_output_buffer: wgpu::Buffer,
    intervention_params_buffer: wgpu::Buffer,
    passes: HashMap<String, Compute>,
}
impl Simulation {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, seed: u32) -> Self {
        let agent_size = MAX_AGENTS as u64 * std::mem::size_of::<AgentGpu>() as u64;
        assert!(
            agent_size <= device.limits().max_storage_buffer_binding_size as u64,
            "GPU storage limit below recurrent-v1 body budget"
        );
        let genome_buffer = buffer(
            device,
            "inherited genomes",
            MAX_AGENTS as u64 * GENOME_SIZE as u64 * 4,
        );
        let agent_buffers = [
            buffer(device, "bodies A", agent_size),
            buffer(device, "bodies B", agent_size),
        ];
        let resource_buffer = buffer(device, "food", 512 * 512 * 4);
        let resource_display_buffer = buffer(device, "food display", 512 * 512 * 4);
        let ground_buffer = buffer(device, "ground", 512 * 512 * 32);
        let fertility_buffer = buffer(device, "soil", 512 * 512 * 4);
        let terrain_buffer = buffer(device, "terrain", 512 * 512 * 16);
        let perception_buffer = buffer(
            device,
            "observations",
            MAX_AGENTS as u64 * std::mem::size_of::<PerceptionGpu>() as u64,
        );
        let decision_buffer = buffer(
            device,
            "intentions",
            MAX_AGENTS as u64 * std::mem::size_of::<DecisionGpu>() as u64,
        );
        let request_buffer = buffer(device, "harvest", MAX_AGENTS as u64 * 4);
        let occupancy_buffer = buffer(device, "occupancy", SPATIAL_CELL_COUNT as u64 * 4);
        let cell_offsets = [
            buffer(device, "cell offsets A", SPATIAL_CELL_COUNT as u64 * 4),
            buffer(device, "cell offsets B", SPATIAL_CELL_COUNT as u64 * 4),
        ];
        let cursors = buffer(device, "scatter cursors", SPATIAL_CELL_COUNT as u64 * 4);
        let indices = buffer(device, "body indices", MAX_AGENTS as u64 * 4);
        let free_flags = buffer(device, "free flags", MAX_AGENTS as u64 * 4);
        let birth_flags = buffer(device, "birth requests", MAX_AGENTS as u64 * 4);
        let free_prefix = [
            buffer(device, "free prefix A", MAX_AGENTS as u64 * 4),
            buffer(device, "free prefix B", MAX_AGENTS as u64 * 4),
        ];
        let birth_prefix = [
            buffer(device, "birth prefix A", MAX_AGENTS as u64 * 4),
            buffer(device, "birth prefix B", MAX_AGENTS as u64 * 4),
        ];
        let free_indices = buffer(device, "free slots", MAX_AGENTS as u64 * 4);
        let parents = buffer(device, "parents", MAX_AGENTS as u64 * 4);
        let claims = buffer(device, "interaction claims", MAX_AGENTS as u64 * 4);
        let death_stats_buffer = buffer(device, "counters", DEATH_STATS_COUNT as u64 * 4);
        let event_buffer = buffer(device, "event ring", EVENT_RING_SIZE as u64 * 40);
        let summary_buffer = buffer(device, "summaries", 4096 * 64);
        let params_buffer = uniform(
            device,
            "parameters",
            std::mem::size_of::<SimParams>() as u64,
        );
        let tick_params_buffer = buffer(
            device,
            "tick parameters",
            32 * std::mem::size_of::<SimParams>() as u64,
        );
        let alive_count_buffer = buffer(device, "alive count", 4);
        let alive_count_readback = readback(device, 4);
        let death_stats_readback = readback(device, DEATH_STATS_COUNT as u64 * 4);
        let selection_params_buffer = uniform(device, "selection", 16);
        let selection_key_buffer = buffer(device, "selection key", 4);
        let selection_output_buffer = buffer(
            device,
            "selected body",
            std::mem::size_of::<SelectionOutput>() as u64,
        );
        let intervention_params_buffer = uniform(device, "intervention", 16);
        let mut passes = HashMap::new();
        macro_rules! add {
            ($name:expr,$file:literal,$entry:expr,$kinds:expr,$groups:expr) => {
                passes.insert(
                    $name.to_string(),
                    Compute::new(device, $name, include_str!($file), $entry, $kinds, $groups),
                );
            };
        }
        add!(
            "resource",
            "../shaders/resource_update.wgsl",
            "main",
            "wuw wr".replace(' ', "").as_str(),
            vec![vec![
                &resource_buffer,
                &params_buffer,
                &fertility_buffer,
                &ground_buffer,
                &terrain_buffer
            ]]
        );
        add!(
            "clear",
            "../shaders/clear_occupancy.wgsl",
            "main",
            "w",
            vec![vec![&occupancy_buffer]]
        );
        add!(
            "count",
            "../shaders/count_occupancy.wgsl",
            "main",
            "rwu",
            pair(|s| vec![&agent_buffers[s], &occupancy_buffer, &params_buffer])
        );
        add!(
            "spatial_init",
            "../shaders/prefix_init.wgsl",
            "main",
            "rw",
            vec![vec![&occupancy_buffer, &cell_offsets[0]]]
        );
        for n in 0..16 {
            let name = format!("spatial_{n}");
            let entry = format!("step_{}", 1u32 << n);
            add!(
                &name,
                "../shaders/prefix_step.wgsl",
                &entry,
                "rw",
                pair(|s| vec![&cell_offsets[s], &cell_offsets[1 - s]])
            );
        }
        add!(
            "cursors",
            "../shaders/prepare_scatter.wgsl",
            "main",
            "rw",
            vec![vec![&cell_offsets[0], &cursors]]
        );
        add!(
            "scatter",
            "../shaders/scatter_agents.wgsl",
            "main",
            "rwwu",
            pair(|s| vec![&agent_buffers[s], &cursors, &indices, &params_buffer])
        );
        add!(
            "perceive",
            "../shaders/perceive.wgsl",
            "main",
            "rrwrrrwu",
            pair(|s| vec![
                &agent_buffers[s],
                &resource_buffer,
                &ground_buffer,
                &occupancy_buffer,
                &cell_offsets[0],
                &indices,
                &perception_buffer,
                &params_buffer
            ])
        );
        add!(
            "decide",
            "../shaders/decide.wgsl",
            "main",
            "rrwur",
            pair(|s| vec![
                &agent_buffers[s],
                &perception_buffer,
                &decision_buffer,
                &params_buffer,
                &genome_buffer
            ])
        );
        add!(
            "consume",
            "../shaders/consume.wgsl",
            "main",
            "rrwwuww",
            pair(|s| vec![
                &agent_buffers[s],
                &decision_buffer,
                &resource_buffer,
                &request_buffer,
                &params_buffer,
                &death_stats_buffer,
                &ground_buffer
            ])
        );
        add!(
            "body",
            "../shaders/update_agents.wgsl",
            "main",
            "rrrw uww".replace(' ', "").as_str(),
            pair(|s| vec![
                &agent_buffers[s],
                &decision_buffer,
                &request_buffer,
                &agent_buffers[1 - s],
                &params_buffer,
                &birth_flags,
                &death_stats_buffer
            ])
        );
        for entry in ["clear", "propose", "resolve"] {
            let name = format!("interact_{entry}");
            add!(
                &name,
                "../shaders/interactions.wgsl",
                entry,
                "wrwuw ww".replace(' ', "").as_str(),
                pair(|s| vec![
                    &agent_buffers[s],
                    &decision_buffer,
                    &claims,
                    &params_buffer,
                    &death_stats_buffer,
                    &ground_buffer,
                    &event_buffer
                ])
            );
        }
        add!(
            "release",
            "../shaders/release_food.wgsl",
            "main",
            "ww",
            pair(|s| vec![&agent_buffers[s], &ground_buffer])
        );
        add!(
            "free",
            "../shaders/free_flags.wgsl",
            "main",
            "rwu",
            pair(|s| vec![&agent_buffers[s], &free_flags, &params_buffer])
        );
        add!(
            "free_init",
            "../shaders/agent_prefix_init.wgsl",
            "main",
            "rw",
            vec![vec![&free_flags, &free_prefix[0]]]
        );
        add!(
            "birth_init",
            "../shaders/agent_prefix_init.wgsl",
            "main",
            "rw",
            vec![vec![&birth_flags, &birth_prefix[0]]]
        );
        for n in 0..14 {
            let entry = format!("step_{}", 1u32 << n);
            let name = format!("free_{n}");
            add!(
                &name,
                "../shaders/agent_prefix_step.wgsl",
                &entry,
                "rw",
                pair(|s| vec![&free_prefix[s], &free_prefix[1 - s]])
            );
            let name = format!("birth_{n}");
            add!(
                &name,
                "../shaders/agent_prefix_step.wgsl",
                &entry,
                "rw",
                pair(|s| vec![&birth_prefix[s], &birth_prefix[1 - s]])
            );
        }
        add!(
            "free_compact",
            "../shaders/compact_agent_indices.wgsl",
            "main",
            "rrw",
            vec![vec![&free_flags, &free_prefix[0], &free_indices]]
        );
        add!(
            "birth_compact",
            "../shaders/compact_agent_indices.wgsl",
            "main",
            "rrw",
            vec![vec![&birth_flags, &birth_prefix[0], &parents]]
        );
        add!(
            "birth",
            "../shaders/apply_births.wgsl",
            "main",
            "wrrrruwrw",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix[0],
                &parents,
                &birth_prefix[0],
                &params_buffer,
                &death_stats_buffer,
                &decision_buffer,
                &genome_buffer
            ])
        );
        add!(
            "alive",
            "../shaders/count_alive.wgsl",
            "main",
            "rw",
            pair(|s| vec![&agent_buffers[s], &alive_count_buffer])
        );
        add!(
            "summary",
            "../shaders/summarize.wgsl",
            "main",
            "rrrwu",
            pair(|s| vec![
                &agent_buffers[s],
                &resource_buffer,
                &ground_buffer,
                &summary_buffer,
                &params_buffer
            ])
        );
        add!(
            "select",
            "../shaders/select_agent.wgsl",
            "main",
            "ruw",
            pair(|s| vec![
                &agent_buffers[s],
                &selection_params_buffer,
                &selection_key_buffer
            ])
        );
        add!(
            "selected",
            "../shaders/resolve_selection.wgsl",
            "main",
            "rrrrw",
            pair(|s| vec![
                &agent_buffers[s],
                &perception_buffer,
                &decision_buffer,
                &selection_key_buffer,
                &selection_output_buffer
            ])
        );
        add!(
            "shock",
            "../shaders/intervene.wgsl",
            "apply",
            "wuw",
            vec![vec![
                &resource_buffer,
                &intervention_params_buffer,
                &ground_buffer
            ]]
        );
        add!(
            "kill",
            "../shaders/kill.wgsl",
            "main",
            "wu",
            pair(|s| vec![&agent_buffers[s], &intervention_params_buffer])
        );
        let mut sim = Self {
            settings: SimSettings::default(),
            seed,
            tick: 0,
            current_buffer: 0,
            genome_buffer,
            agent_buffers,
            resource_buffer,
            resource_display_buffer,
            ground_buffer,
            perception_buffer,
            occupancy_buffer,
            params_buffer,
            alive_count_buffer,
            decision_buffer,
            fertility_buffer,
            terrain_buffer,
            terrain_epoch: 0,
            death_stats_buffer,
            event_buffer,
            summary_buffer,
            tick_params_buffer,
            alive_count_readback,
            death_stats_readback,
            selection_params_buffer,
            selection_key_buffer,
            selection_output_buffer,
            intervention_params_buffer,
            passes,
        };
        sim.reset(queue);
        sim
    }
    pub fn reset(&mut self, queue: &wgpu::Queue) {
        self.settings.validate().expect("valid reset settings");
        queue.write_buffer(
            &self.genome_buffer,
            0,
            bytemuck::cast_slice(&build_genomes(self.seed, &self.settings)),
        );
        let data = build_agents(self.seed, &self.settings);
        for b in &self.agent_buffers {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&data));
        }
        let habitat = build_habitat(self.seed);
        let food = build_resources(&habitat);
        for b in [&self.resource_buffer, &self.resource_display_buffer] {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&food));
        }
        queue.write_buffer(
            &self.ground_buffer,
            0,
            bytemuck::cast_slice(&build_ground(&habitat)),
        );
        queue.write_buffer(
            &self.fertility_buffer,
            0,
            bytemuck::cast_slice(&habitat.iter().map(|h| 0.4 + h * 0.35).collect::<Vec<_>>()),
        );
        queue.write_buffer(
            &self.terrain_buffer,
            0,
            bytemuck::cast_slice(&build_terrain_pair(self.seed, 0)),
        );
        for b in [
            &self.event_buffer,
            &self.death_stats_buffer,
            &self.perception_buffer,
            &self.decision_buffer,
        ] {
            queue.write_buffer(b, 0, &vec![0; b.size() as usize]);
        }
        self.tick = 0;
        self.current_buffer = 0;
        self.terrain_epoch = 0;
        self.update_params(queue);
    }
    pub fn update_params(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(&params_for(self.tick, &self.settings, self.seed)),
        );
    }
    fn dispatch(&self, e: &mut wgpu::CommandEncoder, name: &str, group: usize, x: u32, y: u32) {
        self.passes[name].dispatch(e, group, x, y);
    }
    pub fn encode_ticks(
        &mut self,
        e: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ticks: u32,
    ) {
        assert!(ticks <= 32);
        if ticks == 0 {
            return;
        }
        let ps: Vec<_> = (0..ticks)
            .map(|n| params_for(self.tick + n, &self.settings, self.seed))
            .collect();
        queue.write_buffer(&self.tick_params_buffer, 0, bytemuck::cast_slice(&ps));
        let groups = MAX_AGENTS.div_ceil(64);
        for offset in 0..ticks {
            let epoch = self.tick / 8192;
            if self.terrain_epoch != epoch && self.settings.evolving_landscape {
                let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain update"),
                    contents: bytemuck::cast_slice(&build_terrain_pair(self.seed, epoch)),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
                e.copy_buffer_to_buffer(&staging, 0, &self.terrain_buffer, 0, staging.size());
                self.terrain_epoch = epoch;
            }
            e.copy_buffer_to_buffer(
                &self.tick_params_buffer,
                offset as u64 * std::mem::size_of::<SimParams>() as u64,
                &self.params_buffer,
                0,
                std::mem::size_of::<SimParams>() as u64,
            );
            let s = self.current_buffer;
            let d = 1 - s;
            // Only slots already dead at tick start may be reused: disjoint parent/child writes.
            self.dispatch(e, "free", s, groups, 1);
            self.dispatch(e, "free_init", 0, groups, 1);
            for n in 0..14 {
                self.dispatch(e, &format!("free_{n}"), n % 2, groups, 1);
            }
            self.dispatch(e, "free_compact", 0, groups, 1);
            self.dispatch(e, "resource", 0, 64, 64);
            self.dispatch(e, "clear", 0, 32, 32);
            self.dispatch(e, "count", s, groups, 1);
            self.dispatch(e, "spatial_init", 0, 1024, 1);
            for n in 0..16 {
                self.dispatch(e, &format!("spatial_{n}"), n % 2, 1024, 1);
            }
            self.dispatch(e, "cursors", 0, 1024, 1);
            self.dispatch(e, "scatter", s, groups, 1);
            self.dispatch(e, "perceive", s, groups, 1);
            self.dispatch(e, "decide", s, groups, 1);
            self.dispatch(e, "consume", s, groups, 1);
            self.dispatch(e, "body", s, groups, 1);
            for n in ["interact_clear", "interact_propose", "interact_resolve"] {
                self.dispatch(e, n, d, groups, 1);
            }
            self.dispatch(e, "birth_init", 0, groups, 1);
            for n in 0..14 {
                self.dispatch(e, &format!("birth_{n}"), n % 2, groups, 1);
            }
            self.dispatch(e, "birth_compact", 0, groups, 1);
            self.dispatch(e, "birth", d, groups, 1);
            self.dispatch(e, "release", d, groups, 1);
            self.current_buffer = d;
            self.tick += 1;
        }
        e.copy_buffer_to_buffer(
            &self.resource_buffer,
            0,
            &self.resource_display_buffer,
            0,
            self.resource_buffer.size(),
        );
        e.clear_buffer(&self.alive_count_buffer, 0, None);
        self.dispatch(e, "alive", self.current_buffer, groups, 1);
    }
    pub fn select_agent(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world_position: [f32; 2],
        radius: f32,
    ) -> Option<SelectionOutput> {
        queue.write_buffer(
            &self.selection_params_buffer,
            0,
            bytemuck::bytes_of(&SelectionParams {
                world_position,
                radius,
                padding: 0.0,
            }),
        );
        queue.write_buffer(&self.selection_key_buffer, 0, bytemuck::bytes_of(&u32::MAX));
        let mut e = device.create_command_encoder(&Default::default());
        e.clear_buffer(&self.selection_output_buffer, 0, None);
        self.dispatch(
            &mut e,
            "select",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        self.dispatch(
            &mut e,
            "selected",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        queue.submit(Some(e.finish()));
        let bytes =
            observability::read_buffer(device, queue, &self.selection_output_buffer).ok()?;
        let result: SelectionOutput = bytemuck::pod_read_unaligned(&bytes);
        (result.selected != 0).then_some(result)
    }
    pub fn apply_resource_shock(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
        delta: f32,
    ) {
        queue.write_buffer(
            &self.intervention_params_buffer,
            0,
            bytemuck::bytes_of(&InterventionParams {
                center,
                radius,
                delta,
            }),
        );
        let mut e = device.create_command_encoder(&Default::default());
        self.dispatch(&mut e, "shock", 0, 64, 64);
        e.copy_buffer_to_buffer(
            &self.resource_buffer,
            0,
            &self.resource_display_buffer,
            0,
            self.resource_buffer.size(),
        );
        queue.submit(Some(e.finish()));
    }
    pub fn kill_agents_in_region(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
    ) {
        queue.write_buffer(
            &self.intervention_params_buffer,
            0,
            bytemuck::bytes_of(&InterventionParams {
                center,
                radius,
                delta: 0.0,
            }),
        );
        let mut e = device.create_command_encoder(&Default::default());
        self.dispatch(
            &mut e,
            "kill",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        self.dispatch(
            &mut e,
            "release",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        queue.submit(Some(e.finish()));
    }
    pub fn copy_alive_count(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            &self.alive_count_buffer,
            0,
            &self.alive_count_readback,
            0,
            4,
        );
    }

    pub fn copy_death_stats(&self, encoder: &mut wgpu::CommandEncoder) {
        encoder.copy_buffer_to_buffer(
            &self.death_stats_buffer,
            0,
            &self.death_stats_readback,
            0,
            (DEATH_STATS_COUNT * 4) as u64,
        );
    }

    pub fn read_death_stats(
        &self,
        device: &wgpu::Device,
    ) -> Option<[u32; DEATH_STATS_COUNT as usize]> {
        let slice = self.death_stats_readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        receiver.recv().ok()?.ok()?;
        let mapped = slice.get_mapped_range();
        let values = bytemuck::cast_slice(&mapped);
        let result: [u32; DEATH_STATS_COUNT as usize] = values.try_into().ok()?;
        drop(mapped);
        self.death_stats_readback.unmap();
        Some(result)
    }

    pub fn read_alive_count(&self, device: &wgpu::Device) -> Option<u32> {
        let slice = self.alive_count_readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        receiver.recv().ok()?.ok()?;
        let mapped = slice.get_mapped_range();
        let result = *bytemuck::from_bytes::<u32>(&mapped);
        drop(mapped);
        self.alive_count_readback.unmap();
        Some(result)
    }
}
fn params_for(tick: u32, s: &SimSettings, seed: u32) -> SimParams {
    SimParams {
        world_size: WORLD_SIZE,
        resource_grid_size: RESOURCE_GRID,
        agent_count: MAX_AGENTS,
        tick,
        time_and_costs: [
            1.0,
            s.resource_regeneration,
            s.movement_energy_cost,
            s.metabolic_cost,
        ],
        resource_and_noise: [
            s.consume_amount,
            s.conversion_efficiency,
            s.heterogeneity,
            0.0,
        ],
        sensor_and_padding: [s.sensor_radius, s.maturity_age, 0.0, s.reproduction_cost],
        physical: [
            f32::from(s.force_enabled),
            f32::from(s.communication_enabled),
            0.0,
            0.0,
        ],
        lifecycle: [seed, s.birth_cooldown, 0, u32::from(s.evolving_landscape)],
    }
}
fn build_agents(seed: u32, s: &SimSettings) -> Vec<AgentGpu> {
    let mut rng = seed.max(1);
    (0..MAX_AGENTS)
        .map(|i| AgentGpu {
            position: [
                random01(&mut rng) * WORLD_SIZE,
                random01(&mut rng) * WORLD_SIZE,
            ],
            energy: 65.0,
            food: if i < s.population { 2.0 } else { 0.0 },
            age: random01(&mut rng) * 300.0,
            max_speed: 1.2,
            sensor_radius: s.sensor_radius,
            max_age: 9000.0 + random01(&mut rng) * 2000.0,
            rng: rng ^ i,
            alive: u32::from(i < s.population),
            generation: 1,
            event_actor: MAX_AGENTS,
            target: MAX_AGENTS,
            lineage_id: i + 1,
            ..Default::default()
        })
        .collect()
}
fn build_genomes(seed: u32, s: &SimSettings) -> Vec<f32> {
    let mut genes = vec![0.0; MAX_AGENTS as usize * GENOME_SIZE];
    let initial = bootstrap_genome();
    let mut rng = seed ^ 0x184a2321;
    for i in 0..s.population as usize {
        let row = &mut genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE];
        if s.founder_genomes.is_empty() {
            for (g, v) in row.iter_mut().zip(initial) {
                *g = v + (random01(&mut rng) - 0.5) * 0.02;
            }
        } else {
            row.copy_from_slice(&s.founder_genomes[i % s.founder_genomes.len()]);
        }
    }
    genes
}
fn build_habitat(seed: u32) -> Vec<f32> {
    build_habitat_at(seed, 0)
}

fn build_terrain_pair(seed: u32, epoch: u32) -> Vec<[f32; 4]> {
    let a = build_habitat_at(seed, epoch);
    let b = build_habitat_at(seed, epoch.wrapping_add(1));
    let ma = (a.iter().sum::<f32>() / a.len() as f32).max(0.001);
    let mb = (b.iter().sum::<f32>() / b.len() as f32).max(0.001);
    a.iter()
        .zip(&b)
        .map(|(a, b)| [*a, *b, *a / ma, *b / mb])
        .collect()
}

fn build_habitat_at(seed: u32, epoch: u32) -> Vec<f32> {
    let mut rng = seed ^ 0xa341_316c;
    let mut patches: Vec<[f32; 7]> = Vec::new();
    for i in 0..24 {
        let mut center = [0.5; 2];
        for _ in 0..64 {
            center = [
                0.06 + random01(&mut rng) * 0.88,
                0.06 + random01(&mut rng) * 0.88,
            ];
            if patches
                .iter()
                .all(|p| (p[0] - center[0]).hypot(p[1] - center[1]) > 0.11)
            {
                break;
            }
        }
        let radius = if i < 5 {
            0.075 + random01(&mut rng) * 0.04
        } else {
            0.02 + random01(&mut rng) * 0.025
        };
        let angle = random01(&mut rng) * std::f32::consts::TAU;
        patches.push([
            center[0],
            center[1],
            radius,
            0.65 + random01(&mut rng) * 0.7,
            angle.cos(),
            angle.sin(),
            1.0,
        ]);
    }
    for (i, p) in patches.iter_mut().enumerate() {
        let mut renewal =
            seed ^ (i as u32).wrapping_mul(7919) ^ (epoch / 3).wrapping_mul(0x9e3779b9);
        if epoch >= 3 {
            p[0] = 0.06 + random01(&mut renewal) * 0.88;
            p[1] = 0.06 + random01(&mut renewal) * 0.88;
        }
        let phase = i as f32 * 2.39996 + epoch as f32 * 0.6;
        p[0] = (p[0] + phase.sin() * 0.035).clamp(0.03, 0.97);
        p[1] = (p[1] + (phase * 0.73).cos() * 0.035).clamp(0.03, 0.97);
        p[2] *= 0.85 + 0.25 * (phase * 0.8).sin();
        // Some regions lapse during a renewal cycle; interpolation fades them
        // out while replacement locations grow, without instantaneous jumps.
        p[6] = if random01(&mut renewal) < 0.16 {
            0.0
        } else {
            0.75 + 0.25 * (phase * 0.9).cos()
        };
    }
    let mut habitat = vec![0.0f32; (RESOURCE_GRID * RESOURCE_GRID) as usize];
    for y in 0..RESOURCE_GRID {
        for x in 0..RESOURCE_GRID {
            let xf = (x as f32 + 0.5) / RESOURCE_GRID as f32;
            let yf = (y as f32 + 0.5) / RESOURCE_GRID as f32;
            // Smooth domain warping and edge detail avoid perfect circles and pixel noise.
            let wx = xf + (terrain_noise(xf * 7.0, yf * 7.0, seed) - 0.5) * 0.045;
            let wy = yf + (terrain_noise(xf * 7.0, yf * 7.0, seed ^ 7919) - 0.5) * 0.045;
            let edge = (terrain_noise(xf * 35.0, yf * 35.0, seed ^ 1237) - 0.5) * 0.3;
            let mut value = 0.0f32;
            for p in &patches {
                let dx = wx - p[0];
                let dy = wy - p[1];
                let u = (dx * p[4] + dy * p[5]) / p[2];
                let v = (-dx * p[5] + dy * p[4]) / (p[2] * p[3]);
                let distance = u.hypot(v) + edge;
                let t = ((1.2 - distance) / 1.2).clamp(0.0, 1.0);
                let patch = t * t * (3.0 - 2.0 * t) * p[6];
                value = value.max(patch);
            }
            let shift = epoch as f32 * 0.025;
            let broad = terrain_noise(xf * 3.3 + shift, yf * 3.3 + shift, seed ^ 991) * 0.55
                + terrain_noise(xf * 7.1 + shift, yf * 7.1, seed ^ 1777) * 0.30
                + terrain_noise(xf * 15.3, yf * 15.3 + shift, seed ^ 3137) * 0.15;
            // Keep a low-yield landscape between peaks. A hard threshold made
            // the world read as isolated islands and forced every trip to be
            // an all-or-nothing migration. The lower shoulder provides
            // forage while the nonlinear upper shoulder preserves rich hubs.
            let t = ((value * 0.8 + broad * 0.35 - 0.22) / 0.78).clamp(0.0, 1.0);
            let shoulder = t * t * (3.0 - 2.0 * t);
            habitat[(y * RESOURCE_GRID + x) as usize] = shoulder * 0.78 + t * 0.22;
        }
    }
    habitat
}

fn terrain_noise(x: f32, y: f32, seed: u32) -> f32 {
    let ix = x.floor() as u32;
    let iy = y.floor() as u32;
    let sample = |dx: u32, dy: u32| {
        let mut h = seed
            ^ ix.wrapping_add(dx).wrapping_mul(0x9e3779b9)
            ^ iy.wrapping_add(dy).wrapping_mul(0x85ebca6b);
        h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
        h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let tx = x.fract();
    let ty = y.fract();
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let top = sample(0, 0) * (1.0 - sx) + sample(1, 0) * sx;
    let bottom = sample(0, 1) * (1.0 - sx) + sample(1, 1) * sx;
    top * (1.0 - sy) + bottom * sy
}

fn build_resources(habitat: &[f32]) -> Vec<u32> {
    habitat
        .iter()
        .map(|h| (h * 0.55 * RESOURCE_SCALE) as u32)
        .collect()
}

fn build_ground(habitat: &[f32]) -> Vec<[u32; 8]> {
    let mean = (habitat.iter().sum::<f32>() / habitat.len() as f32).max(0.001);
    habitat
        .iter()
        .map(|h| [0, 0, 0, 0, 0, 0, h.to_bits(), (h / mean).to_bits()])
        .collect()
}

fn random01(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    ((*state >> 8) as f32) / 16_777_215.0
}

pub fn shader_source(source: &str) -> String {
    format!("{}\n{}", include_str!("../shaders/common.wgsl"), source)
}

#[path = "observability.rs"]
pub mod observability;
#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
