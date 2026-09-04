@group(0) @binding(0) var<storage, read_write> occupancy: array<atomic<u32>>;
const GRID: u32 = 256u;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x < GRID && id.y < GRID) { atomicStore(&occupancy[id.y * GRID + id.x], 0u); }
}
