// Manufacture into free slots. Fusion reuses a consumed packet slot, so it
// remains possible at full storage capacity.
@group(0) @binding(0) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> free_indices:array<u32>;
@group(0) @binding(2) var<storage,read> free_prefix:array<u32>;
@group(0) @binding(3) var<storage,read> parents:array<u32>;
@group(0) @binding(4) var<storage,read> birth_prefix:array<u32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(7) var<storage,read> decisions:array<Decision>;
@group(0) @binding(8) var<storage,read> claims:array<u32>;
fn fresh(p:Agent,pi:u32,ci:u32)->Agent {
 var child:Agent;
 child.packet_size=p.packet_size;child.active_mask=p.active_mask;
 child.plasticity_rate=p.plasticity_rate;child.trace_retention=p.trace_retention;
 child.learned_weight_retention=p.learned_weight_retention;
 child.parameter_mutation_rate=p.parameter_mutation_rate;
 child.parameter_mutation_step=p.parameter_mutation_step;
 child.topology_mutation_rate=p.topology_mutation_rate;
 child.max_speed=p.max_speed;child.sensor_radius=p.sensor_radius;
 child.max_age=9000.0+2000.0*random01(p.rng^params.tick^0x13579bdfu);
 child.rng=hash_u32(p.rng^params.tick^ci);
 child.generation=agents[ci].generation+1u;
 child.birth_tick=params.tick;child.birth_high=params.clock.x;child.birth_parent_slot=pi;
 child.founder_family=p.founder_family;
 child.lineage_id=counter_add(10,1u)+INVALID+1u;child.lineage_high=atomicLoad(&stats[50]);
 return child;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let rank=id.x;if(rank>=min(free_prefix[INVALID-1u],birth_prefix[INVALID-1u])){return;}
 let parent_rank=(rank+hash_u32(params.tick)%birth_prefix[INVALID-1u])%birth_prefix[INVALID-1u];
 let pi=parents[parent_rank];let ci=free_indices[rank];var p=agents[pi];let d=decisions[pi];
 if(p.alive!=ORGANISM||agents[ci].alive!=0u||d.selected_action!=PRODUCE_PACKET||p.energy<p.packet_size){return;}
 var packet=fresh(p,pi,ci);packet.alive=PACKET;packet.energy=p.packet_size;
 packet.position=wrap_world(p.position+body_to_world(d.placement,p.heading)*2.0,params.world_size.xy);
 // Deposited packets have no propulsion or inherited body velocity.
 packet.parent_lineage=p.lineage_id;packet.parent_high=p.lineage_high;packet.ancestry_depth=p.ancestry_depth;
 p.energy-=p.packet_size;p.spent+=p.packet_size;p.packets_produced++;
 if(p.energy<=0.0){p.alive=0u;counter_add(1,1u);}
 agents[pi]=p;agents[ci]=packet;counter_add(19,1u);
}
@compute @workgroup_size(64)
fn fusion(@builtin(global_invocation_id) id:vec3<u32>){
 let pi=id.x;if(pi>=params.agent_count||claims[2u*INVALID+pi]!=INVALID){return;}
 let qi=claims[INVALID+pi];if(qi>=params.agent_count){return;}
 var p=agents[pi];var q=agents[qi];
 if(p.alive!=PACKET||q.alive!=PACKET||(p.parent_lineage==q.parent_lineage && p.parent_high==q.parent_high)){return;}
 let energy=p.energy+q.energy;
 var child=fresh(p,pi,qi);
 child.position=wrap_world(p.position+torus_delta(p.position,q.position,params.world_size.xy)*0.5,params.world_size.xy);
 p.alive=0u;q.alive=0u;p.energy=0.0;q.energy=0.0;
 agents[pi]=p;agents[qi]=q;
 // Physical fusion can exhaust its reserves without constructing a body.
 if(energy<=params.sensor_and_padding.w){counter_add(38,1u);return;}
 child.alive=ORGANISM;child.energy=energy-params.sensor_and_padding.w;
 child.heading=6.283185307*random01(child.rng);
 child.parent_lineage=p.parent_lineage;child.parent_high=p.parent_high;
 child.ancestry_depth=max(p.ancestry_depth,q.ancestry_depth)+1u;
 atomicMax(&stats[23],child.ancestry_depth);
 agents[qi]=child;counter_add(3,1u);counter_add(22,1u);
}
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicAdd(&stats[40u+index],1u);}
 return prior;
}
