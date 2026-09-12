// Read-only diagnostics use the same pre-learning effective readout as decisions.
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> decisions:array<Decision>;
@group(0) @binding(2) var<storage,read> genomes0:array<f32>;
@group(0) @binding(3) var<storage,read> genomes1:array<f32>;
@group(0) @binding(4) var<storage,read> fast0:array<f32>;
@group(0) @binding(5) var<storage,read> fast1:array<f32>;
@group(0) @binding(6) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(7) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(8) var<uniform> params:SimParams;
fn readout(slot:u32,offset:u32)->f32 {
 let index=OUTPUT_BASE+offset;let at=slot*GENOME_BANK_STRIDE+index%GENOME_BANK_STRIDE;
 var value=0.0;if(index<GENOME_BANK_STRIDE){value=genomes0[at];}else{value=genomes1[at];}
 let fast_index=HIDDEN_COUNT*INPUT_COUNT+2u*HIDDEN_COUNT*HIDDEN_COUNT+offset;
 let learned_at=slot*FAST_BANK_STRIDE+fast_index%FAST_BANK_STRIDE;
 if(fast_index<FAST_BANK_STRIDE){value+=fast0[learned_at];}else{value+=fast1[learned_at];}
 return clamp(value,-4.0,4.0);
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}let a=agents[i];if(a.alive!=ORGANISM){return;}let d=decisions[i];if(d.invalid!=0u){return;}
 if((hash_u32(a.lineage_id^params.tick^0x51ed270bu)&511u)==0u){
 var memory_effect=0.0;var old_norm_sq=0.0;var new_norm_sq=0.0;
  for(var h=0u;h<HIDDEN_COUNT;h++){if(!unit_active(a.active_mask,h)){continue;}
   memory_effect+=readout(i,d.selected_action*HIDDEN_COUNT+h)*(1.0-d.update_gates[h])*a.hidden[h];
   old_norm_sq+=a.hidden[h]*a.hidden[h];new_norm_sq+=d.hidden[h]*d.hidden[h];
  }
  var counterfactual_action=NONE;var counterfactual_best=-3.4e38;
  for(var k=0u;k<6u;k++){
   var allowed=true;
   if(k==TRANSFER && (params.resource_and_noise.w<0.5)){allowed=false;}
   if(k==EMIT && (params.resource_and_noise.w<0.5 || params.physical.y<0.5)){allowed=false;}
   if(k==APPLY_FORCE && (params.resource_and_noise.w<0.5 || params.physical.x<0.5)){allowed=false;}
   if(allowed){
    var memory_for_action=0.0;
    for(var h=0u;h<HIDDEN_COUNT;h++){if(!unit_active(a.active_mask,h)){continue;}memory_for_action+=readout(i,k*HIDDEN_COUNT+h)*(1.0-d.update_gates[h])*a.hidden[h];}
    let counterfactual_score=d.scores[k]-memory_for_action;
    if(counterfactual_score>counterfactual_best){counterfactual_best=counterfactual_score;counterfactual_action=k;}
   }
  }
  let sequence=counter_add(8,1u);
  // amount is the selected action's score contribution from the recurrent state;
  // position stores old and new hidden-state norms for this diagnostic event.
  events[sequence%65536u]=InteractionEvent(params.tick,i,counterfactual_action,MEMORY_SAMPLE,memory_effect,sequence,a.lineage_id,d.selected_action,vec2<f32>(sqrt(old_norm_sq),sqrt(new_norm_sq)),vec2<f32>(a.food,d.inputs[2]),d.selected_action,0u,params.clock.x,a.lineage_high,0u,0u);
 }
}

// Cumulative telemetry uses low/high words and never controls the ecology.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicAdd(&stats[40u+index],1u);}
 return prior;
}
