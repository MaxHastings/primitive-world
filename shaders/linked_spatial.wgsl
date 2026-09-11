@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read_write> occupancy:array<atomic<u32>>;
@group(0) @binding(2) var<storage,read_write> heads:array<atomic<u32>>;
@group(0) @binding(3) var<storage,read_write> next:array<u32>;
@group(0) @binding(4) var<uniform> params:SimParams;
@compute @workgroup_size(64)
fn clear(@builtin(global_invocation_id) id:vec3<u32>){
 if(id.x<65536u){atomicStore(&occupancy[id.x],0u);atomicStore(&heads[id.x],INVALID);}
}
@compute @workgroup_size(64)
fn link(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count||agents[i].alive==0u){return;}
 let wrapped=wrap_world(agents[i].position,params.world_size.xy);
 let cell=clamp(floor(wrapped/params.world_size.xy*256.0),vec2<f32>(0.0),vec2<f32>(255.0));
 let ci=u32(cell.y)*256u+u32(cell.x);
 next[i]=atomicExchange(&heads[ci],i);atomicAdd(&occupancy[ci],1u);
}
