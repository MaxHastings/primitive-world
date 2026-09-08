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
 let child_energy=params.sensor_and_padding.w*0.8*d.amount;
 let cost=params.sensor_and_padding.w*0.2+child_energy;
 if(p.energy<cost){atomicAdd(&stats[17],1u);return;}
 var child:Agent;let angle=random01(p.rng)*6.2831853;
 child.position=clamp(p.position+vec2<f32>(cos(angle),sin(angle))*2.0,vec2<f32>(0),params.world_size.xy);
 child.energy=child_energy;child.food=0.0;p.energy-=cost;p.spent+=cost;
 p.next_birth=params.tick+params.lifecycle.y;p.lifetime_births++;
 if(p.energy<=0.0){p.alive=0u;atomicAdd(&stats[1],1u);}
 child.max_speed=p.max_speed;child.sensor_radius=p.sensor_radius;
 child.max_age=9000.0+2000.0*random01(p.rng^ci);
 // Fixed construction cost is checked first. Stream parameters directly into the
 // disjoint child slot, avoiding a full private genome array for each GPU lane.
 var mutation_rng=p.rng^ci^params.tick;
 let scale=sqrt(mutation_temperature(&mutation_rng));
 let mutation_probability=clamp(params.mutation.x*scale,0.0,1.0);
 let mutation_magnitude=max(params.mutation.y*scale,0.000001);
 var changed=false;
 for(var k=0u;k<GENOME_SIZE;k++){
  let old=genomes[pi*GENOME_SIZE+k];
  let next=mutate_parameter(old,&mutation_rng,mutation_probability,mutation_magnitude);
  changed=changed || next!=old;genomes[ci*GENOME_SIZE+k]=next;
 }
 if(!changed){
  let k=min(u32(mutation_draw(&mutation_rng)*f32(GENOME_SIZE)),GENOME_SIZE-1u);
  let old=genomes[pi*GENOME_SIZE+k];let direction=select(-1.0,1.0,mutation_draw(&mutation_rng)>=0.5);
  let next=clamp(old+direction*mutation_magnitude,-4.0,4.0);
  genomes[ci*GENOME_SIZE+k]=select(next,clamp(old-direction*mutation_magnitude,-4.0,4.0),next==old);
 }
 child.alive=1u;child.rng=hash_u32(p.rng^ci^params.tick);child.generation=agents[ci].generation+1u;
 child.target_id=INVALID;
 child.lineage_id=atomicAdd(&stats[10],1u)+INVALID+1u;
 child.parent_lineage=p.lineage_id;child.birth_tick=params.tick;child.birth_parent_slot=pi;child.ancestry_depth=p.ancestry_depth+1u;
 atomicMax(&stats[23],child.ancestry_depth);
 child.founder_family=p.founder_family;

 agents[pi]=p;agents[ci]=child;atomicAdd(&stats[3],1u);atomicAdd(&stats[22],1u);
}
