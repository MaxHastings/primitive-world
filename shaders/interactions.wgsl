struct InteractionEvent { tick:u32, actor:u32, other:u32, action:u32, amount:f32, sequence:u32, position:vec2<f32>, };
@group(0) @binding(6) var<storage,read_write> events:array<InteractionEvent>;
@group(0) @binding(7) var<storage,read_write> relations:array<Relation>;
fn record(actor:u32,other:u32,action:u32,amount:f32,position:vec2<f32>) {
  let sequence=atomicAdd(&stats[8],1u);
  events[sequence%65536u]=InteractionEvent(params.tick,actor,other,action,amount,sequence,position);
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
  if (d.target_id>=params.agent_count || d.target_id==i || (d.selected_action!=GIVE && d.selected_action!=FORCE && d.selected_action!=COMMUNICATE)) { return; }
  let a=agents[i]; let b=agents[d.target_id];
  let distance=length(a.position-b.position);
  let contact_radius=select(INTERACTION_RADIUS,min(a.sensor_radius,b.sensor_radius),d.selected_action==COMMUNICATE);
  if (a.alive==0u || b.alive==0u || distance>contact_radius) { return; }
  if (d.selected_action==GIVE && (a.food<=0.0 || b.food>=FOOD_CAPACITY)) { return; }
  atomicMin(&claims[i],priority(i)); atomicMin(&claims[d.target_id],priority(i));
}
@compute @workgroup_size(64)
fn resolve(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  let d=decisions[i]; let j=d.target_id;
  if (j>=params.agent_count || j==i || (d.selected_action!=GIVE && d.selected_action!=FORCE && d.selected_action!=COMMUNICATE)) { return; }
  if (atomicLoad(&claims[i])!=priority(i) || atomicLoad(&claims[j])!=priority(i)) { return; }
  // Accepted pairs are disjoint. No invocation can read a record another pair writes.
  var a=agents[i]; var b=agents[j];
  let distance=length(a.position-b.position);
  let contact_radius=select(INTERACTION_RADIUS,min(a.sensor_radius,b.sensor_radius),d.selected_action==COMMUNICATE);
  if (distance>contact_radius) { return; }
  if (d.selected_action==COMMUNICATE) {
    let report=a.places[min(u32(d.amount),3u)];
    var replace=0u; var weakest=1000.0; var duplicate=false;
    for (var k=0u;k<4u;k++) {
      if (length(b.places[k].position-report.position)<4.0 && b.places[k].confidence>0.0) {
        if (b.places[k].observed>=report.observed) { duplicate=true; }
        replace=k; weakest=-1.0; break;
      }
      let quality=b.places[k].food*b.places[k].confidence*exp(-f32(params.tick-b.places[k].observed)/1800.0);
      if (quality<weakest) { weakest=quality; replace=k; }
    }
    if (!duplicate && report.confidence>0.0) {
      b.places[replace]=report; b.places[replace].confidence*=0.8;
      record(i,j,COMMUNICATE,report.food,a.position); atomicAdd(&stats[9],1u);
    }
    var slot=0u; var weakest_relation=1000.0;
    for (var k=0u;k<8u;k++) {
      let r=relations[i*8u+k];
      if (r.target_slot==j && r.target_generation==b.generation) { slot=k; break; }
      let retention=r.familiarity+r.benefit+r.navigation+r.harm*2.0;
      if (r.target_slot>=INVALID || retention<weakest_relation) { slot=k; weakest_relation=retention; }
    }
    var remembered=relations[i*8u+slot];
    if (remembered.target_slot!=j || remembered.target_generation!=b.generation) {
      remembered=Relation(j,b.generation,0.01,0.0,0.0,params.tick,0.0,0.0,0.0,0.0,0u,0u);
    }
    remembered.last_report_tick=max(1u,params.tick);
    remembered.last_report_observed=report.observed;
    relations[i*8u+slot]=remembered;
    a.energy=max(0.0,a.energy-0.02); a.last_communication=params.tick;
    agents[i]=a; agents[j]=b; return;
  }
  if (d.selected_action==GIVE) {
    let amount=min(min(a.food,d.amount),max(0.0,FOOD_CAPACITY-b.food));
    if (amount<=0.0) { return; }
    a.food-=amount; b.food+=amount;
    b.event_amount=amount;
    record(i,j,GIVE,amount,a.position);
    atomicAdd(&stats[4],1u); atomicAdd(&stats[6],u32(amount*1000.0));
  } else {
    let chance=a.energy/max(a.energy+b.energy,0.001);
    a.energy=max(0.0,a.energy-0.6); b.energy=max(0.0,b.energy-0.3);
    var taken=0.0;
    if (random01(a.rng ^ b.rng ^ params.tick)<chance) {
      taken=min(b.food,d.amount);
      b.food-=taken; atomicAdd(&ground[ground_index(b.position)].dropped,u32(round(taken*1000.0)));
      var direction=unit_vector(b.position-a.position);
      if (length(direction)<0.1) { direction=vec2<f32>(1.0,0.0); }
      b.position=clamp(b.position+direction*3.0,vec2<f32>(0.0),vec2<f32>(params.world_size));
      b.commit_until=params.tick;
    }
    b.event_amount=-max(0.3,taken);
    record(i,j,FORCE,taken,a.position);
    atomicAdd(&stats[5],1u);
    if (a.energy<=0.0) { a.alive=0u; atomicAdd(&stats[7],1u); }
    if (b.energy<=0.0) { b.alive=0u; atomicAdd(&stats[7],1u); }
  }
  b.event_actor=i; b.event_generation=a.generation; b.event_tick=params.tick;
  agents[i]=a; agents[j]=b;
}
