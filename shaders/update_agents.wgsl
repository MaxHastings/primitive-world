struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct Perception {
  resource_here: f32, resource_north: f32, resource_east: f32, resource_south: f32,
  resource_west: f32, local_density: f32, padding: u32, gradient: vec2<f32>,
};
struct Decision { scores: array<f32, 5>, selected_action: u32, padding: vec2<u32>, };
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> source_agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> decisions: array<Decision>;
@group(0) @binding(3) var<storage, read> requests: array<u32>;
@group(0) @binding(4) var<storage, read_write> destination_agents: array<Agent>;
@group(0) @binding(5) var<uniform> params: SimParams;
@group(0) @binding(6) var<storage, read_write> birth_flags: array<u32>;
@group(0) @binding(7) var<storage, read_write> stats: array<atomic<u32>>;

const MAX_AGE: f32 = 10000.0;

fn action_direction(action: u32) -> vec2<f32> {
  if (action == 1u) { return vec2<f32>(0.0, -1.0); }
  if (action == 2u) { return vec2<f32>(1.0, 0.0); }
  if (action == 3u) { return vec2<f32>(0.0, 1.0); }
  if (action == 4u) { return vec2<f32>(-1.0, 0.0); }
  return vec2<f32>(0.0);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= 100000u) { return; }
  let old = source_agents[index];
  if (index >= params.agent_count) {
    birth_flags[index] = 0u;
    destination_agents[index] = Agent(old.position, old.velocity, 0.0, old.age, old.max_speed, old.sensor_radius, old.exploration, old.resource_attraction, old.persistence, old.risk, old.rng, 0u);
    return;
  }
  if (old.alive == 0u) { birth_flags[index] = 0u; destination_agents[index] = old; return; }
  let decision = decisions[index];
  let direction = action_direction(decision.selected_action);
  var velocity = mix(old.velocity, direction, 0.24 + 0.52 * old.persistence);
  if (length(velocity) > 0.001) { velocity = normalize(velocity); }
  let juvenile_factor = 0.45 + 0.55 * clamp(old.age / max(params.sensor_and_padding.y, 1.0), 0.0, 1.0);
  let displacement = velocity * old.max_speed * juvenile_factor * params.time_and_costs.x;
  var position = old.position + displacement;
  if (position.x < 0.0 || position.x > params.world_size) { velocity.x = -velocity.x; position.x = clamp(position.x, 0.0, params.world_size); }
  if (position.y < 0.0 || position.y > params.world_size) { velocity.y = -velocity.y; position.y = clamp(position.y, 0.0, params.world_size); }
  let movement_cost = length(displacement) * params.time_and_costs.z;
  let energy = old.energy + f32(requests[index]) / 1000.0 * params.resource_and_noise.y - movement_cost - params.time_and_costs.w;
  let age = old.age + params.time_and_costs.x;
  let age_roll = (old.rng ^ u32(age) ^ params.tick) & 1023u;
  let age_dead = age >= MAX_AGE && age_roll < 1u;
  let alive = select(0u, 1u, energy > 0.0 && !age_dead);
  if (alive == 0u) {
    if (age_dead) { atomicAdd(&stats[2], 1u); }
    else { atomicAdd(&stats[1], 1u); }
  }
  let birth_roll = (old.rng ^ u32(old.age) ^ params.tick) & 1023u;
  let energy_surplus = max(0.0, energy - params.sensor_and_padding.z);
  let birth_window = min(32u, 6u + u32(energy_surplus * 2.0));
  birth_flags[index] = u32(alive != 0u && age >= params.sensor_and_padding.y && age < MAX_AGE && energy >= params.sensor_and_padding.z && birth_roll < birth_window);
  destination_agents[index] = Agent(position, velocity, max(0.0, energy), age, old.max_speed, old.sensor_radius, old.exploration, old.resource_attraction, old.persistence, old.risk, old.rng * 1664525u + 1013904223u, alive);
}
