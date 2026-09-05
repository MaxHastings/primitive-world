@group(0) @binding(0) var<storage,read> source:array<Agent>;
@group(0) @binding(1) var<storage,read> decisions:array<Decision>;
@group(0) @binding(2) var<storage,read> requests:array<u32>;
@group(0) @binding(3) var<storage,read_write> destination:array<Agent>;
@group(0) @binding(4) var<uniform> params:SimParams;
@group(0) @binding(5) var<storage,read_write> births:array<u32>;
@group(0) @binding(6) var<storage,read_write> stats:array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}var a=source[i];births[i]=0u;
 if(a.alive==0u){destination[i]=a;return;}let d=decisions[i];
 atomicAdd(&stats[24u+d.selected_action],1u);atomicAdd(&stats[31],d.invalid);
 a.collected=f32(requests[i])/1000.0;a.ingested=0.0;a.spent=0.0;a.received=0.0;
 a.food+=a.collected;
 // Digestion is rate-limited physiology, not a brain action or resource gift.
 // Gathering is still chosen; every absorbed unit is removed from inventory.
 {
  let amount=min(min(a.food,0.1),max(0.0,100.0-a.energy)/params.resource_and_noise.y);
  a.food-=amount;a.energy+=amount*params.resource_and_noise.y;a.ingested=amount;
  atomicAdd(&stats[0],u32(round(amount*1000.0)));
 }
 let juvenile=0.6+0.4*clamp(a.age/max(params.sensor_and_padding.y,1.0),0.0,1.0);
 var movement=d.movement*a.max_speed*juvenile;
 let cost=length(movement)*params.time_and_costs.z;
 if(cost>a.energy){movement*=a.energy/max(cost,0.00001);}
 let old=a.position;a.position=clamp(old+movement,vec2<f32>(0),vec2<f32>(params.world_size));
 a.velocity=a.position-old;a.moved=a.velocity;let distance=length(a.velocity);
 a.spent=distance*params.time_and_costs.z;a.energy=max(0.0,a.energy-a.spent);
 let metabolism=min(a.energy,params.time_and_costs.w);a.energy-=metabolism;a.spent+=metabolism;
 a.distance_travelled+=distance;a.age+=1.0;a.action=d.selected_action;a.target_id=d.target_id;
 // A local emission is body output, not an exclusive claim on a recipient.
 // Only full affordable emissions occur. Receivers sample it next tick.
 if(d.selected_action==EMIT && params.physical.y>=0.5 && a.energy>=0.02){
  a.energy-=0.02;a.spent+=0.02;a.signal_payload=d.payload;a.signal_tick=params.tick+1u;
  atomicAdd(&stats[9],1u);
 }
 a.hidden=d.hidden;a.rng=hash_u32(a.rng+params.tick+1u);
 if(a.energy<=0.0||a.age>=a.max_age){a.alive=0u;if(a.age>=a.max_age){atomicAdd(&stats[2],1u);}else{atomicAdd(&stats[1],1u);}}
 if(a.alive!=0u && d.selected_action==REPRODUCE){
  atomicAdd(&stats[20],1u);
  let cost=params.sensor_and_padding.w*(0.2+0.8*d.amount);
  let mature=a.age>=params.sensor_and_padding.y;let energetic=a.energy>=cost;
  let ready=params.tick>=a.next_birth;
  atomicAdd(&stats[16],u32(!mature));atomicAdd(&stats[17],u32(!energetic));
  atomicAdd(&stats[19],u32(!ready));
  births[i]=u32(mature&&energetic&&ready);atomicAdd(&stats[21],births[i]);
 }
 destination[i]=a;
}
