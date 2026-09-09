@group(0) @binding(0) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> free_indices:array<u32>;
@group(0) @binding(2) var<storage,read> free_prefix:array<u32>;
@group(0) @binding(3) var<storage,read> parents:array<u32>;
@group(0) @binding(4) var<storage,read> birth_prefix:array<u32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(7) var<storage,read> decisions:array<Decision>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let rank=id.x;if(rank>=min(free_prefix[INVALID-1u],birth_prefix[INVALID-1u])){return;}
 let parent_rank=(rank+hash_u32(params.tick)%birth_prefix[INVALID-1u])%birth_prefix[INVALID-1u];
 let pi=parents[parent_rank];let ci=free_indices[rank];var p=agents[pi];let d=decisions[pi];
 if(p.alive==0u||agents[ci].alive!=0u||d.selected_action!=REPRODUCE){return;}
 let child_energy=params.sensor_and_padding.w*0.8*d.amount;let cost=params.sensor_and_padding.w*0.2+child_energy;
 if(p.energy<cost){counter_add(17,1u);return;}
 var child:Agent;
 // Controller-directed placement is bounded and transformed only at this
 // physical boundary; it cannot encode an absolute world direction.
 child.position=wrap_world(p.position+body_to_world(d.placement,p.heading)*2.0,params.world_size.xy);
 child.energy=child_energy;child.food=0.0;p.energy-=cost;p.spent+=cost;p.next_birth=params.tick+params.lifecycle.y;p.lifetime_births++;
 if(p.energy<=0.0){p.alive=0u;counter_add(1,1u);}
 child.max_speed=p.max_speed;child.sensor_radius=p.sensor_radius;child.heading=(p.heading+6.283185307*random01(p.rng^params.tick))%6.283185307;child.velocity=vec2<f32>(0.0);child.max_age=9000.0+2000.0*random01(p.rng^params.tick^0x13579bdfu);
 // The banked inheritance pass below owns all genome and topology writes.
 child.active_mask=p.active_mask;child.plasticity_rate=p.plasticity_rate;child.trace_retention=p.trace_retention;child.learned_weight_retention=p.learned_weight_retention;child.parameter_mutation_rate=p.parameter_mutation_rate;child.parameter_mutation_step=p.parameter_mutation_step;child.topology_mutation_rate=p.topology_mutation_rate;
 child.alive=1u;child.rng=hash_u32(p.rng^params.tick);child.generation=agents[ci].generation+1u;
 child.lineage_id=counter_add(10,1u)+INVALID+1u;child.parent_lineage=p.lineage_id;child.birth_tick=params.tick;child.birth_parent_slot=pi;
 child.ancestry_depth=p.ancestry_depth+1u;atomicMax(&stats[23],child.ancestry_depth);child.founder_family=p.founder_family;
 agents[pi]=p;agents[ci]=child;counter_add(3,1u);counter_add(22,1u);
}

// Accounting horizons are explicit engine limits, never ecological extinction.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicStore(&stats[36],1u);}
 return prior;
}
