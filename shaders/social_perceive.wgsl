@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> cell_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> agent_indices: array<u32>;
@group(0) @binding(3) var<storage, read_write> memory: array<Relation>;
@group(0) @binding(4) var<storage, read_write> social_perceptions: array<SocialPerception>;
@group(0) @binding(5) var<uniform> params: SimParams;

fn blank_relation() -> Relation {
  return Relation(INVALID,0u,0.0,0.0,0.0,params.tick,0.0,0.0,0.0,0.0,0u,0u);
}
fn blank_candidate() -> SocialCandidate {
  return SocialCandidate(INVALID,0u,vec2<f32>(0.0),vec2<f32>(0.0),0.0,0.0,0.0,0.0,0.0,0.0,INVALID,0u,0u,0.0,0u,0u);
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i=id.x;
  if (i>=params.agent_count) { return; }
  let a=agents[i];
  var s: SocialPerception;
  s.avoidance=vec2<f32>(0.0); s.known_strength=0.0; s.danger=0.0;
  s.give_target=INVALID; s.force_target=INVALID; s.give_value=-1000.0; s.force_value=-1000.0;
  s.report_target=INVALID; s.report_place=0u; s.companion_position=vec2<f32>(0.0);
  s.companion_velocity=vec2<f32>(0.0); s.companion_value=0.0; s.report_value=-1000.0;
  for (var k=0u; k<8u; k++) {
    s.candidates[k]=blank_candidate();
    // Legacy relation storage remains in the ABI for old checkpoints and
    // observation, but it is not a source of motivation in the kernel.
    memory[i*8u+k]=blank_relation();
  }
  if (a.alive==0u) { social_perceptions[i]=s; return; }

  // Expose nearby bodies and visible carried matter. This pass does not rank
  // people, infer virtue or danger, choose targets, or interpret signals.
  let center=vec2<i32>(a.position/params.world_size*256.0);
  let radius=min(6,i32(ceil(a.sensor_radius/8.0)));
  let side=u32(radius*2+1);
  let cells=side*side;
  let offset=hash_u32(a.rng^params.tick)%cells;
  var encountered=0u;
  for (var c=0u; c<cells; c++) {
    if (encountered>=48u) { break; }
    let n=(c+offset)%cells;
    let cell=center+vec2<i32>(i32(n%side)-radius,i32(n/side)-radius);
    if (any(cell<vec2<i32>(0)) || any(cell>=vec2<i32>(256))) { continue; }
    let ci=u32(cell.y)*256u+u32(cell.x);
    var start=0u;
    if (ci>0u) { start=cell_offsets[ci-1u]; }
    let count=cell_offsets[ci]-start;
    if (count==0u) { continue; }
    for (var j=0u; j<min(count,2u); j++) {
      let cursor=start+(hash_u32(a.rng^ci^params.tick)+j)%count;
      let other=agent_indices[cursor];
      if (other==i || other>=params.agent_count) { continue; }
      let b=agents[other];
      let distance=length(b.position-a.position);
      if (b.alive==0u || distance>a.sensor_radius) { continue; }
      encountered++;
      for (var k=0u; k<8u; k++) {
        if (s.candidates[k].target_slot==INVALID) {
          s.candidates[k]=SocialCandidate(other,b.generation,b.position,b.velocity,distance,b.food,0.0,0.0,0.0,0.0,b.event_actor,b.event_generation,b.event_tick,b.event_amount,0u,0u);
          break;
        }
      }
    }
  }
  social_perceptions[i]=s;
}
