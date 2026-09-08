// Observer-only reduction; no learned state crosses back into the simulation.
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> fast0:array<f32>;
@group(0) @binding(2) var<storage,read> fast1:array<f32>;
@group(0) @binding(3) var<storage,read_write> totals:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}var total=0.0;
 if(agents[i].alive!=0u){for(var k=0u;k<FAST_BANK_STRIDE;k++){let at=i*FAST_BANK_STRIDE+k;total+=abs(fast0[at])+abs(fast1[at]);}}
 totals[i]=total;
}
