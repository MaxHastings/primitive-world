@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> resources:array<u32>;
@group(0) @binding(2) var<storage,read_write> ground:array<Ground>;
@group(0) @binding(3) var<storage,read> occupancy:array<u32>;
@group(0) @binding(4) var<storage,read> offsets:array<u32>;
@group(0) @binding(5) var<storage,read> indices:array<u32>;
@group(0) @binding(6) var<storage,read_write> perceptions:array<Perception>;
@group(0) @binding(7) var<uniform> params:SimParams;
@group(0) @binding(8) var<storage,read> live_slots:array<u32>;
// SPATIAL_ITERATION

// Eight lanes share one organism. The compact list dispatches eight organisms
// per workgroup, so no extra pass or indirect argument is needed.
var<workgroup> food_sums:array<atomic<u32>,128>;
var<workgroup> food_counts:array<atomic<u32>,128>;
fn perception_to_body(v:vec2<f32>,c:f32,s:f32)->vec2<f32>{return vec2<f32>(c*v.x-s*v.y,s*v.x+c*v.y);}

@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_index) local:u32){
 let subgroup=local/8u;let lane=local%8u;
 let rank=group.x*8u+subgroup;
 // All lanes participate in both barriers, including the unused tail.
 atomicStore(&food_sums[local],0u);atomicStore(&food_sums[local+64u],0u);
 atomicStore(&food_counts[local],0u);atomicStore(&food_counts[local+64u],0u);
 workgroupBarrier();
 var i=0u;var a:Agent;var organism=false;
 if(rank<live_slots[3]){i=live_slots[4u+rank];a=agents[i];organism=a.alive==ORGANISM;}
 if(organism){
  let r=a.sensor_radius;let r2=r*r;let near2=r2*0.25;
  let cell_size=params.world_size.xy/512.0;
  let lo=vec2<i32>(floor((a.position-vec2<f32>(r))/cell_size));
  let hi=vec2<i32>(floor((a.position+vec2<f32>(r))/cell_size));
  let width=u32(hi.x-lo.x+1);let height=u32(hi.y-lo.y+1);
  let c=cos(-a.heading);let s=sin(-a.heading);
  for(var n=lane;n<width*height;n+=8u){
   let x=lo.x+i32(n%width);let y=lo.y+i32(n/width);
   let cell=vec2<u32>(wrap_grid_index(x,512),wrap_grid_index(y,512));
   let center=(vec2<f32>(f32(cell.x),f32(cell.y))+vec2<f32>(0.5))*cell_size;
   let delta=torus_delta(a.position,center,params.world_size.xy);
   let distance2=dot(delta,delta);if(distance2>r2){continue;}
   let sector=sensory_sector(perception_to_body(delta,c,s));
   let region=sector+select(0u,8u,distance2>near2);
   let cell_index=cell.y*512u+cell.x;
   let milli=resources[cell_index]+min(atomicLoad(&ground[cell_index].dropped),8000u);
   atomicAdd(&food_sums[subgroup*16u+region],milli);
   atomicAdd(&food_counts[subgroup*16u+region],1u);
  }
 }
 workgroupBarrier();
 if(lane!=0u||rank>=live_slots[3]){return;}
 if(!organism){var empty:Perception;perceptions[i]=empty;return;}
 var p:Perception;
 let here=ground_index(a.position,params.world_size.xy);
 p.resource_here=f32(resources[here]+min(atomicLoad(&ground[here].dropped),8000u))/1000.0;
 for(var k=0u;k<16u;k++){
  let at=subgroup*16u+k;
  p.regions[k].food=(f32(atomicLoad(&food_sums[at]))/1000.0)/f32(max(1u,atomicLoad(&food_counts[at])));
 }
 let r=a.sensor_radius;let r2=r*r;let near2=r2*0.25;
 let c=cos(-a.heading);let s=sin(-a.heading);
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
    let region=sensory_sector(perception_to_body(delta,c,s))+select(0u,8u,distance2>near2);
    p.regions[region].bodies+=1.0;p.regions[region].velocity+=perception_to_body(b.velocity-a.velocity,c,s);p.nearby_count+=1.0;
    if(b.signal_tick==params.tick && b.signal_high==params.clock.x && (params.tick>0u || params.clock.x>0u)){p.regions[region].signal+=b.signal_payload;}
    p.regions[region].pressure+=max(0.0,1.0-sqrt(distance2)/r);
   }
  }
 }
 for(var k=0u;k<16u;k++){let count=max(1.0,p.regions[k].bodies);p.regions[k].velocity/=count;p.regions[k].signal=clamp(p.regions[k].signal/count,-1.0,1.0);p.regions[k].pressure=clamp(p.regions[k].pressure/count,0.0,1.0);}
 perceptions[i]=p;
}
