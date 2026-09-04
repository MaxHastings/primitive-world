struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};
struct Perception {
  resource_here: f32, resource_north: f32, resource_east: f32, resource_south: f32,
  resource_west: f32, local_density: f32, padding: u32, gradient: vec2<f32>,
};
struct Decision { scores: array<f32, 5>, selected_action: u32, padding: vec2<u32>, };

@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read_write> decisions: array<Decision>;
@group(0) @binding(3) var<uniform> params: SimParams;

fn hash_u32(input: u32) -> u32 {
  var value = input;
  value = (value ^ 61u) ^ (value >> 16u); value = value + (value << 3u);
  value = value ^ (value >> 4u); value = value * 0x27d4eb2du; return value ^ (value >> 15u);
}
fn random_signed(seed: u32) -> f32 { return f32(hash_u32(seed) & 65535u) / 32767.5 - 1.0; }

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.agent_count) { return; }
  let agent = agents[index];
  let p = perceptions[index];
  if (agent.alive == 0u) {
    decisions[index] = Decision(array<f32, 5>(-1000.0, -1000.0, -1000.0, -1000.0, -1000.0), 0u, vec2<u32>(0u));
    return;
  }
  let hunger = 1.0 - clamp(agent.energy / 100.0, 0.0, 1.0);
  let previous = normalize(agent.velocity);
  let values = array<f32, 5>(p.resource_here, p.resource_north, p.resource_east, p.resource_south, p.resource_west);
  let dirs = array<vec2<f32>, 5>(vec2<f32>(0.0), vec2<f32>(0.0, -1.0), vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), vec2<f32>(-1.0, 0.0));
  var scores: array<f32, 5>;
  var best = 0u;
  var best_score = -1000000.0;
  for (var action = 0u; action < 5u; action = action + 1u) {
    let movement = select(1.0, 0.0, action == 0u);
    let persistence = dot(dirs[action], previous) * agent.persistence * movement;
    let cost = movement * params.time_and_costs.z * (0.5 + hunger);
    let exploration = random_signed(agent.rng ^ params.tick ^ action * 0x9e3779b9u) * agent.exploration * params.resource_and_noise.w;
    let density_penalty = p.local_density * agent.risk * movement * 0.18;
    // Compare directions with the resource underfoot. Absolute resource levels make
    // every direction look equally attractive inside a rich patch, leaving noise and
    // momentum to dominate the choice. Remaining still receives a local food reward;
    // movement receives only the directional improvement over the current cell.
    let resource_signal = select(values[action] - p.resource_here, p.resource_here * 0.25, action == 0u);
    let resource_score = resource_signal * agent.resource_attraction * (0.7 + 1.3 * hunger);
    let score = resource_score + persistence - cost - density_penalty * (1.0 + 1.5 * agent.risk) + exploration;
    scores[action] = score;
    if (score > best_score) { best_score = score; best = action; }
  }
  decisions[index] = Decision(scores, best, vec2<u32>(0u));
}
