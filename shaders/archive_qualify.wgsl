// One candidacy per observed individual, regardless of age or behavior.
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read_write> qualified:array<u32>;
@group(0) @binding(2) var<storage,read_write> candidates:array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> previous:array<Agent>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}
 if(agents[i].alive!=0u){atomicAdd(&candidates[1],1u);}
 let lineage=agents[i].lineage_id;
 if(lineage==0u || qualified[i]==lineage){return;}
 if(agents[i].alive==0u && (previous[i].alive==0u || previous[i].lineage_id!=lineage)){return;}
 qualified[i]=lineage;
 let index=atomicAdd(&candidates[0],1u);atomicStore(&candidates[4u+index],i);
}
