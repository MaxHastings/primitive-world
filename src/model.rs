//! recurrent-v1 body/controller ABI. No semantic memory or desirability scorer.
use bytemuck::{Pod, Zeroable};
pub const MODEL_ID: &str = "recurrent-v1";
pub const MAX_AGENTS: u32 = 16_384;
pub const RESOURCE_GRID: u32 = 512;
pub const OCCUPANCY_GRID: u32 = 256;
pub const SPATIAL_CELL_COUNT: u32 = OCCUPANCY_GRID * OCCUPANCY_GRID;
pub const WORLD_SIZE: f32 = 2048.0;
pub const DEATH_STATS_COUNT: u32 = 32;
pub const EVENT_RING_SIZE: u32 = 65_536;
pub const INPUTS: usize = 64;
pub const HIDDEN: usize = 16;
pub const OUTPUTS: usize = 16;
pub const RECURRENT_ROW: usize = INPUTS + HIDDEN + 1;
pub const OUTPUT_BASE: usize = HIDDEN * RECURRENT_ROW;
pub const GENOME_SIZE: usize = OUTPUT_BASE + OUTPUTS * (HIDDEN + 1);
pub const ACTION_NAMES: [&str; 7] = [
    "none",
    "collect",
    "ingest",
    "transfer",
    "force",
    "emit",
    "reproduce",
];
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
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
    pub alive: u32,
    pub attention: f32,
    pub rng: u32,
    pub generation: u32,
    pub next_birth: u32,
    pub max_age: f32,
    pub event_amount: f32,
    pub event_tick: u32,
    pub event_actor: u32,
    pub event_generation: u32,
    pub last_communication: u32,
    pub collected: f32,
    pub ingested: f32,
    pub spent: f32,
    pub received: f32,
    pub moved: [f32; 2],
    pub lineage_id: u32,
    pub parent_lineage: u32,
    pub birth_tick: u32,
    pub birth_parent_slot: u32,
    pub ancestry_depth: u32,
    pub lifetime_births: u32,
    pub distance_travelled: f32,
    pub observer_padding: u32,
    pub hidden: [f32; HIDDEN],
}
impl Default for AgentGpu {
    fn default() -> Self {
        Self::zeroed()
    }
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SampleGpu {
    pub offset: [f32; 2],
    pub food: f32,
    pub padding: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct BodyGpu {
    pub offset: [f32; 2],
    pub velocity: [f32; 2],
    pub food: f32,
    pub event: f32,
    pub slot: u32,
    pub generation: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct PerceptionGpu {
    pub resource_here: f32,
    pub local_count: f32,
    pub padding: [f32; 2],
    pub samples: [SampleGpu; 8],
    pub bodies: [BodyGpu; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DecisionGpu {
    pub scores: [f32; 7],
    pub selected_action: u32,
    pub movement: [f32; 2],
    pub attention: f32,
    pub amount: f32,
    pub payload: f32,
    pub target: u32,
    pub target_generation: u32,
    pub invalid: u32,
    pub hidden: [f32; HIDDEN],
    pub inputs: [f32; INPUTS],
}
impl Default for DecisionGpu {
    fn default() -> Self {
        Self::zeroed()
    }
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
    pub physical: [f32; 4],
    pub lifecycle: [u32; 4],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct SelectionParams {
    pub world_position: [f32; 2],
    pub radius: f32,
    pub padding: f32,
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
    pub selected: u32,
    pub padding: u32,
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimSettings {
    pub population: u32,
    pub resource_regeneration: f32,
    pub movement_energy_cost: f32,
    pub metabolic_cost: f32,
    /// Actuator sensitivity, not minimum effort or maximum body speed.
    /// Older checkpoint-12 worlds retain the historical response of one.
    #[serde(default = "legacy_motor_response_gain")]
    pub motor_response_gain: f32,
    pub consume_amount: f32,
    pub conversion_efficiency: f32,
    pub heterogeneity: f32,
    pub sensor_radius: f32,
    pub reproduction_cost: f32,
    pub maturity_age: f32,
    pub birth_cooldown: u32,
    pub force_enabled: bool,
    pub communication_enabled: bool,
    pub evolving_landscape: bool,
    pub founder_genomes: Vec<Vec<f32>>,
    pub founder_name: String,
}
impl Default for SimSettings {
    fn default() -> Self {
        Self {
            population: 1000,
            resource_regeneration: 0.01,
            movement_energy_cost: 0.01,
            metabolic_cost: 0.06,
            motor_response_gain: 4.0,
            consume_amount: 25.0,
            conversion_efficiency: 8.0,
            heterogeneity: 0.85,
            sensor_radius: 24.0,
            reproduction_cost: 50.0,
            maturity_age: 400.0,
            birth_cooldown: 240,
            force_enabled: true,
            communication_enabled: true,
            evolving_landscape: true,
            founder_genomes: crate::founders::bundled().genomes.clone(),
            founder_name: crate::founders::bundled().name.clone(),
        }
    }
}
impl SimSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.population > MAX_AGENTS
            || self.birth_cooldown > 1_000_000
            || [
                self.resource_regeneration,
                self.movement_energy_cost,
                self.metabolic_cost,
                self.motor_response_gain,
                self.consume_amount,
                self.conversion_efficiency,
                self.heterogeneity,
                self.sensor_radius,
                self.reproduction_cost,
                self.maturity_age,
            ]
            .iter()
            .any(|x| !x.is_finite() || *x < 0.0)
            || self.sensor_radius < 4.0
            || self.sensor_radius > 48.0
            || self.conversion_efficiency <= 0.0
            || self.reproduction_cost < 1.0
            || self.reproduction_cost > 100.0
            || self.consume_amount > 8000.0
            || self.resource_regeneration > 1.0
            || self.movement_energy_cost > 100.0
            || self.metabolic_cost > 100.0
            || !(0.1..=32.0).contains(&self.motor_response_gain)
            || self.conversion_efficiency < 0.000001
            || self.conversion_efficiency > 1000.0
            || self.heterogeneity > 1.0
            || self.maturity_age > 11000.0
        {
            return Err("Invalid recurrent-v1 physical settings".into());
        }
        crate::founders::validate_genomes(&self.founder_genomes)
    }
}
fn legacy_motor_response_gain() -> f32 {
    1.0
}

/// Declared mutable starting dispositions, NOT a runtime policy fallback.
pub fn bootstrap_genome() -> [f32; GENOME_SIZE] {
    let mut g = [0.0; GENOME_SIZE];
    for (h, input) in [0, 1, 2, 16, 19, 22, 25, 3].into_iter().enumerate() {
        g[h * RECURRENT_ROW + input] = 1.0;
    }
    let row = |o: usize| OUTPUT_BASE + o * (HIDDEN + 1);
    g[row(0) + HIDDEN] = -0.1;
    g[row(1) + 2] = 3.0;
    g[row(1) + 1] = -1.0;
    g[row(1) + HIDDEN] = 0.2;
    g[row(2)] = -3.0;
    g[row(2) + 1] = 1.0;
    g[row(2) + HIDDEN] = 1.7;
    for o in 3..6 {
        g[row(o) + HIDDEN] = -0.3;
    }
    g[row(6)] = 3.0;
    g[row(6) + 1] = 2.0;
    g[row(6) + HIDDEN] = -2.1;
    // Mutable local steering disposition; no runtime random destinations.
    g[row(7) + 4] = 0.5;
    g[row(7) + 6] = -0.5;
    g[row(8) + 5] = 0.5;
    g[row(8) + 3] = -0.5;
    g[row(10) + HIDDEN] = 3.0;
    g
}
