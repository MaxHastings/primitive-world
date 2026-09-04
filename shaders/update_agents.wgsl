@group(0) @binding(0) var<storage, read> source_agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> decisions: array<Decision>;
@group(0) @binding(3) var<storage, read> requests: array<u32>;
@group(0) @binding(4) var<storage, read_write> destination_agents: array<Agent>;
@group(0) @binding(5) var<uniform> params: SimParams;
@group(0) @binding(6) var<storage, read_write> birth_flags: array<u32>;
@group(0) @binding(7) var<storage, read_write> stats: array<atomic<u32>>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  var a=source_agents[i];
  birth_flags[i]=0u;
  if (a.alive==0u) { destination_agents[i]=a; return; }
  let d=decisions[i]; let p=perceptions[i];
  atomicAdd(&stats[24u+min(d.selected_action,6u)],1u);
  var foods = array<f32,5>(p.resource_here,p.resource_north,p.resource_east,p.resource_south,p.resource_west);
  var dirs = array<vec2<f32>,5>(vec2<f32>(0),vec2<f32>(0,-1),vec2<f32>(1,0),vec2<f32>(0,1),vec2<f32>(-1,0));
  // Refresh observed places (including depleted ones); replace only weaker stale estimates.
  for (var sample=0u; sample<5u; sample++) {
    let position=clamp(a.position+dirs[sample]*a.sensor_radius,vec2<f32>(0.0),vec2<f32>(params.world_size));
    var slot=0u; var weakest=1000.0; var matched=false;
    for (var k=0u; k<16u; k++) {
      if (ground_index(a.places[k].position)==ground_index(position) && a.places[k].confidence>0.0) {
        slot=k; matched=true; break;
      }
      let value=a.places[k].confidence*a.places[k].food*exp(-f32(params.tick-a.places[k].observed)/6000.0);
      if (value<weakest) { weakest=value; slot=k; }
    }
    var duplicate=false;
    for(var k=0u;k<16u;k++){
      if(a.places[k].confidence>0.0 && length(a.places[k].position-position)<a.sensor_radius*1.5){duplicate=true;}
    }
    // Keep geographically distinct fixed anchors. Never slide an old coordinate
    // along with the body, which erased the route back to a previously visited patch.
    if (matched || (!duplicate && foods[sample]>weakest+0.02)) { a.places[slot]=Place(position,foods[sample],params.tick,i,a.generation,1.0,0u); }
  }
  if (a.guide_result!=0.0) { a.guide_id=INVALID; }
  a.guide_result=0.0;
  if (a.guide_id<INVALID && params.tick>a.guide_started && length(a.position-a.guide_position)<a.sensor_radius) {
    let useful=p.resource_here>=max(0.02,a.guide_expected*0.25);
    // Encountering usable food verifies guidance. An empty point is a failure
    // only on arrival, not while still approaching the reported patch.
    if (useful || length(a.position-a.guide_position)<=2.0) { a.guide_result=select(-1.0,1.0,useful); }
  }
  a.food += f32(requests[i])/1000.0;
  if (d.selected_action==EAT) {
    let amount=min(min(a.food,0.1),max(0.0,100.0-a.energy)/max(params.resource_and_noise.y,0.001));
    a.food-=amount; a.energy+=amount*params.resource_and_noise.y;
    atomicAdd(&stats[0],u32(amount*1000.0));
  }
  a.velocity=vec2<f32>(0.0);
  if (d.selected_action==MOVE) {
    let juvenile=0.6+0.4*clamp(a.age/max(params.sensor_and_padding.y,1.0),0.0,1.0);
    let delta=d.goal-a.position;
    let distance=min(length(delta),a.max_speed*juvenile*params.time_and_costs.x);
    a.velocity=unit_vector(delta)*distance;
    if (length(a.goal-d.goal)>1.0 || params.tick>=a.commit_until) {
      // Refining the route locally must not erase who supplied the destination.
      if (a.guide_result==0.0 && length(d.goal-a.guide_position)>a.sensor_radius) { a.guide_id=INVALID; }
      for (var k=0u; k<16u; k++) {
        if (a.guide_id==INVALID && a.guide_result==0.0 && length(a.places[k].position-d.goal)<1.0 && a.places[k].source_id!=i && a.places[k].source_id<INVALID) {
          a.guide_id=a.places[k].source_id; a.guide_generation=a.places[k].source_generation;
          a.guide_expected=a.places[k].food; a.guide_started=params.tick;
          a.guide_position=a.places[k].position;
        }
      }
    }
    a.position=clamp(a.position+a.velocity,vec2<f32>(0.0),vec2<f32>(params.world_size));
    a.distance_travelled+=distance;
    a.energy-=distance*params.time_and_costs.z;
    if (length(a.goal-d.goal)>1.0 || params.tick>=a.commit_until) {
      let travel_ticks=length(delta)/max(a.max_speed*juvenile*params.time_and_costs.x,0.1);
      a.commit_until=params.tick+u32(clamp(ceil(travel_ticks*2.0),24.0,192.0));
      a.goal_score=d.scores[MOVE];
    }
    a.goal=d.goal;
  }
  // Eating or gathering pauses a trip without replacing its destination.
  a.action=d.selected_action; a.target_id=d.target_id;
  a.sensor_radius=params.sensor_and_padding.x;
  a.energy=max(0.0,a.energy-params.time_and_costs.w);
  a.age+=params.time_and_costs.x;
  a.rng=hash_u32(a.rng+params.tick+1u);
  if (a.energy<=0.0 || a.age>=a.max_age) {
    a.alive=0u;
    if (a.age>=a.max_age) { atomicAdd(&stats[2],1u); } else { atomicAdd(&stats[1],1u); }
  }
  // Reproduction is a settled-surplus behavior. Prevent a moving agent from
  // reproducing on the same tick it is paying travel costs or arriving at a
  // newly found patch; this removes the depletion -> migration -> birth pulse.
  birth_flags[i]=u32(a.alive!=0u && d.selected_action!=MOVE && a.age>=params.sensor_and_padding.y && params.tick>=a.next_birth && a.energy>=max(params.sensor_and_padding.z,params.sensor_and_padding.w+10.0) && a.food>=2.0 && (a.rng&1023u)<3u);
  if(a.alive!=0u){
    let mature=a.age>=params.sensor_and_padding.y;
    let energetic=a.energy>=max(params.sensor_and_padding.z,params.sensor_and_padding.w+10.0);
    let stocked=a.food>=2.0;
    let settled=d.selected_action!=MOVE;
    let ready=params.tick>=a.next_birth;
    atomicAdd(&stats[16],u32(!mature));atomicAdd(&stats[17],u32(!energetic));
    atomicAdd(&stats[18],u32(!stocked));atomicAdd(&stats[19],u32(!settled));
    atomicAdd(&stats[20],u32(!ready));
    atomicAdd(&stats[21],u32(mature && energetic && stocked && settled && ready));
    atomicAdd(&stats[22],birth_flags[i]);
  }
  destination_agents[i]=a;
}
