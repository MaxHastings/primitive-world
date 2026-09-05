@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> resources:array<u32>;
@group(0) @binding(2) var<storage,read_write> ground:array<Ground>;
@group(0) @binding(3) var<storage,read> occupancy:array<u32>;
@group(0) @binding(4) var<storage,read> offsets:array<u32>;
@group(0) @binding(5) var<storage,read> indices:array<u32>;
@group(0) @binding(6) var<storage,read_write> perceptions:array<Perception>;
@group(0) @binding(7) var<uniform> params:SimParams;
fn food_at_index(i:u32)->f32{return f32(resources[i]+min(atomicLoad(&ground[i].dropped),8000u))/1000.0;}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];var p:Perception;
 for(var k=0u;k<8u;k++){p.bodies[k].slot=INVALID;}
 if(a.alive==0u){perceptions[i]=p;return;}
 p.resource_here=food_at_index(ground_index(a.position));
 let r=a.sensor_radius;let r2=r*r;let near2=r2*0.25;
 // Integrate every food-cell center in the local disk. Grid resolution,
 // rather than isolated probes, determines the remaining spatial aliasing.
 var cells:array<u32,16>;
 let lo=vec2<i32>(clamp(floor((a.position-vec2<f32>(r))/4.0),vec2<f32>(0),vec2<f32>(511)));
 let hi=vec2<i32>(clamp(floor((a.position+vec2<f32>(r))/4.0),vec2<f32>(0),vec2<f32>(511)));
 for(var y=lo.y;y<=hi.y;y++){
  for(var x=lo.x;x<=hi.x;x++){
   let delta=(vec2<f32>(f32(x),f32(y))+vec2<f32>(0.5))*4.0-a.position;
   let distance2=dot(delta,delta);if(distance2>r2){continue;}
   let region=sensory_sector(delta)+select(0u,8u,distance2>near2);
   p.regions[region].food+=food_at_index(u32(y)*512u+u32(x));cells[region]++;
  }
 }
 for(var k=0u;k<16u;k++){p.regions[k].food/=f32(max(1u,cells[k]));}
 // All in-range bodies contribute to crowding; nearest per compass sector
 // is individually observable. No per-tick random subsampling.
 var nearest:array<f32,8>;
 for(var k=0u;k<8u;k++){nearest[k]=r2+1.0;}
 let blo=vec2<i32>(clamp(floor((a.position-vec2<f32>(r))/8.0),vec2<f32>(0),vec2<f32>(255)));
 let bhi=vec2<i32>(clamp(floor((a.position+vec2<f32>(r))/8.0),vec2<f32>(0),vec2<f32>(255)));
 for(var y=blo.y;y<=bhi.y;y++){
  for(var x=blo.x;x<=bhi.x;x++){
   let ci=u32(y)*256u+u32(x);if(occupancy[ci]==0u){continue;}
   var start=0u;if(ci>0u){start=offsets[ci-1u];}
   for(var j=start;j<offsets[ci];j++){
    let other=indices[j];if(other==i||other>=INVALID){continue;}
    let b=agents[other];if(b.alive==0u){continue;}
    let delta=b.position-a.position;let distance2=dot(delta,delta);if(distance2>r2){continue;}
    let sector=sensory_sector(delta);let region=sector+select(0u,8u,distance2>near2);
    p.regions[region].bodies+=1.0;p.nearby_count+=1.0;
    // Slot breaks exact-distance ties only; never enters cognition.
    if(distance2>nearest[sector]||(distance2==nearest[sector]&&other>=p.bodies[sector].slot)){continue;}
    nearest[sector]=distance2;
    let present=b.signal_tick==params.tick && params.tick>0u;
    p.bodies[sector].offset=delta;p.bodies[sector].velocity=b.velocity;
    p.bodies[sector].signal_present=f32(present);
    p.bodies[sector].signal=select(0.0,b.signal_payload,present);
    p.bodies[sector].slot=other;p.bodies[sector].generation=b.generation;
   }
  }
 }
 perceptions[i]=p;
}
