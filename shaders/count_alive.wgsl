@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> alive_count: atomic<u32>;
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x < 100000u && agents[id.x].alive != 0u) { atomicAdd(&alive_count, 1u); }
}
