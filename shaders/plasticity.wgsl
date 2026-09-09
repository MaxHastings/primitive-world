@group(0) @binding(0) var<storage,read> before:array<Agent>;
@group(0) @binding(1) var<storage,read_write> after:array<Agent>;
@group(0) @binding(2) var<storage,read> decisions:array<Decision>;
@group(0) @binding(3) var<storage,read_write> fast0:array<f32>;
@group(0) @binding(4) var<storage,read_write> fast1:array<f32>;
@group(0) @binding(5) var<storage,read_write> traces:array<f32>;
@group(0) @binding(6) var<uniform> params:SimParams;
@group(0) @binding(7) var<storage,read_write> stats:array<atomic<u32>>;

fn fast_input(h:u32,k:u32)->u32{return h*INPUT_COUNT+k;}
fn fast_recurrent(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+h*HIDDEN_COUNT+k;}
fn fast_gate(h:u32,k:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+HIDDEN_COUNT*HIDDEN_COUNT+h*HIDDEN_COUNT+k;}
fn fast_output(o:u32,h:u32)->u32{return HIDDEN_COUNT*INPUT_COUNT+2u*HIDDEN_COUNT*HIDDEN_COUNT+o*HIDDEN_COUNT+h;}
fn trace_input(k:u32)->u32{return k;}
fn trace_hidden(h:u32)->u32{return INPUT_COUNT+h;}
fn trace_output(o:u32)->u32{return INPUT_COUNT+HIDDEN_COUNT+o;}

fn update_fast(slot:u32,index:u32,pre:f32,post:f32,rate:f32,retention:f32,change:ptr<function,f32>){
 let old=fast_value(slot,index);
 let next=clamp(retention*old+rate*pre*post,-1.0,1.0);
 (*change)+=abs(next-old);set_fast(slot,index,next);
}

@group(0) @binding(8) var<storage,read> live_slots:array<u32>;
// Stage input deltas contiguously, then preserve each unit's write-cost order.
const COALESCED_INPUTS:bool=true;
var<workgroup> learned_inputs:array<f32,HIDDEN_COUNT*INPUT_COUNT>;
var<workgroup> changes:array<f32,32>;
// Every active unit uses the same presynaptic traces and output activations.
// Cache them once per body; connection updates retain their original order.
var<workgroup> input_traces:array<f32,INPUT_COUNT>;
var<workgroup> hidden_traces:array<f32,HIDDEN_COUNT>;
var<workgroup> output_activity:array<f32,OUTPUT_COUNT>;
@compute @workgroup_size(32)
fn main(@builtin(workgroup_id) group:vec3<u32>, @builtin(local_invocation_index) h:u32){
 let i=live_slots[4u+group.x];
 // The entire workgroup takes the same body-death branch.
 if(after[i].alive==0u){return;}
 let trace_base=i*TRACE_COUNT;var change=0.0;let mask=after[i].active_mask;
 if(decisions[i].invalid!=0u){
  for(var k=h;k<CONNECTION_COUNT;k+=32u){let value=fast_value(i,k);if(finite(value)){change+=abs(value);}set_fast(i,k,0.0);}
  for(var k=h;k<TRACE_COUNT;k+=32u){let value=traces[trace_base+k];if(finite(value)){change+=abs(value);}traces[trace_base+k]=0.0;}
  if(unit_active(mask,h)&&finite(before[i].hidden[h])){change+=abs(before[i].hidden[h]);}
 }else{
  if(COALESCED_INPUTS){for(var at=h;at<HIDDEN_COUNT*INPUT_COUNT;at+=32u){if(unit_active(mask,at/INPUT_COUNT)){learned_inputs[at]=fast_value(i,at);}}}
  let retention=after[i].trace_retention;
  for(var k=h;k<INPUT_COUNT;k+=32u){let at=trace_base+k;let old=traces[at];let value=clamp(retention*old+(1.0-retention)*decisions[i].inputs[k],-1.0,1.0);change+=abs(value-old);traces[at]=value;input_traces[k]=value;}
  if(unit_active(mask,h)){let at=trace_base+INPUT_COUNT+h;let old=traces[at];let value=clamp(retention*old+(1.0-retention)*decisions[i].hidden[h],-1.0,1.0);change+=abs(value-old);traces[at]=value;hidden_traces[h]=value;}
  if(h<OUTPUT_COUNT){let at=trace_base+INPUT_COUNT+HIDDEN_COUNT+h;let old=traces[at];let activation=tanh(decisions[i].outputs[h]);output_activity[h]=activation;let value=clamp(retention*old+(1.0-retention)*activation,-1.0,1.0);change+=abs(value-old);traces[at]=value;}
 }
 workgroupBarrier();
 if(decisions[i].invalid==0u && unit_active(mask,h)){
  let rate=after[i].plasticity_rate[h];let retention=after[i].learned_weight_retention;
  let candidate=decisions[i].candidate[h];
  for(var k=0u;k<INPUT_COUNT;k++){
   if(COALESCED_INPUTS){let at=fast_input(h,k);let old=learned_inputs[at];let next=clamp(retention*old+rate*input_traces[k]*candidate,-1.0,1.0);change+=abs(next-old);learned_inputs[at]=next;}
   else{update_fast(i,fast_input(h,k),input_traces[k],candidate,rate,retention,&change);}
  }
  for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){
   let pre=hidden_traces[k];
   update_fast(i,fast_recurrent(h,k),pre,candidate,rate,retention,&change);
   update_fast(i,fast_gate(h,k),pre,decisions[i].update_gates[h],rate,retention,&change);
  }}
  for(var o=0u;o<OUTPUT_COUNT;o++){update_fast(i,fast_output(o,h),hidden_traces[h],output_activity[o],rate,retention,&change);}
  change+=abs(decisions[i].hidden[h]-before[i].hidden[h]);
 }
 changes[h]=change;workgroupBarrier();
 if(COALESCED_INPUTS && decisions[i].invalid==0u){for(var at=h;at<HIDDEN_COUNT*INPUT_COUNT;at+=32u){if(unit_active(mask,at/INPUT_COUNT)){set_fast(i,at,learned_inputs[at]);}}}

 if(h==0u){var total=0.0;for(var k=0u;k<32u;k++){total+=changes[k];}let cost=total*params.environment.w;var a=after[i];let paid=min(a.energy,cost);
  counter_add(34,u32(round(paid*1000.0)));a.spent+=paid;a.energy=max(0.0,a.energy-cost);
  if(a.energy<=0.0){a.alive=0u;counter_add(1,1u);}after[i]=a;
 }
}
fn fast_value(slot:u32,index:u32)->f32 {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){return fast0[at];}return fast1[at];}
fn set_fast(slot:u32,index:u32,value:f32) {let at=slot*FAST_BANK_STRIDE+index%FAST_BANK_STRIDE;if(index<FAST_BANK_STRIDE){fast0[at]=value;}else{fast1[at]=value;}}

// Accounting horizons are explicit engine limits, never ecological extinction.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicStore(&stats[36],1u);}
 return prior;
}
