pub use crate::model::*;
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, sync::mpsc};
use wgpu::util::DeviceExt;
const RESOURCE_SCALE: f32 = 1000.0;

fn parameter_stride(device: &wgpu::Device) -> u64 {
    (std::mem::size_of::<SimParams>() as u64).next_multiple_of(u64::from(
        device.limits().min_uniform_buffer_offset_alignment,
    ))
}

pub(crate) struct Compute {
    pipeline: wgpu::ComputePipeline,
    groups: Vec<wgpu::BindGroup>,
    parameter_binding: Option<usize>,
    binding_buffers: Vec<Vec<wgpu::Buffer>>,
    tick_groups: Vec<wgpu::BindGroup>,
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
        // SimParams is the shared 144-byte uniform; other uniforms (selection,
        // interventions) retain their ordinary bindings and zero offsets.
        let parameter_binding = kinds.chars().enumerate().find_map(|(i, k)| {
            (k == 'u' && buffers[0][i].size() == std::mem::size_of::<SimParams>() as u64)
                .then_some(i)
        });
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
                    has_dynamic_offset: parameter_binding == Some(i),
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
        let binding_buffers = buffers
            .iter()
            .map(|bs| bs.iter().map(|b| (*b).clone()).collect())
            .collect();
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
            parameter_binding,
            binding_buffers,
            tick_groups: Vec::new(),
            #[cfg(test)]
            timing: None,
        }
    }
    fn prepare_tick_groups(&mut self, device: &wgpu::Device, parameters: &wgpu::Buffer) {
        let Some(binding) = self.parameter_binding else {
            return;
        };
        if !self.tick_groups.is_empty() {
            return;
        }
        let layout = self.pipeline.get_bind_group_layout(0);
        self.tick_groups =
            self.binding_buffers
                .iter()
                .map(|buffers| {
                    let entries: Vec<_> = buffers
                        .iter()
                        .enumerate()
                        .map(|(i, buffer)| wgpu::BindGroupEntry {
                            binding: i as u32,
                            resource: if i == binding {
                                wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                                    buffer: parameters,
                                    offset: 0,
                                    size: wgpu::BufferSize::new(
                                        std::mem::size_of::<SimParams>() as u64
                                    ),
                                })
                            } else {
                                buffer.as_entire_binding()
                            },
                        })
                        .collect();
                    device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("tick parameters"),
                        layout: &layout,
                        entries: &entries,
                    })
                })
                .collect();
    }
    fn bind(&self, pass: &mut wgpu::ComputePass<'_>, group: usize, tick_offset: Option<u32>) {
        if self.parameter_binding.is_some() {
            let (groups, offset) = match tick_offset {
                Some(offset) => (&self.tick_groups, offset),
                None => (&self.groups, 0),
            };
            pass.set_bind_group(0, &groups[group], &[offset]);
        } else {
            pass.set_bind_group(0, &self.groups[group], &[]);
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
    #[cfg(test)]
    fn dispatch_indirect(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        group: usize,
        arguments: &wgpu::Buffer,
    ) {
        let mut pass = encoder.begin_compute_pass(&self.descriptor());
        pass.set_pipeline(&self.pipeline);
        self.bind(&mut pass, group, None);
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
        self.bind(&mut pass, group, None);
        pass.dispatch_workgroups(x, y, 1);
    }
}
// Compute dispatches have separate usage scopes, so wgpu inserts storage
// dependencies within a pass. Copies, clears, and observers flush the batch.
#[derive(Default)]
struct ComputeBatch<'a> {
    commands: Vec<ComputeCommand<'a>>,
    separate: bool,
    tick_offset: Option<u32>,
}
struct ComputeCommand<'a> {
    compute: &'a Compute,
    group: usize,
    dimensions: [u32; 2],
    arguments: Option<&'a wgpu::Buffer>,
    tick_offset: Option<u32>,
}
impl<'a> ComputeBatch<'a> {
    fn dispatch(&mut self, compute: &'a Compute, group: usize, x: u32, y: u32) {
        self.commands.push(ComputeCommand {
            compute,
            group,
            dimensions: [x, y],
            arguments: None,
            tick_offset: self.tick_offset,
        });
    }
    fn indirect(&mut self, compute: &'a Compute, group: usize, args: &'a wgpu::Buffer) {
        self.commands.push(ComputeCommand {
            compute,
            group,
            dimensions: [0, 0],
            arguments: Some(args),
            tick_offset: self.tick_offset,
        });
    }
    fn scan(&mut self, passes: &'a HashMap<String, Compute>, name: &str, count: u32) {
        self.dispatch(
            &passes[&format!("{name}_blocks")],
            0,
            count.div_ceil(256),
            1,
        );
        self.dispatch(&passes[&format!("{name}_sums")], 0, 1, 1);
        self.dispatch(&passes[&format!("{name}_add")], 0, count.div_ceil(256), 1);
    }
    fn flush(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.commands.is_empty() {
            return;
        }
        let separate = self.separate;
        // Preserve per-dispatch timestamp probes without requiring timestamp
        // writes inside passes on production adapters.
        #[cfg(test)]
        let separate = separate || self.commands.iter().any(|c| c.compute.timing.is_some());
        if separate {
            for c in self.commands.drain(..) {
                let mut pass = encoder.begin_compute_pass(&c.compute.descriptor());
                pass.set_pipeline(&c.compute.pipeline);
                c.compute.bind(&mut pass, c.group, c.tick_offset);
                if let Some(args) = c.arguments {
                    pass.dispatch_workgroups_indirect(args, 0);
                } else {
                    pass.dispatch_workgroups(c.dimensions[0], c.dimensions[1], 1);
                }
            }
        } else {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("simulation dispatch batch"),
                ..Default::default()
            });
            for c in self.commands.drain(..) {
                pass.set_pipeline(&c.compute.pipeline);
                c.compute.bind(&mut pass, c.group, c.tick_offset);
                if let Some(args) = c.arguments {
                    pass.dispatch_workgroups_indirect(args, 0);
                } else {
                    pass.dispatch_workgroups(c.dimensions[0], c.dimensions[1], 1);
                }
            }
        }
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
    pub progress: crate::evolution::Progress,
    pub settings: SimSettings,
    pub seed: u32,
    pub tick: u32,
    /// Retained only for old diagnostics; ecology no longer escalates by age.
    pub environment_start_age: u32,
    /// Environment/action time at this world's tick zero. This is separate
    /// from tick so every world's survival duration is measured from zero.
    pub assisted: bool,
    pub current_buffer: usize,
    pub(crate) genome_buffers: [wgpu::Buffer; GENOME_BANK_COUNT],
    pub(crate) reservoir_genome_buffers: [wgpu::Buffer; GENOME_BANK_COUNT],
    pub(crate) reservoir_traits_buffer: wgpu::Buffer,
    pub(crate) reservoir_rng_buffer: wgpu::Buffer,
    reservoir_claims_buffer: wgpu::Buffer,
    /// Non-inherited local plasticity state.  This is cleared for every child.
    pub(crate) fast_weight_buffers: [wgpu::Buffer; 2],
    pub(crate) trace_buffer: wgpu::Buffer,
    pub(crate) learned_summary_buffer: wgpu::Buffer,
    pub agent_buffers: [wgpu::Buffer; 2],
    pub resource_buffer: wgpu::Buffer,
    pub resource_display_buffer: wgpu::Buffer,
    pub ground_buffer: wgpu::Buffer,
    pub perception_buffer: wgpu::Buffer,
    pub occupancy_buffer: wgpu::Buffer,
    pub params_buffer: wgpu::Buffer,
    pub alive_count_buffer: wgpu::Buffer,
    pub family_observer: Option<crate::family_observer::FamilyObserver>,
    #[cfg(test)]
    pub funnel_observer: Option<funnel_audit::FunnelObserver>,
    #[cfg(test)]
    pub(crate) defer_reservoir_admission: bool,
    #[cfg(test)]
    pub(crate) retain_packet_endowment: bool,
    #[cfg(test)]
    pub(crate) parent_admission: Option<parent_admission::ParentAdmission>,
    #[cfg(test)]
    claims_buffer: wgpu::Buffer,
    #[cfg(test)]
    audit_cell_offsets: wgpu::Buffer,
    #[cfg(test)]
    audit_indices: wgpu::Buffer,
    pub(crate) active_indices: wgpu::Buffer,
    birth_dispatch: wgpu::Buffer,
    inheritance_dispatch: wgpu::Buffer,
    #[cfg(test)]
    reference_inheritance: bool,
    cognitive_dispatch: wgpu::Buffer,
    birth_flags: wgpu::Buffer,
    pub(crate) decision_buffer: wgpu::Buffer,
    fertility_buffer: wgpu::Buffer,
    ecology_buffer: wgpu::Buffer,
    terrain_buffer: wgpu::Buffer,
    terrain_epoch: u32,
    #[cfg(test)]
    separate_compute_passes: bool,
    #[cfg(test)]
    reference_tick_boundaries: bool,
    #[cfg(test)]
    linked_spatial: bool,
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
            MAX_AGENTS as u64 * GENOME_BANK_STRIDE as u64 * 4
                <= u64::from(device.limits().max_storage_buffer_binding_size)
                    .min(device.limits().max_buffer_size),
            "GPU storage limit below one masked-brain genome bank"
        );
        let genome_buffers = std::array::from_fn(|bank| {
            buffer(
                device,
                if bank == 0 {
                    "inherited genomes bank 0"
                } else {
                    "inherited genomes bank 1"
                },
                MAX_AGENTS as u64 * GENOME_BANK_STRIDE as u64 * 4,
            )
        });
        let reservoir_genome_buffers = std::array::from_fn(|bank| {
            buffer(
                device,
                if bank == 0 {
                    "rolling hereditary genomes 0"
                } else {
                    "rolling hereditary genomes 1"
                },
                HEREDITARY_RESERVOIR_SIZE as u64 * GENOME_BANK_STRIDE as u64 * 4,
            )
        });
        let reservoir_traits_buffer = buffer(
            device,
            "rolling hereditary traits",
            HEREDITARY_RESERVOIR_SIZE as u64 * std::mem::size_of::<CognitiveTraits>() as u64,
        );
        let reservoir_rng_buffer = buffer(device, "rolling hereditary rng", 4);
        let reservoir_claims_buffer = buffer(
            device,
            "reservoir replacement claims and birth count",
            (HEREDITARY_RESERVOIR_SIZE as u64 + 1) * 4,
        );
        let fast_weight_buffers = std::array::from_fn(|_| {
            buffer(
                device,
                "lifetime learned connection bank",
                MAX_AGENTS as u64 * FAST_BANK_STRIDE as u64 * 4,
            )
        });
        let trace_buffer = buffer(
            device,
            "local activity traces",
            MAX_AGENTS as u64 * TRACE_COUNT as u64 * 4,
        );
        let learned_summary_buffer =
            buffer(device, "learned magnitude summaries", MAX_AGENTS as u64 * 4);
        let agent_buffers = [
            buffer(device, "bodies A", agent_size),
            buffer(device, "bodies B", agent_size),
        ];
        let resource_buffer = buffer(device, "food", 512 * 512 * 4);
        let resource_display_buffer = buffer(device, "food display", 512 * 512 * 4);
        let ground_buffer = buffer(device, "ground", 512 * 512 * 32);
        let fertility_buffer = buffer(device, "soil", 512 * 512 * 4);
        let ecology_buffer = buffer(device, "water nutrient detritus", 512 * 512 * 16);
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
        let cognitive_dispatch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cooperative cognitive work count"),
            size: 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let birth_dispatch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("eligible birth work count"),
            size: 12,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let inheritance_dispatch = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cooperative inheritance work count"),
            size: 12,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::INDIRECT,
            mapped_at_creation: false,
        });
        let parents = buffer(device, "parents", MAX_AGENTS as u64 * 4);
        let claims = buffer(device, "interaction claims", MAX_AGENTS as u64 * 12);
        let death_stats_buffer = buffer(device, "counters", DEATH_STATS_COUNT as u64 * 4);
        let event_buffer = buffer(
            device,
            "event ring",
            EVENT_RING_SIZE as u64 * std::mem::size_of::<observability::InteractionEvent>() as u64,
        );
        let summary_buffer = buffer(device, "summaries", observability::METRICS_SUMMARY_SIZE);
        let params_buffer = uniform(
            device,
            "parameters",
            std::mem::size_of::<SimParams>() as u64,
        );
        let tick_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("aligned tick parameters"),
            size: 1024 * parameter_stride(device),
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
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
            "wuw wrw".replace(' ', "").as_str(),
            vec![vec![
                &resource_buffer,
                &params_buffer,
                &fertility_buffer,
                &ground_buffer,
                &terrain_buffer,
                &ecology_buffer
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
        for entry in ["clear", "link"] {
            add!(
                &format!("linked_{entry}"),
                "../shaders/linked_spatial.wgsl",
                entry,
                "rwwwu",
                pair(|s| vec![
                    &agent_buffers[s],
                    &occupancy_buffer,
                    &cell_offsets,
                    &indices,
                    &params_buffer
                ])
            );
        }
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
            "rrwurrrr",
            pair(|s| vec![
                &agent_buffers[s],
                &perception_buffer,
                &decision_buffer,
                &params_buffer,
                &genome_buffers[0],
                &genome_buffers[1],
                &fast_weight_buffers[0],
                &fast_weight_buffers[1],
            ])
        );
        passes.insert(
            "decide_live".into(),
            Compute::new(
                device,
                "decide_live",
                include_str!("../shaders/decide_parallel.wgsl"),
                "main",
                "rrwurrrrr",
                pair(|s| {
                    vec![
                        &agent_buffers[s],
                        &perception_buffer,
                        &decision_buffer,
                        &params_buffer,
                        &genome_buffers[0],
                        &genome_buffers[1],
                        &fast_weight_buffers[0],
                        &fast_weight_buffers[1],
                        &active_indices,
                    ]
                }),
            ),
        );
        add!(
            "observe_signals",
            "../shaders/observe_signals.wgsl",
            "main",
            "rrruww",
            pair(|s| vec![
                &agent_buffers[s],
                &perception_buffer,
                &decision_buffer,
                &params_buffer,
                &event_buffer,
                &death_stats_buffer
            ])
        );
        add!(
            "observe_memory",
            "../shaders/observe_memory.wgsl",
            "main",
            "rrrrrrwwu",
            pair(|s| vec![
                &agent_buffers[s],
                &decision_buffer,
                &genome_buffers[0],
                &genome_buffers[1],
                &fast_weight_buffers[0],
                &fast_weight_buffers[1],
                &event_buffer,
                &death_stats_buffer,
                &params_buffer
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
        add!(
            "consume",
            "../shaders/consume.wgsl",
            "main",
            "rrwwuwwrr",
            pair(|s| vec![
                &agent_buffers[s],
                &decision_buffer,
                &resource_buffer,
                &request_buffer,
                &params_buffer,
                &death_stats_buffer,
                &ground_buffer,
                &cell_offsets,
                &indices
            ])
        );
        #[cfg(test)]
        add!(
            "body",
            "../shaders/update_agents.wgsl",
            "main",
            "rrrw uwwrw".replace(' ', "").as_str(),
            pair(|s| vec![
                &agent_buffers[s],
                &decision_buffer,
                &request_buffer,
                &agent_buffers[1 - s],
                &params_buffer,
                &birth_flags,
                &death_stats_buffer,
                &active_indices,
                &event_buffer
            ])
        );
        passes.insert(
            "body_live".into(),
            Compute::new(
                device,
                "body_live",
                &live_source(include_str!("../shaders/update_agents.wgsl"), 7),
                "main",
                "rrrw uwwrw".replace(' ', "").as_str(),
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
                        &event_buffer,
                    ]
                }),
            ),
        );
        for entry in ["clear", "propose", "resolve", "production"] {
            let name = format!("interact_{entry}");
            add!(
                &name,
                "../shaders/interactions.wgsl",
                entry,
                "wrwuw wwrrw".replace(' ', "").as_str(),
                pair(|s| vec![
                    &agent_buffers[s],
                    &decision_buffer,
                    &claims,
                    &params_buffer,
                    &death_stats_buffer,
                    &ground_buffer,
                    &event_buffer,
                    &cell_offsets,
                    &indices,
                    &birth_flags,
                ])
            );
        }
        add!(
            "release",
            "../shaders/release_food.wgsl",
            "main",
            "wwu",
            pair(|s| vec![&agent_buffers[s], &ground_buffer, &params_buffer])
        );
        add!(
            "free",
            "../shaders/free_flags.wgsl",
            "main",
            "rwuw",
            pair(|s| vec![
                &agent_buffers[s],
                &free_flags,
                &params_buffer,
                &reservoir_claims_buffer
            ])
        );
        add!(
            "free_compact",
            "../shaders/compact_slots.wgsl",
            "main",
            "rrwwwww",
            vec![vec![
                &free_flags,
                &free_prefix,
                &free_indices,
                &active_indices,
                &perception_buffer,
                &decision_buffer,
                &cognitive_dispatch
            ]]
        );
        add!(
            "birth_compact",
            "../shaders/compact_agent_indices.wgsl",
            "main",
            "rrwrwww",
            vec![vec![
                &birth_flags,
                &birth_prefix,
                &parents,
                &free_prefix,
                &birth_dispatch,
                &death_stats_buffer,
                &inheritance_dispatch
            ]]
        );
        add!(
            "birth",
            "../shaders/apply_births.wgsl",
            "main",
            "wrrrruwrr",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix,
                &parents,
                &birth_prefix,
                &params_buffer,
                &death_stats_buffer,
                &decision_buffer,
                &claims
            ])
        );
        add!(
            "inherit_genomes",
            "../shaders/inherit_genomes.wgsl",
            "parallel",
            "wrrrruwwwrr",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix,
                &parents,
                &birth_prefix,
                &params_buffer,
                &genome_buffers[0],
                &genome_buffers[1],
                &death_stats_buffer,
                &claims,
                &agent_buffers[1 - s],
            ])
        );
        add!(
            "fusion",
            "../shaders/apply_births.wgsl",
            "fusion",
            "wrrrruwrr",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix,
                &parents,
                &birth_prefix,
                &params_buffer,
                &death_stats_buffer,
                &decision_buffer,
                &claims
            ])
        );
        #[cfg(test)]
        {
            let capped = "min(energy-params.sensor_and_padding.w,reserve_capacity(0.0,params.sensor_and_padding.y))";
            let original = include_str!("../shaders/apply_births.wgsl");
            let paid = "child.energy=energy-params.sensor_and_padding.w;";
            assert_eq!(original.matches(paid).count(), 1);
            let legacy = original.replace(paid, &format!("child.energy={capped};"));
            passes.insert(
                "fusion_legacy_clipped".to_string(),
                Compute::new(
                    device,
                    "legacy clipped packet endowment",
                    &legacy,
                    "fusion",
                    "wrrrruwrr",
                    pair(|i| {
                        vec![
                            &agent_buffers[i],
                            &free_indices,
                            &free_prefix,
                            &parents,
                            &birth_prefix,
                            &params_buffer,
                            &death_stats_buffer,
                            &decision_buffer,
                            &claims,
                        ]
                    }),
                ),
            );
        }
        add!(
            "inherit_fusion",
            "../shaders/inherit_genomes.wgsl",
            "fusion",
            "wrrrruwwwrr",
            pair(|s| vec![
                &agent_buffers[s],
                &free_indices,
                &free_prefix,
                &parents,
                &birth_prefix,
                &params_buffer,
                &genome_buffers[0],
                &genome_buffers[1],
                &death_stats_buffer,
                &claims,
                &agent_buffers[1 - s]
            ])
        );
        for (name, entry) in [
            ("claim_reservoir", "claim"),
            ("update_reservoir", "main"),
            ("advance_reservoir", "advance"),
        ] {
            add!(
                name,
                "../shaders/update_reservoir.wgsl",
                entry,
                "rrrrrurrwwwww",
                pair(|s| vec![
                    &agent_buffers[s],
                    &free_indices,
                    &free_prefix,
                    &parents,
                    &birth_prefix,
                    &params_buffer,
                    &genome_buffers[0],
                    &genome_buffers[1],
                    &reservoir_genome_buffers[0],
                    &reservoir_genome_buffers[1],
                    &reservoir_traits_buffer,
                    &reservoir_rng_buffer,
                    &reservoir_claims_buffer,
                ])
            );
        }
        add!(
            "plastic",
            "../shaders/plasticity.wgsl",
            "main",
            "rwrwwwuwr",
            pair(|s| vec![
                &agent_buffers[s],
                &agent_buffers[1 - s],
                &decision_buffer,
                &fast_weight_buffers[0],
                &fast_weight_buffers[1],
                &trace_buffer,
                &params_buffer,
                &death_stats_buffer,
                &active_indices,
            ])
        );
        add!(
            "summarize_learning",
            "../shaders/summarize_learning.wgsl",
            "main",
            "rrrw",
            pair(|s| vec![
                &agent_buffers[s],
                &fast_weight_buffers[0],
                &fast_weight_buffers[1],
                &learned_summary_buffer
            ])
        );
        add!(
            "reset_cognitive_birth_state",
            "../shaders/reset_cognitive_birth_state.wgsl",
            "main",
            "rwwwu",
            pair(|s| vec![
                &agent_buffers[s],
                &fast_weight_buffers[0],
                &fast_weight_buffers[1],
                &trace_buffer,
                &params_buffer,
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
            "ruwu",
            pair(|s| vec![
                &agent_buffers[s],
                &selection_params_buffer,
                &selection_key_buffer,
                &params_buffer,
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
            "wuwu",
            vec![vec![
                &resource_buffer,
                &intervention_params_buffer,
                &ground_buffer,
                &params_buffer,
            ]]
        );
        add!(
            "paint",
            "../shaders/intervene.wgsl",
            "paint",
            "wuwu",
            vec![vec![
                &resource_buffer,
                &intervention_params_buffer,
                &ground_buffer,
                &params_buffer,
            ]]
        );
        add!(
            "kill",
            "../shaders/kill.wgsl",
            "main",
            "wuu",
            pair(|s| vec![
                &agent_buffers[s],
                &intervention_params_buffer,
                &params_buffer
            ])
        );
        let mut sim = Self {
            device: device.clone(),
            progress: crate::evolution::Progress::initial(seed),
            settings: SimSettings::default(),
            seed,
            tick: 0,
            environment_start_age: 0,
            assisted: false,
            current_buffer: 0,
            genome_buffers,
            reservoir_genome_buffers,
            reservoir_traits_buffer,
            reservoir_rng_buffer,
            reservoir_claims_buffer,
            fast_weight_buffers,
            trace_buffer,
            learned_summary_buffer,
            agent_buffers,
            resource_buffer,
            resource_display_buffer,
            ground_buffer,
            perception_buffer,
            occupancy_buffer,
            params_buffer,
            alive_count_buffer,
            family_observer: None,
            #[cfg(test)]
            funnel_observer: None,
            #[cfg(test)]
            defer_reservoir_admission: false,
            #[cfg(test)]
            retain_packet_endowment: true,
            #[cfg(test)]
            parent_admission: None,
            #[cfg(test)]
            claims_buffer: claims,
            #[cfg(test)]
            audit_cell_offsets: cell_offsets,
            #[cfg(test)]
            audit_indices: indices,
            decision_buffer,
            active_indices,
            birth_dispatch,
            inheritance_dispatch,
            #[cfg(test)]
            reference_inheritance: false,
            cognitive_dispatch,
            birth_flags,
            fertility_buffer,
            ecology_buffer,
            terrain_buffer,
            terrain_epoch: 0,
            #[cfg(test)]
            separate_compute_passes: false,
            #[cfg(test)]
            reference_tick_boundaries: false,
            #[cfg(test)]
            linked_spatial: true,
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
        self.reset_with_population_at(queue, None, None, 0);
        // Seed the pool by uniform founder sampling with replacement.
        // The pool holds inherited state only.
        let mut pool_settings = self.settings.clone();
        pool_settings.population = pool_settings.population.max(1);
        let genes = build_genomes(self.seed, &pool_settings);
        let traits = build_traits(self.seed, &pool_settings);
        let mut rng = self.seed ^ 0x85eb_ca6b;
        let mut pool = vec![0.0; HEREDITARY_RESERVOIR_SIZE as usize * GENOME_SIZE];
        let mut pool_traits = Vec::with_capacity(HEREDITARY_RESERVOIR_SIZE as usize);
        for slot in 0..HEREDITARY_RESERVOIR_SIZE as usize {
            let source =
                ((random01(&mut rng) * traits.len() as f32) as usize).min(traits.len() - 1);
            pool[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
                .copy_from_slice(&genes[source * GENOME_SIZE..(source + 1) * GENOME_SIZE]);
            let mut record = traits[source];
            record.padding = [0; 2];
            pool_traits.push(record);
        }
        self.write_banked(queue, &self.reservoir_genome_buffers, &pool);
        queue.write_buffer(
            &self.reservoir_traits_buffer,
            0,
            bytemuck::cast_slice(&pool_traits),
        );
        queue.write_buffer(
            &self.reservoir_rng_buffer,
            0,
            bytemuck::bytes_of(&(self.seed ^ 0x9e37_79b9)),
        );
    }
    #[cfg(test)]
    pub(crate) fn reset_with_genomes_at(
        &mut self,
        queue: &wgpu::Queue,
        founders: Option<&[f32]>,
        environment_start_age: u32,
    ) {
        self.reset_with_population_at(queue, founders, None, environment_start_age);
    }
    pub(crate) fn reset_with_population_at(
        &mut self,
        queue: &wgpu::Queue,
        founders: Option<&[f32]>,
        founder_traits: Option<&[CognitiveTraits]>,
        environment_start_age: u32,
    ) {
        // Clear unused storage on the GPU; upload only the founding population.
        let mut clear = self.device.create_command_encoder(&Default::default());
        for buffer in self.genome_buffers.iter().chain([
            &self.fast_weight_buffers[0],
            &self.fast_weight_buffers[1],
            &self.trace_buffer,
            &self.event_buffer,
            &self.death_stats_buffer,
            &self.perception_buffer,
            &self.decision_buffer,
        ]) {
            clear.clear_buffer(buffer, 0, None);
        }
        queue.submit(Some(clear.finish()));
        self.family_observer = None;
        #[cfg(test)]
        {
            self.funnel_observer = None;
        }
        self.assisted = !self.settings.founder_genomes.is_empty();
        if self.assisted {
            queue.write_buffer(&self.death_stats_buffer, 30 * 4, bytemuck::bytes_of(&1u32));
        }
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
            self.write_genomes(queue, genes);
        }
        let traits = founder_traits.map_or_else(
            || build_traits(self.seed, &self.settings),
            ToOwned::to_owned,
        );
        let data = build_agents_with_traits(self.seed, &self.settings, &traits);
        for b in &self.agent_buffers {
            queue.write_buffer(b, 0, bytemuck::cast_slice(&data));
        }
        self.environment_start_age = environment_start_age;
        let environment_epoch = environment_start_age / TERRAIN_EPOCH_TICKS;
        let terrain_a =
            build_habitat_at(self.seed, environment_epoch, self.settings.habitat_contrast);
        let terrain_b = build_habitat_at(
            self.seed,
            environment_epoch + 1,
            self.settings.habitat_contrast,
        );
        let terrain_phase =
            (environment_start_age % TERRAIN_EPOCH_TICKS) as f32 / TERRAIN_EPOCH_TICKS as f32;
        let terrain_blend = terrain_phase * terrain_phase * (3.0 - 2.0 * terrain_phase);
        let habitat: Vec<_> = terrain_a
            .iter()
            .zip(&terrain_b)
            .map(|(a, b)| a + (b - a) * terrain_blend)
            .collect();
        // Initial physical stocks describe an established landscape, not an
        // age-dependent subsidy. Subsequent climate acts through stored water.
        let ecology = vec![[0.7f32, 3.0, 0.3, 0.0]; (RESOURCE_GRID * RESOURCE_GRID) as usize];
        queue.write_buffer(&self.ecology_buffer, 0, bytemuck::cast_slice(&ecology));
        let food = crate::environment::rotate_grid(
            build_resources_with_coverage(&habitat, 0.45),
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
                habitat
                    .iter()
                    .map(|h| (0.4 + h * 0.35).clamp(0.0, 1.0))
                    .collect::<Vec<_>>(),
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
    pub(crate) fn reset_live_world(
        &mut self,
        queue: &wgpu::Queue,
        genomes: &[f32],
        traits: &[CognitiveTraits],
    ) {
        let progress = self.progress.clone();
        let assisted = self.assisted || progress.history.iter().any(|o| o.assisted);
        self.reset_with_population_at(queue, Some(genomes), Some(traits), 0);
        self.progress = progress;
        self.assisted |= assisted;
        if self.assisted {
            queue.write_buffer(&self.death_stats_buffer, 30 * 4, bytemuck::bytes_of(&1u32));
        }
    }
    pub fn update_params(&self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.params_buffer,
            0,
            bytemuck::bytes_of(&params_for(
                self.tick,
                configured_ecology_time(self.tick, &self.settings)
                    .saturating_add(self.environment_start_age),
                &self.settings,
                self.seed,
            )),
        );
    }
    /// Upload a packed CPU population into the controller's fixed-size GPU
    /// banks.  The split is a storage detail; every caller still deals in one
    /// complete inherited genome per organism.
    pub(crate) fn write_genomes(&self, queue: &wgpu::Queue, genomes: &[f32]) {
        self.write_banked(queue, &self.genome_buffers, genomes);
    }
    fn write_banked(
        &self,
        queue: &wgpu::Queue,
        banks: &[wgpu::Buffer; GENOME_BANK_COUNT],
        genomes: &[f32],
    ) {
        assert_eq!(genomes.len() % GENOME_SIZE, 0);
        let rows = genomes.len() / GENOME_SIZE;
        for (bank, buffer) in banks.iter().enumerate() {
            let start = bank * GENOME_BANK_STRIDE;
            let width = (GENOME_SIZE - start).min(GENOME_BANK_STRIDE);
            let mut packed = vec![0.0f32; rows * GENOME_BANK_STRIDE];
            for row in 0..rows {
                packed[row * GENOME_BANK_STRIDE..row * GENOME_BANK_STRIDE + width].copy_from_slice(
                    &genomes[row * GENOME_SIZE + start..row * GENOME_SIZE + start + width],
                );
            }
            queue.write_buffer(buffer, 0, bytemuck::cast_slice(&packed));
        }
    }
    #[cfg(test)]
    pub(crate) fn write_genome_slot(&self, queue: &wgpu::Queue, slot: usize, genome: &[f32]) {
        assert_eq!(genome.len(), GENOME_SIZE);
        for bank in 0..GENOME_BANK_COUNT {
            let start = bank * GENOME_BANK_STRIDE;
            let width = (GENOME_SIZE - start).min(GENOME_BANK_STRIDE);
            queue.write_buffer(
                &self.genome_buffers[bank],
                (slot * GENOME_BANK_STRIDE * std::mem::size_of::<f32>()) as u64,
                bytemuck::cast_slice(&genome[start..start + width]),
            );
        }
    }
    pub(crate) fn read_genomes(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        rows: usize,
    ) -> Result<Vec<f32>, String> {
        self.read_genome_slots(device, queue, &(0..rows).collect::<Vec<_>>())
    }
    pub(crate) fn read_genome_slots(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slots: &[usize],
    ) -> Result<Vec<f32>, String> {
        if slots.is_empty() {
            return Ok(Vec::new());
        }
        if slots.iter().any(|&slot| slot >= MAX_AGENTS as usize) {
            return Err("Genome slot out of range".into());
        }
        let packed = buffer(
            device,
            "sampled inherited genomes",
            (slots.len() * GENOME_SIZE * 4) as u64,
        );
        let mut encoder = device.create_command_encoder(&Default::default());
        for (row, &slot) in slots.iter().enumerate() {
            for bank in 0..GENOME_BANK_COUNT {
                let start = bank * GENOME_BANK_STRIDE;
                let width = (GENOME_SIZE - start).min(GENOME_BANK_STRIDE);
                encoder.copy_buffer_to_buffer(
                    &self.genome_buffers[bank],
                    (slot * GENOME_BANK_STRIDE * 4) as u64,
                    &packed,
                    ((row * GENOME_SIZE + start) * 4) as u64,
                    (width * 4) as u64,
                );
            }
        }
        queue.submit(Some(encoder.finish()));
        observability::read_buffer(device, queue, &packed)
            .map(|bytes| bytemuck::cast_slice(&bytes).to_vec())
    }
    fn dispatch(&self, e: &mut wgpu::CommandEncoder, name: &str, group: usize, x: u32, y: u32) {
        self.passes[name].dispatch(e, group, x, y);
    }
    #[cfg(test)]
    fn scan(&self, e: &mut wgpu::CommandEncoder, name: &str, count: u32) {
        let mut batch = ComputeBatch::default();
        batch.scan(&self.passes, name, count);
        batch.flush(e);
    }

    pub fn encode_ticks(
        &mut self,
        e: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ticks: u32,
    ) {
        assert!(ticks <= 1024);
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
                    configured_ecology_time(tick, &self.settings)
                        .saturating_add(self.environment_start_age),
                    &self.settings,
                    self.seed,
                )
            })
            .collect();
        let stride = parameter_stride(device);
        let mut upload = vec![0u8; ticks as usize * stride as usize];
        for (slot, params) in upload.chunks_exact_mut(stride as usize).zip(&ps) {
            slot[..std::mem::size_of::<SimParams>()].copy_from_slice(bytemuck::bytes_of(params));
        }
        queue.write_buffer(&self.tick_params_buffer, 0, &upload);
        for compute in self.passes.values_mut() {
            compute.prepare_tick_groups(device, &self.tick_params_buffer);
        }
        #[cfg(test)]
        let reference = self.reference_tick_boundaries;
        #[cfg(not(test))]
        let reference = false;
        #[cfg(test)]
        let linked = self.linked_spatial;
        #[cfg(not(test))]
        let linked = true;
        // Observer pipelines use the public current-parameter uniform. Publish
        // their epoch explicitly; normal playback only publishes once per batch.
        let observing = self.family_observer.is_some();
        #[cfg(test)]
        let observing =
            observing || self.funnel_observer.is_some() || self.parent_admission.is_some();
        let groups = MAX_AGENTS.div_ceil(64);
        let mut batch = ComputeBatch::default();
        #[cfg(test)]
        {
            batch.separate = self.separate_compute_passes;
        }
        for offset in 0..ticks {
            let environment_tick = configured_ecology_time(self.tick, &self.settings)
                .saturating_add(self.environment_start_age);
            let epoch = environment_tick / TERRAIN_EPOCH_TICKS;
            if self.terrain_epoch != epoch && self.settings.evolving_landscape {
                let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("terrain update"),
                    contents: bytemuck::cast_slice(&crate::environment::rotate_grid(
                        build_terrain_pair(self.seed, epoch, self.settings.habitat_contrast),
                        RESOURCE_GRID as usize,
                        self.settings.environment_rotation,
                    )),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
                batch.flush(e);
                e.copy_buffer_to_buffer(&staging, 0, &self.terrain_buffer, 0, staging.size());
                self.terrain_epoch = epoch;
            }
            if reference || observing {
                batch.flush(e);
                e.copy_buffer_to_buffer(
                    &self.tick_params_buffer,
                    offset as u64 * stride,
                    &self.params_buffer,
                    0,
                    std::mem::size_of::<SimParams>() as u64,
                );
            }
            batch.tick_offset = (!reference).then_some((offset as u64 * stride) as u32);
            let s = self.current_buffer;
            let d = 1 - s;
            // Only slots already dead at tick start may be reused: disjoint parent/child writes.
            batch.dispatch(&self.passes["free"], s, groups, 1);
            batch.scan(&self.passes, "free", MAX_AGENTS);
            batch.dispatch(&self.passes["free_compact"], 0, groups, 1);
            batch.dispatch(&self.passes["resource"], 0, 64, 64);
            if linked {
                batch.dispatch(&self.passes["linked_clear"], s, 1024, 1);
                batch.dispatch(&self.passes["linked_link"], s, groups, 1);
            } else {
                batch.dispatch(&self.passes["clear"], 0, 32, 32);
                batch.dispatch(&self.passes["count"], s, groups, 1);
                batch.scan(&self.passes, "spatial", SPATIAL_CELL_COUNT);
                batch.dispatch(&self.passes["cursors"], 0, 1024, 1);
                batch.dispatch(&self.passes["scatter"], s, groups, 1);
            }
            batch.indirect(&self.passes["perceive_live"], s, &self.active_indices);
            batch.indirect(&self.passes["decide_live"], s, &self.cognitive_dispatch);
            batch.dispatch(&self.passes["observe_signals"], s, groups, 1);
            batch.dispatch(&self.passes["observe_memory"], s, groups, 1);
            batch.dispatch(
                &self.passes["consume"],
                s,
                RESOURCE_GRID * RESOURCE_GRID / 64,
                1,
            );
            // Preserve dead records (including slot generations) with a bulk GPU
            // copy. Only living bodies need the expensive structured update.
            batch.flush(e);
            e.copy_buffer_to_buffer(
                &self.agent_buffers[s],
                0,
                &self.agent_buffers[d],
                0,
                self.agent_buffers[s].size(),
            );
            batch.flush(e);
            e.clear_buffer(&self.birth_flags, 0, None);
            batch.indirect(&self.passes["body_live"], s, &self.active_indices);
            // Decisions act with the previous lifetime state.  Only after the
            // body update do local traces and fast weights change, and their
            // exact write cost is debited before interaction or birth.
            batch.indirect(&self.passes["plastic"], s, &self.cognitive_dispatch);
            // Contact is resolved after voluntary integration, so rebuild the
            // spatial index from the actual post-movement bodies.
            if linked {
                batch.dispatch(&self.passes["linked_clear"], d, 1024, 1);
                batch.dispatch(&self.passes["linked_link"], d, groups, 1);
            } else {
                batch.dispatch(&self.passes["clear"], 0, 32, 32);
                batch.dispatch(&self.passes["count"], d, groups, 1);
                batch.scan(&self.passes, "spatial", SPATIAL_CELL_COUNT);
                batch.dispatch(&self.passes["cursors"], 0, 1024, 1);
                batch.dispatch(&self.passes["scatter"], d, groups, 1);
            }
            #[cfg(test)]
            if let Some(observer) = &self.funnel_observer {
                batch.flush(e);
                observer.before_contacts(e, d);
            }
            for n in [
                "interact_clear",
                "interact_propose",
                "interact_resolve",
                "interact_production",
            ] {
                batch.dispatch(&self.passes[n], d, groups, 1);
            }
            batch.scan(&self.passes, "birth", MAX_AGENTS);
            batch.dispatch(&self.passes["birth_compact"], 0, groups, 1);
            batch.indirect(&self.passes["birth"], d, &self.birth_dispatch);
            #[cfg(test)]
            let inheritance_args = if self.reference_inheritance {
                &self.birth_dispatch
            } else {
                &self.inheritance_dispatch
            };
            #[cfg(not(test))]
            let inheritance_args = &self.inheritance_dispatch;
            batch.indirect(&self.passes["inherit_genomes"], d, inheritance_args);
            #[cfg(test)]
            if let Some(observer) = &self.funnel_observer {
                batch.flush(e);
                observer.before_fusion(e, d);
            }
            #[cfg(test)]
            if let Some(admission) = &self.parent_admission {
                batch.flush(e);
                admission.before_fusion(e, d);
            }
            #[cfg(test)]
            let fusion_pass = if self.retain_packet_endowment {
                "fusion"
            } else {
                "fusion_legacy_clipped"
            };
            #[cfg(not(test))]
            let fusion_pass = "fusion";
            batch.dispatch(&self.passes[fusion_pass], d, groups, 1);
            batch.dispatch(&self.passes["inherit_fusion"], d, groups, 1);
            #[cfg(test)]
            let admit = !self.defer_reservoir_admission && self.parent_admission.is_none();
            #[cfg(not(test))]
            let admit = true;
            if admit {
                if reference {
                    batch.flush(e);
                    e.clear_buffer(&self.reservoir_claims_buffer, 0, None);
                }
                batch.dispatch(&self.passes["claim_reservoir"], d, groups, 1);
                // The destination is the hereditary pool, not the body-slot domain.
                batch.dispatch(
                    &self.passes["update_reservoir"],
                    d,
                    HEREDITARY_RESERVOIR_SIZE.div_ceil(64),
                    1,
                );
                batch.dispatch(&self.passes["advance_reservoir"], d, 1, 1);
            }
            #[cfg(test)]
            if let Some(admission) = &self.parent_admission {
                batch.flush(e);
                admission.after_fusion(e, d, &self.reservoir_claims_buffer);
            }
            batch.dispatch(&self.passes["reset_cognitive_birth_state"], d, groups, 1);
            batch.dispatch(&self.passes["release"], d, groups, 1);
            if let Some(observer) = &self.family_observer {
                batch.flush(e);
                observer.encode(e, d);
            }
            #[cfg(test)]
            if let Some(observer) = &self.funnel_observer {
                batch.flush(e);
                observer.after_tick(e, d);
            }
            self.current_buffer = d;
            self.tick += 1;
        }
        batch.flush(e);
        e.copy_buffer_to_buffer(
            &self.tick_params_buffer,
            (ticks - 1) as u64 * stride,
            &self.params_buffer,
            0,
            std::mem::size_of::<SimParams>() as u64,
        );
        batch.tick_offset = None;
        e.copy_buffer_to_buffer(
            &self.resource_buffer,
            0,
            &self.resource_display_buffer,
            0,
            self.resource_buffer.size(),
        );
        batch.flush(e);
        e.clear_buffer(&self.alive_count_buffer, 0, None);
        batch.dispatch(&self.passes["alive"], self.current_buffer, groups, 1);
        batch.flush(e);
    }
    pub fn select_agent(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        world_position: [f32; 2],
        radius: f32,
    ) -> Option<SelectionOutput> {
        self.update_params(queue);
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
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
        delta: f32,
    ) {
        self.assisted = true;
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
    pub fn paint_food(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        center: [f32; 2],
        radius: f32,
        delta: f32,
    ) {
        self.assisted = true;
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
        self.dispatch(&mut e, "paint", 0, 64, 64);
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
fn configured_ecology_time(tick: u32, _s: &SimSettings) -> u32 {
    tick
}

fn params_for(tick: u32, environment_tick: u32, s: &SimSettings, seed: u32) -> SimParams {
    let climate = crate::climate::at(environment_tick, seed, s.flourishing_start);
    let regional_epoch = environment_tick / 47_003;
    let local_epoch = environment_tick / 997;
    SimParams {
        world_size: [
            s.habitat_width,
            s.habitat_height,
            juvenile_gathering_floor(tick),
            climate.temperature,
        ],
        resource_grid_size: RESOURCE_GRID,
        agent_count: MAX_AGENTS,
        tick,
        world_padding: u32::from(s.fractional_gathering),
        time_and_costs: [
            climate.rainfall,
            s.resource_regeneration,
            s.movement_energy_cost,
            s.metabolic_cost,
        ],
        resource_and_noise: [
            s.consume_amount,
            s.conversion_efficiency,
            s.heterogeneity,
            f32::from(s.social_actions_enabled),
        ],
        sensor_and_padding: [
            s.sensor_radius,
            s.maturity_age,
            packet_upkeep(tick),
            s.fusion_loss,
        ],
        physical: [
            f32::from(s.force_enabled),
            f32::from(s.communication_enabled),
            s.motor_response_gain,
            packet_fusion_radius(tick),
        ],
        lifecycle: [
            seed,
            regional_epoch,
            s.environment_rotation,
            environment_tick,
        ],
        mutation: [
            BASE_MUTATION_PROBABILITY,
            BASE_MUTATION_MAGNITUDE,
            f32::from(s.evolving_landscape),
            s.active_unit_upkeep,
        ],
        environment: [
            f32::from_bits(environment_tick % 47_003),
            f32::from_bits(local_epoch),
            f32::from_bits(environment_tick % 997),
            s.memory_write_energy,
        ],
    }
}
#[cfg(test)]
fn build_agents(seed: u32, s: &SimSettings) -> Vec<AgentGpu> {
    let traits = build_traits(seed, s);
    build_agents_with_traits(seed, s, &traits)
}
fn build_agents_with_traits(
    seed: u32,
    s: &SimSettings,
    traits: &[CognitiveTraits],
) -> Vec<AgentGpu> {
    build_agent_records(seed, s, traits, MAX_AGENTS)
}

fn build_agent_records(
    seed: u32,
    s: &SimSettings,
    traits: &[CognitiveTraits],
    count: u32,
) -> Vec<AgentGpu> {
    let mut rng = seed.max(1);
    (0..count)
        .map(|i| {
            let trait_index = (i as usize).min(traits.len().saturating_sub(1));
            let trait_data = traits.get(trait_index).copied().unwrap_or(CognitiveTraits {
                active_mask: 1,
                padding: [0; 2],
                packet_size: 16.0,
                plasticity_rate: [0.0; HIDDEN],
                trace_retention: 0.9,
                learned_weight_retention: 0.99,
                parameter_mutation_rate: 1.0,
                parameter_mutation_step: 1.0,
                topology_mutation_rate: 1.0,
            });
            AgentGpu {
                position: crate::environment::rotate_point_rect(
                    [
                        random01(&mut rng) * s.habitat_width,
                        random01(&mut rng) * s.habitat_height,
                    ],
                    s.habitat_width,
                    s.habitat_height,
                    s.environment_rotation,
                ),
                // The initial population has no provisioning population before it.
                // Seed mature bodies with random controllers; all actual births
                // start at age zero and obey the same ontogenetic physiology.
                energy: 35.0,
                food: 0.0,
                age: s.maturity_age,
                max_speed: 1.2,
                sensor_radius: s.sensor_radius,
                heading: (random01(&mut rng) * std::f32::consts::TAU
                    + s.environment_rotation as f32 * std::f32::consts::FRAC_PI_2)
                    % std::f32::consts::TAU,
                max_age: 9000.0 + random01(&mut rng) * 2000.0,
                rng: rng ^ i,
                alive: u32::from(i < s.population),
                generation: 1,
                lineage_id: i + 1,
                founder_family: if s.founder_genomes.is_empty() {
                    i
                } else {
                    (i as usize % s.founder_genomes.len()) as u32
                },
                packet_size: trait_data.packet_size,
                active_mask: trait_data.active_mask,
                plasticity_rate: trait_data.plasticity_rate,
                trace_retention: trait_data.trace_retention,
                learned_weight_retention: trait_data.learned_weight_retention,
                parameter_mutation_rate: trait_data.parameter_mutation_rate,
                parameter_mutation_step: trait_data.parameter_mutation_step,
                topology_mutation_rate: trait_data.topology_mutation_rate,
                ..Default::default()
            }
        })
        .collect()
}
fn build_traits(seed: u32, s: &SimSettings) -> Vec<CognitiveTraits> {
    let mut rng = seed ^ 0x6d2b_79f5;
    (0..s.population as usize)
        .map(|i| {
            if !s.founder_traits.is_empty() {
                s.founder_traits[i % s.founder_traits.len()]
            } else {
                let active_mask = crate::brain::random_active_mask(&mut rng);
                let (
                    plasticity_rate,
                    trace_retention,
                    learned_weight_retention,
                    parameter_mutation_rate,
                    parameter_mutation_step,
                    topology_mutation_rate,
                ) = crate::brain::random_plasticity(&mut rng);
                CognitiveTraits {
                    active_mask,
                    padding: [0; 2],
                    packet_size: crate::brain::random_packet_size(&mut rng),
                    plasticity_rate,
                    trace_retention,
                    learned_weight_retention,
                    parameter_mutation_rate,
                    parameter_mutation_step,
                    topology_mutation_rate,
                }
            }
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
fn build_terrain_pair(seed: u32, epoch: u32, contrast: f32) -> Vec<[f32; 4]> {
    let a = build_habitat_at(seed, epoch, contrast);
    let b = build_habitat_at(seed, epoch.wrapping_add(1), contrast);
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

/// Only geography changes: retain the legacy keyframe's food/capacity budget.
/// The reference map contributes one scalar, never patch locations or routes.
fn build_habitat_at(seed: u32, epoch: u32, contrast: f32) -> Vec<f32> {
    let reference = build_legacy_habitat_at(seed, epoch, 1.0);
    let target = reference.iter().map(|v| f64::from(*v)).sum::<f64>();
    let mut habitat = Vec::with_capacity(reference.len());
    for y in 0..RESOURCE_GRID {
        for x in 0..RESOURCE_GRID {
            habitat.push(correlated_habitat(
                (x as f32 + 0.5) / RESOURCE_GRID as f32,
                (y as f32 + 0.5) / RESOURCE_GRID as f32,
                seed,
                epoch,
            ));
        }
    }
    let total = habitat.iter().map(|v| f64::from(*v)).sum::<f64>();
    let scale = (target / total.max(f64::MIN_POSITIVE)) as f32;
    let mean = (target / habitat.len() as f64) as f32;
    for value in &mut habitat {
        *value = mean + contrast * (*value * scale - mean);
    }
    habitat
}

/// Periodic value noise with a C2 quintic interpolant, including at the seam.
fn periodic_terrain_noise(x: f32, y: f32, period: i32, seed: u32) -> f32 {
    let x = x.rem_euclid(period as f32);
    let y = y.rem_euclid(period as f32);
    let ix = x.floor() as i32;
    let iy = y.floor() as i32;
    let sample = |dx: i32, dy: i32| {
        let mut h = seed
            ^ ((ix + dx).rem_euclid(period) as u32).wrapping_mul(0x9e3779b9)
            ^ ((iy + dy).rem_euclid(period) as u32).wrapping_mul(0x85ebca6b);
        h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
        h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let fade = |t: f32| t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let sx = fade(x - x.floor());
    let sy = fade(y - y.floor());
    let top = sample(0, 0) * (1.0 - sx) + sample(1, 0) * sx;
    let bottom = sample(0, 1) * (1.0 - sx) + sample(1, 1) * sx;
    top * (1.0 - sy) + bottom * sy
}

fn correlated_habitat(x: f32, y: f32, seed: u32, epoch: u32) -> f32 {
    let x = x.rem_euclid(1.0);
    let y = y.rem_euclid(1.0);
    let key = seed ^ epoch.wrapping_mul(0x9e3779b9);
    // Retain smooth domain warping, now periodic in both world directions.
    let wx = x + (periodic_terrain_noise(x * 7.0, y * 7.0, 7, seed) - 0.5) * 0.045;
    let wy = y + (periodic_terrain_noise(x * 7.0, y * 7.0, 7, seed ^ 7919) - 0.5) * 0.045;
    let noise = |frequency: i32, salt: u32| {
        periodic_terrain_noise(
            wx * frequency as f32,
            wy * frequency as f32,
            frequency,
            key ^ salt,
        )
    };
    let broad = noise(4, 991);
    let medium = noise(11, 1777);
    let detail = noise(23, 3137);
    // Smooth narrow bands around random iso-contours; no endpoints or routes.
    let ridge_signal = 2.0 * noise(8, 0x5f3759df) - 1.0;
    let ridge = (-ridge_signal * ridge_signal * 65.0).exp();
    let field = 0.55 * broad + 0.28 * medium + 0.07 * detail + 0.10 * ridge;
    // A smooth zero shoulder creates actual barren areas without a hard edge.
    let t = ((field - 0.43) / 0.57).clamp(0.0, 1.0);
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

// Frozen pre-correlated generator: economic calibration and diagnostic baseline.
fn build_legacy_habitat_at(seed: u32, epoch: u32, contrast: f32) -> Vec<f32> {
    let mut rng = seed ^ 0xa341_316c;
    let mut patches: Vec<[f32; 7]> = Vec::new();
    // Territory moves at constant strength from the first tick.
    let migration_pressure = 1.0;
    let fragmentation = 1.0;
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
        let radius = base_radius;
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
    // Moving fragmentation samples negative coordinates. Float-to-u32 casts
    // saturate them to zero, and fract() is negative there: together those
    // extrapolated the noise and introduced a seam at every integer row.
    let ix = x.floor() as i32 as u32;
    let iy = y.floor() as i32 as u32;
    let sample = |dx: u32, dy: u32| {
        let mut h = seed
            ^ ix.wrapping_add(dx).wrapping_mul(0x9e3779b9)
            ^ iy.wrapping_add(dy).wrapping_mul(0x85ebca6b);
        h = (h ^ (h >> 16)).wrapping_mul(0x7feb352d);
        h = (h ^ (h >> 15)).wrapping_mul(0x846ca68b);
        (h ^ (h >> 16)) as f32 / u32::MAX as f32
    };
    let tx = x - x.floor();
    let ty = y - y.floor();
    let sx = tx * tx * (3.0 - 2.0 * tx);
    let sy = ty * ty * (3.0 - 2.0 * ty);
    let top = sample(0, 0) * (1.0 - sx) + sample(1, 0) * sx;
    let bottom = sample(0, 1) * (1.0 - sx) + sample(1, 1) * sx;
    top * (1.0 - sy) + bottom * sy
}

#[cfg(test)]
fn build_resources(habitat: &[f32]) -> Vec<u32> {
    build_resources_with_coverage(habitat, 0.0)
}
fn build_resources_with_coverage(habitat: &[f32], coverage: f32) -> Vec<u32> {
    habitat
        .iter()
        .map(|h| ((h + (1.0 - h) * coverage * 0.2) * 0.55 * RESOURCE_SCALE) as u32)
        .collect()
}

// Body development remains age-dependent; these constants never track world age.
pub(crate) fn juvenile_gathering_floor(_tick: u32) -> f32 {
    0.01
}
pub(crate) fn packet_upkeep(_tick: u32) -> f32 {
    0.02
}
pub(crate) fn packet_fusion_radius(_tick: u32) -> f32 {
    2.0
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
    let source = source.replace(
        "// SPATIAL_ITERATION",
        include_str!("../shaders/spatial_iteration.wgsl"),
    );
    let constants = [
        ("LIVE_WORKGROUP_SIZE", LIVE_WORKGROUP_SIZE),
        ("INPUT_COUNT", INPUTS),
        ("HIDDEN_COUNT", HIDDEN),
        ("OUTPUT_COUNT", OUTPUTS),
        ("GENOME_SIZE", GENOME_SIZE),
        ("GENOME_BANK_STRIDE", GENOME_BANK_STRIDE),
        ("CONNECTION_COUNT", CONNECTION_COUNT),
        ("FAST_BANK_STRIDE", FAST_BANK_STRIDE),
        ("TRACE_COUNT", TRACE_COUNT),
        ("NODE_BIAS", NODE_BIAS),
        ("GATE_BIAS", GATE_BIAS),
        ("OUTPUT_BIAS", OUTPUT_BIAS),
        ("INPUT_BASE", INPUT_BASE),
        ("RECURRENT_BASE", RECURRENT_BASE),
        ("GATE_BASE", GATE_BASE),
        ("OUTPUT_BASE", OUTPUT_BASE),
        ("FORCE_OUTPUT", FORCE_OUTPUT),
        ("PLACEMENT_OUTPUT", PLACEMENT_OUTPUT),
    ]
    .map(|(name, value)| format!("const {name}:u32={value}u;"))
    .join("\n");
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

#[cfg(test)]
#[path = "funnel_audit.rs"]
pub(crate) mod funnel_audit;

#[cfg(test)]
#[path = "habitat_tests.rs"]
mod habitat_tests;

#[cfg(test)]
#[path = "parent_admission.rs"]
mod parent_admission;
