@group(0) @binding(0) var<storage, read_write> resources: array<u32>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<storage, read_write> fertility: array<f32>;
@group(0) @binding(3) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(4) var<storage, read> terrain: array<vec4<f32>>;
@group(0) @binding(5) var<storage, read_write> ecology: array<vec4<f32>>;
const GRID:u32=512u;
fn climate_corner(p:vec2<u32>, epoch:u32, salt:u32)->f32 {
  return f32(hash_u32(params.lifecycle.x ^ salt ^ p.x*1973u ^ p.y*9277u ^ epoch*26699u)&65535u)/65535.0;
}
fn spatial(uv:vec2<f32>, scale:u32, epoch:u32, salt:u32)->f32 {
  let p=fract(uv)*f32(scale); let lo=vec2<u32>(floor(p)); let hi=(lo+1u)%scale;
  let f=fract(p); let t=f*f*(3.0-2.0*f);
  return mix(mix(climate_corner(lo,epoch,salt),climate_corner(vec2<u32>(hi.x,lo.y),epoch,salt),t.x),
    mix(climate_corner(vec2<u32>(lo.x,hi.y),epoch,salt),climate_corner(hi,epoch,salt),t.x),t.y);
}
fn weather(uv:vec2<f32>, scale:u32, period:u32, salt:u32)->f32 {
  let epoch=params.lifecycle.w/period;
  let t=f32(params.lifecycle.w%period)/f32(period);
  let blend=t*t*t*(t*(t*6.0-15.0)+10.0);
  return mix(spatial(uv,scale,epoch,salt),spatial(uv,scale,epoch+1u,salt),blend);
}
@compute @workgroup_size(8,8,1)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
  if(id.x>=GRID || id.y>=GRID){return;}
  let index=id.y*GRID+id.x;
  let old_value=resources[index];
  if(params.mutation.z!=0.0){
    let phase=f32(params.lifecycle.w%1000000u)/1000000.0;
    let blend=phase*phase*(3.0-2.0*phase);
    ground[index].habitat=mix(terrain[index].x,terrain[index].y,blend);
    ground[index].productivity=mix(terrain[index].z,terrain[index].w,blend);
  }
  var canonical=id.xy;
  for(var turn=0u;turn<params.lifecycle.z;turn++){canonical=vec2<u32>(canonical.y,GRID-1u-canonical.x);}
  let uv=(vec2<f32>(canonical)+0.5)/f32(GRID);
  // Seeded substrate is static; all temporal variation comes from weather.
  let elevation=spatial(uv,7u,0u,173u);
  let retention=0.25+0.65*spatial(uv,11u,0u,3137u);
  let permeability=0.1+0.8*spatial(uv,9u,0u,712u);
  let regional=weather(uv,5u,47003u,119u);
  let local=weather(uv,23u,997u,197u);
  let rainfall=params.time_and_costs.x*exp(1.5*(regional-0.5)+0.6*(local-0.5));
  let temperature=clamp(params.world_size.w+0.2*(regional-0.5)+0.08*(local-0.5)-0.12*(elevation-0.5),0.0,1.0);
  var pools=ecology[index]; // water, mineral, detritus, reserved
  let rain=0.00008*rainfall*(0.65+0.35*retention);
  let evaporation=0.00004*(0.3+1.7*temperature)*(1.0-0.35*retention);
  let drainage=max(pools.x-0.12,0.0)*0.00006*permeability*(0.4+0.6*elevation);
  let transpiration=f32(old_value)/1000.0*0.000015*(0.4+temperature);
  // Rain is external input; evaporation/drainage/runoff leave this local store.
  pools.x=clamp(pools.x+rain-evaporation-drainage-transpiration,0.0,2.0);
  let decomposed=min(pools.z,pools.z*0.00003*min(pools.x,1.0)*(0.3+temperature));
  pools.z-=decomposed;
  pools.y+=decomposed+max(8.0-pools.y,0.0)*0.000004*permeability;
  let moisture=pools.x/(0.25+pools.x);
  let thermal=max(0.08,1.0-2.0*abs(temperature-0.5));
  let nutrient=pools.y/(0.3+pools.y);
  let extracted=f32(atomicExchange(&ground[index].extracted,0u))/1000.0;
  let soil=clamp(fertility[index]+(0.55*min(pools.x,1.5)-fertility[index])*0.00008-extracted*0.004,0.02,1.0);
  fertility[index]=soil;
  // Habitable coverage follows water, mineral and temperature continuously.
  // Retentive, less permeable substrate provides refuges without refuge modes.
  let suitability=smoothstep(0.12,0.65,pools.x)*nutrient*thermal;
  let coverage=0.55*suitability;
  // Favorable weather supports sparse growth between the established patches,
  // while their substrate advantage and visible edges remain intact.
  let geography=mix(ground[index].habitat,1.0,coverage*0.2);
  let productivity=mix(ground[index].productivity,1.0,coverage*0.2);
  let heterogeneity=clamp(params.resource_and_noise.z,0.0,1.0);
  let variation=mix(0.5,0.65*regional+0.35*local,heterogeneity);
  let growth=params.time_and_costs.y*(0.2+0.8*variation)*(0.25+0.75*soil)*productivity*moisture*thermal*nutrient;
  let capacity=(0.25+0.75*soil)*1000.0*geography;
  // Existing vegetation recedes gradually; material transfers to detritus.
  let old_food=f32(old_value);
  let delta=select(min(min(growth*14.0,pools.y*1000.0),max(capacity-old_food,0.0)),
    -(old_food-capacity)*0.01,old_food>capacity);
  let accumulation=ground[index].remainder+delta;
  let whole=floor(accumulation);
  ground[index].remainder=min(accumulation-whole,0.99999994);
  var next=old_value;
  if(whole<0.0){
    let loss=min(old_value,u32(-whole)); next-=loss; pools.z+=f32(loss)/1000.0;
  }else{
    let added=min(u32(whole),u32(pools.y*1000.0)); next+=added;
    pools.y=max(0.0,pools.y-f32(added)/1000.0);
  }
  ecology[index]=pools;
  if(next>=old_value){ground[index].produced+=next-old_value;}
  else{ground[index].weather_loss+=old_value-next;}
  resources[index]=next;
}
