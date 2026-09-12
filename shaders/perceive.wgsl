@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> resources:array<u32>;
@group(0) @binding(2) var<storage,read_write> ground:array<Ground>;
@group(0) @binding(3) var<storage,read> occupancy:array<u32>;
@group(0) @binding(4) var<storage,read> offsets:array<u32>;
@group(0) @binding(5) var<storage,read> indices:array<u32>;
@group(0) @binding(6) var<storage,read_write> perceptions:array<Perception>;
@group(0) @binding(7) var<uniform> params:SimParams;
// SPATIAL_ITERATION
// Exact reference paths remain available for paired performance probes.
const HOISTED_ROTATION:bool=true;
fn food_at_index(i:u32)->f32{return f32(resources[i]+min(atomicLoad(&ground[i].dropped),8000u))/1000.0;}
fn perception_to_body(v:vec2<f32>,heading:f32,c:f32,s:f32)->vec2<f32>{if(!HOISTED_ROTATION){return world_to_body(v,heading);}return vec2<f32>(c*v.x-s*v.y,s*v.x+c*v.y);}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];var p:Perception;
 if(a.alive!=ORGANISM){perceptions[i]=p;return;}
 let body_heading=-a.heading;let body_c=cos(body_heading);let body_s=sin(body_heading);
 p.resource_here=food_at_index(ground_index(a.position,params.world_size.xy));
 let r=a.sensor_radius;let r2=r*r;let near2=r2*0.25;
 // Integrate every food-cell center in the local disk. Grid resolution,
 // rather than isolated probes, determines the remaining spatial aliasing.
 var cells:array<u32,16>;
 let food_cell_size=params.world_size.xy/512.0;
 let lo=vec2<i32>(floor((a.position-vec2<f32>(r))/food_cell_size));
 let hi=vec2<i32>(floor((a.position+vec2<f32>(r))/food_cell_size));
 for(var y=lo.y;y<=hi.y;y++){
  for(var x=lo.x;x<=hi.x;x++){
   let cell=vec2<u32>(wrap_grid_index(x,512),wrap_grid_index(y,512));
   let center=(vec2<f32>(f32(cell.x),f32(cell.y))+vec2<f32>(0.5))*food_cell_size;
   let delta=torus_delta(a.position,center,params.world_size.xy);
   let distance2=dot(delta,delta);if(distance2>r2){continue;}
   let region=sensory_sector(perception_to_body(delta,a.heading,body_c,body_s))+select(0u,8u,distance2>near2);
   p.regions[region].food+=food_at_index(cell.y*512u+cell.x);cells[region]++;
  }
 }
 for(var k=0u;k<16u;k++){p.regions[k].food/=f32(max(1u,cells[k]));}
 let body_cell_size=params.world_size.xy/256.0;
 let blo=vec2<i32>(floor((a.position-vec2<f32>(r))/body_cell_size));
 let bhi=vec2<i32>(floor((a.position+vec2<f32>(r))/body_cell_size));
 for(var y=blo.y;y<=bhi.y;y++){
  for(var x=blo.x;x<=bhi.x;x++){
   let cell=vec2<u32>(wrap_grid_index(x,256),wrap_grid_index(y,256));
   let ci=cell.y*256u+cell.x;if(occupancy[ci]==0u){continue;}
   for(var j=spatial_first(ci);j!=spatial_end(ci);j=spatial_next(j)){
    let other=spatial_slot(j);if(other==i||other>=INVALID){continue;}
    let b=agents[other];if(b.alive==0u){continue;}
    let delta=torus_delta(a.position,b.position,params.world_size.xy);let distance2=dot(delta,delta);if(distance2>r2){continue;}
    let region=sensory_sector(perception_to_body(delta,a.heading,body_c,body_s))+select(0u,8u,distance2>near2);
    p.regions[region].bodies+=1.0;p.regions[region].velocity+=perception_to_body(b.velocity-a.velocity,a.heading,body_c,body_s);p.nearby_count+=1.0;
    // Signal is an aggregate signed activity, never a sender record.
    if(b.signal_tick==params.tick && b.signal_high==params.clock.x && (params.tick>0u || params.clock.x>0u)){p.regions[region].signal+=b.signal_payload;}
    // Smooth bounded proximity/contact pressure, shared by every sample.
    p.regions[region].pressure+=max(0.0,1.0-sqrt(distance2)/r);
   }
  }
 }
 for(var k=0u;k<16u;k++){let count=max(1.0,p.regions[k].bodies);p.regions[k].velocity/=count;p.regions[k].signal=clamp(p.regions[k].signal/count,-1.0,1.0);p.regions[k].pressure=clamp(p.regions[k].pressure/count,0.0,1.0);}
 perceptions[i]=p;
}
