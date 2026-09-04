@group(0) @binding(0) var<storage, read> source: array<u32>;
@group(0) @binding(1) var<storage, read_write> destination: array<u32>;
fn scan_step(id: u32, stride: u32) {
  if (id >= 100000u) { return; }
  destination[id] = select(source[id], source[id] + source[id - stride], id >= stride);
}
@compute @workgroup_size(64, 1, 1) fn step_1(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 1u); }
@compute @workgroup_size(64, 1, 1) fn step_2(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 2u); }
@compute @workgroup_size(64, 1, 1) fn step_4(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 4u); }
@compute @workgroup_size(64, 1, 1) fn step_8(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 8u); }
@compute @workgroup_size(64, 1, 1) fn step_16(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 16u); }
@compute @workgroup_size(64, 1, 1) fn step_32(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 32u); }
@compute @workgroup_size(64, 1, 1) fn step_64(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 64u); }
@compute @workgroup_size(64, 1, 1) fn step_128(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 128u); }
@compute @workgroup_size(64, 1, 1) fn step_256(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 256u); }
@compute @workgroup_size(64, 1, 1) fn step_512(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 512u); }
@compute @workgroup_size(64, 1, 1) fn step_1024(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 1024u); }
@compute @workgroup_size(64, 1, 1) fn step_2048(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 2048u); }
@compute @workgroup_size(64, 1, 1) fn step_4096(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 4096u); }
@compute @workgroup_size(64, 1, 1) fn step_8192(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 8192u); }
@compute @workgroup_size(64, 1, 1) fn step_16384(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 16384u); }
@compute @workgroup_size(64, 1, 1) fn step_32768(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 32768u); }
@compute @workgroup_size(64, 1, 1) fn step_65536(@builtin(global_invocation_id) id: vec3<u32>) { scan_step(id.x, 65536u); }
