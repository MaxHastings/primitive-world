@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> ground: array<Ground>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=INVALID || agents[i].alive!=0u || agents[i].food<=0.0) { return; }
  atomicAdd(&ground[ground_index(agents[i].position)].dropped,u32(round(agents[i].food*1000.0)));
  agents[i].food=0.0;
}
