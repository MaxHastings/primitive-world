// Read-only opportunity geometry after movement and before contact allocation.
@group(0) @binding(0) var<storage,read> bodies:array<Agent>;
@group(0) @binding(1) var<uniform> params:SimParams;
@group(0) @binding(2) var<storage,read> decisions:array<Decision>;
@group(0) @binding(3) var<storage,read> offsets:array<u32>;
@group(0) @binding(4) var<storage,read> indices:array<u32>;
// SPATIAL_ITERATION
struct ContactLife {lineage:u32,generation:u32,body_ticks:u32,food_ticks:u32,intent_ticks:u32,observed_ticks:u32,padding1:u32,padding2:u32,};
@group(0) @binding(5) var<storage,read_write> lives:array<ContactLife>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=bodies[i];
 if(a.alive!=ORGANISM || a.ancestry_depth==0u || a.age>=params.sensor_and_padding.y){return;}
 var life=lives[i];if(life.lineage!=a.lineage_id || life.generation!=a.generation){life=ContactLife(a.lineage_id,a.generation,0u,0u,0u,0u,0u,0u);}
 var near=false;var food=false;var intent=false;
 let size=params.world_size.xy/256.0;let base=vec2<i32>(floor(wrap_world(a.position,params.world_size.xy)/size));
 let reach=vec2<i32>(ceil(vec2<f32>(INTERACTION_RADIUS)/size));
 for(var oy=-reach.y;oy<=reach.y;oy++){for(var ox=-reach.x;ox<=reach.x;ox++){
  let cell=vec2<u32>(wrap_grid_index(base.x+ox,256),wrap_grid_index(base.y+oy,256));let ci=cell.y*256u+cell.x;
  for(var k=spatial_first(ci);k!=spatial_end(ci);k=spatial_next(k)){
   let j=spatial_slot(k);if(j==i){continue;}let b=bodies[j];if(b.alive!=ORGANISM){continue;}
   let delta=torus_delta(a.position,b.position,params.world_size.xy);if(dot(delta,delta)>INTERACTION_RADIUS*INTERACTION_RADIUS){continue;}
   near=true;
   if(b.food>0.0 && a.food<inventory_capacity(a.age,params.sensor_and_padding.y)){
    food=true;if(decisions[j].selected_action==TRANSFER){intent=true;}
   }
  }
 }}
 life.observed_ticks++;life.body_ticks+=u32(near);life.food_ticks+=u32(food);life.intent_ticks+=u32(intent);lives[i]=life;
}
