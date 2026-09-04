@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> social: array<SocialPerception>;
@group(0) @binding(3) var<storage, read_write> decisions: array<Decision>;
@group(0) @binding(4) var<uniform> params: SimParams;
@group(0) @binding(5) var<storage, read> neural_weights: NeuralWeights;
@group(0) @binding(6) var<storage, read_write> neural_state: array<NeuralState>;
@group(0) @binding(7) var<storage,read> neural_resources:array<u32>;
@group(0) @binding(8) var<storage,read_write> neural_ground:array<Ground>;
fn access_change(a: Agent, s: SocialPerception, destination: vec2<f32>) -> f32 {
  if (s.companion_value<=0.0) { return 0.0; }
  let social_scale=clamp(1.0+a.genome[3],0.0,2.0);
  let future=s.companion_position+s.companion_velocity*8.0;
  let delta=destination-a.position;
  let step=unit_vector(delta)*min(length(delta),a.max_speed*8.0);
  let staying=1.0-smoothstep(INTERACTION_RADIUS,max(a.sensor_radius,7.0),length(future-a.position));
  let moving=1.0-smoothstep(INTERACTION_RADIUS,max(a.sensor_radius,7.0),length(future-a.position-step));
  return (moving-staying)*s.companion_value*params.social_weights.x*social_scale;
}
fn social_travel_value(a: Agent, s: SocialPerception, destination: vec2<f32>) -> f32 {
  let social_scale=clamp(1.0+a.genome[3],0.0,2.0);
  return access_change(a,s,destination)
    + dot(unit_vector(destination-a.position),s.avoidance)*params.social_weights.x*social_scale;
}
fn genome_scale(a: Agent, index: u32, amplitude: f32) -> f32 {
  return clamp(1.0+a.genome[index]*amplitude,0.1,2.0);
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  var a=agents[i]; let p=perceptions[i]; var s=social[i];
  var d: Decision;
  d.target_id=INVALID; d.goal=a.position;
  d.scores=array<f32,7>(0.02,-1000.0,-1000.0,-1000.0,-1000.0,-1000.0,-1000.0);
  if (a.alive==0u) {
    if(neural_state[i].valid!=0u) {
      for (var h=0u; h<NEURAL_HIDDEN; h++) { neural_state[i].hidden[h]=0.0; }
      neural_state[i].valid=0u;
    }
    decisions[i]=d; return;
  }
  // Perception exposes facts; the agent interprets them. The kernel supplies
  // no helping, revenge, reputation, or report objective. Heritable traits
  // determine whether a physical opportunity becomes an intent.
  var transfer_target=s.give_target;
  var transfer_value=s.give_value;
  var force_target=s.force_target;
  var force_value=s.force_value;
  var emit_target=s.report_target;
  var emit_value=s.report_value;
  var nearest_target=INVALID;
  var nearest_value=-1000.0;
  for (var k=0u; k<8u; k++) {
    let c=s.candidates[k];
    if (c.target_slot>=params.agent_count || c.target_generation==0u) { continue; }
    let closeness=1.0-clamp(c.distance/max(a.sensor_radius,1.0),0.0,1.0);
    if (closeness>nearest_value) { nearest_value=closeness; nearest_target=c.target_slot; }
    if (c.distance<=INTERACTION_RADIUS) {
      let transfer=max(0.0,a.food-1.5)*max(0.0,1.0-c.food/2.0)*closeness;
      if (transfer>transfer_value) { transfer_value=transfer; transfer_target=c.target_slot; }
      let physical_force=c.food*closeness-0.5;
      if (physical_force>force_value) { force_value=physical_force; force_target=c.target_slot; }
    }
  }
  if (emit_target>=INVALID && nearest_target<params.agent_count && a.genome[6]>0.0) {
    emit_target=nearest_target;
    emit_value=a.genome[6]*max(nearest_value,0.0);
  }
  s.give_target=transfer_target; s.give_value=transfer_value;
  s.force_target=force_target; s.force_value=force_value;
  s.report_target=emit_target; s.report_value=emit_value;
  if(params.neural_config.x!=0u){ decisions[i]=neural_decision(i,a,p,s);return; }
  let hunger=clamp(1.0-a.energy/100.0,0.0,1.0);
  let stock_need=clamp(1.0-a.food/FOOD_CAPACITY,0.0,1.0);
  let urgency=(0.3+0.7*hunger)*genome_scale(a,0u,0.7);
  var dirs=array<vec2<f32>,4>(vec2<f32>(0,-1),vec2<f32>(1,0),vec2<f32>(0,1),vec2<f32>(-1,0));
  var foods=array<f32,4>(p.resource_north,p.resource_east,p.resource_south,p.resource_west);
  var best_move=-1000.0;
  var goal=a.goal;
  // Compare destination-specific food and crowding, plus accessibility of observed people.
  for (var k=0u; k<4u; k++) {
    let candidate=clamp(a.position+dirs[k]*a.sensor_radius,vec2<f32>(0.0),vec2<f32>(params.world_size));
    let supply=foods[k]/(1.0+p.crowd[k]*0.25);
    let score=(supply-p.projected_food)*stock_need*urgency
      + social_travel_value(a,s,candidate)
      - max(0.0,p.crowd[k]-1.0)*0.015 - params.time_and_costs.z*8.0;
    if (length(candidate-a.position)>1.0 && score>best_move) { best_move=score; goal=candidate; }
  }
  for (var k=0u; k<4u; k++) {
    let place=a.places[k];
    if (place.confidence<=0.0) { continue; }
    let distance=length(place.position-a.position);
    if (distance<a.sensor_radius*0.5) { continue; }
    let trip_cost=distance*(params.time_and_costs.z+params.time_and_costs.w/max(a.max_speed,0.1));
    if (trip_cost+5.0>a.energy+a.food*params.resource_and_noise.y) { continue; }
    let confidence=place.confidence*exp(-f32(params.tick-place.observed)/1800.0);
    let expected=mix(0.12,place.food,confidence);
    let score=(expected-p.projected_food)*stock_need*urgency
      - distance*params.time_and_costs.z*0.1 + social_travel_value(a,s,place.position);
    if (score>best_move) { best_move=score; goal=place.position; }
  }
  // A sustained exploratory destination, rather than new white noise every tick.
  let angle=random01(a.rng ^ i*0x9e3779b9u)*6.2831853;
  let explore_goal=clamp(a.position+vec2<f32>(cos(angle),sin(angle))*a.sensor_radius*2.0,vec2<f32>(0.0),vec2<f32>(params.world_size));
  let search_value=stock_need*(0.10+0.20*hunger)*(1.0-p.projected_food)-0.06;
  let explore_score=(search_value*genome_scale(a,1u,1.0))+social_travel_value(a,s,explore_goal);
  if (explore_score>best_move) { best_move=explore_score; goal=explore_goal; }
  if (s.companion_value>0.0 && params.social_weights.x>0.0) {
    let companion_goal=clamp(s.companion_position+s.companion_velocity*8.0,vec2<f32>(0.0),vec2<f32>(params.world_size));
    let score=search_value+social_travel_value(a,s,companion_goal);
    if (length(companion_goal-a.position)>INTERACTION_RADIUS && score>best_move) {
      best_move=score; goal=companion_goal;
    }
  }
  if (params.tick<a.commit_until && length(a.goal-a.position)>2.0 && s.danger<0.25) {
    let continuation=(0.18+social_travel_value(a,s,a.goal))*genome_scale(a,2u,1.0);
    // Compare alternatives against the value that justified this trip, not its
    // small continuation utility. Minor changes must not cause repeated U-turns.
    // The low action utility still lets harvesting/eating interrupt the journey.
    if (best_move<=max(0.18,a.goal_score)+0.08 && access_change(a,s,a.goal)>-0.03) { best_move=continuation; goal=a.goal; }
  }
  d.goal=goal; d.scores[MOVE]=best_move;
  d.scores[MOVE] *= genome_scale(a,2u,0.5);
  if (a.food<FOOD_CAPACITY-0.001 && p.resource_here>0.001) {
    // Value what this action can collect, including a full harvest from a modest patch.
    let harvest_limit=max(params.resource_and_noise.x/1000.0,0.001);
    let attainable=min(min(p.resource_here,harvest_limit),FOOD_CAPACITY-a.food);
    d.scores[HARVEST]=attainable/harvest_limit*0.3*stock_need*(0.7+hunger)-p.competition_pressure*0.04;
  }
  if (a.food>0.001 && a.energy<99.0) {
    let bite=min(min(a.food,0.1),(100.0-a.energy)/max(params.resource_and_noise.y,0.001));
    d.scores[EAT]=hunger*hunger*1.4*bite/0.1*genome_scale(a,0u,0.7);
  }
  if (transfer_target<params.agent_count && a.energy>30.0) { d.scores[TRANSFER]=transfer_value*genome_scale(a,4u,1.0); }
  if (params.social_weights.w>0.5 && force_target<params.agent_count) { d.scores[APPLY_FORCE]=force_value*genome_scale(a,5u,1.0); }
  if (params.lifecycle.z!=0u && emit_target<INVALID && a.energy>30.0 && params.tick>=a.last_communication+4u) {
    d.scores[EMIT]=emit_value*genome_scale(a,6u,1.0);
  }
  var best=-10000.0;
  for (var k=0u; k<7u; k++) {
    // Small bounded tie-breaking noise; never turns an unavailable action into an available one.
    let score=d.scores[k]+(random01(a.rng ^ k*0x9e3779b9u)-0.5)*params.resource_and_noise.w*0.15;
    if (score>best) { best=score; d.selected_action=k; }
  }
  if (d.selected_action==TRANSFER) { d.target_id=transfer_target; d.amount=min(0.5,max(0.0,a.food-1.5)); }
  if (d.selected_action==APPLY_FORCE) { d.target_id=force_target; d.amount=1.0; }
  if (d.selected_action==EMIT) { d.target_id=emit_target; d.amount=clamp(a.genome[6],-1.0,1.0); }
  decisions[i]=d;
}
