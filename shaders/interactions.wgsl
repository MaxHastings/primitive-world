@group(0) @binding(6) var<storage,read_write> events:array<InteractionEvent>;
fn record(actor:u32,other:u32,action:u32,amount:f32,position:vec2<f32>){let sequence=counter_add(8,1u);events[sequence%65536u]=InteractionEvent(params.tick,actor,other,action,amount,sequence,agents[actor].lineage_id,agents[other].lineage_id,position,vec2<f32>(0.0),0u,0u);}
@group(0) @binding(5) var<storage,read_write> ground:array<Ground>;
@group(0) @binding(0) var<storage,read_write> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> decisions:array<Decision>;
@group(0) @binding(2) var<storage,read_write> claims:array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(7) var<storage,read> offsets:array<u32>;
@group(0) @binding(8) var<storage,read> indices:array<u32>;
fn priority(i:u32)->u32{return hash_u32(agents[i].lineage_id^params.tick);}
// Deterministic physical arbitration chooses the nearest present contact.  It
// is world-owned: no controller output can name or rank another body.
fn contact(i:u32)->u32{
 let a=agents[i];if(a.alive==0u){return INVALID;}let size=params.world_size.xy/256.0;let base=vec2<i32>(floor(wrap_world(a.position,params.world_size.xy)/size));var best=INVALID;var best2=INTERACTION_RADIUS*INTERACTION_RADIUS+1.0;
 let reach=vec2<i32>(ceil(vec2<f32>(INTERACTION_RADIUS)/size));for(var oy=-reach.y;oy<=reach.y;oy++){for(var ox=-reach.x;ox<=reach.x;ox++){let cell=vec2<u32>(wrap_grid_index(base.x+ox,256),wrap_grid_index(base.y+oy,256));let ci=cell.y*256u+cell.x;var start=0u;if(ci>0u){start=offsets[ci-1u];}for(var k=start;k<offsets[ci];k++){let j=indices[k];if(j==i||j>=params.agent_count){continue;}let b=agents[j];if(b.alive==0u){continue;}let delta=torus_delta(a.position,b.position,params.world_size.xy);let distance2=dot(delta,delta);if(distance2>INTERACTION_RADIUS*INTERACTION_RADIUS||distance2>best2||(distance2==best2&&priority(j)>=priority(best))){continue;}best=j;best2=distance2;}}}
 return best;
}
@compute @workgroup_size(64)
fn clear(@builtin(global_invocation_id) id:vec3<u32>){if(id.x<params.agent_count){atomicStore(&claims[id.x],0xffffffffu);atomicStore(&claims[INVALID+id.x],INVALID);atomicStore(&claims[2u*INVALID+id.x],0u);}}
@compute @workgroup_size(64)
fn propose(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x;if(i>=params.agent_count){return;}let d=decisions[i];if(d.selected_action!=TRANSFER&&d.selected_action!=APPLY_FORCE){return;}if(d.selected_action==APPLY_FORCE&&(params.physical.x<0.5||length(d.force)<=0.0)){return;}let j=contact(i);if(j>=params.agent_count){return;}let a=agents[i];let b=agents[j];if(d.selected_action==TRANSFER&&(a.food<=0.0||b.food>=FOOD_CAPACITY)){return;}atomicStore(&claims[INVALID+i],j);atomicStore(&claims[2u*INVALID+i],priority(i));atomicMin(&claims[i],priority(i));atomicMin(&claims[j],priority(i));}
@compute @workgroup_size(64)
fn resolve(@builtin(global_invocation_id) id:vec3<u32>){let i=id.x;if(i>=params.agent_count){return;}let d=decisions[i];if(d.selected_action!=TRANSFER&&d.selected_action!=APPLY_FORCE){return;}if(d.selected_action==APPLY_FORCE&&params.physical.x<0.5){return;}let j=atomicLoad(&claims[INVALID+i]);let rank=atomicLoad(&claims[2u*INVALID+i]);if(j>=params.agent_count||atomicLoad(&claims[i])!=rank||atomicLoad(&claims[j])!=rank){return;}var a=agents[i];var b=agents[j];if(a.alive==0u||b.alive==0u){return;}
 if(d.selected_action==TRANSFER){let amount=min(min(a.food,d.amount),max(0.0,FOOD_CAPACITY-b.food));if(amount<=0.0){return;}a.food-=amount;b.food+=amount;b.received+=amount;record(i,j,TRANSFER,amount,a.position);counter_add(4,1u);counter_add(6,u32(amount*1000.0));}
 else {var impulse=body_to_world(d.force,a.heading)*3.0;let requested_cost=0.1*dot(impulse,impulse);if(requested_cost>a.energy){impulse*=sqrt(a.energy/max(requested_cost,0.00001));}let cost=min(a.energy,0.1*dot(impulse,impulse));a.energy-=cost;a.spent+=cost;a.velocity-=impulse;b.velocity+=impulse;counter_add(12,1u);counter_add(13,u32(round(cost*1000.0)));counter_add(15,u32(round(length(impulse)*1000.0)));record(i,j,APPLY_FORCE,length(impulse),a.position);counter_add(5,1u);if(a.energy<=0.0){a.alive=0u;counter_add(7,1u);}}
 agents[i]=a;agents[j]=b;
}

// Accounting horizons are explicit engine limits, never ecological extinction.
// Food low-word counter 0 has the explicitly maintained high word at 14.
fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicStore(&stats[36],1u);}
 return prior;
}
