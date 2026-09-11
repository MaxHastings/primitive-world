@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> genomes0:array<f32>;
@group(0) @binding(5) var<storage,read> genomes1:array<f32>;
@group(0) @binding(6) var<storage,read> fast0:array<f32>;
@group(0) @binding(7) var<storage,read> fast1:array<f32>;
// These constants retain exact reference paths for paired performance probes.
const SPECIALIZED_BANKS:bool=true;
const ROW_WISE_INPUTS:bool=true;
fn gene(slot:u32,index:u32)->f32 {if(!SPECIALIZED_BANKS){let bank=index/GENOME_BANK_STRIDE;let local=index%GENOME_BANK_STRIDE;let at=slot*GENOME_BANK_STRIDE+local;if(bank==0u){return genomes0[at];}return genomes1[at];}let base=slot*GENOME_BANK_STRIDE;if(index<GENOME_BANK_STRIDE){return genomes0[base+index];}return genomes1[base+index-GENOME_BANK_STRIDE];}
fn gene0(slot:u32,index:u32)->f32 {if(SPECIALIZED_BANKS){return genomes0[slot*GENOME_BANK_STRIDE+index];}return gene(slot,index);}
fn gene1(slot:u32,index:u32)->f32 {if(SPECIALIZED_BANKS){return genomes1[slot*GENOME_BANK_STRIDE+index-GENOME_BANK_STRIDE];}return gene(slot,index);}
fn effective(genome:f32,slot:u32,index:u32)->f32{return clamp(genome+fast_value(slot,index),-4.0,4.0);}
fn effective1(genome:f32,slot:u32,index:u32)->f32{if(SPECIALIZED_BANKS){return clamp(genome+fast1[slot*FAST_BANK_STRIDE+index-FAST_BANK_STRIDE],-4.0,4.0);}return effective(genome,slot,index);}
fn fast_input(h:u32,k:u32)->u32{return h*INPUT_COUNT+k;}
fn fast_recurrent(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+h*HIDDEN_COUNT+k;}
fn fast_gate(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+HIDDEN_COUNT*HIDDEN_COUNT+h*HIDDEN_COUNT+k;}
fn fast_output(o:u32,h:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+2u*HIDDEN_COUNT*HIDDEN_COUNT+o*HIDDEN_COUNT+h;}

@group(0) @binding(8) var<storage,read> live_slots:array<u32>;
// Cooperative contiguous loads retain the original per-unit accumulation order.
const COALESCED_INPUTS:bool=true;
const COOPERATIVE_OUTPUTS:bool=true;
var<workgroup> input_weights:array<f32,HIDDEN_COUNT*INPUT_COUNT>;
var<workgroup> x:array<f32,INPUT_COUNT>;
var<workgroup> candidates:array<f32,HIDDEN_COUNT>;
var<workgroup> states:array<f32,HIDDEN_COUNT>;
var<workgroup> gates:array<f32,HIDDEN_COUNT>;
var<workgroup> outputs:array<f32,OUTPUT_COUNT>;
var<workgroup> organism:u32;
var<workgroup> fault:atomic<u32>;
@compute @workgroup_size(32)
fn main(@builtin(workgroup_id) group:vec3<u32>, @builtin(local_invocation_index) h:u32){
 let i=live_slots[4u+group.x];let mask=agents[i].active_mask;
 if(h==0u){organism=u32(agents[i].alive==ORGANISM);}
 let is_organism=workgroupUniformLoad(&organism);
 if(is_organism==0u){if(h==0u){var empty:Decision;decisions[i]=empty;}return;}
 if(h==0u){let a=agents[i];let p=perceptions[i];

 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;x[3]=a.age/10000.0;let self_velocity=world_to_body(a.velocity,a.heading);x[4]=self_velocity.x/1.2;x[5]=self_velocity.y/1.2;
 // Raw body state and consequences only: no named outcome, action, or
 // cooldown inputs.
 x[6]=select(0.0,(a.energy-bitcast<f32>(a.physical_previous[0]))/100.0,a.lived_ticks!=0u);
 x[7]=select(0.0,(a.food-bitcast<f32>(a.physical_previous[1]))/8.0,a.lived_ticks!=0u);
 let moved=world_to_body(a.moved,a.heading);x[8]=moved.x/1.2;x[9]=moved.y/1.2;
 x[10]=p.nearby_count/16.0;
 }
 // Each sample has independent channels. Assemble them cooperatively instead
 // of serializing all 107 input writes and checks through lane zero.
 if(h<16u){let n=11u+h*6u;let sample=perceptions[i].regions[h];x[n]=sample.food;x[n+1u]=sample.bodies/16.0;x[n+2u]=sample.velocity.x/1.2;x[n+3u]=sample.velocity.y/1.2;x[n+4u]=sample.signal;x[n+5u]=sample.pressure;}
 workgroupBarrier();
 for(var k=h;k<INPUT_COUNT;k+=32u){if(!finite(x[k])){atomicStore(&fault,1u);x[k]=0.0;}x[k]=clamp(x[k],-8.0,8.0);}
 workgroupBarrier();
 if(COALESCED_INPUTS){
  if(ROW_WISE_INPUTS){
   for(var unit=0u;unit<HIDDEN_COUNT;unit++){
    if(!unit_active(mask,unit)){continue;}
    for(var k=h;k<INPUT_COUNT;k+=32u){let at=unit*INPUT_COUNT+k;input_weights[at]=effective(gene(i,INPUT_BASE+at),i,at);}
   }
  }else{
   for(var at=h;at<HIDDEN_COUNT*INPUT_COUNT;at+=32u){if(unit_active(mask,at/INPUT_COUNT)){input_weights[at]=effective(gene(i,INPUT_BASE+at),i,at);}}
  }
  workgroupBarrier();
 }
 if(unit_active(mask,h)){
  var sum=gene0(i,NODE_BIAS+h);
  for(var k=0u;k<INPUT_COUNT;k++){var weight=0.0;if(COALESCED_INPUTS){weight=input_weights[h*INPUT_COUNT+k];}else{weight=effective(gene(i,INPUT_BASE+h*INPUT_COUNT+k),i,fast_input(h,k));}sum+=weight*x[k];}
  for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){sum+=effective1(gene1(i,RECURRENT_BASE+h*HIDDEN_COUNT+k),i,fast_recurrent(h,k))*agents[i].hidden[k];}}
  if(!finite(sum)){atomicStore(&fault,1u);sum=0.0;}candidates[h]=tanh(sum);
 }
 workgroupBarrier();
 if(unit_active(mask,h)){
  var sum=gene0(i,GATE_BIAS+h);
  for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){sum+=effective1(gene1(i,GATE_BASE+h*HIDDEN_COUNT+k),i,fast_gate(h,k))*candidates[k];}}
  if(!finite(sum)){atomicStore(&fault,1u);sum=0.0;}let gate=clamp(sum,0.0,1.0);gates[h]=gate;
  states[h]=(1.0-gate)*agents[i].hidden[h]+gate*candidates[h];if(!finite(states[h])){atomicStore(&fault,1u);states[h]=0.0;}
 }
 workgroupBarrier();
 if(h<OUTPUT_COUNT){var value=gene0(i,OUTPUT_BIAS+h);for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){value+=effective1(gene1(i,OUTPUT_BASE+h*HIDDEN_COUNT+k),i,fast_output(h,k))*states[k];}}if(!finite(value)){atomicStore(&fault,1u);value=0.0;}outputs[h]=value;}
 workgroupBarrier();
 if(h==0u){let p=perceptions[i];var d:Decision;d.evaluated=1u;d.invalid=atomicLoad(&fault);
 if(!COOPERATIVE_OUTPUTS){for(var k=0u;k<INPUT_COUNT;k++){d.inputs[k]=x[k];}}
 for(var k=0u;k<HIDDEN_COUNT;k++){d.candidate[k]=candidates[k];d.hidden[k]=states[k];d.update_gates[k]=gates[k];}
 for(var k=0u;k<OUTPUT_COUNT;k++){d.outputs[k]=outputs[k];}
 for(var k=0u;k<6u;k++){d.scores[k]=d.outputs[k];}
 var best=d.outputs[NONE];d.selected_action=NONE;
 for(var k=TRANSFER;k<6u;k++){if(k==TRANSFER && params.resource_and_noise.w<0.5){continue;}if(k==EMIT && (params.resource_and_noise.w<0.5 || params.physical.y<0.5)){continue;}if(k==APPLY_FORCE && (params.resource_and_noise.w<0.5 || params.physical.x<0.5)){continue;}if(d.outputs[k]>best){best=d.outputs[k];d.selected_action=k;}}
 d.movement=vec2<f32>(tanh(d.outputs[7]*params.physical.z),tanh(d.outputs[6]));d.amount=1.0/(1.0+exp(-clamp(d.outputs[8],-20.0,20.0)));d.payload=tanh(d.outputs[9]);
 let force_raw=vec2<f32>(d.outputs[FORCE_OUTPUT],d.outputs[FORCE_OUTPUT+1u]);d.force=unit_vector(force_raw)*tanh(length(force_raw));let placement_raw=vec2<f32>(d.outputs[PLACEMENT_OUTPUT],d.outputs[PLACEMENT_OUTPUT+1u]);d.placement=unit_vector(placement_raw)*tanh(length(placement_raw));
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;d.force=vec2<f32>(0);d.placement=vec2<f32>(0);for(var h=0u;h<HIDDEN_COUNT;h++){d.hidden[h]=0.0;d.candidate[h]=0.0;d.update_gates[h]=0.0;}}
 if(COOPERATIVE_OUTPUTS){
  decisions[i].selected_action=d.selected_action;decisions[i].evaluated=d.evaluated;
  decisions[i].movement=d.movement;decisions[i].amount=d.amount;decisions[i].payload=d.payload;
  decisions[i].invalid=d.invalid;decisions[i].decision_padding=0u;
  decisions[i].force=d.force;decisions[i].placement=d.placement;decisions[i].memory_write_cost=0.0;
 }else{decisions[i]=d;}
 }
 // Disjoint fields are written by adjacent lanes, with unchanged arithmetic.
 if(COOPERATIVE_OUTPUTS){
  for(var k=h;k<INPUT_COUNT;k+=32u){decisions[i].inputs[k]=x[k];}
  if(h<HIDDEN_COUNT){let valid=atomicLoad(&fault)==0u;decisions[i].candidate[h]=select(0.0,candidates[h],valid);decisions[i].hidden[h]=select(0.0,states[h],valid);decisions[i].update_gates[h]=select(0.0,gates[h],valid);}
  if(h<OUTPUT_COUNT){decisions[i].outputs[h]=outputs[h];}
  if(h<6u){decisions[i].scores[h]=outputs[h];}
 }
}
fn fast_value(slot:u32,index:u32)->f32 {if(!SPECIALIZED_BANKS){let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[at];}return fast1[at];}let base=slot*FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[base+index];}return fast1[base+index-FAST_BANK_STRIDE];}
