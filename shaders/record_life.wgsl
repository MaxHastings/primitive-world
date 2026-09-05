// End-of-tick measurements after all physical effects, before dead inventory is released.
@group(0) @binding(0) var<storage,read> previous:array<Agent>;
@group(0) @binding(1) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(2) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(3) var<uniform> params:SimParams;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}
 // Newborns did not act this tick. Dead slots must never accumulate stale feedback.
 if(previous[i].alive==0u || previous[i].lineage_id!=agents[i].lineage_id){return;}
 var a=agents[i];
 a.life.end_energy=a.energy;a.life.end_food=a.food;
 a.life.energy_sum+=a.energy;a.life.food_sum+=a.food;
 a.life.collected+=a.collected;a.life.consumed+=a.ingested;
 a.life.received+=a.received;a.life.spent+=a.spent;a.life.distance+=length(a.velocity);
 a.life.local_food_sum+=perceptions[i].resource_here;a.life.nearby_sum+=perceptions[i].nearby_count;
 if(a.alive==0u){a.life.death_tick=params.tick+1u;}
 agents[i]=a;
}
