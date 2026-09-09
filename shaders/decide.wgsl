@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> genomes0:array<f32>;
@group(0) @binding(5) var<storage,read> genomes1:array<f32>;
@group(0) @binding(6) var<storage,read> fast0:array<f32>;
@group(0) @binding(7) var<storage,read> fast1:array<f32>;
fn gene(slot:u32,index:u32)->f32 { let bank=index/GENOME_BANK_STRIDE;let local=index%GENOME_BANK_STRIDE;let at=slot*GENOME_BANK_STRIDE+local; if(bank==0u){return genomes0[at];} return genomes1[at]; }
fn effective(genome:f32,slot:u32,index:u32)->f32{return clamp(genome+fast_value(slot,index),-4.0,4.0);}
fn fast_input(h:u32,k:u32)->u32{return h*INPUT_COUNT+k;}
fn fast_recurrent(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+h*HIDDEN_COUNT+k;}
fn fast_gate(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+HIDDEN_COUNT*HIDDEN_COUNT+h*HIDDEN_COUNT+k;}
fn fast_output(o:u32,h:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+2u*HIDDEN_COUNT*HIDDEN_COUNT+o*HIDDEN_COUNT+h;}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];let p=perceptions[i];var d:Decision;
 if(a.alive!=ORGANISM){decisions[i]=d;return;}d.evaluated=1u;
 var x:array<f32,INPUT_COUNT>;
 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;x[3]=a.age/10000.0;let self_velocity=world_to_body(a.velocity,a.heading);x[4]=self_velocity.x/1.2;x[5]=self_velocity.y/1.2;
 // Raw body state and consequences only: no named outcome, action, or
 // cooldown inputs.
 x[6]=select(0.0,(a.energy-bitcast<f32>(a.physical_previous[0]))/100.0,a.lived_ticks!=0u);
 x[7]=select(0.0,(a.food-bitcast<f32>(a.physical_previous[1]))/8.0,a.lived_ticks!=0u);
 let moved=world_to_body(a.moved,a.heading);x[8]=moved.x/1.2;x[9]=moved.y/1.2;
 x[10]=p.nearby_count/16.0;
 for(var k=0u;k<16u;k++){let n=11u+k*6u;let sample=p.regions[k];x[n]=sample.food;x[n+1u]=sample.bodies/16.0;x[n+2u]=sample.velocity.x/1.2;x[n+3u]=sample.velocity.y/1.2;x[n+4u]=sample.signal;x[n+5u]=sample.pressure;}
 for(var k=0u;k<INPUT_COUNT;k++){if(!finite(x[k])){d.invalid=1u;x[k]=0.0;}x[k]=clamp(x[k],-8.0,8.0);d.inputs[k]=x[k];}
 for(var h=0u;h<HIDDEN_COUNT;h++){
  if(!unit_active(a.active_mask,h)){continue;}var sum=gene(i,NODE_BIAS+h);
  for(var k=0u;k<INPUT_COUNT;k++){sum+=effective(gene(i,INPUT_BASE+h*INPUT_COUNT+k),i,fast_input(h,k))*x[k];}
  for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(a.active_mask,k)){sum+=effective(gene(i,RECURRENT_BASE+h*HIDDEN_COUNT+k),i,fast_recurrent(h,k))*a.hidden[k];}}
  if(!finite(sum)){d.invalid=1u;sum=0.0;}d.candidate[h]=tanh(sum);
 }
 for(var h=0u;h<HIDDEN_COUNT;h++){
  if(!unit_active(a.active_mask,h)){continue;}var sum=gene(i,GATE_BIAS+h);
  for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(a.active_mask,k)){sum+=effective(gene(i,GATE_BASE+h*HIDDEN_COUNT+k),i,fast_gate(h,k))*d.candidate[k];}}
  if(!finite(sum)){d.invalid=1u;sum=0.0;}let gate=clamp(sum,0.0,1.0);d.update_gates[h]=gate;
  d.hidden[h]=(1.0-gate)*a.hidden[h]+gate*d.candidate[h];if(!finite(d.hidden[h])){d.invalid=1u;d.hidden[h]=0.0;}
 }
 for(var o=0u;o<OUTPUT_COUNT;o++){var value=gene(i,OUTPUT_BIAS+o);for(var h=0u;h<HIDDEN_COUNT;h++){if(unit_active(a.active_mask,h)){value+=effective(gene(i,OUTPUT_BASE+o*HIDDEN_COUNT+h),i,fast_output(o,h))*d.hidden[h];}}if(!finite(value)){d.invalid=1u;value=0.0;}d.outputs[o]=value;}
 for(var k=0u;k<6u;k++){d.scores[k]=d.outputs[k];}
 var best=d.outputs[NONE];d.selected_action=NONE;
 for(var k=TRANSFER;k<6u;k++){if(k==TRANSFER && params.resource_and_noise.w<0.5){continue;}if(k==EMIT && (params.resource_and_noise.w<0.5 || params.physical.y<0.5)){continue;}if(k==APPLY_FORCE && (params.resource_and_noise.w<0.5 || params.physical.x<0.5)){continue;}if(d.outputs[k]>best){best=d.outputs[k];d.selected_action=k;}}
 d.movement=vec2<f32>(tanh(d.outputs[7]*params.physical.z),tanh(d.outputs[6]));d.amount=1.0/(1.0+exp(-clamp(d.outputs[8],-20.0,20.0)));d.payload=tanh(d.outputs[9]);
 let force_raw=vec2<f32>(d.outputs[FORCE_OUTPUT],d.outputs[FORCE_OUTPUT+1u]);d.force=unit_vector(force_raw)*tanh(length(force_raw));let placement_raw=vec2<f32>(d.outputs[PLACEMENT_OUTPUT],d.outputs[PLACEMENT_OUTPUT+1u]);d.placement=unit_vector(placement_raw)*tanh(length(placement_raw));
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;d.force=vec2<f32>(0);d.placement=vec2<f32>(0);for(var h=0u;h<HIDDEN_COUNT;h++){d.hidden[h]=0.0;d.candidate[h]=0.0;d.update_gates[h]=0.0;}}
 decisions[i]=d;
}

fn fast_value(slot:u32,index:u32)->f32 {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[at];}return fast1[at];}
