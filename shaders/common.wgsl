// Shared Rust/WGSL storage contract. Keep sizes covered by simulation tests.
struct Place { position: vec2<f32>, food: f32, observed: u32, source_id: u32, source_generation: u32, confidence: f32, padding: u32, };
struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  food: f32, action: u32, target_id: u32, commit_until: u32,
  goal: vec2<f32>, rng: u32, alive: u32,
  generation: u32, event_actor: u32, event_generation: u32, event_tick: u32,
  event_amount: f32, goal_score: f32, next_birth: u32, max_age: f32, last_communication: u32, guide_id: u32, guide_generation: u32, guide_started: u32, guide_expected: f32, guide_result: f32, guide_position: vec2<f32>, places: array<Place, 4>,
};
struct Perception {
  resource_here: f32, resource_north: f32, resource_east: f32, resource_south: f32,
  resource_west: f32, local_density: f32, local_count: f32, projected_food: f32,
  competition_pressure: f32, padding: u32, gradient: vec2<f32>, crowd: array<f32, 4>,
};
struct SocialPerception {
  avoidance: vec2<f32>, known_strength: f32, danger: f32,
  give_target: u32, force_target: u32, give_value: f32, force_value: f32, report_target: u32, report_place: u32,
  companion_position: vec2<f32>, companion_velocity: vec2<f32>, companion_value: f32, report_value: f32,
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
};
const INVALID: u32 = 100000u;
const FOOD_CAPACITY: f32 = 8.0;
const INTERACTION_RADIUS: f32 = 6.0;
const WAIT: u32 = 0u;
const MOVE: u32 = 1u;
const HARVEST: u32 = 2u;
const EAT: u32 = 3u;
const GIVE: u32 = 4u;
const FORCE: u32 = 5u;
const COMMUNICATE: u32 = 6u;
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
