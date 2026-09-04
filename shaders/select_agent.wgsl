struct SelectionParams { world_position: vec2<f32>, radius: f32, padding: f32, };
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<uniform> selection: SelectionParams;
@group(0) @binding(2) var<storage, read_write> key: atomic<u32>;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= INVALID) { return; }
  let agent = agents[index];
  if (agent.alive == 0u) { return; }
  let distance = length(agent.position - selection.world_position);
  if (distance <= selection.radius) {
    let quantized = min(u32(distance * 1000.0), 0x7fffu);
    atomicMin(&key, (quantized << 17u) | (index & 0x1ffffu));
  }
}
