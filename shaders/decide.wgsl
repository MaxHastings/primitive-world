@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> genomes:array<f32>;
@group(0) @binding(6) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(7) var<storage,read_write> stats:array<atomic<u32>>;
// Each body gets a stable, identity-derived unlock point for an action. The
// chance ramps from none to every body across the interval; it is unrelated to
// fitness, energy, ancestry depth, or action results.
fn progressively_available(a:Agent, action:u32, start:u32, end:u32)->bool{
 let environment_tick=params.lifecycle.w;
 if(end==0u || environment_tick>=end){return true;}
 if(environment_tick<start){return false;}
 let progress=f32(environment_tick-start)/f32(end-start);
 let key=a.lineage_id^(action*0x9e3779b9u)^params.lifecycle.x;
 return random01(key)<progress;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];let p=perceptions[i];var d:Decision;d.target_id=INVALID;
 if(a.alive==0u){decisions[i]=d;return;}
 d.evaluated=1u;
 var x:array<f32,INPUT_COUNT>;
 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;
 x[3]=a.age/10000.0;x[4]=a.velocity.x/1.2;x[5]=a.velocity.y/1.2;
 x[6]=a.collected;x[7]=a.ingested;x[8]=a.spent;x[9]=a.received;
 x[10]=a.moved.x/1.2;x[11]=a.moved.y/1.2;
 x[12]=p.nearby_count/16.0;
 x[13]=f32(a.next_birth-min(a.next_birth,params.tick))/240.0;
 for(var k=0u;k<6u;k++){x[14u+k]=f32(a.action==k);}
 for(var k=0u;k<16u;k++){x[20u+k*2u]=p.regions[k].food;x[21u+k*2u]=p.regions[k].bodies/16.0;}
 for(var k=0u;k<8u;k++){let b=p.bodies[k];let n=52u+k*7u;
  if(b.slot<INVALID){x[n]=b.offset.x/a.sensor_radius;x[n+1u]=b.offset.y/a.sensor_radius;x[n+2u]=b.velocity.x/1.2;x[n+3u]=b.velocity.y/1.2;x[n+4u]=b.signal;x[n+5u]=1.0;x[n+6u]=b.signal_present;}
 }
 for(var k=0u;k<INPUT_COUNT;k++){if(!finite(x[k])){d.invalid=1u;}x[k]=clamp(x[k],-8.0,8.0);d.inputs[k]=x[k];}

 let base=i*GENOME_SIZE;
 var candidate:array<f32,HIDDEN_COUNT>;
 for(var h=0u;h<HIDDEN_COUNT;h++){
  var sum=genomes[base+NODE_BIAS+h];
  for(var k=0u;k<INPUT_COUNT;k++){sum+=genomes[base+INPUT_BASE+h*INPUT_COUNT+k]*x[k];}
  for(var k=0u;k<HIDDEN_COUNT;k++){sum+=genomes[base+RECURRENT_BASE+h*HIDDEN_COUNT+k]*a.hidden[k];}
  if(!finite(sum)){d.invalid=1u;sum=0.0;}candidate[h]=tanh(sum);
 }
 for(var h=0u;h<HIDDEN_COUNT;h++){
  var sum=genomes[base+GATE_BIAS+h];
  for(var k=0u;k<HIDDEN_COUNT;k++){sum+=genomes[base+GATE_BASE+h*HIDDEN_COUNT+k]*candidate[k];}
  if(!finite(sum)){d.invalid=1u;sum=0.0;}
  let gate=clamp(sum,0.0,1.0);d.update_gates[h]=gate;
  d.hidden[h]=(1.0-gate)*a.hidden[h]+gate*candidate[h];
  if(!finite(d.hidden[h])){d.invalid=1u;}
 }
 var out:array<f32,OUTPUT_COUNT>;
 for(var o=0u;o<OUTPUT_COUNT;o++){
  out[o]=genomes[base+OUTPUT_BIAS+o];
  for(var h=0u;h<HIDDEN_COUNT;h++){out[o]+=genomes[base+OUTPUT_BASE+o*HIDDEN_COUNT+h]*d.hidden[h];}
 }
 for(var o=0u;o<OUTPUT_COUNT;o++){if(!finite(out[o])){d.invalid=1u;out[o]=0.0;}}
 var best=-3.4e38;
 for(var k=0u;k<6u;k++){
  d.scores[k]=out[k];
  // The fourth physical parameter is the bootstrap duration. Each capability
  // becomes available to a growing, neutral hash-selected share of bodies;
  // this only masks logits and never supplies a behavior or a reward.
  let ramp=u32(params.physical.w);
  // Give survival and reproduction a full bootstrap before social actions
  // begin competing with collection. Social capabilities still emerge from
  // the same neutral per-body unlock law; only their shared start is delayed.
  let social_delay=ramp;
  if(k==REPRODUCE && !progressively_available(a,REPRODUCE,0u,ramp/20u)){continue;}
  if(k==TRANSFER && (!progressively_available(a,TRANSFER,social_delay+ramp/20u,social_delay+ramp*3u/10u) || params.resource_and_noise.w<0.5)){continue;}
  if(k==EMIT && (!progressively_available(a,EMIT,social_delay+ramp*3u/10u,social_delay+ramp*3u/5u) || params.resource_and_noise.w<0.5 || params.physical.y<0.5)){continue;}
  if(k==APPLY_FORCE && (!progressively_available(a,APPLY_FORCE,social_delay+ramp*3u/5u,social_delay+ramp) || params.resource_and_noise.w<0.5 || params.physical.x<0.5)){continue;}
  if(out[k]>best){best=out[k];d.selected_action=k;}
 }
 // Continuous actuator calibration: no minimum motion or preferred heading.
 let raw=vec2<f32>(out[6],out[7]);d.movement=unit_vector(raw)*tanh(length(raw)*params.physical.z);
 d.amount=1.0/(1.0+exp(-clamp(out[8],-20.0,20.0)));d.payload=tanh(out[9]);
 let force_raw=vec2<f32>(out[FORCE_OUTPUT],out[FORCE_OUTPUT+1u]);d.force=unit_vector(force_raw)*tanh(length(force_raw));
 best=-3.4e38;
 for(var k=0u;k<8u;k++){if(p.bodies[k].slot<INVALID && out[10u+k]>best){best=out[10u+k];d.target_id=p.bodies[k].slot;d.target_generation=p.bodies[k].generation;}}
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
   events[sequence%65536u]=InteractionEvent(params.tick,i,b.slot,SIGNAL_OBSERVED,b.signal,sequence,a.lineage_id,d.selected_action,a.position);
  }
 }
 if(!any_signal && control_target<INVALID && (hash_u32(a.lineage_id^params.tick)&63u)==0u){
  let sequence=atomicAdd(&stats[8],1u);
  events[sequence%65536u]=InteractionEvent(params.tick,i,control_target,SIGNAL_CONTROL,0.0,sequence,a.lineage_id,d.selected_action,a.position);
 }
 // Fault containment only: do not replace finite but ineffective intentions.
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;d.force=vec2<f32>(0);for(var h=0u;h<HIDDEN_COUNT;h++){d.hidden[h]=0.0;}}
 if((hash_u32(a.lineage_id^params.tick^0x51ed270bu)&511u)==0u){
 var memory_effect=0.0;var old_norm_sq=0.0;var new_norm_sq=0.0;let base=i*GENOME_SIZE;
  for(var h=0u;h<HIDDEN_COUNT;h++){
   memory_effect+=genomes[base+OUTPUT_BASE+d.selected_action*HIDDEN_COUNT+h]*(1.0-d.update_gates[h])*a.hidden[h];
   old_norm_sq+=a.hidden[h]*a.hidden[h];new_norm_sq+=d.hidden[h]*d.hidden[h];
  }
  var counterfactual_action=NONE;var counterfactual_best=-3.4e38;
  let ramp=u32(params.physical.w);let social_delay=ramp;
  for(var k=0u;k<6u;k++){
   var allowed=true;
   if(k==REPRODUCE && !progressively_available(a,REPRODUCE,0u,ramp/20u)){allowed=false;}
   if(k==TRANSFER && (!progressively_available(a,TRANSFER,social_delay+ramp/20u,social_delay+ramp*3u/10u) || params.resource_and_noise.w<0.5)){allowed=false;}
   if(k==EMIT && (!progressively_available(a,EMIT,social_delay+ramp*3u/10u,social_delay+ramp*3u/5u) || params.resource_and_noise.w<0.5 || params.physical.y<0.5)){allowed=false;}
   if(k==APPLY_FORCE && (!progressively_available(a,APPLY_FORCE,social_delay+ramp*3u/5u,social_delay+ramp) || params.resource_and_noise.w<0.5 || params.physical.x<0.5)){allowed=false;}
   if(allowed){
    var memory_for_action=0.0;
    for(var h=0u;h<HIDDEN_COUNT;h++){memory_for_action+=genomes[base+OUTPUT_BASE+k*HIDDEN_COUNT+h]*(1.0-d.update_gates[h])*a.hidden[h];}
    let counterfactual_score=d.scores[k]-memory_for_action;
    if(counterfactual_score>counterfactual_best){counterfactual_best=counterfactual_score;counterfactual_action=k;}
   }
  }
  let sequence=atomicAdd(&stats[8],1u);
  // amount is the selected action's score contribution from the recurrent state;
  // position stores old and new hidden-state norms for this diagnostic event.
  events[sequence%65536u]=InteractionEvent(params.tick,i,counterfactual_action,MEMORY_SAMPLE,memory_effect,sequence,a.lineage_id,d.selected_action,vec2<f32>(sqrt(old_norm_sq),sqrt(new_norm_sq)));
 }
 decisions[i]=d;
}
