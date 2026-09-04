@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> occupancy: array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params: SimParams;
const GRID: u32 = 256u;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let index = id.x;
  if (index >= params.agent_count || agents[index].alive == 0u) { return; }
  let cell = clamp(agents[index].position / params.world_size * f32(GRID), vec2<f32>(vec2(0.0)), vec2<f32>(f32(GRID - 1u)));
  atomicAdd(&occupancy[u32(cell.y) * GRID + u32(cell.x)], 1u);
}
