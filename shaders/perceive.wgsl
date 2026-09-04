@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> resources:array<u32>;
@group(0) @binding(2) var<storage,read_write> ground:array<Ground>;
@group(0) @binding(3) var<storage,read> occupancy:array<u32>;
@group(0) @binding(4) var<storage,read> offsets:array<u32>;
@group(0) @binding(5) var<storage,read> indices:array<u32>;
@group(0) @binding(6) var<storage,read_write> perceptions:array<Perception>;
@group(0) @binding(7) var<uniform> params:SimParams;
fn food_at(pos:vec2<f32>)->f32{let i=ground_index(pos);return f32(resources[i]+min(atomicLoad(&ground[i].dropped),8000u))/1000.0;}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];var p:Perception;
 for(var k=0u;k<4u;k++){p.bodies[k].slot=INVALID;}
 if(a.alive==0u){perceptions[i]=p;return;}
 p.resource_here=food_at(a.position);
 let center=vec2<i32>(clamp(a.position/8.0,vec2<f32>(0),vec2<f32>(255)));
 p.local_count=f32(occupancy[u32(center.y)*256u+u32(center.x)]);
 var dirs=array<vec2<f32>,4>(vec2<f32>(0,-1),vec2<f32>(1,0),vec2<f32>(0,1),vec2<f32>(-1,0));
 for(var k=0u;k<8u;k++){
  let range=select(a.sensor_radius/6.0,a.sensor_radius,k>=4u);
  let pos=clamp(a.position+rotate(dirs[k%4u],a.attention)*range,vec2<f32>(0),vec2<f32>(params.world_size));
  // Field stores avoid a Sample-valued temporary that intermittently hits
  // Naga24's unsupported HLSL write_value_type(Struct) compilation path.
  p.samples[k].offset=pos-a.position;
  p.samples[k].food=food_at(pos);
  p.samples[k].padding=0.0;
 }
 // Bounded neutral sampling; no preference based on food, conduct or identity.
 let radius=min(6,i32(ceil(a.sensor_radius/8.0)));let side=u32(2*radius+1);let cells=side*side;
 let start_cell=hash_u32(a.rng^params.tick)%cells;var count=0u;
 for(var c=0u;c<cells;c++){
  if(count>=4u){break;}let n=(c+start_cell)%cells;
  let cell=center+vec2<i32>(i32(n%side)-radius,i32(n/side)-radius);
  if(any(cell<vec2<i32>(0))||any(cell>=vec2<i32>(256))){continue;}
  let ci=u32(cell.y)*256u+u32(cell.x);var start=0u;if(ci>0u){start=offsets[ci-1u];}
  let available=offsets[ci]-start;if(available==0u){continue;}
  for(var j=0u;j<min(2u,available);j++){
   if(count>=4u){break;}
   let other=indices[start+(hash_u32(a.rng^ci^params.tick)+j)%available];
   if(other==i||other>=INVALID){continue;}let b=agents[other];
   if(b.alive==0u||length(b.position-a.position)>a.sensor_radius){continue;}
   let event=select(0.0,b.event_amount,b.event_tick+1u==params.tick);
   p.bodies[count]=Body(b.position-a.position,b.velocity,b.food,event,other,b.generation);count++;
  }
 }
 perceptions[i]=p;
}
