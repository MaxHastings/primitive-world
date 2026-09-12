@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read_write> fast0:array<f32>;
@group(0) @binding(2) var<storage,read_write> fast1:array<f32>;
@group(0) @binding(3) var<storage,read_write> traces:array<f32>;
@group(0) @binding(4) var<uniform> params:SimParams;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}let a=agents[i];
 // Only actual newborns reset here. Founders retain their first tick of learning.
 if(a.alive!=ORGANISM || a.ancestry_depth==0u || a.lived_ticks!=0u || (a.birth_tick!=params.tick || a.birth_high!=params.clock.x)){return;}
 for(var k=0u;k<CONNECTION_COUNT;k++){set_fast(i,k,0.0);}
 for(var k=0u;k<TRACE_COUNT;k++){traces[i*TRACE_COUNT+k]=0.0;}
}

fn fast_value(slot:u32,index:u32)->f32 {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[at];}return fast1[at];}
fn set_fast(slot:u32,index:u32,value:f32) {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){fast0[at]=value;}else{fast1[at]=value;}}
