@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> social: array<SocialPerception>;
@group(0) @binding(3) var<storage, read_write> decisions: array<Decision>;
@group(0) @binding(4) var<uniform> params: SimParams;
@group(0) @binding(5) var<storage, read> neural_weights: NeuralWeights;
@group(0) @binding(6) var<storage, read_write> neural_state: array<f32>;
fn access_change(a: Agent, s: SocialPerception, destination: vec2<f32>) -> f32 {
  if (s.companion_value<=0.0) { return 0.0; }
  let future=s.companion_position+s.companion_velocity*8.0;
  let delta=destination-a.position;
  let step=unit_vector(delta)*min(length(delta),a.max_speed*8.0);
  let staying=1.0-smoothstep(INTERACTION_RADIUS,max(a.sensor_radius,7.0),length(future-a.position));
  let moving=1.0-smoothstep(INTERACTION_RADIUS,max(a.sensor_radius,7.0),length(future-a.position-step));
  return (moving-staying)*s.companion_value*params.social_weights.x;
}
fn social_travel_value(a: Agent, s: SocialPerception, destination: vec2<f32>) -> f32 {
  return access_change(a,s,destination)
    + dot(unit_vector(destination-a.position),s.avoidance)*params.social_weights.x;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  var a=agents[i]; let p=perceptions[i]; let s=social[i];
  var d: Decision;
  d.target_id=INVALID; d.goal=a.position;
  d.scores=array<f32,7>(0.02,-1000.0,-1000.0,-1000.0,-1000.0,-1000.0,-1000.0);
  if (a.alive==0u) {
    for (var h=0u; h<NEURAL_HIDDEN; h++) { neural_state[i*NEURAL_HIDDEN+h]=0.0; }
    decisions[i]=d; return;
  }
  let hunger=clamp(1.0-a.energy/100.0,0.0,1.0);
  let stock_need=clamp(1.0-a.food/FOOD_CAPACITY,0.0,1.0);
  let urgency=0.3+0.7*hunger;
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
  let explore_score=search_value+social_travel_value(a,s,explore_goal);
  if (explore_score>best_move) { best_move=explore_score; goal=explore_goal; }
  if (s.companion_value>0.0 && params.social_weights.x>0.0) {
    let companion_goal=clamp(s.companion_position+s.companion_velocity*8.0,vec2<f32>(0.0),vec2<f32>(params.world_size));
    let score=search_value+social_travel_value(a,s,companion_goal);
    if (length(companion_goal-a.position)>INTERACTION_RADIUS && score>best_move) {
      best_move=score; goal=companion_goal;
    }
  }
  if (params.tick<a.commit_until && length(a.goal-a.position)>2.0 && s.danger<0.25) {
    let continuation=0.18+social_travel_value(a,s,a.goal);
    // Compare alternatives against the value that justified this trip, not its
    // small continuation utility. Minor changes must not cause repeated U-turns.
    // The low action utility still lets harvesting/eating interrupt the journey.
    if (best_move<=max(0.18,a.goal_score)+0.08 && access_change(a,s,a.goal)>-0.03) { best_move=continuation; goal=a.goal; }
  }
  d.goal=goal; d.scores[MOVE]=best_move;
  if (a.food<FOOD_CAPACITY-0.001 && p.resource_here>0.001) {
    // Value what this action can collect, including a full harvest from a modest patch.
    let harvest_limit=max(params.resource_and_noise.x/1000.0,0.001);
    let attainable=min(min(p.resource_here,harvest_limit),FOOD_CAPACITY-a.food);
    d.scores[HARVEST]=attainable/harvest_limit*0.3*stock_need*(0.7+hunger)-p.competition_pressure*0.04;
  }
  if (a.food>0.001 && a.energy<99.0) {
    let bite=min(min(a.food,0.1),(100.0-a.energy)/max(params.resource_and_noise.y,0.001));
    d.scores[EAT]=hunger*hunger*1.4*bite/0.1;
  }
  if (s.give_target<params.agent_count && a.energy>30.0) { d.scores[GIVE]=s.give_value; }
  if (params.social_weights.w>0.5 && s.force_target<params.agent_count) { d.scores[FORCE]=s.force_value; }
  if (params.lifecycle.z!=0u && s.report_target<INVALID && a.energy>30.0 && params.tick>=a.last_communication+4u) {
    d.scores[COMMUNICATE]=s.report_value;
  }
  var neural_selected=INVALID;
  if (params.neural_config.x!=0u) {
    var obs=array<f32,12>(
      clamp(a.energy/100.0,0.0,1.0), clamp(a.food/FOOD_CAPACITY,0.0,1.0),
      clamp(p.resource_here,0.0,1.0), clamp(p.resource_north,0.0,1.0), clamp(p.resource_east,0.0,1.0),
      clamp(p.resource_south,0.0,1.0), clamp(p.resource_west,0.0,1.0), clamp(p.local_density,0.0,1.0),
      clamp(p.projected_food,0.0,1.0), clamp(p.competition_pressure,0.0,1.0), clamp(s.known_strength,0.0,1.0), clamp(s.danger,0.0,1.0));
    var old: array<f32,16>;
    for (var h=0u; h<NEURAL_HIDDEN; h++) { old[h]=neural_state[i*NEURAL_HIDDEN+h]; }
    for (var h=0u; h<NEURAL_HIDDEN; h++) {
      var value=neural_weights.hidden_bias[h];
      for (var j=0u; j<NEURAL_OBSERVATIONS; j++) { value += neural_weights.input[h*NEURAL_OBSERVATIONS+j]*obs[j]; }
      for (var j=0u; j<NEURAL_HIDDEN; j++) { value += neural_weights.recurrent[h*NEURAL_HIDDEN+j]*old[j]; }
      neural_state[i*NEURAL_HIDDEN+h]=tanh(value);
    }
    var logits: array<f32,7>;
    for (var action=0u; action<NEURAL_ACTIONS; action++) {
      var value=neural_weights.output_bias[action];
      for (var h=0u; h<NEURAL_HIDDEN; h++) { value += neural_weights.output[action*NEURAL_HIDDEN+h]*neural_state[i*NEURAL_HIDDEN+h]; }
      logits[action]=value;
    }
    var neural_best=-100000.0;
    for (var action=0u; action<NEURAL_ACTIONS; action++) {
      // Neural logits are a bounded preference adjustment. The authored score
      // remains the baseline utility and still supplies all physical/social
      // affordance masks; neutral weights therefore preserve baseline motion.
      let combined=d.scores[action]+clamp(logits[action],-2.0,2.0);
      if (d.scores[action]>-500.0 && combined>neural_best) { neural_best=combined; neural_selected=action; }
    }
  }
  var best=-10000.0;
  for (var k=0u; k<7u; k++) {
    // Small bounded tie-breaking noise; never turns an unavailable action into an available one.
    let score=d.scores[k]+(random01(a.rng ^ k*0x9e3779b9u)-0.5)*params.resource_and_noise.w*0.15;
    if (score>best) { best=score; d.selected_action=k; }
  }
  if (a.energy<20.0 && a.food>0.001) { d.selected_action=EAT; }
  if (neural_selected<INVALID) { d.selected_action=neural_selected; }
  if (d.selected_action==GIVE) { d.target_id=s.give_target; d.amount=min(0.5,max(0.0,a.food-1.5)); }
  if (d.selected_action==FORCE) { d.target_id=s.force_target; d.amount=1.0; }
  if (d.selected_action==COMMUNICATE) { d.target_id=s.report_target; d.amount=f32(s.report_place); }
  decisions[i]=d;
}
