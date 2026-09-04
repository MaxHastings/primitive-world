struct InterventionParams {
  center: vec2<f32>,
  radius: f32,
  delta: f32,
};
struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};

@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<uniform> intervention: InterventionParams;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= 100000u) { return; }
  if (length(agents[id.x].position - intervention.center) <= intervention.radius) {
    agents[id.x].alive = 0u;
    agents[id.x].energy = 0.0;
  }
}
