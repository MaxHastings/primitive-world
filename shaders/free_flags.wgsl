struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> flags: array<u32>;
@group(0) @binding(2) var<uniform> params: SimParams;
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= 100000u) { return; }
  flags[id.x] = u32(id.x < params.agent_count && agents[id.x].alive == 0u);
}
