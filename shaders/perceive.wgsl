struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};
struct Perception {
  resource_here: f32,
  resource_north: f32,
  resource_east: f32,
  resource_south: f32,
  resource_west: f32,
  local_density: f32,
  padding: u32,
  gradient: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> resources: array<u32>;
@group(0) @binding(2) var<storage, read> occupancy: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> perceptions: array<Perception>;
@group(0) @binding(4) var<uniform> params: SimParams;

const RESOURCE_GRID: u32 = 512u;
const AGENT_GRID: u32 = 256u;
const RESOURCE_SCALE: f32 = 1000.0;

fn resource_at(cell: vec2<i32>) -> f32 {
  let x = clamp(cell.x, 0, i32(RESOURCE_GRID - 1u));
  let y = clamp(cell.y, 0, i32(RESOURCE_GRID - 1u));
  return f32(resources[u32(y) * RESOURCE_GRID + u32(x)]) / RESOURCE_SCALE;
}

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.agent_count) { return; }
  let agent = agents[index];
  if (agent.alive == 0u) {
    perceptions[index] = Perception(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0u, vec2<f32>(0.0));
    return;
  }
  let base = clamp(agent.position / params.world_size * f32(RESOURCE_GRID), vec2<f32>(vec2(0.0)), vec2<f32>(f32(RESOURCE_GRID - 1u)));
  let cell = vec2<i32>(base);
  let here = resource_at(cell);
  let sensor_cells = max(1, i32(params.sensor_and_padding.x / 4.0));
  let north = resource_at(cell + vec2<i32>(0, -sensor_cells));
  let east = resource_at(cell + vec2<i32>(sensor_cells, 0));
  let south = resource_at(cell + vec2<i32>(0, sensor_cells));
  let west = resource_at(cell + vec2<i32>(-sensor_cells, 0));
  let density_cell = clamp(agent.position / params.world_size * f32(AGENT_GRID), vec2<f32>(vec2(0.0)), vec2<f32>(f32(AGENT_GRID - 1u)));
  let density_index = u32(density_cell.y) * AGENT_GRID + u32(density_cell.x);
  let count = atomicLoad(&occupancy[density_index]);
  perceptions[index] = Perception(here, north, east, south, west, min(f32(count) / 24.0, 1.0), 0u, vec2<f32>(east - west, south - north));
}
