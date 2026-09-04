use std::sync::mpsc;

use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;
use crate::neural::{NeuralWeights, HIDDEN as NEURAL_HIDDEN};

pub const MAX_AGENTS: u32 = 100_000;
pub const RESOURCE_GRID: u32 = 512;
pub const OCCUPANCY_GRID: u32 = 256;
pub const SPATIAL_CELL_COUNT: u32 = OCCUPANCY_GRID * OCCUPANCY_GRID;
pub const WORLD_SIZE: f32 = 2048.0;
pub const RESOURCE_SCALE: f32 = 1000.0;
pub const DEATH_STATS_COUNT: u32 = 12;
pub const SOCIAL_SLOTS: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct PlaceGpu {
    pub position: [f32; 2],
    pub food: f32,
    pub observed: u32,
    pub source_id: u32,
    pub source_generation: u32,
    pub confidence: f32,
    pub _padding: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct AgentGpu {
    pub position: [f32; 2],
    pub velocity: [f32; 2],
    pub energy: f32,
    pub age: f32,
    pub max_speed: f32,
    pub sensor_radius: f32,
    pub food: f32,
    pub action: u32,
    pub target: u32,
    pub commit_until: u32,
    pub goal: [f32; 2],
    pub rng: u32,
    pub alive: u32,
    pub generation: u32,
    pub event_actor: u32,
    pub event_generation: u32,
    pub event_tick: u32,
    pub event_amount: f32,
    pub goal_score: f32,
    pub next_birth: u32,
    pub max_age: f32,
    pub last_communication: u32,
    pub guide_id: u32,
    pub guide_generation: u32,
    pub guide_started: u32,
    pub guide_expected: f32,
    pub guide_result: f32,
    pub guide_position: [f32; 2],
    pub places: [PlaceGpu; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct PerceptionGpu {
    pub resource_here: f32,
    pub resource_north: f32,
    pub resource_east: f32,
    pub resource_south: f32,
    pub resource_west: f32,
    pub local_density: f32,
    pub local_count: f32,
    pub projected_food: f32,
    pub competition_pressure: f32,
    pub _padding: u32,
    pub gradient: [f32; 2],
    pub crowd: [f32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SocialPerceptionGpu {
    pub avoidance: [f32; 2],
    pub known_strength: f32,
    pub danger: f32,
    pub give_target: u32,
    pub force_target: u32,
    pub give_value: f32,
    pub force_value: f32,
    pub report_target: u32,
    pub report_place: u32,
    pub companion_position: [f32; 2],
    pub companion_velocity: [f32; 2],
    pub companion_value: f32,
    pub report_value: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SocialRelationGpu {
    pub target_slot: u32,
    pub target_generation: u32,
    pub familiarity: f32,
    pub benefit: f32,
    pub harm: f32,
    pub last_seen_tick: u32,
    pub benefit_evidence: f32,
    pub harm_evidence: f32,
    pub navigation: f32,
    pub navigation_evidence: f32,
    pub last_report_tick: u32,
    pub last_report_observed: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct DecisionGpu {
    pub scores: [f32; 7],
    pub selected_action: u32,
    pub target: u32,
    pub _target_padding: u32,
    pub goal: [f32; 2],
    pub amount: f32,
    pub _padding: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct SimParams {
    pub world_size: f32,
    pub resource_grid_size: u32,
    pub agent_count: u32,
    pub tick: u32,
    pub time_and_costs: [f32; 4],
    pub resource_and_noise: [f32; 4],
    pub sensor_and_padding: [f32; 4],
    pub social_weights: [f32; 4],
    pub lifecycle: [u32; 4],
    pub neural_config: [u32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SelectionParams {
    pub world_position: [f32; 2],
    pub radius: f32,
    pub _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct InterventionParams {
    pub center: [f32; 2],
    pub radius: f32,
    pub delta: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SelectionOutput {
    pub agent: AgentGpu,
    pub perception: PerceptionGpu,
    pub decision: DecisionGpu,
    pub social: SocialPerceptionGpu,
    pub relations: [SocialRelationGpu; 8],
    pub selected: u32,
    pub _padding: [u32; 5],
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SimSettings {
    pub population: u32,
    pub resource_regeneration: f32,
    pub movement_energy_cost: f32,
    pub metabolic_cost: f32,
    pub consume_amount: f32,
    pub conversion_efficiency: f32,
    pub heterogeneity: f32,
    pub exploration_noise: f32,
    pub sensor_radius: f32,
    pub reproduction_threshold: f32,
    pub reproduction_cost: f32,
    pub maturity_age: f32,
    pub birth_cooldown: u32,
    pub social_access: f32,
    pub social_concern: f32,
    pub reciprocity: f32,
    pub force_enabled: bool,
    pub communication_enabled: bool,
    #[serde(default)]
    pub evolving_landscape: bool,
    #[serde(default)]
    pub neural_policy: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        Self {
            population: 1_000,
            resource_regeneration: 0.01,
            movement_energy_cost: 0.01,
            metabolic_cost: 0.06,
            consume_amount: 25.0,
            conversion_efficiency: 8.0,
            heterogeneity: 0.85,
            exploration_noise: 0.10,
            sensor_radius: 24.0,
            reproduction_threshold: 75.0,
            reproduction_cost: 50.0,
            maturity_age: 400.0,
            birth_cooldown: 240,
            social_access: 0.35,
            social_concern: 0.25,
            reciprocity: 0.5,
            force_enabled: true,
            communication_enabled: true,
            evolving_landscape: true,
            neural_policy: false,
        }
    }
}

pub struct Simulation {
    pub settings: SimSettings,
    terrain_buffer: wgpu::Buffer,
    terrain_epoch: u32,
    summary_pipeline: wgpu::ComputePipeline,
    summary_bind_groups: [wgpu::BindGroup; 2],
    summary_buffer: wgpu::Buffer,
    event_buffer: wgpu::Buffer,
    pub seed: u32,
    pub tick: u32,
    pub current_buffer: usize,
    pub agent_buffers: [wgpu::Buffer; 2],
    pub resource_buffer: wgpu::Buffer,
    pub ground_buffer: wgpu::Buffer,
    death_pipeline: wgpu::ComputePipeline,
    death_bind_groups: [wgpu::BindGroup; 2],
    pub resource_display_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    fertility_buffer: wgpu::Buffer,
    pub perception_buffer: wgpu::Buffer,
    _social_perception_buffer: wgpu::Buffer,
    social_memory_buffer: wgpu::Buffer,
    _decision_buffer: wgpu::Buffer,
    neural_weights_buffer: wgpu::Buffer,
    neural_state_buffer: wgpu::Buffer,
    _request_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    birth_flags: wgpu::Buffer,
    #[allow(dead_code)]
    free_flags: wgpu::Buffer,
    #[allow(dead_code)]
    free_prefix: [wgpu::Buffer; 2],
    #[allow(dead_code)]
    free_indices: wgpu::Buffer,
    #[allow(dead_code)]
    birth_prefix: [wgpu::Buffer; 2],
    #[allow(dead_code)]
    birth_parents: wgpu::Buffer,
    pub occupancy_buffer: wgpu::Buffer,
    // These buffers are produced every tick for future local-neighbor queries.
    #[allow(dead_code)]
    pub cell_offsets: [wgpu::Buffer; 2],
    #[allow(dead_code)]
    pub cell_cursors: wgpu::Buffer,
    #[allow(dead_code)]
    pub agent_indices: wgpu::Buffer,
    pub alive_count_buffer: wgpu::Buffer,
    alive_count_readback: wgpu::Buffer,
    death_stats_buffer: wgpu::Buffer,
    death_stats_readback: wgpu::Buffer,
    pub params_buffer: wgpu::Buffer,
    tick_params_buffer: wgpu::Buffer,
    interaction_pipelines: [wgpu::ComputePipeline; 3],
    interaction_bind_groups: [wgpu::BindGroup; 2],
    selection_params_buffer: wgpu::Buffer,
    selection_key_buffer: wgpu::Buffer,
    selection_output_buffer: wgpu::Buffer,
    selection_readback: wgpu::Buffer,
    intervention_params_buffer: wgpu::Buffer,
    resource_update_pipeline: wgpu::ComputePipeline,
    clear_occupancy_pipeline: wgpu::ComputePipeline,
    count_occupancy_pipeline: wgpu::ComputePipeline,
    perception_pipeline: wgpu::ComputePipeline,
    social_pipeline: wgpu::ComputePipeline,
    decision_pipeline: wgpu::ComputePipeline,
    consume_pipeline: wgpu::ComputePipeline,
    update_pipeline: wgpu::ComputePipeline,
    select_pipeline: wgpu::ComputePipeline,
    selection_resolve_pipeline: wgpu::ComputePipeline,
    resource_intervention_pipeline: wgpu::ComputePipeline,
    kill_agents_pipeline: wgpu::ComputePipeline,
    count_alive_pipeline: wgpu::ComputePipeline,
    prefix_init_pipeline: wgpu::ComputePipeline,
    prefix_step_pipelines: Vec<wgpu::ComputePipeline>,
    prepare_scatter_pipeline: wgpu::ComputePipeline,
    scatter_pipeline: wgpu::ComputePipeline,
    free_flags_pipeline: wgpu::ComputePipeline,
    agent_prefix_init_pipeline: wgpu::ComputePipeline,
    agent_prefix_step_pipelines: Vec<wgpu::ComputePipeline>,
    compact_pipeline: wgpu::ComputePipeline,
    birth_pipeline: wgpu::ComputePipeline,
    resource_update_bind_group: wgpu::BindGroup,
    clear_occupancy_bind_group: wgpu::BindGroup,
    count_bind_groups: [wgpu::BindGroup; 2],
    perception_bind_groups: [wgpu::BindGroup; 2],
    social_bind_groups: [wgpu::BindGroup; 2],
    decision_bind_groups: [wgpu::BindGroup; 2],
    consume_bind_groups: [wgpu::BindGroup; 2],
    update_bind_groups: [[wgpu::BindGroup; 2]; 2],
    select_bind_groups: [wgpu::BindGroup; 2],
    selection_resolve_bind_groups: [wgpu::BindGroup; 2],
    resource_intervention_bind_group: wgpu::BindGroup,
    kill_agents_bind_groups: [wgpu::BindGroup; 2],
    count_alive_bind_groups: [wgpu::BindGroup; 2],
    prefix_init_bind_group: wgpu::BindGroup,
    prefix_step_bind_groups: [[wgpu::BindGroup; 2]; 2],
    prepare_scatter_bind_groups: [wgpu::BindGroup; 2],
    scatter_bind_groups: [wgpu::BindGroup; 2],
    free_flags_bind_groups: [wgpu::BindGroup; 2],
    free_prefix_init_bind_group: wgpu::BindGroup,
    birth_prefix_init_bind_group: wgpu::BindGroup,
    free_prefix_step_bind_groups: [[wgpu::BindGroup; 2]; 2],
    birth_prefix_step_bind_groups: [[wgpu::BindGroup; 2]; 2],
    free_compact_bind_groups: [wgpu::BindGroup; 2],
    birth_compact_bind_groups: [wgpu::BindGroup; 2],
    birth_bind_groups: [wgpu::BindGroup; 2],
}

impl Simulation {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, seed: u32) -> Self {
        let agent_data = build_agents(seed, &SimSettings::default());
        let agent_bytes = bytemuck::cast_slice(&agent_data);
        let agent_buffers = [
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("agents state A"),
                contents: agent_bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }),
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("agents state B"),
                contents: agent_bytes,
                usage: wgpu::BufferUsages::STORAGE
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
            }),
        ];

        let habitat = build_habitat(seed);
        let terrain_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("landscape keyframes"),
            contents: bytemuck::cast_slice(&build_terrain_pair(seed, 0)),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let resources = build_resources(&habitat);
        let resource_bytes = bytemuck::cast_slice(&resources);
        let resource_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("resource field atomic units"),
            contents: resource_bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let ground_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ground food and resource accounting"),
            contents: bytemuck::cast_slice(&build_ground(&habitat)),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let resource_display_buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("resource field render copy"),
                contents: resource_bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });
        let fertility_data: Vec<f32> = habitat.iter().map(|value| 0.4 + value * 0.35).collect();
        let fertility_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("persistent soil fertility"),
            contents: bytemuck::cast_slice(&fertility_data),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let perception_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("perception buffer"),
            size: (MAX_AGENTS as u64) * std::mem::size_of::<PerceptionGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let social_perception_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("social perception buffer"),
            size: (MAX_AGENTS as u64) * std::mem::size_of::<SocialPerceptionGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let social_memory = build_social_memory();
        let social_memory_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bounded social memory"),
            contents: bytemuck::cast_slice(&social_memory),
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
        });
        let decision_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("decision buffer"),
            size: (MAX_AGENTS as u64) * std::mem::size_of::<DecisionGpu>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let neural_weights = NeuralWeights::baseline();
        let neural_weights_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("shared recurrent neural policy weights"),
            contents: bytemuck::cast_slice(&neural_weights.flat()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });
        let neural_state_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("private per-agent recurrent state"),
            size: MAX_AGENTS as u64 * NEURAL_HIDDEN as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let request_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("resource request buffer"),
            size: (MAX_AGENTS as u64) * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let birth_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("birth candidate flags"),
            size: (MAX_AGENTS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let free_flags = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("free agent flags"),
            size: (MAX_AGENTS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let free_prefix = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("free agent prefix A"),
                size: (MAX_AGENTS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("free agent prefix B"),
                size: (MAX_AGENTS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];
        let free_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("free agent indices"),
            size: (MAX_AGENTS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let birth_prefix = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("birth candidate prefix A"),
                size: (MAX_AGENTS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("birth candidate prefix B"),
                size: (MAX_AGENTS * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];
        let birth_parents = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("birth parent indices"),
            size: (MAX_AGENTS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let occupancy_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("occupancy grid atomic counts"),
            size: (OCCUPANCY_GRID * OCCUPANCY_GRID * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let cell_offsets = [
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("spatial prefix buffer A"),
                size: (SPATIAL_CELL_COUNT * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("spatial prefix buffer B"),
                size: (SPATIAL_CELL_COUNT * 4) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        ];
        let cell_cursors = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spatial scatter cursors"),
            size: (SPATIAL_CELL_COUNT * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let agent_indices = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("spatial cell agent indices"),
            size: (MAX_AGENTS * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let alive_count_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("living agent counter"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let alive_count_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("living agent counter readback"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let death_stats_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation telemetry counters"),
            size: (DEATH_STATS_COUNT * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let death_stats_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation telemetry readback"),
            size: (DEATH_STATS_COUNT * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("simulation parameters"),
            size: std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let tick_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("per-tick parameter staging"),
            size: 32 * std::mem::size_of::<SimParams>() as u64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let selection_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection parameters"),
            size: std::mem::size_of::<SelectionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let selection_key_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection atomic key"),
            size: 4,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let selection_output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection output"),
            size: std::mem::size_of::<SelectionOutput>() as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let selection_readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("selection readback"),
            size: std::mem::size_of::<SelectionOutput>() as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let intervention_params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world intervention parameters"),
            size: std::mem::size_of::<InterventionParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = |label: &str, source: &str| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(shader_source(source).into()),
            })
        };
        let resource_update_shader = shader(
            "resource update shader",
            include_str!("../shaders/resource_update.wgsl"),
        );
        let clear_occupancy_shader = shader(
            "clear occupancy shader",
            include_str!("../shaders/clear_occupancy.wgsl"),
        );
        let count_occupancy_shader = shader(
            "count occupancy shader",
            include_str!("../shaders/count_occupancy.wgsl"),
        );
        let perception_shader = shader(
            "perception shader",
            include_str!("../shaders/perceive.wgsl"),
        );
        let social_shader = shader(
            "social perception shader",
            include_str!("../shaders/social_perceive.wgsl"),
        );
        let decision_shader = shader("decision shader", include_str!("../shaders/decide.wgsl"));
        let consume_shader = shader(
            "resource consume shader",
            include_str!("../shaders/consume.wgsl"),
        );
        let update_shader = shader(
            "agent update shader",
            include_str!("../shaders/update_agents.wgsl"),
        );
        let select_shader = shader(
            "agent selection shader",
            include_str!("../shaders/select_agent.wgsl"),
        );
        let resolve_shader = shader(
            "selection resolve shader",
            include_str!("../shaders/resolve_selection.wgsl"),
        );
        let intervention_shader = shader(
            "world intervention shader",
            include_str!("../shaders/intervene.wgsl"),
        );
        let kill_shader = shader("kill region shader", include_str!("../shaders/kill.wgsl"));
        let count_alive_shader = shader(
            "living agent count shader",
            include_str!("../shaders/count_alive.wgsl"),
        );
        let prefix_init_shader = shader(
            "spatial prefix init shader",
            include_str!("../shaders/prefix_init.wgsl"),
        );
        let prefix_step_shader = shader(
            "spatial prefix step shader",
            include_str!("../shaders/prefix_step.wgsl"),
        );
        let prepare_scatter_shader = shader(
            "spatial cursor shader",
            include_str!("../shaders/prepare_scatter.wgsl"),
        );
        let scatter_shader = shader(
            "spatial scatter shader",
            include_str!("../shaders/scatter_agents.wgsl"),
        );
        let birth_shader = shader(
            "agent birth shader",
            include_str!("../shaders/apply_births.wgsl"),
        );
        let free_flags_shader = shader(
            "free agent flags shader",
            include_str!("../shaders/free_flags.wgsl"),
        );
        let agent_prefix_init_shader = shader(
            "agent prefix init shader",
            include_str!("../shaders/agent_prefix_init.wgsl"),
        );
        let agent_prefix_step_shader = shader(
            "agent prefix step shader",
            include_str!("../shaders/agent_prefix_step.wgsl"),
        );
        let compact_shader = shader(
            "compact agent indices shader",
            include_str!("../shaders/compact_agent_indices.wgsl"),
        );

        let resource_update_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("resource update layout"),
                entries: &[
                    storage_entry(4, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    uniform_entry(1, wgpu::ShaderStages::COMPUTE),
                    storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let resource_update_pipeline = compute_pipeline(
            device,
            "resource update",
            &resource_update_layout,
            &resource_update_shader,
            "main",
        );
        let resource_update_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("resource update bind group"),
            layout: &resource_update_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: terrain_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: ground_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: resource_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: fertility_buffer.as_entire_binding(),
                },
            ],
        });

        let clear_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("clear occupancy layout"),
            entries: &[storage_entry(0, wgpu::ShaderStages::COMPUTE, false)],
        });
        let clear_occupancy_pipeline = compute_pipeline(
            device,
            "clear occupancy",
            &clear_layout,
            &clear_occupancy_shader,
            "main",
        );
        let clear_occupancy_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("clear occupancy bind group"),
            layout: &clear_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: occupancy_buffer.as_entire_binding(),
            }],
        });

        let count_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("occupancy count layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(2, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let count_occupancy_pipeline = compute_pipeline(
            device,
            "count occupancy",
            &count_layout,
            &count_occupancy_shader,
            "main",
        );
        let count_bind_groups = [
            make_count_group(
                device,
                &count_layout,
                &agent_buffers[0],
                &occupancy_buffer,
                &params_buffer,
            ),
            make_count_group(
                device,
                &count_layout,
                &agent_buffers[1],
                &occupancy_buffer,
                &params_buffer,
            ),
        ];

        let perception_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("perception layout"),
            entries: &[
                storage_entry(5, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(4, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let perception_pipeline = compute_pipeline(
            device,
            "perceive local world",
            &perception_layout,
            &perception_shader,
            "main",
        );
        let perception_bind_groups = [
            make_perception_group(
                device,
                &perception_layout,
                &agent_buffers[0],
                &resource_buffer,
                &occupancy_buffer,
                &perception_buffer,
                &params_buffer,
                &ground_buffer,
            ),
            make_perception_group(
                device,
                &perception_layout,
                &agent_buffers[1],
                &resource_buffer,
                &occupancy_buffer,
                &perception_buffer,
                &params_buffer,
                &ground_buffer,
            ),
        ];

        let social_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("social perception layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(4, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(5, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let social_pipeline = compute_pipeline(
            device,
            "observe nearby agents",
            &social_layout,
            &social_shader,
            "main",
        );
        let social_bind_groups = [
            make_social_group(
                device,
                &social_layout,
                &agent_buffers[0],
                &cell_offsets[0],
                &agent_indices,
                &social_memory_buffer,
                &social_perception_buffer,
                &params_buffer,
            ),
            make_social_group(
                device,
                &social_layout,
                &agent_buffers[1],
                &cell_offsets[0],
                &agent_indices,
                &social_memory_buffer,
                &social_perception_buffer,
                &params_buffer,
            ),
        ];

        let decision_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("decision layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(4, wgpu::ShaderStages::COMPUTE),
                storage_entry(5, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(6, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let decision_pipeline = compute_pipeline(
            device,
            "score primitive actions",
            &decision_layout,
            &decision_shader,
            "main",
        );
        let decision_bind_groups = [
            make_decision_group(
                device,
                &decision_layout,
                &agent_buffers[0],
                &perception_buffer,
                &social_perception_buffer,
                &decision_buffer,
                &params_buffer,
                &neural_weights_buffer,
                &neural_state_buffer,
            ),
            make_decision_group(
                device,
                &decision_layout,
                &agent_buffers[1],
                &perception_buffer,
                &social_perception_buffer,
                &decision_buffer,
                &params_buffer,
                &neural_weights_buffer,
                &neural_state_buffer,
            ),
        ];

        let consume_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("consume layout"),
            entries: &[
                storage_entry(6, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(4, wgpu::ShaderStages::COMPUTE),
                storage_entry(5, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let consume_pipeline = compute_pipeline(
            device,
            "consume local resource",
            &consume_layout,
            &consume_shader,
            "main",
        );
        let consume_bind_groups = [
            make_consume_group(
                device,
                &consume_layout,
                &agent_buffers[0],
                &decision_buffer,
                &resource_buffer,
                &request_buffer,
                &params_buffer,
                &death_stats_buffer,
                &ground_buffer,
            ),
            make_consume_group(
                device,
                &consume_layout,
                &agent_buffers[1],
                &decision_buffer,
                &resource_buffer,
                &request_buffer,
                &params_buffer,
                &death_stats_buffer,
                &ground_buffer,
            ),
        ];

        let update_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("agent update layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(4, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(5, wgpu::ShaderStages::COMPUTE),
                storage_entry(6, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(7, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let update_pipeline = compute_pipeline(
            device,
            "update agents",
            &update_layout,
            &update_shader,
            "main",
        );
        let update_bind_groups = [
            [
                make_update_group(
                    device,
                    &update_layout,
                    &agent_buffers[0],
                    &agent_buffers[0],
                    &perception_buffer,
                    &decision_buffer,
                    &request_buffer,
                    &params_buffer,
                    &birth_flags,
                    &death_stats_buffer,
                ),
                make_update_group(
                    device,
                    &update_layout,
                    &agent_buffers[0],
                    &agent_buffers[1],
                    &perception_buffer,
                    &decision_buffer,
                    &request_buffer,
                    &params_buffer,
                    &birth_flags,
                    &death_stats_buffer,
                ),
            ],
            [
                make_update_group(
                    device,
                    &update_layout,
                    &agent_buffers[1],
                    &agent_buffers[0],
                    &perception_buffer,
                    &decision_buffer,
                    &request_buffer,
                    &params_buffer,
                    &birth_flags,
                    &death_stats_buffer,
                ),
                make_update_group(
                    device,
                    &update_layout,
                    &agent_buffers[1],
                    &agent_buffers[1],
                    &perception_buffer,
                    &decision_buffer,
                    &request_buffer,
                    &params_buffer,
                    &birth_flags,
                    &death_stats_buffer,
                ),
            ],
        ];

        let select_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("select layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                uniform_entry(1, wgpu::ShaderStages::COMPUTE),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let select_pipeline = compute_pipeline(
            device,
            "select nearest agent",
            &select_layout,
            &select_shader,
            "main",
        );
        let select_bind_groups = [
            make_select_group(
                device,
                &select_layout,
                &agent_buffers[0],
                &selection_params_buffer,
                &selection_key_buffer,
            ),
            make_select_group(
                device,
                &select_layout,
                &agent_buffers[1],
                &selection_params_buffer,
                &selection_key_buffer,
            ),
        ];

        let resolve_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("selection resolve layout"),
            entries: &[
                storage_entry(6, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(4, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(5, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let selection_resolve_pipeline = compute_pipeline(
            device,
            "resolve selected agent",
            &resolve_layout,
            &resolve_shader,
            "main",
        );
        let selection_resolve_bind_groups = [
            make_resolve_group(
                device,
                &resolve_layout,
                &agent_buffers[0],
                &perception_buffer,
                &decision_buffer,
                &social_perception_buffer,
                &selection_key_buffer,
                &selection_output_buffer,
                &social_memory_buffer,
            ),
            make_resolve_group(
                device,
                &resolve_layout,
                &agent_buffers[1],
                &perception_buffer,
                &decision_buffer,
                &social_perception_buffer,
                &selection_key_buffer,
                &selection_output_buffer,
                &social_memory_buffer,
            ),
        ];

        let intervention_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("resource intervention layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    uniform_entry(1, wgpu::ShaderStages::COMPUTE),
                    storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let resource_intervention_pipeline = compute_pipeline(
            device,
            "change resource field",
            &intervention_layout,
            &intervention_shader,
            "apply",
        );
        let resource_intervention_bind_group =
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("resource intervention bind group"),
                layout: &intervention_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: resource_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: intervention_params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ground_buffer.as_entire_binding(),
                    },
                ],
            });
        let kill_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("kill region layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(1, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let kill_agents_pipeline = compute_pipeline(
            device,
            "kill agents in region",
            &kill_layout,
            &kill_shader,
            "main",
        );
        let kill_agents_bind_groups = [
            make_intervention_group(
                device,
                &kill_layout,
                &agent_buffers[0],
                &intervention_params_buffer,
            ),
            make_intervention_group(
                device,
                &kill_layout,
                &agent_buffers[1],
                &intervention_params_buffer,
            ),
        ];
        let count_alive_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("living agent count layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let count_alive_pipeline = compute_pipeline(
            device,
            "count living agents",
            &count_alive_layout,
            &count_alive_shader,
            "main",
        );
        let count_alive_bind_groups = [
            make_intervention_group(
                device,
                &count_alive_layout,
                &agent_buffers[0],
                &alive_count_buffer,
            ),
            make_intervention_group(
                device,
                &count_alive_layout,
                &agent_buffers[1],
                &alive_count_buffer,
            ),
        ];

        let prefix_init_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("spatial prefix init layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let prefix_init_pipeline = compute_pipeline(
            device,
            "initialize spatial prefix",
            &prefix_init_layout,
            &prefix_init_shader,
            "main",
        );
        let prefix_init_bind_group = make_prefix_init_group(
            device,
            &prefix_init_layout,
            &occupancy_buffer,
            &cell_offsets[0],
        );

        let prefix_step_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("spatial prefix step layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let prefix_step_pipelines = [
            1u32, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768,
        ]
        .into_iter()
        .map(|stride| {
            compute_pipeline(
                device,
                &format!("spatial prefix stride {stride}"),
                &prefix_step_layout,
                &prefix_step_shader,
                &format!("step_{stride}"),
            )
        })
        .collect();
        let prefix_step_bind_groups = [
            [
                make_prefix_step_group(
                    device,
                    &prefix_step_layout,
                    &cell_offsets[0],
                    &cell_offsets[0],
                ),
                make_prefix_step_group(
                    device,
                    &prefix_step_layout,
                    &cell_offsets[0],
                    &cell_offsets[1],
                ),
            ],
            [
                make_prefix_step_group(
                    device,
                    &prefix_step_layout,
                    &cell_offsets[1],
                    &cell_offsets[0],
                ),
                make_prefix_step_group(
                    device,
                    &prefix_step_layout,
                    &cell_offsets[1],
                    &cell_offsets[1],
                ),
            ],
        ];

        let prepare_scatter_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("spatial cursor layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let prepare_scatter_pipeline = compute_pipeline(
            device,
            "prepare spatial scatter cursors",
            &prepare_scatter_layout,
            &prepare_scatter_shader,
            "main",
        );
        let prepare_scatter_bind_groups = [
            make_prepare_scatter_group(
                device,
                &prepare_scatter_layout,
                &cell_offsets[0],
                &cell_cursors,
            ),
            make_prepare_scatter_group(
                device,
                &prepare_scatter_layout,
                &cell_offsets[1],
                &cell_cursors,
            ),
        ];

        let scatter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("spatial scatter layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(3, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let scatter_pipeline = compute_pipeline(
            device,
            "scatter agents into spatial cells",
            &scatter_layout,
            &scatter_shader,
            "main",
        );
        let scatter_bind_groups = [
            make_scatter_group(
                device,
                &scatter_layout,
                &agent_buffers[0],
                &cell_cursors,
                &agent_indices,
                &params_buffer,
            ),
            make_scatter_group(
                device,
                &scatter_layout,
                &agent_buffers[1],
                &cell_cursors,
                &agent_indices,
                &params_buffer,
            ),
        ];

        let free_flags_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("free agent flags layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(2, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let free_flags_pipeline = compute_pipeline(
            device,
            "mark free agent slots",
            &free_flags_layout,
            &free_flags_shader,
            "main",
        );
        let free_flags_bind_groups = [
            make_free_flags_group(
                device,
                &free_flags_layout,
                &agent_buffers[0],
                &free_flags,
                &params_buffer,
            ),
            make_free_flags_group(
                device,
                &free_flags_layout,
                &agent_buffers[1],
                &free_flags,
                &params_buffer,
            ),
        ];

        let agent_prefix_init_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("agent prefix init layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let agent_prefix_init_pipeline = compute_pipeline(
            device,
            "initialize agent prefix",
            &agent_prefix_init_layout,
            &agent_prefix_init_shader,
            "main",
        );
        let free_prefix_init_bind_group = make_agent_prefix_init_group(
            device,
            &agent_prefix_init_layout,
            &free_flags,
            &free_prefix[0],
        );
        let birth_prefix_init_bind_group = make_agent_prefix_init_group(
            device,
            &agent_prefix_init_layout,
            &birth_flags,
            &birth_prefix[0],
        );

        let agent_prefix_step_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("agent prefix step layout"),
                entries: &[
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let agent_prefix_step_pipelines = [
            1u32, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192, 16384, 32768, 65536,
        ]
        .into_iter()
        .map(|stride| {
            compute_pipeline(
                device,
                &format!("agent prefix stride {stride}"),
                &agent_prefix_step_layout,
                &agent_prefix_step_shader,
                &format!("step_{stride}"),
            )
        })
        .collect();
        let free_prefix_step_bind_groups =
            make_agent_prefix_step_groups(device, &agent_prefix_step_layout, &free_prefix);
        let birth_prefix_step_bind_groups =
            make_agent_prefix_step_groups(device, &agent_prefix_step_layout, &birth_prefix);

        let compact_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("compact agent indices layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let compact_pipeline = compute_pipeline(
            device,
            "compact agent indices",
            &compact_layout,
            &compact_shader,
            "main",
        );
        let free_compact_bind_groups = [
            make_compact_group(
                device,
                &compact_layout,
                &free_flags,
                &free_prefix[1],
                &free_indices,
            ),
            make_compact_group(
                device,
                &compact_layout,
                &free_flags,
                &free_prefix[1],
                &free_indices,
            ),
        ];
        let birth_compact_bind_groups = [
            make_compact_group(
                device,
                &compact_layout,
                &birth_flags,
                &birth_prefix[1],
                &birth_parents,
            ),
            make_compact_group(
                device,
                &compact_layout,
                &birth_flags,
                &birth_prefix[1],
                &birth_parents,
            ),
        ];

        let birth_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("agent birth layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(4, wgpu::ShaderStages::COMPUTE, true),
                uniform_entry(5, wgpu::ShaderStages::COMPUTE),
                storage_entry(6, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(7, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let birth_pipeline = compute_pipeline(
            device,
            "apply agent births",
            &birth_layout,
            &birth_shader,
            "main",
        );
        let birth_bind_groups = [
            make_birth_group(
                device,
                &birth_layout,
                &agent_buffers[0],
                &free_indices,
                &free_prefix[1],
                &birth_parents,
                &birth_prefix[1],
                &params_buffer,
                &death_stats_buffer,
                &social_memory_buffer,
            ),
            make_birth_group(
                device,
                &birth_layout,
                &agent_buffers[1],
                &free_indices,
                &free_prefix[1],
                &birth_parents,
                &birth_prefix[1],
                &params_buffer,
                &death_stats_buffer,
                &social_memory_buffer,
            ),
        ];

        let event_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("recent interaction events"),
            size: 65536 * 32,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let interaction_shader = shader(
            "pair interactions",
            include_str!("../shaders/interactions.wgsl"),
        );
        let interaction_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("pair interaction layout"),
                entries: &[
                    storage_entry(7, wgpu::ShaderStages::COMPUTE, false),
                    storage_entry(6, wgpu::ShaderStages::COMPUTE, false),
                    storage_entry(5, wgpu::ShaderStages::COMPUTE, false),
                    storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                    storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                    storage_entry(2, wgpu::ShaderStages::COMPUTE, false),
                    uniform_entry(3, wgpu::ShaderStages::COMPUTE),
                    storage_entry(4, wgpu::ShaderStages::COMPUTE, false),
                ],
            });
        let claims = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("disjoint interaction claims"),
            size: MAX_AGENTS as u64 * 4,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let interaction_pipelines = ["clear", "propose", "resolve"].map(|entry| {
            compute_pipeline(
                device,
                entry,
                &interaction_layout,
                &interaction_shader,
                entry,
            )
        });
        let interaction_bind_groups = [0, 1].map(|index| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("pair interactions"),
                layout: &interaction_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: social_memory_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: event_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: ground_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: agent_buffers[index].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: decision_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: claims.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: death_stats_buffer.as_entire_binding(),
                    },
                ],
            })
        });
        let death_shader = shader(
            "release dead inventories",
            include_str!("../shaders/release_food.wgsl"),
        );
        let death_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("release food layout"),
            entries: &[
                storage_entry(0, wgpu::ShaderStages::COMPUTE, false),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, false),
            ],
        });
        let death_pipeline = compute_pipeline(
            device,
            "release carried food",
            &death_layout,
            &death_shader,
            "main",
        );
        let death_bind_groups = [0, 1].map(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("release carried food"),
                layout: &death_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: agent_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: ground_buffer.as_entire_binding(),
                    },
                ],
            })
        });
        let summary_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("world summary chunks"),
            size: 4096 * 64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let summary_shader = shader("world summary", include_str!("../shaders/summarize.wgsl"));
        let summary_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("world summary layout"),
            entries: &[
                storage_entry(5, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(0, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(1, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(2, wgpu::ShaderStages::COMPUTE, true),
                storage_entry(3, wgpu::ShaderStages::COMPUTE, false),
                uniform_entry(4, wgpu::ShaderStages::COMPUTE),
            ],
        });
        let summary_pipeline = compute_pipeline(
            device,
            "world summary",
            &summary_layout,
            &summary_shader,
            "main",
        );
        let summary_bind_groups = [0, 1].map(|i| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("world summary"),
                layout: &summary_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: social_memory_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: agent_buffers[i].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: resource_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: ground_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: summary_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            })
        });
        let params = params_for(0, &SimSettings::default(), seed);
        queue.write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));
        queue.write_buffer(
            &death_stats_buffer,
            0,
            bytemuck::cast_slice(&[0u32; DEATH_STATS_COUNT as usize]),
        );

        Self {
            terrain_buffer,
            terrain_epoch: 0,
            settings: SimSettings::default(),
            seed,
            summary_pipeline,
            summary_bind_groups,
            summary_buffer,
            event_buffer,
            tick: 0,
            current_buffer: 0,
            agent_buffers,
            resource_buffer,
            ground_buffer,
            death_pipeline,
            death_bind_groups,
            resource_display_buffer,
            fertility_buffer,
            perception_buffer,
            _social_perception_buffer: social_perception_buffer,
            social_memory_buffer,
            _decision_buffer: decision_buffer,
            neural_weights_buffer,
            neural_state_buffer,
            _request_buffer: request_buffer,
            birth_flags,
            free_flags,
            free_prefix,
            free_indices,
            birth_prefix,
            birth_parents,
            occupancy_buffer,
            cell_offsets,
            cell_cursors,
            agent_indices,
            alive_count_buffer,
            alive_count_readback,
            death_stats_buffer,
            death_stats_readback,
            params_buffer,
            tick_params_buffer,
            interaction_pipelines,
            interaction_bind_groups,
            selection_params_buffer,
            selection_key_buffer,
            selection_output_buffer,
            selection_readback,
            intervention_params_buffer,
            resource_update_pipeline,
            clear_occupancy_pipeline,
            count_occupancy_pipeline,
            perception_pipeline,
            social_pipeline,
            decision_pipeline,
            consume_pipeline,
            update_pipeline,
            select_pipeline,
            selection_resolve_pipeline,
            resource_intervention_pipeline,
            kill_agents_pipeline,
            count_alive_pipeline,
            prefix_init_pipeline,
            prefix_step_pipelines,
            prepare_scatter_pipeline,
            scatter_pipeline,
            free_flags_pipeline,
            agent_prefix_init_pipeline,
            agent_prefix_step_pipelines,
            compact_pipeline,
            birth_pipeline,
            resource_update_bind_group,
            clear_occupancy_bind_group,
            count_bind_groups,
            perception_bind_groups,
            social_bind_groups,
            decision_bind_groups,
            consume_bind_groups,
            update_bind_groups,
            select_bind_groups,
            selection_resolve_bind_groups,
            resource_intervention_bind_group,
            kill_agents_bind_groups,
            count_alive_bind_groups,
            prefix_init_bind_group,
            prefix_step_bind_groups,
            prepare_scatter_bind_groups,
            scatter_bind_groups,
            free_flags_bind_groups,
            free_prefix_init_bind_group,
            birth_prefix_init_bind_group,
            free_prefix_step_bind_groups,
            birth_prefix_step_bind_groups,
            free_compact_bind_groups,
            birth_compact_bind_groups,
            birth_bind_groups,
        }
    }

    pub fn reset(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.terrain_buffer,
            0,
            bytemuck::cast_slice(&build_terrain_pair(self.seed, 0)),
        );
        self.terrain_epoch = 0;
        let agents = build_agents(self.seed, &self.settings);
        let habitat = build_habitat(self.seed);
        let resources = build_resources(&habitat);
        queue.write_buffer(&self.agent_buffers[0], 0, bytemuck::cast_slice(&agents));
        queue.write_buffer(&self.agent_buffers[1], 0, bytemuck::cast_slice(&agents));
        queue.write_buffer(&self.resource_buffer, 0, bytemuck::cast_slice(&resources));
        queue.write_buffer(&self.event_buffer, 0, &vec![0; 65536 * 32]);
        queue.write_buffer(&self.neural_state_buffer, 0, &vec![0u8; MAX_AGENTS as usize * NEURAL_HIDDEN * 4]);
        queue.write_buffer(
            &self.ground_buffer,
            0,
            bytemuck::cast_slice(&build_ground(&habitat)),
        );
        let social_memory = build_social_memory();
        queue.write_buffer(
            &self.social_memory_buffer,
            0,
            bytemuck::cast_slice(&social_memory),
        );
        let fertility_data: Vec<f32> = habitat.iter().map(|value| 0.4 + value * 0.35).collect();
        queue.write_buffer(
            &self.fertility_buffer,
            0,
            bytemuck::cast_slice(&fertility_data),
        );
        queue.write_buffer(
            &self.resource_display_buffer,
            0,
            bytemuck::cast_slice(&resources),
        );
        queue.write_buffer(
            &self.death_stats_buffer,
            0,
            bytemuck::cast_slice(&[0u32; DEATH_STATS_COUNT as usize]),
        );
        self.tick = 0;
        self.current_buffer = 0;
    }

    pub fn update_params(&self, queue: &wgpu::Queue) {
        let params = params_for(self.tick, &self.settings, self.seed);
        queue.write_buffer(&self.params_buffer, 0, bytemuck::bytes_of(&params));
    }

    pub fn set_neural_weights(&self, queue: &wgpu::Queue, weights: &NeuralWeights) -> Result<(), String> {
        weights.validate()?;
        queue.write_buffer(&self.neural_weights_buffer, 0, bytemuck::cast_slice(&weights.flat()));
        Ok(())
    }


    pub fn encode_ticks(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        ticks: u32,
    ) {
        if ticks == 0 {
            return;
        }
        assert!(ticks <= 32, "submit at most 32 ticks per batch");
        let tick_params: Vec<_> = (0..ticks)
            .map(|offset| params_for(self.tick.wrapping_add(offset), &self.settings, self.seed))
            .collect();
        queue.write_buffer(
            &self.tick_params_buffer,
            0,
            bytemuck::cast_slice(&tick_params),
        );
        for offset in 0..ticks {
            let epoch = self.tick / 8192;
            if self.settings.evolving_landscape && epoch != self.terrain_epoch {
                let staging = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("next landscape keyframes"),
                    contents: bytemuck::cast_slice(&build_terrain_pair(self.seed, epoch)),
                    usage: wgpu::BufferUsages::COPY_SRC,
                });
                // Ordered copy handles a keyframe boundary inside a tick batch.
                encoder.copy_buffer_to_buffer(&staging, 0, &self.terrain_buffer, 0, staging.size());
                self.terrain_epoch = epoch;
            }
            encoder.copy_buffer_to_buffer(
                &self.tick_params_buffer,
                offset as u64 * std::mem::size_of::<SimParams>() as u64,
                &self.params_buffer,
                0,
                std::mem::size_of::<SimParams>() as u64,
            );
            let src = self.current_buffer;
            let dst = 1 - src;
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("mark free agent slots"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.free_flags_pipeline);
                pass.set_bind_group(0, &self.free_flags_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("initialize free-slot prefix"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.agent_prefix_init_pipeline);
                pass.set_bind_group(0, &self.free_prefix_init_bind_group, &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            let mut free_prefix_buffer = 0usize;
            for pipeline in &self.agent_prefix_step_pipelines {
                let destination_buffer = 1 - free_prefix_buffer;
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("free-slot prefix scan"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(
                    0,
                    &self.free_prefix_step_bind_groups[free_prefix_buffer][destination_buffer],
                    &[],
                );
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
                free_prefix_buffer = destination_buffer;
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compact free agent slots"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.compact_pipeline);
                pass.set_bind_group(0, &self.free_compact_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("resource regeneration"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.resource_update_pipeline);
                pass.set_bind_group(0, &self.resource_update_bind_group, &[]);
                pass.dispatch_workgroups((RESOURCE_GRID + 7) / 8, (RESOURCE_GRID + 7) / 8, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("clear occupancy"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.clear_occupancy_pipeline);
                pass.set_bind_group(0, &self.clear_occupancy_bind_group, &[]);
                pass.dispatch_workgroups((OCCUPANCY_GRID + 7) / 8, (OCCUPANCY_GRID + 7) / 8, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("count occupancy"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.count_occupancy_pipeline);
                pass.set_bind_group(0, &self.count_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("initialize spatial prefix"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.prefix_init_pipeline);
                pass.set_bind_group(0, &self.prefix_init_bind_group, &[]);
                pass.dispatch_workgroups((SPATIAL_CELL_COUNT + 63) / 64, 1, 1);
            }
            let mut prefix_buffer = 0usize;
            for pipeline in &self.prefix_step_pipelines {
                let destination_buffer = 1 - prefix_buffer;
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("spatial prefix scan"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(
                    0,
                    &self.prefix_step_bind_groups[prefix_buffer][destination_buffer],
                    &[],
                );
                pass.dispatch_workgroups((SPATIAL_CELL_COUNT + 63) / 64, 1, 1);
                prefix_buffer = destination_buffer;
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("initialize spatial scatter cursors"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.prepare_scatter_pipeline);
                pass.set_bind_group(0, &self.prepare_scatter_bind_groups[prefix_buffer], &[]);
                pass.dispatch_workgroups((SPATIAL_CELL_COUNT + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("scatter agents into spatial cells"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.scatter_pipeline);
                pass.set_bind_group(0, &self.scatter_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("local perception"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.perception_pipeline);
                pass.set_bind_group(0, &self.perception_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("social perception and relationship update"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.social_pipeline);
                pass.set_bind_group(0, &self.social_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("primitive action scoring"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.decision_pipeline);
                pass.set_bind_group(0, &self.decision_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("resource consumption"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.consume_pipeline);
                pass.set_bind_group(0, &self.consume_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("agent state update"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.update_pipeline);
                pass.set_bind_group(0, &self.update_bind_groups[src][dst], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            for pipeline in &self.interaction_pipelines {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("resolve disjoint local interactions"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &self.interaction_bind_groups[dst], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("release inventories on death"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.death_pipeline);
                pass.set_bind_group(0, &self.death_bind_groups[dst], &[]);
                pass.dispatch_workgroups(MAX_AGENTS.div_ceil(64), 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("initialize birth-candidate prefix"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.agent_prefix_init_pipeline);
                pass.set_bind_group(0, &self.birth_prefix_init_bind_group, &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            let mut birth_prefix_buffer = 0usize;
            for pipeline in &self.agent_prefix_step_pipelines {
                let destination_buffer = 1 - birth_prefix_buffer;
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("birth-candidate prefix scan"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(
                    0,
                    &self.birth_prefix_step_bind_groups[birth_prefix_buffer][destination_buffer],
                    &[],
                );
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
                birth_prefix_buffer = destination_buffer;
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("compact birth candidates"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.compact_pipeline);
                pass.set_bind_group(0, &self.birth_compact_bind_groups[src], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("apply agent births"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&self.birth_pipeline);
                pass.set_bind_group(0, &self.birth_bind_groups[dst], &[]);
                pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
            }
            self.current_buffer = dst;
            self.tick = self.tick.wrapping_add(1);
        }
        encoder.copy_buffer_to_buffer(
            &self.resource_buffer,
            0,
            &self.resource_display_buffer,
            0,
            (RESOURCE_GRID * RESOURCE_GRID * 4) as u64,
        );
        encoder.clear_buffer(&self.alive_count_buffer, 0, None);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("count living agents"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.count_alive_pipeline);
            pass.set_bind_group(0, &self.count_alive_bind_groups[self.current_buffer], &[]);
            pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
        }
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
                _padding: 0.0,
            }),
        );
        queue.write_buffer(&self.selection_key_buffer, 0, bytemuck::bytes_of(&u32::MAX));
        let src = self.current_buffer;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("select agent encoder"),
        });
        encoder.clear_buffer(&self.selection_output_buffer, 0, None);
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("select nearest agent"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.select_pipeline);
            pass.set_bind_group(0, &self.select_bind_groups[src], &[]);
            pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
        }
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("resolve selected agent"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.selection_resolve_pipeline);
            pass.set_bind_group(0, &self.selection_resolve_bind_groups[src], &[]);
            pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &self.selection_output_buffer,
            0,
            &self.selection_readback,
            0,
            std::mem::size_of::<SelectionOutput>() as u64,
        );
        queue.submit(Some(encoder.finish()));
        let slice = self.selection_readback.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        let _ = device.poll(wgpu::Maintain::Wait);
        receiver.recv().ok()?.ok()?;
        let mapped = slice.get_mapped_range();
        let result = *bytemuck::from_bytes::<SelectionOutput>(&mapped);
        drop(mapped);
        self.selection_readback.unmap();
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
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("resource intervention encoder"),
        });
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("resource intervention"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&self.resource_intervention_pipeline);
        pass.set_bind_group(0, &self.resource_intervention_bind_group, &[]);
        pass.dispatch_workgroups((RESOURCE_GRID + 7) / 8, (RESOURCE_GRID + 7) / 8, 1);
        drop(pass);
        encoder.copy_buffer_to_buffer(
            &self.resource_buffer,
            0,
            &self.resource_display_buffer,
            0,
            (RESOURCE_GRID * RESOURCE_GRID * 4) as u64,
        );
        queue.submit(Some(encoder.finish()));
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
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("agent intervention encoder"),
        });
        for bind_group in [&self.kill_agents_bind_groups[self.current_buffer]] {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("kill agents in region"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.kill_agents_pipeline);
            pass.set_bind_group(0, bind_group, &[]);
            pass.dispatch_workgroups((MAX_AGENTS + 63) / 64, 1, 1);
        }
        queue.submit(Some(encoder.finish()));
    }
}

fn storage_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    read_only: bool,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn uniform_entry(binding: u32, visibility: wgpu::ShaderStages) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn compute_pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    module: &wgpu::ShaderModule,
    entry_point: &str,
) -> wgpu::ComputePipeline {
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[layout],
        push_constant_ranges: &[],
    });

    device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some(label),
        layout: Some(&pipeline_layout),
        module,
        entry_point: Some(entry_point),
        compilation_options: Default::default(),
        cache: None,
    })
}

fn make_count_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    occupancy: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("count occupancy bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: occupancy.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn make_prefix_init_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    occupancy: &wgpu::Buffer,
    prefix: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("spatial prefix init bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: occupancy.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: prefix.as_entire_binding(),
            },
        ],
    })
}

fn make_prefix_step_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    source: &wgpu::Buffer,
    destination: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("spatial prefix step bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: source.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: destination.as_entire_binding(),
            },
        ],
    })
}

fn make_prepare_scatter_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    prefix: &wgpu::Buffer,
    cursors: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("spatial cursor bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: prefix.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: cursors.as_entire_binding(),
            },
        ],
    })
}

fn make_scatter_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    cursors: &wgpu::Buffer,
    agent_indices: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("spatial scatter bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: cursors.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: agent_indices.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn make_free_flags_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    flags: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("free agent flags bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: flags.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn make_agent_prefix_init_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    flags: &wgpu::Buffer,
    prefix: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("agent prefix init bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: flags.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: prefix.as_entire_binding(),
            },
        ],
    })
}

fn make_agent_prefix_step_groups(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffers: &[wgpu::Buffer; 2],
) -> [[wgpu::BindGroup; 2]; 2] {
    [
        [
            make_prefix_step_group(device, layout, &buffers[0], &buffers[0]),
            make_prefix_step_group(device, layout, &buffers[0], &buffers[1]),
        ],
        [
            make_prefix_step_group(device, layout, &buffers[1], &buffers[0]),
            make_prefix_step_group(device, layout, &buffers[1], &buffers[1]),
        ],
    ]
}

fn make_compact_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    flags: &wgpu::Buffer,
    prefix: &wgpu::Buffer,
    indices: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("compact agent indices bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: flags.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: prefix.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: indices.as_entire_binding(),
            },
        ],
    })
}

fn make_birth_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    free_indices: &wgpu::Buffer,
    free_prefix: &wgpu::Buffer,
    birth_parents: &wgpu::Buffer,
    birth_prefix: &wgpu::Buffer,
    params: &wgpu::Buffer,
    stats: &wgpu::Buffer,
    social_memory: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("agent birth bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: free_indices.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: free_prefix.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: birth_parents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: birth_prefix.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: stats.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: social_memory.as_entire_binding(),
            },
        ],
    })
}

fn make_perception_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    resources: &wgpu::Buffer,
    occupancy: &wgpu::Buffer,
    perception: &wgpu::Buffer,
    params: &wgpu::Buffer,

    ground: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("perception bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 5,
                resource: ground.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: resources.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: occupancy.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: perception.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn make_decision_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    perception: &wgpu::Buffer,
    social_perception: &wgpu::Buffer,
    decision: &wgpu::Buffer,
    params: &wgpu::Buffer,
    neural_weights: &wgpu::Buffer,
    neural_state: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("decision bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: perception.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: social_perception.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: decision.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry { binding: 5, resource: neural_weights.as_entire_binding() },
            wgpu::BindGroupEntry { binding: 6, resource: neural_state.as_entire_binding() },
        ],
    })
}

fn make_social_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    cell_offsets: &wgpu::Buffer,
    agent_indices: &wgpu::Buffer,
    social_memory: &wgpu::Buffer,
    social_perception: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("social perception bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: cell_offsets.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: agent_indices.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: social_memory.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: social_perception.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn make_consume_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    decisions: &wgpu::Buffer,
    resources: &wgpu::Buffer,
    requests: &wgpu::Buffer,
    params: &wgpu::Buffer,
    stats: &wgpu::Buffer,

    ground: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("consume bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 6,
                resource: ground.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: decisions.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: resources.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: requests.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: stats.as_entire_binding(),
            },
        ],
    })
}

fn make_update_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    src: &wgpu::Buffer,
    dst: &wgpu::Buffer,
    perception: &wgpu::Buffer,
    decision: &wgpu::Buffer,
    requests: &wgpu::Buffer,
    params: &wgpu::Buffer,
    birth_flags: &wgpu::Buffer,
    stats: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("agent update bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: src.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: perception.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: decision.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: requests.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: dst.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 6,
                resource: birth_flags.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 7,
                resource: stats.as_entire_binding(),
            },
        ],
    })
}

fn make_select_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    selection_params: &wgpu::Buffer,
    key: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("select bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: selection_params.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: key.as_entire_binding(),
            },
        ],
    })
}

fn make_resolve_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    agents: &wgpu::Buffer,
    perceptions: &wgpu::Buffer,
    decisions: &wgpu::Buffer,
    social: &wgpu::Buffer,
    key: &wgpu::Buffer,
    output: &wgpu::Buffer,

    memory: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("selection resolve bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 6,
                resource: memory.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: agents.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: perceptions.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: decisions.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: social.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: key.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output.as_entire_binding(),
            },
        ],
    })
}

fn make_intervention_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("intervention bind group"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn params_for(tick: u32, settings: &SimSettings, seed: u32) -> SimParams {
    SimParams {
        world_size: WORLD_SIZE,
        resource_grid_size: RESOURCE_GRID,
        // The buffer is a fixed-capacity population pool. `population` controls how
        // many agents are alive after reset; dead slots remain available for births.
        agent_count: MAX_AGENTS,
        tick,
        time_and_costs: [
            1.0,
            settings.resource_regeneration,
            settings.movement_energy_cost,
            settings.metabolic_cost,
        ],
        resource_and_noise: [
            settings.consume_amount,
            settings.conversion_efficiency,
            settings.heterogeneity,
            settings.exploration_noise,
        ],
        sensor_and_padding: [
            settings.sensor_radius,
            settings.maturity_age,
            settings.reproduction_threshold,
            settings.reproduction_cost,
        ],
        social_weights: [
            settings.social_access,
            settings.social_concern,
            settings.reciprocity,
            u32::from(settings.force_enabled) as f32,
        ],
        lifecycle: [
            seed,
            settings.birth_cooldown,
            u32::from(settings.communication_enabled),
            u32::from(settings.evolving_landscape),
        ],
        neural_config: [u32::from(settings.neural_policy), 0, 0, 0],
    }
}

fn build_agents(seed: u32, settings: &SimSettings) -> Vec<AgentGpu> {
    let mut rng = seed.max(1);
    (0..MAX_AGENTS)
        .map(|i| {
            let x = random01(&mut rng) * WORLD_SIZE;
            let y = random01(&mut rng) * WORLD_SIZE;
            let direction = random01(&mut rng) * std::f32::consts::TAU;
            AgentGpu {
                position: [x, y],
                velocity: [0.0; 2],
                energy: 65.0,
                age: random01(&mut rng) * 300.0,
                max_speed: 1.2,
                sensor_radius: settings.sensor_radius,
                food: if i < settings.population { 2.0 } else { 0.0 },
                goal: [
                    (x + direction.cos() * 32.0).clamp(0.0, WORLD_SIZE),
                    (y + direction.sin() * 32.0).clamp(0.0, WORLD_SIZE),
                ],
                rng: rng ^ i.wrapping_mul(0x9e3779b9),
                alive: u32::from(i < settings.population),
                generation: 1,
                target: MAX_AGENTS,
                event_actor: MAX_AGENTS,
                guide_id: MAX_AGENTS,
                max_age: 9000.0 + random01(&mut rng) * 2000.0,
                ..Default::default()
            }
        })
        .collect()
}

fn build_social_memory() -> Vec<SocialRelationGpu> {
    vec![
        SocialRelationGpu {
            target_slot: MAX_AGENTS,
            target_generation: 0,
            familiarity: 0.0,
            benefit: 0.0,
            harm: 0.0,
            last_seen_tick: 0,
            benefit_evidence: 0.0,
            harm_evidence: 0.0,
            navigation: 0.0,
            navigation_evidence: 0.0,
            last_report_tick: 0,
            last_report_observed: 0,
        };
        (MAX_AGENTS * SOCIAL_SLOTS) as usize
    ]
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

#[cfg(test)]
#[path = "simulation_tests.rs"]
mod tests;

#[path = "observability.rs"]
pub mod observability;
