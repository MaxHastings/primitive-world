//! primitive-world: fixed-frame sensing, chosen gathering, automatic digestion.
use bytemuck::{Pod, Zeroable};
/// Persistence accepts only this model's controller and lifetime-state layout.
pub const MODEL_ID: &str = "primitive-v29-composable-reservoir";
pub const FOUNDER_BANK_VERSION: u32 = 17;
pub const CHECKPOINT_VERSION: u32 = 42;
pub const CHECKPOINT_MAGIC: &[u8; 12] = b"PRIMWORLD042";
/// Initial body upkeep while a fresh world establishes its first life cycles.
pub const METABOLIC_START_COST: f32 = 0.01;
pub const DEFAULT_METABOLIC_RAMP_TICKS: u32 = 50_000;
/// Fixed rolling hereditary storage; independent of body-engine capacity.
pub const HEREDITARY_RESERVOIR_SIZE: u32 = 4_096;
/// Incremental maintenance paid for each expressed recurrent unit.
pub const DEFAULT_ACTIVE_UNIT_UPKEEP: f32 = 0.0005;
/// Energy paid for each unit of actual bounded memory-state change.
pub const DEFAULT_MEMORY_WRITE_ENERGY: f32 = 0.0001;
/// Blind per-birth connection mutation probability and bounded magnitude.
pub const BASE_MUTATION_PROBABILITY: f32 = 0.25;
pub const BASE_MUTATION_MAGNITUDE: f32 = 0.03;
pub const MAX_AGENTS: u32 = 16_384;
/// Reserve room for the largest permitted birth cooldown in shader tick arithmetic.
pub const MAX_WORLD_TICKS: u32 = u32::MAX - 1_000_001;
pub const LIVE_WORKGROUP_SIZE: usize = 8;
pub const RESOURCE_GRID: u32 = 512;
pub const OCCUPANCY_GRID: u32 = 256;
pub const SPATIAL_CELL_COUNT: u32 = OCCUPANCY_GRID * OCCUPANCY_GRID;
pub const WORLD_SIZE: f32 = 2048.0;
/// Cumulative physical and cognitive accounting counters.
pub const DEATH_STATS_COUNT: u32 = 37;
pub const EVENT_RING_SIZE: u32 = 65_536;
pub const SECTORS: usize = 8;
pub const SECTOR_NAMES: [&str; SECTORS] = ["E", "SE", "S", "SW", "W", "NW", "N", "NE"];
pub const REGIONS: usize = SECTORS * 2;
pub const NEIGHBOR_BASE: usize = 52;
pub const NEIGHBOR_INPUTS: usize = 7;
pub const INPUTS: usize = NEIGHBOR_BASE + SECTORS * NEIGHBOR_INPUTS;
/// The model has sixteen equivalent potential units.  The inherited active
/// mask, not this engineering ceiling, determines an organism's capacity.
pub const HIDDEN: usize = 16;
pub const OUTPUTS: usize = 20;
pub const FORCE_OUTPUT: usize = 18;
pub const NODE_BIAS: usize = 0;
pub const GATE_BIAS: usize = HIDDEN;
pub const OUTPUT_BIAS: usize = 2 * HIDDEN;
pub const INPUT_BASE: usize = OUTPUT_BIAS + OUTPUTS;
pub const RECURRENT_BASE: usize = INPUT_BASE + HIDDEN * INPUTS;
pub const GATE_BASE: usize = RECURRENT_BASE + HIDDEN * HIDDEN;
pub const OUTPUT_BASE: usize = GATE_BASE + HIDDEN * HIDDEN;
pub const GENOME_SIZE: usize = OUTPUT_BASE + OUTPUTS * HIDDEN;
/// Two equal parameter banks keep each dense full-population buffer below the
/// common 256 MiB WebGPU storage-binding limit without reducing body capacity.
pub const GENOME_BANK_COUNT: usize = 2;
pub const GENOME_BANK_STRIDE: usize = GENOME_SIZE.div_ceil(GENOME_BANK_COUNT);
/// All non-bias controller values.  Learned deltas are kept only for these
/// connections; biases remain inherited-only.
pub const CONNECTION_COUNT: usize = HIDDEN * INPUTS + 2 * HIDDEN * HIDDEN + OUTPUTS * HIDDEN;
pub const FAST_BANK_STRIDE: usize = CONNECTION_COUNT / 2;
pub const TRACE_COUNT: usize = INPUTS + HIDDEN + OUTPUTS;
pub const ACTIVE_MASK_ALL: u32 = (1u32 << HIDDEN) - 1;
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, serde::Serialize, serde::Deserialize)]
pub struct CognitiveTraits {
    pub active_mask: u32,
    pub padding: [u32; 3],
    pub plasticity_rate: [f32; HIDDEN],
    pub trace_retention: f32,
    pub learned_weight_retention: f32,
    /// Heritable, bounded multiplier for mutation magnitude/budget.  It is
    /// evolutionary variation, not a runtime cognitive mechanism.
    pub mutation_scale: f32,
}
impl CognitiveTraits {
    pub fn validate(&self) -> bool {
        self.active_mask != 0
            && self.active_mask & !ACTIVE_MASK_ALL == 0
            && self.trace_retention.is_finite()
            && self.learned_weight_retention.is_finite()
            && (0.0..=0.9999).contains(&self.trace_retention)
            && (0.0..=0.9999).contains(&self.learned_weight_retention)
            && self.mutation_scale.is_finite()
            && (0.25..=4.0).contains(&self.mutation_scale)
            && self
                .plasticity_rate
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 0.2)
    }
}
pub const ACTION_NAMES: [&str; 6] = ["none", "collect", "transfer", "force", "emit", "reproduce"];
pub const EMIT: u32 = 4;
/// Event-ring action code for a receiver decision made while a signal was visible.
pub const SIGNAL_OBSERVED: u32 = 6;
/// Event-ring action code for a matched nearby-body decision without a visible signal.
pub const SIGNAL_CONTROL: u32 = 7;
/// Event-ring action code for a sampled recurrent-memory diagnostic.
pub const MEMORY_SAMPLE: u32 = 8;
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
    /// Bit-encoded previous energy, inventory, and requested gathering effort.
    pub physical_previous: [u32; 3],
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
    /// Heritable topology.  A bit is set exactly when its recurrent unit is
    /// expressed; every inactive unit is semantically inert.
    pub active_mask: u32,
    /// One signed, inherited plasticity coefficient for each unit is stored in
    /// the organism rather than a separate cognitive subsystem.
    pub plasticity_rate: [f32; HIDDEN],
    /// Inherited organism-wide persistence for local activity traces.
    pub trace_retention: f32,
    /// Inherited organism-wide persistence for lifetime-only learned deltas.
    pub learned_weight_retention: f32,
    pub hidden: [f32; HIDDEN],
    /// Actual evaluated ticks, excluding the randomized initial biological age.
    pub lived_ticks: u32,
    pub cognitive_padding: u32,
    /// Heritable bounded multiplier for offspring variation.
    pub mutation_scale: f32,
}
impl Default for AgentGpu {
    fn default() -> Self {
        // CPU fixtures represent a fully expressed controller unless they are
        // specifically exercising topology.  GPU-created unused slots remain
        // zeroed and dead.
        Self {
            active_mask: ACTIVE_MASK_ALL,
            mutation_scale: 1.0,
            ..Self::zeroed()
        }
    }
}
impl AgentGpu {
    #[cfg(test)]
    pub fn cognitive_traits(&self) -> CognitiveTraits {
        CognitiveTraits {
            active_mask: self.active_mask,
            padding: [0; 3],
            plasticity_rate: self.plasticity_rate,
            trace_retention: self.trace_retention,
            learned_weight_retention: self.learned_weight_retention,
            mutation_scale: self.mutation_scale,
        }
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
    pub evaluated: u32,
    pub movement: [f32; 2],
    pub amount: f32,
    pub payload: f32,
    pub target: u32,
    pub target_generation: u32,
    pub invalid: u32,
    pub body_padding: u32,
    pub force: [f32; 2],
    /// Candidate and output activities are retained only for the immediately
    /// following local-plasticity pass; they are not inherited state.
    pub candidate: [f32; HIDDEN],
    pub hidden: [f32; HIDDEN],
    pub update_gates: [f32; HIDDEN],
    pub outputs: [f32; OUTPUTS],
    pub memory_write_cost: f32,
    // WGSL aligns vec3<u32> to sixteen bytes, so this explicit four-word
    // region preserves the CPU/GPU storage contract before inputs.
    pub decision_padding: [u32; 4],
    pub inputs: [f32; INPUTS],
    /// Explicitly matches WGSL's sixteen-byte `Decision` array stride.
    pub decision_end_padding: u32,
}
impl Default for DecisionGpu {
    fn default() -> Self {
        Self::zeroed()
    }
}
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug)]
pub struct SimParams {
    pub world_size: [f32; 4],
    pub resource_grid_size: u32,
    pub agent_count: u32,
    pub tick: u32,
    pub world_padding: u32,
    pub time_and_costs: [f32; 4],
    pub resource_and_noise: [f32; 4],
    pub sensor_and_padding: [f32; 4],
    pub physical: [f32; 4],
    pub lifecycle: [u32; 4],
    pub mutation: [f32; 4],
    /// Capped ecological pressure: extended mobility, fragmentation, seasons.
    pub environment: [f32; 4],
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
    /// WGSL aligns `Decision` to sixteen bytes after the eight-byte-aligned
    /// perception record.
    pub decision_alignment_padding: [u32; 2],
    pub decision: DecisionGpu,
    pub selected: u32,
    pub padding: u32,
    pub selection_padding: [u32; 2],
}
fn no_environment_rotation(rotation: &u32) -> bool {
    *rotation == 0
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SimSettings {
    /// Logical habitat width. Wallpaper mode sets this to the monitor width.
    #[serde(default = "default_habitat_width")]
    pub habitat_width: f32,
    /// Logical habitat height. Wallpaper mode sets this to the monitor height.
    #[serde(default = "default_habitat_height")]
    pub habitat_height: f32,
    /// Quarter turns of the full environment/initial positions, never a brain input.
    #[serde(default, skip_serializing_if = "no_environment_rotation")]
    pub environment_rotation: u32,
    /// 0 = uniform productivity, 1 = full patch/gap contrast. No controller input.
    pub habitat_contrast: f32,
    pub population: u32,
    pub resource_regeneration: f32,
    pub movement_energy_cost: f32,
    /// Metabolism reached after the deterministic world-start ramp.
    pub metabolic_cost: f32,
    /// Missing in flat-metabolism format-42 checkpoints; zero preserves their
    /// original physics instead of silently turning them into a hybrid ramp.
    #[serde(default)]
    pub metabolic_ramp_ticks: u32,
    pub active_unit_upkeep: f32,
    pub memory_write_energy: f32,
    /// Actuator sensitivity, not minimum effort or maximum body speed.
    pub motor_response_gain: f32,
    pub consume_amount: f32,
    pub conversion_efficiency: f32,
    pub heterogeneity: f32,
    pub sensor_radius: f32,
    pub reproduction_cost: f32,
    pub maturity_age: f32,
    pub birth_cooldown: u32,
    /// Enables transfer, force, and signalling as selectable controller actions.
    /// They are enabled in the ordinary world; this is an experiment control,
    /// never a behavior rule.
    pub social_actions_enabled: bool,
    pub force_enabled: bool,
    pub communication_enabled: bool,
    pub evolving_landscape: bool,
    pub founder_genomes: Vec<Vec<f32>>,
    /// Topology and plasticity are inherited alongside every founder genome.
    #[serde(default)]
    pub founder_traits: Vec<CognitiveTraits>,
    pub founder_name: String,
}
impl Default for SimSettings {
    fn default() -> Self {
        Self {
            habitat_width: WORLD_SIZE,
            habitat_height: WORLD_SIZE,
            environment_rotation: 0,
            habitat_contrast: 1.0,
            population: 1000,
            resource_regeneration: 0.01,
            movement_energy_cost: 0.01,
            metabolic_cost: 0.06,
            metabolic_ramp_ticks: DEFAULT_METABOLIC_RAMP_TICKS,
            active_unit_upkeep: DEFAULT_ACTIVE_UNIT_UPKEEP,
            memory_write_energy: DEFAULT_MEMORY_WRITE_ENERGY,
            motor_response_gain: 4.0,
            consume_amount: 25.0,
            conversion_efficiency: 8.0,
            heterogeneity: 0.85,
            sensor_radius: 24.0,
            reproduction_cost: 50.0,
            maturity_age: 400.0,
            birth_cooldown: 240,
            social_actions_enabled: true,
            force_enabled: true,
            communication_enabled: true,
            evolving_landscape: true,
            founder_genomes: Vec::new(),
            founder_traits: Vec::new(),
            founder_name: "primitive-world-random".into(),
        }
    }
}
impl SimSettings {
    pub fn validate(&self) -> Result<(), String> {
        if self.environment_rotation > 3
            || !self.habitat_width.is_finite()
            || !self.habitat_height.is_finite()
            || !(256.0..=32768.0).contains(&self.habitat_width)
            || !(256.0..=32768.0).contains(&self.habitat_height)
            || self.population > MAX_AGENTS
            || self.birth_cooldown > 1_000_000
            || [
                self.habitat_contrast,
                self.resource_regeneration,
                self.movement_energy_cost,
                self.metabolic_cost,
                self.active_unit_upkeep,
                self.memory_write_energy,
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
            || self.habitat_contrast > 1.0
            || self.heterogeneity > 1.0
            || self.maturity_age > 11000.0
        {
            return Err("Invalid primitive-world physical settings".into());
        }
        crate::founders::validate_genomes(&self.founder_genomes)?;
        if !self.founder_traits.is_empty()
            && (self.founder_traits.len() != self.founder_genomes.len()
                || self.founder_traits.iter().any(|t| !t.validate()))
        {
            return Err("Invalid founder cognitive traits".into());
        }
        Ok(())
    }
}

fn default_habitat_width() -> f32 {
    WORLD_SIZE
}

fn default_habitat_height() -> f32 {
    WORLD_SIZE
}

/// Fixed recurrent brains with random inherited parameters.
pub fn random_genome(rng: &mut u32) -> [f32; GENOME_SIZE] {
    crate::brain::random_genome(rng)
}
