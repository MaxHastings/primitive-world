@group(0) @binding(6) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> decisions: array<Decision>;
@group(0) @binding(2) var<storage, read_write> resources: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> requests: array<u32>;
@group(0) @binding(4) var<uniform> params: SimParams;
@group(0) @binding(5) var<storage, read_write> stats: array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  requests[i]=0u;
  let a=agents[i];
  if (a.alive==0u || decisions[i].selected_action!=HARVEST) { return; }
  let cell=vec2<u32>(clamp(a.position/params.world_size*512.0,vec2<f32>(0.0),vec2<f32>(511.0)));
  let ri=cell.y*512u+cell.x;
  let requested=min(u32(params.resource_and_noise.x),u32(max(0.0,FOOD_CAPACITY-a.food)*1000.0));
  // Pick up dropped supplies first, then harvest only the remaining requested amount.
  for (var attempt=0u; attempt<16u; attempt++) {
    let available=atomicLoad(&ground[ri].dropped);
    let taken=min(available,requested);
    if (taken==0u) { break; }
    let exchange=atomicCompareExchangeWeak(&ground[ri].dropped,available,available-taken);
    if (exchange.exchanged) { requests[i]=taken; break; }
  }
  for (var attempt=0u; attempt<16u; attempt++) {
    let available=atomicLoad(&resources[ri]);
    let taken=min(available,requested-requests[i]);
    if (taken==0u) { break; }
    let exchange = atomicCompareExchangeWeak(&resources[ri],available,available-taken);
    if (exchange.exchanged) {
      requests[i]+=taken; atomicAdd(&ground[ri].extracted,taken); break;
    }
  }
}
