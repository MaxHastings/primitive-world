struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct Decision { scores: array<f32, 5>, selected_action: u32, padding: vec2<u32>, };
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};

@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> decisions: array<Decision>;
@group(0) @binding(2) var<storage, read_write> resources: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> requests: array<u32>;
@group(0) @binding(4) var<uniform> params: SimParams;
@group(0) @binding(5) var<storage, read_write> stats: array<atomic<u32>>;

const GRID: u32 = 512u;
const RESOURCE_SCALE: u32 = 1000u;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.agent_count || agents[index].alive == 0u) { requests[index] = 0u; return; }
  let position = clamp(agents[index].position / params.world_size * f32(GRID), vec2<f32>(vec2(0.0)), vec2<f32>(f32(GRID - 1u)));
  let resource_index = u32(position.y) * GRID + u32(position.x);
  let requested = min(u32(params.resource_and_noise.x), RESOURCE_SCALE);
  var taken = 0u;
  for (var attempt = 0u; attempt < 12u; attempt = attempt + 1u) {
    let old_value = atomicLoad(&resources[resource_index]);
    if (old_value == 0u) { break; }
    let amount = min(old_value, requested);
    let result = atomicCompareExchangeWeak(&resources[resource_index], old_value, old_value - amount);
    if (result.exchanged) { taken = amount; break; }
  }
  requests[index] = taken;
  if (taken > 0u) { atomicAdd(&stats[0], taken); }
}
