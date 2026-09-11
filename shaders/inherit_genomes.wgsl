@group(0) @binding(0) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> free_indices:array<u32>;
@group(0) @binding(2) var<storage,read> free_prefix:array<u32>;
@group(0) @binding(3) var<storage,read> parents:array<u32>;
@group(0) @binding(4) var<storage,read> birth_prefix:array<u32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@group(0) @binding(6) var<storage,read_write> genomes0:array<f32>;
@group(0) @binding(7) var<storage,read_write> genomes1:array<f32>;
@group(0) @binding(8) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(9) var<storage,read> claims:array<u32>;
@group(0) @binding(10) var<storage,read> source_agents:array<Agent>;
fn mutation_draw(rng:ptr<function,u32>)->f32 {
 *rng=(*rng)*1664525u+1013904223u;return f32((*rng)>>8u)/16777216.0;
}
fn gene(slot:u32,index:u32)->f32 {let local=index%GENOME_BANK_STRIDE;let at=slot*GENOME_BANK_STRIDE+local;if(index<GENOME_BANK_STRIDE){return genomes0[at];}return genomes1[at];}
fn set_gene(slot:u32,index:u32,value:f32) {let local=index%GENOME_BANK_STRIDE;let at=slot*GENOME_BANK_STRIDE+local;if(index<GENOME_BANK_STRIDE){genomes0[at]=value;}else{genomes1[at]=value;}}
fn nth_set(mask:u32,n:u32)->u32 {var seen=0u;for(var h=0u;h<HIDDEN_COUNT;h++){if(unit_active(mask,h)){if(seen==n){return h;}seen++;}}return 0u;}
fn nth_clear(mask:u32,n:u32)->u32 {var seen=0u;for(var h=0u;h<HIDDEN_COUNT;h++){if(!unit_active(mask,h)){if(seen==n){return h;}seen++;}}return 0u;}

// Same ordering as brain::mutate_expressed, without a genome-sized local array.
fn expressed_index(choice:u32, mask:u32)->u32 {
 let capacity=countOneBits(mask);
 let width=2u+INPUT_COUNT+2u*capacity+OUTPUT_COUNT;
 if(choice>=capacity*width){return OUTPUT_BIAS+choice-capacity*width;}
 let h=nth_set(mask,choice/width);var k=choice%width;
 if(k==0u){return NODE_BIAS+h;}if(k==1u){return GATE_BIAS+h;}k-=2u;
 if(k<INPUT_COUNT){return INPUT_BASE+h*INPUT_COUNT+k;}k-=INPUT_COUNT;
 if(k<2u*capacity){let other=nth_set(mask,k/2u);return select(RECURRENT_BASE,GATE_BASE,k%2u==1u)+h*HIDDEN_COUNT+other;}k-=2u*capacity;
 return OUTPUT_BASE+k*HIDDEN_COUNT+h;
}
fn jitter(value:f32,rng:ptr<function,u32>)->f32 {return clamp(value+(mutation_draw(rng)*2.0-1.0)*0.01,-4.0,4.0);}
fn clone_unit(ci:u32,donor:u32,new_unit:u32,mask:u32,rng:ptr<function,u32>) {
 set_gene(ci,NODE_BIAS+new_unit,jitter(gene(ci,NODE_BIAS+donor),rng));
 set_gene(ci,GATE_BIAS+new_unit,jitter(gene(ci,GATE_BIAS+donor),rng));
 for(var k=0u;k<INPUT_COUNT;k++){set_gene(ci,INPUT_BASE+new_unit*INPUT_COUNT+k,jitter(gene(ci,INPUT_BASE+donor*INPUT_COUNT+k),rng));}
 for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){
  set_gene(ci,RECURRENT_BASE+new_unit*HIDDEN_COUNT+k,jitter(gene(ci,RECURRENT_BASE+donor*HIDDEN_COUNT+k),rng));
  set_gene(ci,GATE_BASE+new_unit*HIDDEN_COUNT+k,jitter(gene(ci,GATE_BASE+donor*HIDDEN_COUNT+k),rng));
 }}
 for(var k=0u;k<HIDDEN_COUNT;k++){if(unit_active(mask,k)){
  let r=gene(ci,RECURRENT_BASE+k*HIDDEN_COUNT+donor)*0.5;
  set_gene(ci,RECURRENT_BASE+k*HIDDEN_COUNT+donor,r);set_gene(ci,RECURRENT_BASE+k*HIDDEN_COUNT+new_unit,r);
  let g=gene(ci,GATE_BASE+k*HIDDEN_COUNT+donor)*0.5;
  set_gene(ci,GATE_BASE+k*HIDDEN_COUNT+donor,g);set_gene(ci,GATE_BASE+k*HIDDEN_COUNT+new_unit,g);
 }}
 for(var b=0u;b<2u;b++){
  let base=select(RECURRENT_BASE,GATE_BASE,b==1u);let value=gene(ci,base+new_unit*HIDDEN_COUNT+donor)*0.5;
  set_gene(ci,base+new_unit*HIDDEN_COUNT+donor,value);set_gene(ci,base+new_unit*HIDDEN_COUNT+new_unit,value);
 }
 for(var o=0u;o<OUTPUT_COUNT;o++){
  let value=gene(ci,OUTPUT_BASE+o*HIDDEN_COUNT+donor)*0.5;
  set_gene(ci,OUTPUT_BASE+o*HIDDEN_COUNT+donor,value);set_gene(ci,OUTPUT_BASE+o*HIDDEN_COUNT+new_unit,value);
 }
}
fn inherit_child(ci:u32,pi:u32,seed:u32) {
 for(var k=0u;k<GENOME_SIZE;k++){set_gene(ci,k,gene(pi,k));}
 mutate_child(ci,pi,seed);
}
fn mutate_child(ci:u32,pi:u32,seed:u32) {
 var child=agents[ci];
 var rng=seed;let capacity=countOneBits(child.active_mask);
 if(capacity>1u){if(mutation_draw(&rng)<min(0.01*child.topology_mutation_rate,1.0)){
  let h=nth_set(child.active_mask,u32(mutation_draw(&rng)*f32(capacity)));
  child.active_mask&=~(1u<<h);counter_add(33,1u);
 }}
 let remaining=countOneBits(child.active_mask);
 if(remaining<HIDDEN_COUNT){if(mutation_draw(&rng)<min(0.01*child.topology_mutation_rate,1.0)){
  let donor=nth_set(child.active_mask,u32(mutation_draw(&rng)*f32(remaining)));
  let new_unit=nth_clear(child.active_mask,u32(mutation_draw(&rng)*f32(HIDDEN_COUNT-remaining)));
  clone_unit(ci,donor,new_unit,child.active_mask,&rng);
  child.active_mask|=1u<<new_unit;child.plasticity_rate[new_unit]=child.plasticity_rate[donor];counter_add(32,1u);
 }}
 // A birth may be an exact copy. Mutation events include small scale drift;
 // no per-birth temperature redraw or compulsory mutation is applied.
 let magnitude=max(0.03*child.parameter_mutation_step,0.000001);
 let draws=u32(mutation_draw(&rng)<min(0.25*child.parameter_mutation_rate,1.0));
 let expressed_count=countOneBits(child.active_mask);
 let count=expressed_count*(2u+INPUT_COUNT+2u*expressed_count+OUTPUT_COUNT)+OUTPUT_COUNT;
 for(var n=0u;n<draws;n++){
  let k=expressed_index(u32(mutation_draw(&rng)*f32(count)),child.active_mask);let old=gene(ci,k);
  let value=clamp(old+(mutation_draw(&rng)*2.0-1.0)*magnitude,-4.0,4.0);set_gene(ci,k,value);
 }
 if(draws!=0u){let h=nth_set(child.active_mask,u32(mutation_draw(&rng)*f32(expressed_count)));
  child.plasticity_rate[h]=clamp(child.plasticity_rate[h]+(mutation_draw(&rng)*2.0-1.0)*magnitude,-0.2,0.2);
  child.trace_retention=clamp(child.trace_retention+(mutation_draw(&rng)*2.0-1.0)*magnitude,0.0,0.9999);
  child.learned_weight_retention=clamp(child.learned_weight_retention+(mutation_draw(&rng)*2.0-1.0)*magnitude,0.0,0.9999);
  child.parameter_mutation_rate=clamp(child.parameter_mutation_rate*(0.97+0.06*mutation_draw(&rng)),0.25,4.0);
  child.parameter_mutation_step=clamp(child.parameter_mutation_step*(0.97+0.06*mutation_draw(&rng)),0.25,4.0);
  child.topology_mutation_rate=clamp(child.topology_mutation_rate*(0.97+0.06*mutation_draw(&rng)),0.25,4.0);
  child.packet_size=clamp(child.packet_size*(0.9+0.2*mutation_draw(&rng)),1.0,48.0);}
 // Read-only accounting: exact inherited equality, including latent genes.
 let parent=agents[pi];var exact=child.packet_size==parent.packet_size && child.active_mask==parent.active_mask
  && child.trace_retention==parent.trace_retention && child.learned_weight_retention==parent.learned_weight_retention
  && child.parameter_mutation_rate==parent.parameter_mutation_rate && child.parameter_mutation_step==parent.parameter_mutation_step
  && child.topology_mutation_rate==parent.topology_mutation_rate;
 for(var h=0u;h<HIDDEN_COUNT;h++){exact=exact && child.plasticity_rate[h]==parent.plasticity_rate[h];}
 for(var k=0u;k<GENOME_SIZE;k++){exact=exact && bitcast<u32>(gene(ci,k))==bitcast<u32>(gene(pi,k));}
 counter_add(37,u32(exact));
 agents[ci]=child;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let rank=id.x;if(rank>=min(free_prefix[INVALID-1u],birth_prefix[INVALID-1u])){return;}
 let parent_rank=(rank+hash_u32(params.tick)%birth_prefix[INVALID-1u])%birth_prefix[INVALID-1u];
 let pi=parents[parent_rank];let ci=free_indices[rank];let child=agents[ci];
 if(child.alive==0u || child.birth_tick!=params.tick || child.birth_parent_slot!=pi){return;}
 for(var k=0u;k<GENOME_SIZE;k++){set_gene(ci,k,gene(pi,k));}
}

// One workgroup per packet copies independent inherited parameters contiguously.
@compute @workgroup_size(64)
fn parallel(@builtin(workgroup_id) id:vec3<u32>, @builtin(local_invocation_index) lane:u32) {
 let rank=id.x;if(rank>=min(free_prefix[INVALID-1u],birth_prefix[INVALID-1u])){return;}
 let parent_rank=(rank+hash_u32(params.tick)%birth_prefix[INVALID-1u])%birth_prefix[INVALID-1u];
 let pi=parents[parent_rank];let ci=free_indices[rank];let child=agents[ci];
 if(child.alive==0u || child.birth_tick!=params.tick || child.birth_parent_slot!=pi){return;}
 for(var k=lane;k<GENOME_SIZE;k+=64u){set_gene(ci,k,gene(pi,k));}
}

@compute @workgroup_size(64)
fn fusion(@builtin(global_invocation_id) id:vec3<u32>){
 let pi=id.x;if(pi>=params.agent_count||claims[2u*INVALID+pi]!=INVALID){return;}
 let qi=claims[INVALID+pi];if(qi>=params.agent_count){return;}let child=agents[qi];
 if(child.alive!=ORGANISM||child.birth_tick!=params.tick||child.birth_parent_slot!=pi){return;}
 let seed=agents[pi].rng^params.tick;
 recombine_child(qi,pi,qi,seed^0xa51293bdu);
 mutate_child(qi,pi,seed);
}
// Accounting horizons are explicit engine limits, never ecological extinction.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicStore(&stats[36],1u);}
 return prior;
}

// A module owns its input/recurrent/gate rows, biases, outgoing readout,
// expression bit and plasticity rate. Global scalar traits segregate blindly.
fn recombine_child(ci:u32,pi:u32,qi:u32,seed:u32) {
 var rng=seed;var child=agents[ci];child.active_mask=0u;
 var donors:array<u32,HIDDEN_COUNT>;
 // Blind redraw only if segregation produced no expressed units.
 loop {child.active_mask=0u;for(var h=0u;h<HIDDEN_COUNT;h++){let donor=select(pi,qi,mutation_draw(&rng)<0.5);donors[h]=donor;child.active_mask|=source_agents[donor].active_mask&(1u<<h);}if(child.active_mask!=0u){break;}}
 for(var h=0u;h<HIDDEN_COUNT;h++) {
 let donor=donors[h];let p=source_agents[donor];
 child.active_mask|=p.active_mask&(1u<<h);child.plasticity_rate[h]=p.plasticity_rate[h];
 set_gene(ci,NODE_BIAS+h,gene(donor,NODE_BIAS+h));set_gene(ci,GATE_BIAS+h,gene(donor,GATE_BIAS+h));
 for(var k=0u;k<INPUT_COUNT;k++){let at=INPUT_BASE+h*INPUT_COUNT+k;set_gene(ci,at,gene(donor,at));}
 for(var k=0u;k<HIDDEN_COUNT;k++){let at=h*HIDDEN_COUNT+k;set_gene(ci,RECURRENT_BASE+at,gene(donor,RECURRENT_BASE+at));set_gene(ci,GATE_BASE+at,gene(donor,GATE_BASE+at));}
 for(var o=0u;o<OUTPUT_COUNT;o++){let at=OUTPUT_BASE+o*HIDDEN_COUNT+h;set_gene(ci,at,gene(donor,at));}
 }
 for(var o=0u;o<OUTPUT_COUNT;o++){let donor=select(pi,qi,mutation_draw(&rng)<0.5);set_gene(ci,OUTPUT_BIAS+o,gene(donor,OUTPUT_BIAS+o));}
 child.packet_size=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].packet_size;
 child.trace_retention=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].trace_retention;
 child.learned_weight_retention=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].learned_weight_retention;
 child.parameter_mutation_rate=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].parameter_mutation_rate;
 child.parameter_mutation_step=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].parameter_mutation_step;
 child.topology_mutation_rate=source_agents[select(pi,qi,mutation_draw(&rng)<0.5)].topology_mutation_rate;
 agents[ci]=child;
}
