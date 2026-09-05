// Optional observer-only counters. No buffer here is read by agent logic.
// 32 words per family; food/energy sums use low/high pairs to avoid overflow.
struct FamilyWindow { start:u32, late_start:u32, end:u32, count:u32, };
@group(0) @binding(0) var<storage,read> bodies:array<Agent>;
@group(0) @binding(1) var<storage,read_write> families:array<atomic<u32>>;
@group(0) @binding(2) var<uniform> params:SimParams;
@group(0) @binding(3) var<uniform> window:FamilyWindow;
@group(0) @binding(4) var<storage,read> previous:array<Agent>;
@group(0) @binding(5) var<storage,read> perceptions:array<Perception>;
fn total(index:u32, value:f32) {
 let amount=u32(round(max(0.0,value)*1000.0));
 let before=atomicAdd(&families[index],amount);
 if(before>0xffffffffu-amount){atomicAdd(&families[index+1u],1u);}
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 if(id.x>=INVALID){return;}
 let a=bodies[id.x];let old=previous[id.x];let family=a.founder_family;let tick=params.tick+1u;
 if(family>=window.count || tick<=window.start || tick>window.end){return;}
 let b=family*32u;
 // Slots alive at tick start cannot be reused this tick. Include terminal flows
 // and deaths once, without recounting stale dead slots on subsequent ticks.
 if(old.alive!=0u && old.lineage_id==a.lineage_id && old.generation==a.generation){
  total(b+20u,a.collected);total(b+22u,a.ingested);
  if(old.ancestry_depth>0u){
   total(b+24u,a.spent);
   if(old.age<params.sensor_and_padding.y){
    atomicAdd(&families[b+27u],1u);
    total(b+16u,a.collected);total(b+18u,a.ingested);
    atomicAdd(&families[b+26u],u32(a.action==COLLECT));
    if(perceptions[id.x].resource_here>=0.001){
     atomicAdd(&families[b+30u],1u);
     atomicAdd(&families[b+31u],u32(a.action==COLLECT));
    }
    if(a.alive!=0u && a.age>=params.sensor_and_padding.y){
     atomicAdd(&families[b+7u],1u);total(b+28u,a.energy);
    }
   }
   if(a.alive==0u){
    if(a.age>=a.max_age){atomicAdd(&families[b+10u],1u);}
    else if(a.energy<=0.0){
     if(a.age<params.sensor_and_padding.y){atomicAdd(&families[b+8u],1u);}
     else{atomicAdd(&families[b+9u],1u);}
    }else{atomicAdd(&families[b+11u],1u);}
   }
  }
 }
 if(a.alive==0u){return;}
 atomicMax(&families[b+6u],tick);atomicMax(&families[b+5u],a.ancestry_depth);
 if(a.ancestry_depth==0u){atomicAdd(&families[b],1u);return;}
 atomicAdd(&families[b+1u],1u);
 if(tick>window.late_start){atomicAdd(&families[b+2u],1u);}
 if(a.age>=params.sensor_and_padding.y){atomicAdd(&families[b+3u],1u);}
 if(a.birth_tick==params.tick){
  atomicAdd(&families[b+4u],1u);total(b+14u,a.energy);
  atomicAdd(&families[b+12u],u32(a.ancestry_depth>=2u));
  // Diagnostic lower bound only: no movement, food or other spending.
  atomicAdd(&families[b+13u],u32(a.energy<params.sensor_and_padding.y*params.time_and_costs.w));
 }
}
