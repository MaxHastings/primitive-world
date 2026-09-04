@group(0) @binding(0) var<storage, read> flags: array<u32>;
@group(0) @binding(1) var<storage, read> prefix: array<u32>;
@group(0) @binding(2) var<storage, read_write> indices: array<u32>;
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= 100000u || flags[id.x] == 0u) { return; }
  indices[prefix[id.x] - 1u] = id.x;
}
