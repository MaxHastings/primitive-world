// Shared Rust/WGSL storage contract. Keep sizes covered by simulation tests.
struct Place { position: vec2<f32>, food: f32, observed: u32, source_id: u32, source_generation: u32, confidence: f32, padding: u32, };
struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  food: f32, action: u32, target_id: u32, commit_until: u32,
  goal: vec2<f32>, rng: u32, alive: u32,
  generation: u32, event_actor: u32, event_generation: u32, event_tick: u32,
  event_amount: f32, goal_score: f32, next_birth: u32, max_age: f32, last_communication: u32, guide_id: u32, guide_generation: u32, guide_started: u32, guide_expected: f32, guide_result: f32, guide_position: vec2<f32>, places: array<Place, 16>, genome: array<f32, 128>, lineage_id: u32, parent_lineage: u32, birth_tick: u32, birth_parent_slot: u32,
  ancestry_depth: u32, lifetime_births: u32, distance_travelled: f32, observer_padding: u32,
};
struct Perception {
  resource_here: f32, resource_north: f32, resource_east: f32, resource_south: f32,
  resource_west: f32, local_density: f32, local_count: f32, projected_food: f32,
  competition_pressure: f32, padding: u32, gradient: vec2<f32>, crowd: array<f32, 4>,
};
struct SocialCandidate {
  target_slot: u32, target_generation: u32, position: vec2<f32>, velocity: vec2<f32>,
  distance: f32, food: f32, familiarity: f32, benefit: f32, harm: f32, navigation: f32,
  event_actor: u32, event_generation: u32, event_tick: u32, event_amount: f32,
  last_report_tick: u32, last_report_observed: u32,
};
struct SocialPerception {
  avoidance: vec2<f32>, known_strength: f32, danger: f32,
  give_target: u32, force_target: u32, give_value: f32, force_value: f32, report_target: u32, report_place: u32,
  companion_position: vec2<f32>, companion_velocity: vec2<f32>, companion_value: f32, report_value: f32,
  candidates: array<SocialCandidate, 8>,
};
struct Relation {
  target_slot: u32, target_generation: u32, familiarity: f32, benefit: f32,
  harm: f32, last_seen_tick: u32, benefit_evidence: f32, harm_evidence: f32, navigation: f32, navigation_evidence: f32,
  last_report_tick: u32, last_report_observed: u32,
};
struct Decision {
  scores: array<f32, 7>, selected_action: u32, target_id: u32, target_padding: u32,
  goal: vec2<f32>, amount: f32, padding: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>, social_weights: vec4<f32>, lifecycle: vec4<u32>,
  neural_config: vec4<u32>,
};
const NEURAL_OBSERVATIONS:u32=24u;
const NEURAL_HIDDEN:u32=32u;
const NEURAL_ACTIONS:u32=14u;
struct NeuralWeights {
 input:array<f32,2304>, recurrent:array<f32,3072>, input_bias:array<f32,96>, recurrent_bias:array<f32,96>, output:array<f32,448>, output_bias:array<f32,14>,
};
struct NeuralState {
 generation:u32, choice:u32, tick:u32, valid:u32,
 hidden:array<f32,32>, before:array<f32,32>, after:array<f32,32>, observation:array<f32,24>,
 mask:array<f32,14>, logits:array<f32,14>, probabilities:array<f32,14>, energy:f32, food:f32,
};
const INVALID: u32 = 100000u;
const FOOD_CAPACITY: f32 = 8.0;
const INTERACTION_RADIUS: f32 = 6.0;
const WAIT: u32 = 0u;
const MOVE: u32 = 1u;
// The decision buffer is an agent-owned action intent. These names describe
// physical affordances; social labels remain observer interpretations.
const COLLECT: u32 = 2u;
const INGEST: u32 = 3u;
const TRANSFER: u32 = 4u;
const APPLY_FORCE: u32 = 5u;
const EMIT: u32 = 6u;
// Temporary aliases keep the legacy experiment and saved traces readable
// while the generic action contract replaces semantic social actions.
const HARVEST: u32 = COLLECT;
const EAT: u32 = INGEST;
const GIVE: u32 = TRANSFER;
const FORCE: u32 = APPLY_FORCE;
const COMMUNICATE: u32 = EMIT;
fn unit_vector(v: vec2<f32>) -> vec2<f32> { return v / max(length(v), 0.0001); }
fn hash_u32(input: u32) -> u32 {
  var v = input;
  v = (v ^ 61u) ^ (v >> 16u); v = v + (v << 3u);
  v = v ^ (v >> 4u); v = v * 0x27d4eb2du; return v ^ (v >> 15u);
}
fn random01(seed: u32) -> f32 { return f32(hash_u32(seed) & 65535u) / 65535.0; }

struct Ground {
  dropped: atomic<u32>, extracted: atomic<u32>, remainder: f32, produced: u32,
  weather_loss: u32, collected: u32, habitat: f32, productivity: f32,
};
fn ground_index(position: vec2<f32>) -> u32 {
  let c=vec2<u32>(clamp(position/4.0,vec2<f32>(0.0),vec2<f32>(511.0)));
  return c.y*512u+c.x;
}
// Diagnostic-only sensing treatments (flags 8/16). No extra food reads, RNG,
// distant knowledge, or changed storage ABI. Channels are sample slots rather
// than compass labels in sweep mode. All consumers use this exact geometry.
fn food_sample_offset(slot: u32, radius: f32, tick: u32, flags: u32) -> vec2<f32> {
  if (slot==0u) { return vec2<f32>(0.0); }
  var dirs=array<vec2<f32>,4>(vec2<f32>(0,-1),vec2<f32>(1,0),vec2<f32>(0,1),vec2<f32>(-1,0));
  var direction=dirs[slot-1u];
  var range=radius;
  if ((flags&8u)!=0u) { range=radius/6.0; }
  if ((flags&16u)!=0u) {
    let phase=tick%6u;
    var ranges=array<f32,3>(radius/6.0,radius/2.0,radius);
    range=ranges[phase%3u];
    if (phase>=3u) {
      direction=vec2<f32>(direction.x-direction.y,direction.x+direction.y)*0.7071067811865476;
    }
  }
  return direction*range;
}
