@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> cell_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> agent_indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> memory: array<Relation>;
@group(0) @binding(4) var<storage, read_write> social_perceptions: array<SocialPerception>;
@group(0) @binding(5) var<uniform> params: SimParams;
fn valid(r: Relation) -> bool {
  if (r.target_slot >= params.agent_count) { return false; }
  return agents[r.target_slot].alive != 0u && agents[r.target_slot].generation == r.target_generation;
}
fn retention(r: Relation) -> f32 {
  if (!valid(r)) { return -1.0; }
  let stale=exp(-f32(params.tick-r.last_seen_tick)/2400.0);
  return (r.familiarity + r.benefit + r.navigation + r.harm * 2.0)*stale;
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i = id.x;
  if (i >= params.agent_count) { return; }
  let a = agents[i];
  var s: SocialPerception;
  s.give_target = INVALID; s.force_target = INVALID; s.report_target=INVALID;
  s.force_value = -1000.0;
  if (a.alive == 0u) { social_perceptions[i] = s; return; }
  var rels: array<Relation, 8>;
  var weakest = 0u;
  for (var k=0u; k<8u; k++) {
    rels[k] = memory[i*8u+k];
    rels[k].familiarity = max(0.0, rels[k].familiarity - 0.00015);
    rels[k].benefit_evidence *= 0.9997; rels[k].harm_evidence *= 0.9998;
    rels[k].benefit *= 0.99995; rels[k].harm *= 0.99997;
    // Absence makes evidence stale slowly; it does not invent a negative encounter.
    if (retention(rels[k]) < retention(rels[weakest])) { weakest = k; }
  }
  // Direct outcomes have priority over incidental encounters, and are processed once.
  if (a.event_actor < params.agent_count && a.event_tick + 1u == params.tick && a.event_amount != 0.0) {
    var slot = weakest;
    for (var k=0u; k<8u; k++) {
      if (rels[k].target_slot == a.event_actor && rels[k].target_generation == a.event_generation) { slot=k; }
    }
    if (rels[slot].target_slot != a.event_actor || rels[slot].target_generation != a.event_generation) {
      rels[slot] = Relation(a.event_actor, a.event_generation, 0.03, 0.0, 0.0, params.tick, 0.0, 0.0, 0.0, 0.0, 0u, 0u);
    }
    if (a.event_amount > 0.0) { rels[slot].benefit = mix(rels[slot].benefit, 1.0, min(0.5,a.event_amount)); rels[slot].benefit_evidence += 1.0; }
    else { rels[slot].harm = mix(rels[slot].harm, 1.0, min(0.8,-a.event_amount+0.25)); rels[slot].harm_evidence += 1.0; }
  }
  if (a.guide_id<INVALID && a.guide_result!=0.0) {
    var slot=weakest;
    for (var k=0u;k<8u;k++) { if (rels[k].target_slot==a.guide_id && rels[k].target_generation==a.guide_generation) { slot=k; } }
    if (rels[slot].target_slot!=a.guide_id || rels[slot].target_generation!=a.guide_generation) {
      rels[slot]=Relation(a.guide_id,a.guide_generation,0.03,0.0,0.0,params.tick,0.0,0.0,0.0,0.0,0u,0u);
    }
    let accurate=select(0.0,1.0,a.guide_result>=-0.1);
    rels[slot].navigation=mix(rels[slot].navigation,accurate,0.3);
    rels[slot].navigation_evidence+=1.0;
  }
  var report_quality=0.0;
  for (var k=0u;k<4u;k++) {
    let quality=a.places[k].food*a.places[k].confidence*exp(-f32(params.tick-a.places[k].observed)/1800.0);
    if (quality>report_quality && length(a.places[k].position-a.position)>a.sensor_radius*0.5) { report_quality=quality; s.report_place=k; }
  }
  // Known companions are always checked, but only contribute inside actual perception.
  for (var k=0u; k<8u; k++) {
    if (!valid(rels[k])) { continue; }
    let b = agents[rels[k].target_slot];
    let delta = b.position-a.position;
    let distance = length(delta);
    if (distance > a.sensor_radius) { continue; }
    rels[k].familiarity = min(1.0, rels[k].familiarity + 0.003);
    rels[k].last_seen_tick = params.tick;
    let closeness = 1.0-distance/a.sensor_radius;
    let affection = rels[k].familiarity * params.social_weights.y + rels[k].benefit * params.social_weights.z;
    let benefit=rels[k].benefit*(0.5+0.5*rels[k].benefit_evidence/(1.0+rels[k].benefit_evidence));
    let harm=rels[k].harm;
    let value=max(0.0,benefit+rels[k].navigation-harm);
    let accessible=value*(0.25+0.75*closeness);
    if (accessible>s.companion_value) {
      s.companion_value=accessible; s.companion_position=b.position; s.companion_velocity=b.velocity;
    }
    // Expected harm supplies avoidance; useful companions enter destination
    // comparisons through predicted access, without another flocking force.
    s.avoidance -= unit_vector(delta)*harm*closeness;
    s.known_strength += max(0.0, value) * closeness;
    s.danger += rels[k].harm * closeness;
    // Intervene only in a recent harmful act witnessed locally, and only within reach.
    if (b.event_amount<0.0 && b.event_tick+1u==params.tick && b.event_actor<INVALID && b.event_actor!=i) {
      let aggressor=agents[b.event_actor];
      if (aggressor.alive!=0u && aggressor.generation==b.event_generation && length(aggressor.position-a.position)<=INTERACTION_RADIUS) {
        let intervention=affection*0.8-0.15-clamp(1.0-a.energy/35.0,0.0,1.0);
        if (intervention>s.force_value) { s.force_value=intervention; s.force_target=b.event_actor; }
      }
    }
    // Need is inferred from visible empty food reserves, never another's exact energy.
    let give = affection * max(0.0, 1.0-b.food/0.5) * clamp((a.food-1.5)/2.0,0.0,1.0) - rels[k].harm - 0.025;
    if (distance <= INTERACTION_RADIUS && give > s.give_value) { s.give_value=give; s.give_target=rels[k].target_slot; }
  }
  // Rotate cell traversal and sample up to two occupants per cell. No fixed compass bias.
  let center = vec2<i32>(a.position / params.world_size * 256.0);
  let radius = min(6, i32(ceil(a.sensor_radius/8.0)));
  let side = u32(radius*2+1);
  let cells = side*side;
  let offset = hash_u32(a.rng ^ params.tick) % cells;
  var encountered = 0u;
  var newcomer = INVALID;
  var new_distance = a.sensor_radius;
  for (var c=0u; c<cells; c++) {
    if (encountered >= 48u) { break; }
    let n = (c+offset)%cells;
    let cell = center + vec2<i32>(i32(n%side)-radius, i32(n/side)-radius);
    if (any(cell < vec2<i32>(0)) || any(cell >= vec2<i32>(256))) { continue; }
    let ci = u32(cell.y)*256u+u32(cell.x);
    var start=0u;
    if (ci>0u) { start=cell_offsets[ci-1u]; }
    let count=cell_offsets[ci]-start;
    for (var j=0u; j<min(count,2u); j++) {
      let cursor=start+(hash_u32(a.rng ^ ci ^ params.tick)+j)%count;
      let other=agent_indices[cursor];
      if (other==i || other>=params.agent_count) { continue; }
      let b=agents[other];
      let distance=length(b.position-a.position);
      if (b.alive==0u || distance>a.sensor_radius) { continue; }
      encountered++;
      var relationship: Relation;
      var known=false;
      for (var k=0u; k<8u; k++) {
        if (rels[k].target_slot==other && rels[k].target_generation==b.generation) { relationship=rels[k]; known=true; }
      }
      // Own delivery history is observable; the recipient's private map is not.
      let report=a.places[s.report_place];
      let already_told=known && relationship.last_report_tick>0u &&
        (relationship.last_report_observed>=report.observed || params.tick<relationship.last_report_tick+128u);
      if (distance<=INTERACTION_RADIUS && report_quality>0.2 && b.food<2.0 && !already_told) {
        let affection=relationship.familiarity*params.social_weights.y+relationship.benefit*params.social_weights.z;
        let value=report_quality*max(0.0,1.0-b.food/2.0)*(params.social_weights.y+affection)*2.0;
        if (value>s.report_value) { s.report_value=value; s.report_target=other; }
      }
      if (!known && distance<new_distance) { newcomer=other; new_distance=distance; }
      if (distance <= INTERACTION_RADIUS && b.food>0.1 && a.food<0.2) {
        let hunger=clamp(1.0-a.energy/25.0,0.0,1.0);
        let score=hunger*min(b.food,1.0)*1.6-0.5-relationship.familiarity*0.4-relationship.benefit-relationship.harm*0.2;
        if (score>s.force_value) { s.force_value=score; s.force_target=other; }
      }
    }
  }
  weakest=0u;
  for (var k=1u; k<8u; k++) { if (retention(rels[k])<retention(rels[weakest])) { weakest=k; } }
  if (newcomer<params.agent_count && retention(rels[weakest])<0.04) {
    rels[weakest]=Relation(newcomer,agents[newcomer].generation,0.01,0.0,0.0,params.tick,0.0,0.0,0.0,0.0,0u,0u);
  }
  s.force_value -= f32(encountered)*0.02;
  s.known_strength=min(s.known_strength,1.0);
  s.avoidance=unit_vector(s.avoidance)*min(length(s.avoidance),1.0);
  for (var k=0u; k<8u; k++) { memory[i*8u+k]=rels[k]; }
  social_perceptions[i]=s;
}
