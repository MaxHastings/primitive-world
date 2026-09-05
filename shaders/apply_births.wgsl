@group(0) @binding(0) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> free_indices:array<u32>;
@group(0) @binding(2) var<storage,read> free_prefix:array<u32>;
@group(0) @binding(3) var<storage,read> parents:array<u32>;
@group(0) @binding(4) var<storage,read> birth_prefix:array<u32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(7) var<storage,read> decisions:array<Decision>;
@group(0) @binding(8) var<storage,read_write> genomes:array<f32>;
// BRAIN_MUTATION
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let rank=id.x;if(rank>=min(free_prefix[INVALID-1u],birth_prefix[INVALID-1u])){return;}
 let parent_rank=(rank+hash_u32(params.tick)%birth_prefix[INVALID-1u])%birth_prefix[INVALID-1u];
 let pi=parents[parent_rank];let ci=free_indices[rank];var p=agents[pi];let d=decisions[pi];
 if(p.alive==0u||agents[ci].alive!=0u||d.selected_action!=REPRODUCE){return;}
 // Construct privately before charging. Rejected births neither publish genomes
 // nor spend parental reserves, including when extra structure is unaffordable.
 var genes:array<f32,GENOME_SIZE>;
 for(var k=0u;k<GENOME_SIZE;k++){genes[k]=genomes[pi*GENOME_SIZE+k];}
 mutate_brain(&genes,p.rng^ci^params.tick,params.mutation);
 let nodes=u32(genes[0]);let edges=u32(genes[1]);
 let encoded=2u+2u*nodes+OUTPUT_COUNT+3u*edges;
 let child_energy=params.sensor_and_padding.w*0.8*d.amount;
 let cost=params.sensor_and_padding.w*0.2+child_energy+params.resource_and_noise.w*f32(encoded);
 if(p.energy<cost){atomicAdd(&stats[17],1u);return;}
 var child:Agent;let angle=random01(p.rng)*6.2831853;
 child.position=clamp(p.position+vec2<f32>(cos(angle),sin(angle))*2.0,vec2<f32>(0),vec2<f32>(params.world_size));
 child.energy=child_energy;child.food=0.0;p.energy-=cost;p.spent+=cost;
 p.next_birth=params.tick+params.lifecycle.y;p.lifetime_births++;
 if(p.energy<=0.0){p.alive=0u;atomicAdd(&stats[1],1u);}
 child.max_speed=p.max_speed;child.sensor_radius=p.sensor_radius;
 child.max_age=9000.0+2000.0*random01(p.rng^ci);
 child.brain_nodes=nodes;child.brain_edges=edges;
 child.node_change=i32(nodes)-i32(p.brain_nodes);child.edge_change=i32(edges)-i32(p.brain_edges);
 for(var k=0u;k<GENOME_SIZE;k++){genomes[ci*GENOME_SIZE+k]=genes[k];}
 child.alive=1u;child.rng=hash_u32(p.rng^ci^params.tick);child.generation=agents[ci].generation+1u;
 child.target_id=INVALID;
 child.lineage_id=atomicAdd(&stats[10],1u)+INVALID+1u;
 child.parent_lineage=p.lineage_id;child.birth_tick=params.tick;child.birth_parent_slot=pi;child.ancestry_depth=p.ancestry_depth+1u;
 child.founder_family=p.founder_family;
 agents[pi]=p;agents[ci]=child;atomicAdd(&stats[3],1u);atomicAdd(&stats[22],1u);
}
