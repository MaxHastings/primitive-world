@group(0) @binding(0) var<storage,read> source:array<Agent>;
@group(0) @binding(1) var<storage,read> decisions:array<Decision>;
@group(0) @binding(2) var<storage,read> requests:array<u32>;
@group(0) @binding(3) var<storage,read_write> destination:array<Agent>;
@group(0) @binding(4) var<uniform> params:SimParams;
@group(0) @binding(5) var<storage,read_write> births:array<u32>;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(8) var<storage,read_write> events:array<InteractionEvent>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}var a=source[i];births[i]=0u;if(a.alive==0u){destination[i]=a;return;}let d=decisions[i];a.lived_ticks++;
 if(id.x==0u){atomicStore(&stats[18],params.tick+1u);}counter_add(24u+d.selected_action,1u);counter_add(31,d.invalid);
 a.physical_previous[0]=bitcast<u32>(a.energy);a.physical_previous[1]=bitcast<u32>(a.food);
 a.collected=f32(requests[i])/1000.0;a.ingested=0.0;a.spent=0.0;a.received=0.0;a.food+=a.collected;
 let amount=min(min(a.food,0.1),max(0.0,100.0-a.energy)/params.resource_and_noise.y);a.food-=amount;a.energy+=amount*params.resource_and_noise.y;a.ingested=amount;
 let food_units=u32(round(amount*1000.0));let food_before=counter_add(0,food_units);if(food_before>0xffffffffu-food_units){counter_add(14,1u);}
 let gather_effort=select(0.0,clamp(d.outputs[1],0.0,1.0),d.invalid==0u);let gather_cost=min(a.energy,gather_effort*GATHER_EFFORT_COST);a.energy-=gather_cost;a.spent+=gather_cost;
 let juvenile=0.6+0.4*clamp(a.age/max(params.sensor_and_padding.y,1.0),0.0,1.0);
 // Heading and velocity are physical lifetime state.  Turning is body-frame
 // effort; thrust is forward only, while existing velocity damps naturally.
 a.heading=a.heading+d.movement.y*0.25;a.heading=a.heading-6.283185307*floor(a.heading/6.283185307);
 var thrust=body_to_world(vec2<f32>(d.movement.x,0.0),a.heading)*a.max_speed*juvenile*0.15;let requested_cost=length(thrust)/0.15*params.time_and_costs.z;if(requested_cost>a.energy){thrust*=a.energy/max(requested_cost,0.00001);}
 a.velocity=a.velocity*0.85+thrust;let movement=a.velocity;
 a.position=wrap_world(a.position+movement,params.world_size.xy);a.moved=movement;let distance=length(thrust);let movement_cost=distance/0.15*params.time_and_costs.z;a.spent+=movement_cost;a.energy=max(0.0,a.energy-movement_cost);
 // Body and cognition have distinct costs.  There is no privileged eight-unit
 // baseline: each expressed unit pays the same incremental upkeep.
 let body_cost=min(a.energy,params.time_and_costs.w);a.energy-=body_cost;
 let unit_cost=min(a.energy,params.mutation.w*f32(countOneBits(a.active_mask)));a.energy-=unit_cost;a.spent+=body_cost+unit_cost;counter_add(35,u32(round(unit_cost*1000.0)));
 a.distance_travelled+=length(movement);a.age+=1.0;a.action=d.selected_action;a.hidden=d.hidden;a.rng=hash_u32(a.rng+params.tick+1u);
 if(a.energy<=0.0||a.age>=a.max_age){a.alive=0u;if(a.age>=a.max_age){counter_add(2,1u);}else{counter_add(1,1u);}}
 let signal_cost=SIGNAL_ACTIVATION_COST+SIGNAL_AMPLITUDE_COST*abs(d.payload);if(a.alive!=0u && d.selected_action==EMIT && params.physical.y>=0.5 && a.energy>=signal_cost){a.energy-=signal_cost;a.spent+=signal_cost;a.signal_payload=d.payload;a.signal_tick=params.tick+1u;let sequence=counter_add(8,1u);events[sequence%65536u]=InteractionEvent(params.tick,i,INVALID,EMIT,d.payload,sequence,a.lineage_id,0u,a.position,vec2<f32>(0.0),0u,0u);counter_add(9,1u);}
 if(a.alive!=0u && d.selected_action==REPRODUCE){counter_add(20,1u);let birth_cost=params.sensor_and_padding.w*(0.2+0.8*d.amount);let mature=a.age>=params.sensor_and_padding.y;let energetic=a.energy>=birth_cost;let ready=params.tick>=a.next_birth;counter_add(16,u32(!mature));counter_add(17,u32(!energetic));counter_add(19,u32(!ready));births[i]=u32(mature&&energetic&&ready);counter_add(21,births[i]);}
 destination[i]=a;
}

// Accounting horizons are explicit engine limits, never ecological extinction.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicStore(&stats[36],1u);}
 return prior;
}
