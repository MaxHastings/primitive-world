@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(5) var<storage,read_write> stats:array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}let a=agents[i];if(a.alive==0u){return;}let p=perceptions[i];let d=decisions[i];
 var any_signal=false;var control_target=INVALID;
 for(var k=0u;k<8u;k++){
  let b=p.bodies[k];
  if(b.slot<INVALID){control_target=select(control_target,b.slot,control_target>=INVALID);}
  // At most one signal record per receiver per dispatch. Eight records per
  // body could wrap the ring inside one parallel pass and race on its slots.
  if(!any_signal && b.slot<INVALID && b.signal_present>0.5){
   any_signal=true;
   let sequence=atomicAdd(&stats[8],1u);
   // other_lineage carries the receiver's selected action for this diagnostic event.
   events[sequence%65536u]=InteractionEvent(params.tick,i,b.slot,SIGNAL_OBSERVED,b.signal,sequence,a.lineage_id,d.selected_action,a.position,vec2<f32>(0.0),d.selected_action,0u);
  }
 }
 if(!any_signal && control_target<INVALID && (hash_u32(a.lineage_id^params.tick)&63u)==0u){
  let sequence=atomicAdd(&stats[8],1u);
  events[sequence%65536u]=InteractionEvent(params.tick,i,control_target,SIGNAL_CONTROL,0.0,sequence,a.lineage_id,d.selected_action,a.position,vec2<f32>(0.0),d.selected_action,0u);
 }
}
