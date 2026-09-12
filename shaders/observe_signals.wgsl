@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(5) var<storage,read_write> stats:array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}let a=agents[i];if(a.alive!=ORGANISM){return;}let p=perceptions[i];let d=decisions[i];
 var any_signal=false;
 for(var k=0u;k<16u;k++){
  // Diagnostics observe the same anonymous aggregate field cognition sees.
  if(!any_signal && abs(p.regions[k].signal)>0.0){
   any_signal=true;
   let sequence=counter_add(8,1u);
   events[sequence%65536u]=InteractionEvent(params.tick,i,INVALID,SIGNAL_OBSERVED,p.regions[k].signal,sequence,a.lineage_id,d.selected_action,a.position,vec2<f32>(0.0),d.selected_action,0u,params.clock.x,a.lineage_high,0u,0u);
  }
 }
}

// Cumulative telemetry uses low/high words and never controls the ecology.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicAdd(&stats[40u+index],1u);}
 return prior;
}
