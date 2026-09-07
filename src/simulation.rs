pub use crate::model::*;
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, sync::mpsc};
use wgpu::util::DeviceExt;
const RESOURCE_SCALE: f32 = 1000.0;

pub(crate) struct Compute {
    pipeline: wgpu::ComputePipeline,
    groups: Vec<wgpu::BindGroup>,
    #[cfg(test)]
    pub(crate) timing: Option<(wgpu::QuerySet, u32)>,
}
fn pair<'a>(f: impl Fn(usize) -> Vec<&'a wgpu::Buffer>) -> Vec<Vec<&'a wgpu::Buffer>> {
    (0..2).map(f).collect()
}
impl Compute {
    pub(crate) fn new(
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
        Self {
            pipeline,
            groups,
            #[cfg(test)]
            timing: None,
        }
    }
    fn descriptor(&self) -> wgpu::ComputePassDescriptor<'_> {
        wgpu::ComputePassDescriptor {
            #[cfg(test)]
            timestamp_writes: self.timing.as_ref().map(|(queries, index)| {
                wgpu::ComputePassTimestampWrites {
                    query_set: queries,
                    beginning_of_pass_write_index: Some(*index),
                    end_of_pass_write_index: Some(*index + 1),
                }
            }),
            ..Default::default()
        }
    }
    fn dispatch_indirect(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        group: usize,
        arguments: &wgpu::Buffer,
    ) {
        let mut pass = encoder.begin_compute_pass(&self.descriptor());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.groups[group], &[]);
        pass.dispatch_workgroups_indirect(arguments, 0);
    }
    pub(crate) fn dispatch(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        group: usize,
        x: u32,
        y: u32,
    ) {
        let mut pass = encoder.begin_compute_pass(&self.descriptor());
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
    device: wgpu::Device,
    pub search: crate::evolution::Search,
    pub progress: crate::evolution::Progress,
    pub settings: SimSettings,
    pub seed: u32,
    pub tick: u32,
    /// Environment/action time at this world's tick zero. This is separate
    /// from tick so every world's survival duration is measured from zero.
    pub environment_start_age: u32,
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
    pub family_observer: Option<crate::family_observer::FamilyObserver>,
    pub(crate) active_indices: wgpu::Buffer,
    birth_dispatch: wgpu::Buffer,
    birth_flags: wgpu::Buffer,
    pub(crate) decision_buffer: wgpu::Buffer,
    fertility_buffer: wgpu::Buffer,
    terrain_buffer: wgpu::Buffer,
    terrain_epoch: u32,
    pub(crate) death_stats_buffer: wgpu::Buffer,
    event_buffer: wgpu::Buffer,
    summary_buffer: wgpu::Buffer,
    tick_params_buffer: wgpu::Buffer,
    alive_count_readback: wgpu::Buffer,
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
            "GPU storage limit below primitive-world body budget"
        );
        assert!(
            MAX_AGENTS as u64 * GENOME_SIZE as u64 * 4
                <= u64::from(device.limits().max_storage_buffer_binding_size)
                    .min(device.limits().max_buffer_size),
            "GPU storage limit below the fixed-brain genome budget"
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
        let cell_offsets = buffer(device, "cell offsets", SPATIAL_CELL_COUNT as u64 * 4);
        let cursors = buffer(device, "scatter cursors", SPATIAL_CELL_COUNT as u64 * 4);
        let indices = buffer(device, "body indices", MAX_AGENTS as u64 * 4);
        let free_flags = buffer(device, "free flags", MAX_AGENTS as u64 * 4);
        let birth_flags = buffer(device, "birth requests", MAX_AGENTS as u64 * 4);
        let free_prefix = buffer(device, "free prefix", MAX_AGENTS as u64 * 4);
        let birth_prefix = buffer(device, "birth prefix", MAX_AGENTS as u64 * 4);
        let active_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("living slots and indirect work count"),
            size: u64::from(MAX_AGENTS + 4) * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let free_indices = buffer(device, "free slots", MAX_AGENTS as u64 * 4);
        let birth_dispatch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eligible birth work count"),
            size: 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
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
        for (name, input, output) in [
            ("spatial", &occupancy_buffer, &cell_offsets),
            ("free", &free_flags, &free_prefix),
            ("birth", &birth_flags, &birth_prefix),
        ] {
            let sums = buffer(
                device,
                "scan block totals",
                (input.size() / 4).div_ceil(256) * 4,
            );
            for entry in ["blocks", "sums", "add"] {
                let key = format!("{name}_{entry}");
                add!(
                    &key,
                    "../shaders/block_scan.wgsl",
                    entry,
                    "rww",
                    vec![vec![input, output, &sums]]
                );
            }
        }
        add!(
            "cursors",
            "../shaders/prepare_scatter.wgsl",
            "main",
            "rw",
            vec![vec![&cell_offsets, &cursors]]
        );
        add!(
            "scatter",
            "../shaders/scatter_agents.wgsl",
            "main",
            "rwwu",
            pair(|s| vec![&agent_buffers[s], &cursors, &indices, &params_buffer])
        );
        #[cfg(test)]
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
                &cell_offsets,
                &indices,
                &perception_buffer,
                &params_buffer
            ])
        );
        #[cfg(test)]
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
        passes.insert(
            "perceive_live".into(),
            Compute::new(
                device,
                "perceive_live",
                &live_source(include_str!("../shaders/perceive.wgsl"), 8),
                "main",
                "rrwrrrwur",
                pair(|s| {
                    vec![
                        &agent_buffers[s],
                        &resource_buffer,
                        &ground_buffer,
                        &occupancy_buffer,
                        &cell_offsets,
                        &indices,
                        &perception_buffer,
                        &params_buffer,
                        &active_indices,
                    ]
                }),
            ),
        );
        passes.insert(
            "decide_live".into(),
            Compute::new(
                device,
                "decide_live",
                &live_source(include_str!("../shaders/decide.wgsl"), 5),
                "main",
                "rrwurr",
                pair(|s| {
                    vec![
                        &agent_buffers[s],
                        &perception_buffer,
                        &decision_buffer,
                        &params_buffer,
                        &genome_buffer,
                        &active_indices,
                    ]
                }),
            ),
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
        #[cfg(test)]
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
        passes.insert(
            "body_live".into(),
            Compute::new(
                device,
                "body_live",
                &live_source(include_str!("../shaders/update_agents.wgsl"), 7),
                "main",
                "rrrw uwwr".replace(' ', "").as_str(),
                pair(|s| {
                    vec![
                        &agent_buffers[s],
                        &decision_buffer,
                        &request_buffer,
                        &agent_buffers[1 - s],
                        &params_buffer,
                        &birth_flags,
                        &death_stats_buffer,
                        &active_indices,
                    ]
                }),
            ),
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
            "free_compact",
            "../shaders/compact_slots.wgsl",
            "main",
            "rrwwww",
            vec![vec![
                &free_flags,
                &free_prefix,
                &free_indices,
                &active_indices,
                &perception_buffer,
                &decision_buffer
            ]]
        );
        add!(
            "birth_compact",
            "../shaders/compact_agent_indices.wgsl",
            "main",
            "rrwrw",
            vec![vec![
                &birth_flags,
                &birth_prefix,
                &parents,
                &free_prefix,
                &birth_dispatch
            ]]
        );
        add!(
            "birth",
            "../shaders/apply_births.wgsl",
            "main",
            "wrrrruwrw",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix,
                &parents,
                &birth_prefix,
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
            device: device.clone(),
            search: crate::evolution::Search::default(),
            progress: crate::evolution::Progress::initial(seed),
            settings: SimSettings::default(),
            seed,
            tick: 0,
            environment_start_age: 0,
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
            family_observer: None,
            decision_buffer,
            active_indices,
            birth_dispatch,
            birth_flags,
            fertility_buffer,
            terrain_buffer,
            terrain_epoch: 0,
            death_stats_buffer,
            event_buffer,
            summary_buffer,
            tick_params_buffer,
            alive_count_readback,
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
        self.reset_with_genomes_at(queue, None, 0);
    }
    pub(crate) fn reset_with_genomes_at(
        &mut self,
        queue: &wgpu::Queue,
        founders: Option<&[f32]>,
        environment_start_age: u32,
    ) {
        // Clear unused storage on the GPU; upload only the founding population.
        let mut clear = self.device.create_command_encoder(&Default::default());
        for buffer in [
            &self.genome_buffer,
            &self.event_buffer,
            &self.death_stats_buffer,
            &self.perception_buffer,
            &self.decision_buffer,
        ] {
            clear.clear_buffer(buffer, 0, None);
        }
        queue.submit(Some(clear.finish()));
        self.family_observer = None;
        self.progress = crate::evolution::Progress::initial(self.seed);
        self.settings.validate().expect("valid reset settings");
        let random;
        let genes = match founders {
            Some(g) => g,
            None => {
                random = build_genomes(self.seed, &self.settings);
                &random
            }
        };
        if !genes.is_empty() {
            queue.write_buffer(&self.genome_buffer, 0, bytemuck::cast_slice(genes));
        }
        self.search = crate::evolution::Search {
            incumbent: genes.to_vec(),
            challenger: Vec::new(),
        };
        let data = build_agents(self.seed, &self.settings);
        for b in &self.agent_buffers {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&data));
        }
        self.environment_start_age = environment_start_age;
        let environment_epoch = environment_start_age / 8192;
        let terrain_a = build_habitat_at(
            self.seed,
            environment_epoch,
            self.settings.habitat_contrast,
            self.settings.metabolic_ramp_ticks,
        );
        let terrain_b = build_habitat_at(
            self.seed,
            environment_epoch + 1,
            self.settings.habitat_contrast,
            self.settings.metabolic_ramp_ticks,
        );
        let terrain_phase = (environment_start_age % 8192) as f32 / 8192.0;
        let terrain_blend = terrain_phase * terrain_phase * (3.0 - 2.0 * terrain_phase);
        let habitat: Vec<_> = terrain_a
            .iter()
            .zip(&terrain_b)
            .map(|(a, b)| a + (b - a) * terrain_blend)
            .collect();
        let food = crate::environment::rotate_grid(
            build_resources(&habitat),
            RESOURCE_GRID as usize,
            self.settings.environment_rotation,
        );
        for b in [&self.resource_buffer, &self.resource_display_buffer] {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&food));
        }
        queue.write_buffer(
            &self.ground_buffer,
            0,
            bytemuck::cast_slice(&crate::environment::rotate_grid(
                build_ground(&habitat),
                RESOURCE_GRID as usize,
                self.settings.environment_rotation,
            )),
        );
        queue.write_buffer(
            &self.fertility_buffer,
            0,
            bytemuck::cast_slice(&crate::environment::rotate_grid(
                habitat.iter().map(|h| 0.4 + h * 0.35).collect::<Vec<_>>(),
                RESOURCE_GRID as usize,
                self.settings.environment_rotation,
            )),
        );
        queue.write_buffer(
            &self.terrain_buffer,
            0,
            bytemuck::cast_slice(&crate::environment::rotate_grid(
                terrain_pair(&terrain_a, &terrain_b),
                RESOURCE_GRID as usize,
                self.settings.environment_rotation,
            )),
        );
        self.tick = 0;
        self.current_buffer = 0;
        self.terrain_epoch = environment_epoch;
        self.update_params(queue);
    }
    pub fn update_params(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(&params_for(
                self.tick,
                self.tick.saturating_add(self.environment_start_age),
                &self.settings,
                self.seed,
            )),
        );
    }
    fn dispatch(&self, e: &mut wgpu::CommandEncoder, name: &str, group: usize, x: u32, y: u32) {
        self.passes[name].dispatch(e, group, x, y);
    }
    fn scan(&self, e: &mut wgpu::CommandEncoder, name: &str, count: u32) {
        self.dispatch(e, &format!("{name}_blocks"), 0, count.div_ceil(256), 1);
        self.dispatch(e, &format!("{name}_sums"), 0, 1, 1);
        self.dispatch(e, &format!("{name}_add"), 0, count.div_ceil(256), 1);
    }

    pub fn encode_ticks(
        &mut self,
        e: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ticks: u32,
    ) {
        assert!(ticks <= 32);
        assert!(
            ticks <= MAX_WORLD_TICKS.saturating_sub(self.tick),
            "World tick capacity reached; save without scoring extinction"
        );
        if ticks == 0 {
            return;
        }
        let ps: Vec<_> = (0..ticks)
            .map(|n| {
                let tick = self.tick + n;
                params_for(
                    tick,
                    tick.saturating_add(self.environment_start_age),
                    &self.settings,
                    self.seed,
                )
            })
            .collect();
        queue.write_buffer(&self.tick_params_buffer, 0, bytemuck::cast_slice(&ps));
        let groups = MAX_AGENTS.div_ceil(64);
        for offset in 0..ticks {
            let environment_tick = self.tick.saturating_add(self.environment_start_age);
            let epoch = environment_tick / 8192;
            if self.terrain_epoch != epoch && self.settings.evolving_landscape {
                let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain update"),
                    contents: bytemuck::cast_slice(&crate::environment::rotate_grid(
                        build_terrain_pair(
                            self.seed,
                            epoch,
                            self.settings.habitat_contrast,
                            self.settings.metabolic_ramp_ticks,
                        ),
                        RESOURCE_GRID as usize,
                        self.settings.environment_rotation,
                    )),
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
            self.scan(e, "free", MAX_AGENTS);
            self.dispatch(e, "free_compact", 0, groups, 1);
            self.dispatch(e, "resource", 0, 64, 64);
            self.dispatch(e, "clear", 0, 32, 32);
            self.dispatch(e, "count", s, groups, 1);
            self.scan(e, "spatial", SPATIAL_CELL_COUNT);
            self.dispatch(e, "cursors", 0, 1024, 1);
            self.dispatch(e, "scatter", s, groups, 1);
            self.passes["perceive_live"].dispatch_indirect(e, s, &self.active_indices);
            self.passes["decide_live"].dispatch_indirect(e, s, &self.active_indices);
            self.dispatch(e, "consume", s, groups, 1);
            // Preserve dead records (including slot generations) with a bulk GPU
            // copy. Only living bodies need the expensive structured update.
            e.copy_buffer_to_buffer(
                &self.agent_buffers[s],
                0,
                &self.agent_buffers[d],
                0,
                self.agent_buffers[s].size(),
            );
            e.clear_buffer(&self.birth_flags, 0, None);
            self.passes["body_live"].dispatch_indirect(e, s, &self.active_indices);
            for n in ["interact_clear", "interact_propose", "interact_resolve"] {
                self.dispatch(e, n, d, groups, 1);
            }
            self.scan(e, "birth", MAX_AGENTS);
            self.dispatch(e, "birth_compact", 0, groups, 1);
            self.passes["birth"].dispatch_indirect(e, d, &self.birth_dispatch);
            self.dispatch(e, "release", d, groups, 1);
            if let Some(observer) = &self.family_observer {
                observer.encode(e, d);
            }
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
    /// Read the same body by slot AND incarnation, not the nearest new neighbor.
    /// Only observer buffers are written; all simulation buffers remain read-only.
    #[cfg(test)]
    pub fn refresh_selected_agent(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        previous: &SelectionOutput,
    ) -> Result<Option<SelectionOutput>, String> {
        if previous.selected == 0 || previous.selected > MAX_AGENTS {
            return Ok(None);
        }
        let slot = previous.selected - 1;
        queue.write_buffer(&self.selection_key_buffer, 0, bytemuck::bytes_of(&slot));
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.clear_buffer(&self.selection_output_buffer, 0, None);
        self.dispatch(
            &mut encoder,
            "selected",
            self.current_buffer,
            MAX_AGENTS.div_ceil(64),
            1,
        );
        queue.submit(Some(encoder.finish()));
        let bytes = observability::read_buffer(device, queue, &self.selection_output_buffer)?;
        let current: SelectionOutput = bytemuck::pod_read_unaligned(&bytes);
        Ok((current.selected == previous.selected
            && current.agent.generation == previous.agent.generation
            && current.agent.lineage_id == previous.agent.lineage_id
            && current.agent.birth_tick == previous.agent.birth_tick)
            .then_some(current))
    }

    pub fn apply_resource_shock(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
        delta: f32,
    ) {
        queue.write_buffer(&self.death_stats_buffer, 30 * 4, bytemuck::bytes_of(&1u32));
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
    #[cfg(test)]
    pub fn kill_agents_in_region(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
    ) {
        queue.write_buffer(&self.death_stats_buffer, 30 * 4, bytemuck::bytes_of(&1u32));
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
    pub fn encode_telemetry(&self, e: &mut wgpu::CommandEncoder, output: &wgpu::Buffer) {
        e.copy_buffer_to_buffer(&self.alive_count_buffer, 0, output, 0, 4);
        e.copy_buffer_to_buffer(
            &self.death_stats_buffer,
            0,
            output,
            4,
            u64::from(DEATH_STATS_COUNT) * 4,
        );
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
pub(crate) fn metabolic_cost_at(tick: u32, s: &SimSettings) -> f32 {
    if s.metabolic_ramp_ticks == 0 {
        return s.metabolic_cost;
    }
    let progress = (tick.min(s.metabolic_ramp_ticks) as f32) / s.metabolic_ramp_ticks as f32;
    METABOLIC_START_COST + (s.metabolic_cost - METABOLIC_START_COST) * progress
}

pub(crate) const MOBILITY_START_TICK: u32 = 50_000;
pub(crate) const MOBILITY_CAP_TICK: u32 = 250_000;
pub(crate) const FRAGMENTATION_CAP_TICK: u32 = 500_000;
pub(crate) const SEASONALITY_CAP_TICK: u32 = 750_000;

fn pressure_between(age: u32, start: u32, end: u32) -> f32 {
    debug_assert!(start < end);
    (age.saturating_sub(start).min(end - start) as f32) / (end - start) as f32
}

/// The effective environment age controls only deterministic ecology. Every
/// pressure ramps smoothly to one and remains capped thereafter. Each world
/// starts from age zero; no earned floor carries pressure between worlds.
pub(crate) fn ecological_pressures(age: u32) -> [f32; 4] {
    [
        pressure_between(age, MOBILITY_START_TICK, MOBILITY_CAP_TICK),
        pressure_between(age, MOBILITY_CAP_TICK, FRAGMENTATION_CAP_TICK),
        pressure_between(age, FRAGMENTATION_CAP_TICK, SEASONALITY_CAP_TICK),
        0.0,
    ]
}

fn params_for(tick: u32, environment_tick: u32, s: &SimSettings, seed: u32) -> SimParams {
    SimParams {
        world_size: WORLD_SIZE,
        resource_grid_size: RESOURCE_GRID,
        agent_count: MAX_AGENTS,
        tick,
        time_and_costs: [
            0.0,
            s.resource_regeneration,
            s.movement_energy_cost,
            metabolic_cost_at(environment_tick, s),
        ],
        resource_and_noise: [
            s.consume_amount,
            s.conversion_efficiency,
            s.heterogeneity,
            f32::from(s.social_actions_enabled),
        ],
        sensor_and_padding: [s.sensor_radius, s.maturity_age, 0.0, s.reproduction_cost],
        physical: [
            f32::from(s.force_enabled),
            f32::from(s.communication_enabled),
            s.motor_response_gain,
            // The decision shader uses this duration to spread each action's
            // availability across individual agents during the bootstrap.
            s.metabolic_ramp_ticks as f32,
        ],
        lifecycle: [
            seed,
            s.birth_cooldown,
            s.environment_rotation,
            environment_tick,
        ],
        mutation: [
            BASE_MUTATION_PROBABILITY,
            BASE_MUTATION_MAGNITUDE,
            f32::from(s.evolving_landscape),
            0.0,
        ],
        environment: ecological_pressures(environment_tick),
    }
}
fn build_agents(seed: u32, s: &SimSettings) -> Vec<AgentGpu> {
    let mut rng = seed.max(1);
    (0..MAX_AGENTS)
        .map(|i| AgentGpu {
            position: crate::environment::rotate_point(
                [
                    random01(&mut rng) * WORLD_SIZE,
                    random01(&mut rng) * WORLD_SIZE,
                ],
                WORLD_SIZE,
                s.environment_rotation,
            ),
            energy: 65.0,
            food: if i < s.population { 2.0 } else { 0.0 },
            age: random01(&mut rng) * 300.0,
            max_speed: 1.2,
            sensor_radius: s.sensor_radius,
            max_age: 9000.0 + random01(&mut rng) * 2000.0,
            rng: rng ^ i,
            alive: u32::from(i < s.population),
            generation: 1,
            target: MAX_AGENTS,
            lineage_id: i + 1,
            founder_family: if s.founder_genomes.is_empty() {
                i
            } else {
                (i as usize % s.founder_genomes.len()) as u32
            },
            ..Default::default()
        })
        .collect()
}
fn build_genomes(seed: u32, s: &SimSettings) -> Vec<f32> {
    let mut genes = vec![0.0; s.population as usize * GENOME_SIZE];
    let mut rng = seed ^ 0x184a2321;
    for i in 0..s.population as usize {
        let row = &mut genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE];
        if s.founder_genomes.is_empty() {
            row.copy_from_slice(&random_genome(&mut rng));
        } else {
            row.copy_from_slice(&s.founder_genomes[i % s.founder_genomes.len()]);
        }
    }
    genes
}
fn build_terrain_pair(seed: u32, epoch: u32, contrast: f32, ramp_ticks: u32) -> Vec<[f32; 4]> {
    let a = build_habitat_at(seed, epoch, contrast, ramp_ticks);
    let b = build_habitat_at(seed, epoch.wrapping_add(1), contrast, ramp_ticks);
    terrain_pair(&a, &b)
}
fn terrain_pair(a: &[f32], b: &[f32]) -> Vec<[f32; 4]> {
    let ma = (a.iter().sum::<f32>() / a.len() as f32).max(0.001);
    let mb = (b.iter().sum::<f32>() / b.len() as f32).max(0.001);
    a.iter()
        .zip(b)
        .map(|(a, b)| [*a, *b, *a / ma, *b / mb])
        .collect()
}

fn build_habitat_at(seed: u32, epoch: u32, contrast: f32, ramp_ticks: u32) -> Vec<f32> {
    let mut rng = seed ^ 0xa341_316c;
    let mut patches: Vec<[f32; 7]> = Vec::new();
    let elapsed = epoch.saturating_mul(8192);
    let bootstrap = if ramp_ticks == 0 {
        0.0
    } else {
        1.0 - (elapsed.min(ramp_ticks) as f32) / ramp_ticks as f32
    };
    // After the forgiving start, territory—not total food or body costs—becomes
    // progressively more mobile through the 250,000-tick mobility cap.
    let migration_pressure = if ramp_ticks == 0 {
        pressure_between(elapsed, 0, MOBILITY_CAP_TICK)
    } else {
        let mobility_window = MOBILITY_CAP_TICK.saturating_sub(ramp_ticks);
        if mobility_window == 0 {
            0.0
        } else {
            elapsed.saturating_sub(ramp_ticks).min(mobility_window) as f32 / mobility_window as f32
        }
    };
    let fragmentation = ecological_pressures(elapsed)[1];
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
        let base_radius = if i < 5 {
            0.075 + random01(&mut rng) * 0.04
        } else {
            0.02 + random01(&mut rng) * 0.025
        };
        let radius = base_radius * (1.0 + bootstrap);
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
        if migration_pressure > 0.0 {
            // Early renewals change one stable map every three keyframes. As
            // pressure grows, blend toward a fresh destination every keyframe.
            // Existing terrain interpolation keeps each relocation continuous.
            let mut rapid_renewal =
                seed ^ (i as u32).wrapping_mul(7919) ^ epoch.wrapping_mul(0x9e3779b9);
            let next_x = 0.06 + random01(&mut rapid_renewal) * 0.88;
            let next_y = 0.06 + random01(&mut rapid_renewal) * 0.88;
            p[0] += (next_x - p[0]) * migration_pressure;
            p[1] += (next_y - p[1]) * migration_pressure;
        }
        let phase = i as f32 * 2.39996 + epoch as f32 * 0.6;
        // Keep the same keyframe cadence while gradually increasing travel
        // through 250k, making settled patches less dependable without
        // teleporting food.
        let drift = 0.035 + 0.22 * migration_pressure;
        p[0] = (p[0] + phase.sin() * drift).clamp(0.03, 0.97);
        p[1] = (p[1] + (phase * 0.73).cos() * drift).clamp(0.03, 0.97);
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
    let workers = std::thread::available_parallelism()
        .map_or(1, usize::from)
        .min(8);
    let rows = (RESOURCE_GRID as usize).div_ceil(workers);
    std::thread::scope(|scope| {
        for (chunk_index, chunk) in habitat
            .chunks_mut(rows * RESOURCE_GRID as usize)
            .enumerate()
        {
            let patches = &patches;
            scope.spawn(move || {
                for (local_y, row) in chunk.chunks_mut(RESOURCE_GRID as usize).enumerate() {
                    let y = chunk_index * rows + local_y;
                    for x in 0..RESOURCE_GRID {
                        let xf = (x as f32 + 0.5) / RESOURCE_GRID as f32;
                        let yf = (y as f32 + 0.5) / RESOURCE_GRID as f32;
                        // Smooth domain warping and edge detail avoid perfect circles and pixel noise.
                        let wx = xf + (terrain_noise(xf * 7.0, yf * 7.0, seed) - 0.5) * 0.045;
                        let wy =
                            yf + (terrain_noise(xf * 7.0, yf * 7.0, seed ^ 7919) - 0.5) * 0.045;
                        let edge = (terrain_noise(xf * 35.0, yf * 35.0, seed ^ 1237) - 0.5) * 0.3;
                        let mut value = 0.0f32;
                        for p in patches {
                            let dx = wx - p[0];
                            let dy = wy - p[1];
                            let u = (dx * p[4] + dy * p[5]) / p[2];
                            let v = (-dx * p[5] + dy * p[4]) / (p[2] * p[3]);
                            let distance = u.hypot(v) + edge;
                            let t = ((1.2 - distance) / 1.2).clamp(0.0, 1.0);
                            let patch = t * t * (3.0 - 2.0 * t) * p[6];
                            value = value.max(patch);
                        }
                        let shift = epoch as f32 * (0.025 + 0.18 * migration_pressure);
                        let broad = terrain_noise(xf * 3.3 + shift, yf * 3.3 + shift, seed ^ 991)
                            * 0.55
                            + terrain_noise(xf * 7.1 + shift, yf * 7.1, seed ^ 1777) * 0.30
                            + terrain_noise(xf * 15.3, yf * 15.3 + shift, seed ^ 3137) * 0.15;
                        // The lower shoulder provides low-yield forage between peaks;
                        // the nonlinear upper shoulder preserves rich hubs.
                        let t = ((value * 0.8 + broad * 0.35 - 0.22) / 0.78).clamp(0.0, 1.0);
                        let shoulder = t * t * (3.0 - 2.0 * t);
                        row[x as usize] = shoulder * 0.78 + t * 0.22;
                    }
                }
            });
        }
    });
    apply_fragmentation(&mut habitat, fragmentation, epoch, seed);
    let mean = habitat.iter().sum::<f32>() / habitat.len() as f32;
    // Contrast changes food distribution, not its mean or the body's costs.
    for value in &mut habitat {
        *value = mean + contrast * (*value - mean);
    }
    habitat
}

/// Fragment broad territory into pockets while preserving the keyframe mean.
fn apply_fragmentation(habitat: &mut [f32], fragmentation: f32, epoch: u32, seed: u32) {
    if fragmentation <= 0.0 {
        return;
    }
    let mean = habitat.iter().sum::<f32>() / habitat.len() as f32;
    let shift = epoch as f32 * 0.19;
    let mut fragmented_mean = 0.0;
    for (index, value) in habitat.iter_mut().enumerate() {
        let x = (index % RESOURCE_GRID as usize) as f32 / RESOURCE_GRID as f32;
        let y = (index / RESOURCE_GRID as usize) as f32 / RESOURCE_GRID as f32;
        let noise = terrain_noise(x * 7.0 + shift, y * 7.0 - shift, seed ^ 0x5f37_59df);
        let mask = ((noise - 0.30) / 0.40).clamp(0.0, 1.0);
        let smooth_mask = mask * mask * (3.0 - 2.0 * mask);
        let factor = 1.0 + fragmentation * (1.8 * smooth_mask - 0.9);
        *value *= factor.max(0.05);
        fragmented_mean += *value;
    }
    let rescale = mean / (fragmented_mean / habitat.len() as f32).max(0.000_001);
    for value in habitat {
        *value *= rescale;
    }
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

pub fn live_source(source: &str, binding: u32) -> String {
    assert_eq!(source.matches("let i=id.x;").count(), 1);
    format!(
        "@group(0) @binding({binding}) var<storage,read> live_slots:array<u32>;\n{}",
        source
            .replace(
                "@workgroup_size(64)",
                "@workgroup_size(LIVE_WORKGROUP_SIZE)"
            )
            .replace(
                "let i=id.x;",
                "if(id.x>=live_slots[3]){return;}let i=live_slots[4u+id.x];"
            )
    )
}

pub fn shader_source(source: &str) -> String {
    let constants = [
        ("LIVE_WORKGROUP_SIZE", LIVE_WORKGROUP_SIZE),
        ("INPUT_COUNT", INPUTS),
        ("HIDDEN_COUNT", HIDDEN),
        ("OUTPUT_COUNT", OUTPUTS),
        ("GENOME_SIZE", GENOME_SIZE),
        ("NODE_BIAS", NODE_BIAS),
        ("GATE_BIAS", GATE_BIAS),
        ("OUTPUT_BIAS", OUTPUT_BIAS),
        ("INPUT_BASE", INPUT_BASE),
        ("RECURRENT_BASE", RECURRENT_BASE),
        ("GATE_BASE", GATE_BASE),
        ("OUTPUT_BASE", OUTPUT_BASE),
        ("FORCE_OUTPUT", FORCE_OUTPUT),
    ]
    .map(|(name, value)| format!("const {name}:u32={value}u;"))
    .join("\n");
    let source = source.replace(
        "// BRAIN_MUTATION",
        include_str!("../shaders/brain_mutation.wgsl"),
    );
    format!(
        "{constants}\n{}\n{source}",
        include_str!("../shaders/common.wgsl")
    )
}

#[path = "observability.rs"]
pub mod observability;
#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;
