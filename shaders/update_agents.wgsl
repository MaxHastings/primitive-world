@group(0) @binding(0) var<storage,read> source:array<Agent>;
@group(0) @binding(1) var<storage,read> decisions:array<Decision>;
@group(0) @binding(2) var<storage,read> requests:array<u32>;
@group(0) @binding(3) var<storage,read_write> destination:array<Agent>;
@group(0) @binding(4) var<uniform> params:SimParams;
@group(0) @binding(5) var<storage,read_write> births:array<u32>;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(8) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(9) var<storage,read_write> motion_bound:array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}var a=source[i];births[i]=0u;
 if(id.x==0u){atomicStore(&stats[18],params.tick+1u);atomicStore(&stats[58],params.clock.x+u32(params.tick==0xffffffffu));}
if(a.alive==0u){destination[i]=a;return;}if(a.alive==PACKET){
  a.age+=f32(params.clock.z);
  a.energy=max(0.0,a.energy-f32(params.clock.z)*params.sensor_and_padding.z*pow(a.packet_size,2.0/3.0));
  a.moved=vec2<f32>(0.0);
  for(var sub=0u;sub<params.clock.z;sub++){
   a.moved+=a.velocity;
   a.position=wrap_world(a.position+a.velocity,params.world_size.xy);
   a.velocity*=0.98;
  }
  atomicMax(&motion_bound[0],u32(ceil(length(a.moved)*1000.0)));
  if(a.energy<=0.0){a.alive=0u;}
  destination[i]=a;return;
 }
 let d=decisions[i];a.lived_ticks++;
 let age_before=a.age;
 counter_add(24u+d.selected_action,1u);counter_add(31,d.invalid);
 a.physical_previous[0]=bitcast<u32>(a.energy);a.physical_previous[1]=bitcast<u32>(a.food);
 a.collected=f32(requests[i])/1000.0;a.ingested=0.0;a.spent=0.0;a.received=0.0;a.moved=vec2<f32>(0.0);
 let gather_effort=select(0.0,clamp(d.outputs[1],0.0,1.0),d.invalid==0u);
 // The decision is held, but per-time physiology and motion are integrated at
 // the old biological resolution. Collection enters inventory after substep 0.
 for(var sub=0u;sub<params.clock.z;sub++){
  let amount=min(min(a.food,0.1),max(0.0,reserve_capacity(a.age,params.sensor_and_padding.y)-a.energy)/params.resource_and_noise.y);
  a.food-=amount;a.energy+=amount*params.resource_and_noise.y;a.ingested+=amount;
  let food_units=u32(round(amount*1000.0));let food_before=counter_add(0,food_units);if(food_before>0xffffffffu-food_units){counter_add(14,1u);}
  if(sub==0u){a.food+=a.collected;}
  let gather_cost=min(a.energy,gather_effort*GATHER_EFFORT_COST);a.energy-=gather_cost;a.spent+=gather_cost;
  let juvenile=0.6+0.4*clamp(a.age/max(params.sensor_and_padding.y,1.0),0.0,1.0);
  var turn_effort=d.movement.y;
  let requested_turn_cost=abs(turn_effort)*TURN_EFFORT_COST;
  if(requested_turn_cost>a.energy){turn_effort*=a.energy/max(requested_turn_cost,0.00001);}
  let turn_cost=min(a.energy,abs(turn_effort)*TURN_EFFORT_COST);a.energy-=turn_cost;a.spent+=turn_cost;
  a.angular_velocity=ANGULAR_DAMPING*a.angular_velocity+turn_effort*TORQUE_STRENGTH;
  a.heading=a.heading+a.angular_velocity;a.heading=a.heading-6.283185307*floor(a.heading/6.283185307);
  var thrust=body_to_world(vec2<f32>(d.movement.x,0.0),a.heading)*a.max_speed*juvenile*0.15;
  let requested_cost=length(thrust)/0.15*params.time_and_costs.z;
  if(requested_cost>a.energy){thrust*=a.energy/max(requested_cost,0.00001);}
  a.velocity=a.velocity*0.85+thrust;let movement=a.velocity;
  a.position=wrap_world(a.position+movement,params.world_size.xy);a.moved+=movement;
  let movement_cost=length(thrust)/0.15*params.time_and_costs.z;a.spent+=movement_cost;a.energy=max(0.0,a.energy-movement_cost);
  let body_cost=min(a.energy,params.time_and_costs.w);a.energy-=body_cost;
  a.spent+=body_cost;
  a.distance_travelled+=length(movement);a.age+=1.0;
  if(a.energy<=0.0||a.age>=a.max_age){a.alive=0u;if(a.age>=a.max_age){counter_add(2,1u);}else{counter_add(1,1u);}break;}
 }
 if(a.v46_natural==1u && age_before<params.sensor_and_padding.y && a.age>=params.sensor_and_padding.y){
  let prior=atomicAdd(&stats[85],1u);
  if(prior==0xffffffffu){atomicAdd(&stats[86],1u);}
 }
 a.action=d.selected_action;a.hidden=d.hidden;a.rng=hash_u32(a.rng+params.tick+1u);
 atomicMax(&motion_bound[0],u32(ceil(length(a.moved)*1000.0)));
 if(a.alive!=0u && d.selected_action==EMIT && params.physical.y>=0.5){a.signal_payload=d.payload;a.signal_tick=params.tick+1u;a.signal_high=params.clock.x+u32(params.tick==0xffffffffu);let sequence=counter_add(8,1u);events[sequence%65536u]=InteractionEvent(params.tick,i,INVALID,EMIT,d.payload,sequence,a.lineage_id,0u,a.position,vec2<f32>(0.0),0u,0u,params.clock.x,a.lineage_high,0u,0u);counter_add(9,1u);}
 destination[i]=a;
}
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicAdd(&stats[40u+index],1u);}
 return prior;
}
