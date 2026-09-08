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
 if(p.energy<cost){atomicAdd(&stats[17],1u);return;}
 var child:Agent;let angle=random01(p.rng)*6.2831853;
 child.position=wrap_world(p.position+vec2<f32>(cos(angle),sin(angle))*2.0,params.world_size.xy);
 child.energy=child_energy;child.food=0.0;p.energy-=cost;p.spent+=cost;p.next_birth=params.tick+params.lifecycle.y;p.lifetime_births++;
 if(p.energy<=0.0){p.alive=0u;atomicAdd(&stats[1],1u);}
 child.max_speed=p.max_speed;child.sensor_radius=p.sensor_radius;child.max_age=9000.0+2000.0*random01(p.rng^ci);
 // The banked inheritance pass below owns all genome and topology writes.
 child.active_mask=p.active_mask;child.plasticity_rate=p.plasticity_rate;child.trace_retention=p.trace_retention;child.learned_weight_retention=p.learned_weight_retention;child.mutation_scale=p.mutation_scale;
 child.alive=1u;child.rng=hash_u32(p.rng^ci^params.tick);child.generation=agents[ci].generation+1u;child.cognitive_padding=0u;
 child.target_id=INVALID;child.lineage_id=atomicAdd(&stats[10],1u)+INVALID+1u;child.parent_lineage=p.lineage_id;child.birth_tick=params.tick;child.birth_parent_slot=pi;
 child.ancestry_depth=p.ancestry_depth+1u;atomicMax(&stats[23],child.ancestry_depth);child.founder_family=p.founder_family;
 agents[pi]=p;agents[ci]=child;atomicAdd(&stats[3],1u);atomicAdd(&stats[22],1u);
}
