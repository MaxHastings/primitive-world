//! primitive-world: fixed-frame sensing, chosen gathering, automatic digestion.
use bytemuck::{Pod, Zeroable};
pub const MODEL_ID: &str = "primitive-v6-variable-brain";
pub const FOUNDER_BANK_VERSION: u32 = 7;
pub const CHECKPOINT_VERSION: u32 = 20;
pub const CHECKPOINT_MAGIC: &[u8; 12] = b"PRIMWORLD020";
pub const MAX_AGENTS: u32 = 16_384;
pub const RESOURCE_GRID: u32 = 512;
pub const OCCUPANCY_GRID: u32 = 256;
pub const SPATIAL_CELL_COUNT: u32 = OCCUPANCY_GRID * OCCUPANCY_GRID;
pub const WORLD_SIZE: f32 = 2048.0;
pub const DEATH_STATS_COUNT: u32 = 32;
pub const EVENT_RING_SIZE: u32 = 65_536;
pub const SECTORS: usize = 8;
pub const SECTOR_NAMES: [&str; SECTORS] = ["E", "SE", "S", "SW", "W", "NW", "N", "NE"];
pub const REGIONS: usize = SECTORS * 2;
pub const NEIGHBOR_BASE: usize = 52;
pub const NEIGHBOR_INPUTS: usize = 7;
pub const INPUTS: usize = NEIGHBOR_BASE + SECTORS * NEIGHBOR_INPUTS;
/// GPU capacity, not inherited dormant structure. Only encoded nodes/edges exist.
pub const HIDDEN: usize = 64;
pub const DEFAULT_NODES: usize = 4;
pub const OUTPUTS: usize = 20;
pub const FORCE_OUTPUT: usize = 18;
pub const MAX_EDGES: usize = 512;
pub const NODE_BIAS: usize = 2;
pub const GATE_BIAS: usize = NODE_BIAS + HIDDEN;
pub const OUTPUT_BIAS: usize = GATE_BIAS + HIDDEN;
pub const EDGE_BASE: usize = OUTPUT_BIAS + OUTPUTS;
pub const GENOME_SIZE: usize = EDGE_BASE + MAX_EDGES * 3;
pub const ACTION_NAMES: [&str; 6] = ["none", "collect", "transfer", "force", "emit", "reproduce"];
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
    pub body_padding: f32,
    pub rng: u32,
    pub generation: u32,
    pub next_birth: u32,
    pub max_age: f32,
    pub signal_payload: f32,
    /// One-based tick of emission; zero means never emitted.
    pub signal_tick: u32,
    pub signal_padding: [u32; 3],
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
    /// Founder genome slot; observer bookkeeping, never a cognitive input.
    pub founder_family: u32,
    pub hidden: [f32; HIDDEN],
    pub brain_nodes: u32,
    pub brain_edges: u32,
    /// Birth-time difference from the parent; zero for fresh founders.
    pub node_change: i32,
    pub edge_change: i32,
    pub life: crate::life_record::LifeRecord,
}
impl Default for AgentGpu {
    fn default() -> Self {
        Self::zeroed()
    }
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct RegionGpu {
    pub food: f32,
    pub bodies: f32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct BodyGpu {
    pub offset: [f32; 2],
    pub velocity: [f32; 2],
    pub signal_present: f32,
    pub signal: f32,
    pub slot: u32,
    pub generation: u32,
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, Default)]
pub struct PerceptionGpu {
    pub resource_here: f32,
    pub nearby_count: f32,
    pub padding: [f32; 2],
    pub regions: [RegionGpu; REGIONS],
    pub bodies: [BodyGpu; SECTORS],
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct DecisionGpu {
    pub scores: [f32; 6],
    pub selected_action: u32,
    pub score_padding: u32,
    pub movement: [f32; 2],
    pub amount: f32,
    pub payload: f32,
    pub target: u32,
    pub target_generation: u32,
    pub invalid: u32,
    pub body_padding: u32,
    pub force: [f32; 2],
    pub brain_nodes: u32,
    pub brain_edges: u32,
    pub hidden: [f32; HIDDEN],
    pub update_gates: [f32; HIDDEN],
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
    pub mutation: [f32; 4],
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
fn no_environment_rotation(rotation: &u32) -> bool {
    *rotation == 0
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimSettings {
    /// Quarter turns of the full environment/initial positions, never a brain input.
    #[serde(default, skip_serializing_if = "no_environment_rotation")]
    pub environment_rotation: u32,
    /// 0 = uniform productivity, 1 = full patch/gap contrast. No controller input.
    pub habitat_contrast: f32,
    pub population: u32,
    pub resource_regeneration: f32,
    pub movement_energy_cost: f32,
    pub metabolic_cost: f32,
    /// Actuator sensitivity, not minimum effort or maximum body speed.
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
    /// Energy charged each tick for one expressed computational unit.
    pub brain_node_cost: f32,
    /// Every encoded connection costs upkeep, including zero-weight connections.
    pub brain_edge_cost: f32,
    /// Energy per logical genome value (counts, biases, edge triples), not padding.
    pub genome_copy_cost: f32,
    pub mutation_probability: f32,
    pub mutation_magnitude: f32,
    /// Probability for each of duplication and deletion at a birth.
    pub node_mutation_rate: f32,
    /// Probability for each of connection insertion and deletion at a birth.
    pub edge_mutation_rate: f32,
    pub founder_genomes: Vec<Vec<f32>>,
    pub founder_name: String,
    /// Explicit genome index for each initial body; empty uses the ordinary bank cycle.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub founder_slots: Vec<u32>,
}
impl Default for SimSettings {
    fn default() -> Self {
        Self {
            environment_rotation: 0,
            habitat_contrast: 1.0,
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
            brain_node_cost: default_brain_node_cost(),
            brain_edge_cost: default_brain_edge_cost(),
            genome_copy_cost: default_genome_copy_cost(),
            mutation_probability: 0.02,
            mutation_magnitude: 0.03,
            node_mutation_rate: 0.04,
            edge_mutation_rate: 0.08,
            founder_genomes: crate::founders::bundled().genomes.clone(),
            founder_name: crate::founders::bundled().name.clone(),
            founder_slots: vec![],
        }
    }
}
impl SimSettings {
    pub fn founder_index(&self, slot: usize) -> usize {
        self.founder_slots
            .get(slot)
            .map_or_else(|| slot % self.founder_genomes.len().max(1), |&i| i as usize)
    }
    pub fn validate(&self) -> Result<(), String> {
        if (!self.founder_slots.is_empty()
            && (self.founder_slots.len() != self.population as usize
                || self
                    .founder_slots
                    .iter()
                    .any(|&i| i as usize >= self.founder_genomes.len())))
            || self.environment_rotation > 3
            || self.population > MAX_AGENTS
            || self.birth_cooldown > 1_000_000
            || [
                self.habitat_contrast,
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
                self.brain_node_cost,
                self.brain_edge_cost,
                self.genome_copy_cost,
                self.mutation_probability,
                self.mutation_magnitude,
                self.node_mutation_rate,
                self.edge_mutation_rate,
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
            || self.habitat_contrast > 1.0
            || self.heterogeneity > 1.0
            || self.maturity_age > 11000.0
            || self.brain_node_cost > 10.0
            || self.brain_edge_cost > 10.0
            || self.genome_copy_cost > 10.0
            || self.mutation_probability > 1.0
            || self.mutation_magnitude > 8.0
            || 2.0 * (self.node_mutation_rate + self.edge_mutation_rate) > 1.0
        {
            return Err("Invalid primitive-world physical settings".into());
        }
        crate::founders::validate_genomes(&self.founder_genomes)
    }
}
/// Four unlabelled units with sparse, exchangeable random sensory connections.
pub fn random_genome(rng: &mut u32) -> [f32; GENOME_SIZE] {
    crate::brain::random_genome(rng)
}

fn default_brain_node_cost() -> f32 {
    0.001
}

fn default_brain_edge_cost() -> f32 {
    0.00002
}

fn default_genome_copy_cost() -> f32 {
    0.001
}
