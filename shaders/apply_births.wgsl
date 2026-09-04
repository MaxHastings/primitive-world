struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> free_indices: array<u32>;
@group(0) @binding(2) var<storage, read> free_prefix: array<u32>;
@group(0) @binding(3) var<storage, read> birth_parents: array<u32>;
@group(0) @binding(4) var<storage, read> birth_prefix: array<u32>;
@group(0) @binding(5) var<uniform> params: SimParams;
@group(0) @binding(6) var<storage, read_write> stats: array<atomic<u32>>;

const MAX_AGENTS: u32 = 100000u;
const OFFSPRING_ENERGY: f32 = 40.0;

fn hash_u32(input: u32) -> u32 {
  var value = input;
  value = (value ^ 61u) ^ (value >> 16u); value = value + (value << 3u);
  value = value ^ (value >> 4u); value = value * 0x27d4eb2du; return value ^ (value >> 15u);
}

fn mutate(value: f32, amount: f32, seed: u32, low: f32, high: f32) -> f32 {
  let noise = f32(hash_u32(seed) & 65535u) / 32767.5 - 1.0;
  return clamp(value + noise * amount, low, high);
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let birth_total = birth_prefix[MAX_AGENTS - 1u];
  let free_total = free_prefix[MAX_AGENTS - 1u];
  let birth_index = id.x;
  if (birth_index >= min(birth_total, free_total) || birth_index >= MAX_AGENTS) { return; }
  let parent_index = birth_parents[birth_index];
  let parent = agents[parent_index];
  let child_slot = free_indices[birth_index];
  if (parent.alive == 0u || agents[child_slot].alive != 0u) { return; }
  if (parent.energy < params.sensor_and_padding.z) { return; }
  agents[parent_index].energy = max(0.0, parent.energy - params.sensor_and_padding.w);
  let seed = parent.rng ^ parent_index ^ params.tick;
  let angle = f32(hash_u32(seed) & 65535u) / 65535.0 * 6.2831853;
  let offset = vec2<f32>(cos(angle), sin(angle)) * 2.0;
  agents[child_slot] = Agent(
    clamp(parent.position + offset, vec2<f32>(0.0), vec2<f32>(params.world_size)),
    vec2<f32>(cos(angle), sin(angle)),
    OFFSPRING_ENERGY,
    0.0,
    mutate(parent.max_speed, 0.10, seed ^ 1u, 0.35, 2.2),
    mutate(parent.sensor_radius, 0.8, seed ^ 2u, 2.0, 40.0),
    mutate(parent.exploration, 0.08, seed ^ 3u, 0.0, 1.5),
    mutate(parent.resource_attraction, 0.10, seed ^ 4u, 0.1, 2.5),
    mutate(parent.persistence, 0.08, seed ^ 5u, 0.0, 1.0),
    mutate(parent.risk, 0.08, seed ^ 6u, 0.0, 1.0),
    hash_u32(seed ^ 7u),
    1u,
  );
  atomicAdd(&stats[3], 1u);
}
