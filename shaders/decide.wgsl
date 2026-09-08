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
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];let p=perceptions[i];var d:Decision;d.target_id=INVALID;
 if(a.alive==0u){decisions[i]=d;return;}d.evaluated=1u;
 var x:array<f32,INPUT_COUNT>;
 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;x[3]=a.age/10000.0;x[4]=a.velocity.x/1.2;x[5]=a.velocity.y/1.2;
 // Raw body state and consequences only: no named outcome, action, or
 // cooldown inputs. Self-propelled velocity and total displacement differ
 // when another body applies force, exposing contact as physics rather than
 // a labelled interaction result.
 x[6]=select(0.0,(a.energy-bitcast<f32>(a.physical_previous[0]))/100.0,a.lived_ticks!=0u);
 x[7]=select(0.0,(a.food-bitcast<f32>(a.physical_previous[1]))/8.0,a.lived_ticks!=0u);
 x[8]=a.moved.x/1.2;x[9]=a.moved.y/1.2;
 x[10]=a.signal_payload;
 x[11]=select(0.0,f32(params.tick-a.signal_tick)/1000.0,a.signal_tick!=0u);
 x[12]=p.nearby_count/16.0;
 for(var k=13u;k<20u;k++){x[k]=0.0;}
 for(var k=0u;k<16u;k++){x[20u+k*2u]=p.regions[k].food;x[21u+k*2u]=p.regions[k].bodies/16.0;}
 for(var k=0u;k<8u;k++){let b=p.bodies[k];let n=52u+k*7u;if(b.slot<INVALID){x[n]=b.offset.x/a.sensor_radius;x[n+1u]=b.offset.y/a.sensor_radius;x[n+2u]=b.velocity.x/1.2;x[n+3u]=b.velocity.y/1.2;x[n+4u]=b.signal;x[n+5u]=1.0;x[n+6u]=b.signal_present;}}
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
 let raw=vec2<f32>(d.outputs[6],d.outputs[7]);d.movement=unit_vector(raw)*tanh(length(raw)*params.physical.z);d.amount=1.0/(1.0+exp(-clamp(d.outputs[8],-20.0,20.0)));d.payload=tanh(d.outputs[9]);
 let force_raw=vec2<f32>(d.outputs[FORCE_OUTPUT],d.outputs[FORCE_OUTPUT+1u]);d.force=unit_vector(force_raw)*tanh(length(force_raw));best=-3.4e38;
 for(var k=0u;k<8u;k++){if(p.bodies[k].slot<INVALID && d.outputs[10u+k]>best){best=d.outputs[10u+k];d.target_id=p.bodies[k].slot;d.target_generation=p.bodies[k].generation;}}
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;d.force=vec2<f32>(0);for(var h=0u;h<HIDDEN_COUNT;h++){d.hidden[h]=0.0;d.candidate[h]=0.0;d.update_gates[h]=0.0;}}
 decisions[i]=d;
}

fn fast_value(slot:u32,index:u32)->f32 {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[at];}return fast1[at];}
