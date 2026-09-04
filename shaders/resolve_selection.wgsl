struct Agent {
  position: vec2<f32>, velocity: vec2<f32>, energy: f32, age: f32, max_speed: f32, sensor_radius: f32,
  exploration: f32, resource_attraction: f32, persistence: f32, risk: f32, rng: u32, alive: u32,
};
struct Perception {
  resource_here: f32, resource_north: f32, resource_east: f32, resource_south: f32,
  resource_west: f32, local_density: f32, padding: u32, gradient: vec2<f32>,
};
struct Decision { scores: array<f32, 5>, selected_action: u32, padding: vec2<u32>, };
struct SelectionOutput {
  agent: Agent,
  perception: Perception,
  decision: Decision,
  selected: u32,
  padding: array<u32, 5>,
};
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> decisions: array<Decision>;
@group(0) @binding(3) var<storage, read> key: atomic<u32>;
@group(0) @binding(4) var<storage, read_write> output: SelectionOutput;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let packed = atomicLoad(&key);
  if (packed != 0xffffffffu && (packed & 0x1ffffu) == id.x) {
    output.agent = agents[id.x];
    output.perception = perceptions[id.x];
    output.decision = decisions[id.x];
    output.selected = id.x + 1u;
  }
}
