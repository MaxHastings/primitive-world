struct InteractionEvent { tick:u32, actor:u32, other:u32, action:u32, amount:f32, sequence:u32, actor_lineage:u32, other_lineage:u32, position:vec2<f32>, };
@group(0) @binding(6) var<storage,read_write> events:array<InteractionEvent>;
fn record(actor:u32,other:u32,action:u32,amount:f32,position:vec2<f32>) {
  let sequence=atomicAdd(&stats[8],1u);
  events[sequence%65536u]=InteractionEvent(params.tick,actor,other,action,amount,sequence,agents[actor].lineage_id,agents[other].lineage_id,position);
}
@group(0) @binding(5) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> decisions: array<Decision>;
@group(0) @binding(2) var<storage, read_write> claims: array<atomic<u32>>;
@group(0) @binding(3) var<uniform> params: SimParams;
@group(0) @binding(4) var<storage, read_write> stats: array<atomic<u32>>;
fn priority(i: u32) -> u32 { return (i + hash_u32(params.tick) % params.agent_count) % params.agent_count; }
@compute @workgroup_size(64)
fn clear(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x<params.agent_count) { atomicStore(&claims[id.x],0xffffffffu); }
}
@compute @workgroup_size(64)
fn propose(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  let d=decisions[i];
  if (d.target_id>=params.agent_count || d.target_id==i || (d.selected_action!=TRANSFER && d.selected_action!=APPLY_FORCE)) { return; }
  let a=agents[i]; let b=agents[d.target_id];
  let distance=length(a.position-b.position);
  let contact_radius=INTERACTION_RADIUS;
  if (a.alive==0u || b.alive==0u || b.generation!=d.target_generation || distance>contact_radius) { return; }
  if(d.selected_action==APPLY_FORCE && params.physical.x<0.5){return;}
  if(d.selected_action==APPLY_FORCE && length(d.force)<=0.0){return;}
  if(d.selected_action==APPLY_FORCE){atomicAdd(&stats[12],1u);}
  if (d.selected_action==TRANSFER && (a.food<=0.0 || b.food>=FOOD_CAPACITY)) { return; }
  atomicMin(&claims[i],priority(i)); atomicMin(&claims[d.target_id],priority(i));
}
@compute @workgroup_size(64)
fn resolve(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  let d=decisions[i]; let j=d.target_id;
  if (j>=params.agent_count || j==i || (d.selected_action!=TRANSFER && d.selected_action!=APPLY_FORCE)) { return; }
  if (atomicLoad(&claims[i])!=priority(i) || atomicLoad(&claims[j])!=priority(i)) { return; }
  // Accepted pairs are disjoint. No invocation can read a record another pair writes.
  var a=agents[i]; var b=agents[j];
  let distance=length(a.position-b.position);
  let contact_radius=INTERACTION_RADIUS;
  if (a.alive==0u || b.alive==0u || b.generation!=d.target_generation || distance>contact_radius) { return; }
  if(d.selected_action==APPLY_FORCE && params.physical.x<0.5){return;}
  if (d.selected_action==TRANSFER) {
    let amount=min(min(a.food,d.amount),max(0.0,FOOD_CAPACITY-b.food));
    if (amount<=0.0) { return; }
    a.food-=amount; b.food+=amount; b.received+=amount;
    record(i,j,TRANSFER,amount,a.position);
    atomicAdd(&stats[4],1u); atomicAdd(&stats[6],u32(amount*1000.0));
  } else {
    // Kinematic contact actuator. The brain chooses displacement direction and
    // magnitude. No success lottery, recipient energy tax, recoil or food spill.
    // A drag cost of 0.2 energy/unit makes carrying another body non-free.
    var displacement=d.force*3.0;
    let requested_cost=length(displacement)*0.2;
    if(requested_cost>a.energy){displacement*=a.energy/max(requested_cost,0.00001);}
    let old=b.position;
    b.position=clamp(old+displacement,vec2<f32>(0),vec2<f32>(params.world_size));
    let actual=b.position-old;let cost=min(a.energy,length(actual)*0.2);
    b.moved+=actual;a.energy-=cost;a.spent+=cost;
    atomicAdd(&stats[13],u32(round(cost*1000.0)));
    atomicAdd(&stats[15],u32(round(length(actual)*1000.0)));
    record(i,j,APPLY_FORCE,length(actual),a.position);
    atomicAdd(&stats[5],1u);
    if(a.energy<=0.0){a.alive=0u;atomicAdd(&stats[7],1u);}
  }
  agents[i]=a; agents[j]=b;
}
