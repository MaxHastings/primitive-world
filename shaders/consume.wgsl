@group(0) @binding(6) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> decisions: array<Decision>;
@group(0) @binding(2) var<storage, read_write> resources: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> requests: array<u32>;
@group(0) @binding(4) var<uniform> params: SimParams;
@group(0) @binding(5) var<storage, read_write> stats: array<atomic<u32>>;
@group(0) @binding(7) var<storage,read> offsets:array<u32>;
@group(0) @binding(8) var<storage,read> indices:array<u32>;
fn demand(i:u32)->u32 {
 let a=agents[i];let d=decisions[i];
 if(a.alive==0u || d.invalid!=0u){return 0u;}
 return min(u32(params.resource_and_noise.x*clamp(d.outputs[1],0.0,1.0)),u32(max(0.0,FOOD_CAPACITY-a.food)*1000.0));
}
// Exact floor(stock * request / total), without u64 or floating rounding.
// request <= 8000, total <= 16384*8000, stock <= total. The bounded
// remainder stays below 3*total, so every intermediate fits in u32.
fn share(stock:u32,request:u32,total:u32)->u32 {
 var result=0u;var remainder=0u;
 for(var bit=12;bit>=0;bit--){
  remainder=remainder*2u+select(0u,stock,(request&(1u<<u32(bit)))!=0u);
  result=result*2u+remainder/total;remainder=remainder%total;
 }
 return result;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let ri=id.x;if(ri>=262144u){return;}
 let cell=vec2<u32>(ri%512u,ri/512u);let ci=(cell.y/2u)*256u+cell.x/2u;
 var start=0u;if(ci>0u){start=offsets[ci-1u];}let end=offsets[ci];
 var total=0u;
 for(var k=start;k<end;k++){let i=indices[k];if(ground_index(agents[i].position,params.world_size.xy)==ri){total+=demand(i);}}
 let dropped=min(atomicLoad(&ground[ri].dropped),total);
 let grown=min(atomicLoad(&resources[ri]),total-dropped);
 var used_drop=0u;var used_grown=0u;
 for(var k=start;k<end;k++){
  let i=indices[k];if(ground_index(agents[i].position,params.world_size.xy)!=ri){continue;}
  requests[i]=0u;if(total==0u){continue;}
  let requested=demand(i);let drop=share(dropped,requested,total);let food=share(grown,requested,total);
  requests[i]=drop+food;used_drop+=drop;used_grown+=food;
 }
 // One invocation owns this food cell. Sub-milli proportional remainders
 // stay in the world instead of being awarded by GPU/storage ordering.
 atomicSub(&ground[ri].dropped,used_drop);atomicSub(&resources[ri],used_grown);
 atomicAdd(&ground[ri].extracted,used_grown);atomicAdd(&ground[ri].collected,used_drop+used_grown);
}
