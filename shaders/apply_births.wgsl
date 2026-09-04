@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> free_indices: array<u32>;
@group(0) @binding(2) var<storage, read> free_prefix: array<u32>;
@group(0) @binding(3) var<storage, read> birth_parents: array<u32>;
@group(0) @binding(4) var<storage, read> birth_prefix: array<u32>;
@group(0) @binding(5) var<uniform> params: SimParams;
@group(0) @binding(6) var<storage, read_write> stats: array<atomic<u32>>;
@group(0) @binding(7) var<storage, read_write> social_memory: array<Relation>;
@group(0) @binding(8) var<storage,read_write> neural_state:array<NeuralState>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let n=id.x;
  if (n>=min(birth_prefix[INVALID-1u],free_prefix[INVALID-1u])) { return; }
  let pi=birth_parents[n]; let ci=free_indices[n];
  var parent=agents[pi];
  let cost=params.sensor_and_padding.w;
  if (parent.alive==0u || agents[ci].alive!=0u || parent.energy<max(params.sensor_and_padding.z,cost+10.0) || parent.food<2.0) { return; }
  parent.next_birth=params.tick+params.lifecycle.y;
  parent.lifetime_births+=1u;
  var child: Agent;
  let angle=random01(parent.rng)*6.2831853;
  child.position=clamp(parent.position+vec2<f32>(cos(angle),sin(angle))*2.0,vec2<f32>(0.0),vec2<f32>(params.world_size));
  child.goal=child.position;
  // Transfer reserves, and dissipate 20% of the reproduction energy cost.
  child.energy=cost*0.8; parent.energy-=cost;
  child.food=1.0; parent.food-=1.0;
  child.max_age=9000.0+random01(parent.rng ^ ci)*2000.0;
  child.max_speed=parent.max_speed; child.sensor_radius=parent.sensor_radius;
  // Sparse, signed mutation preserves most of a functional controller. The
  // mutation process is fixed; no cost-free copying-fidelity gene is selected.
  for (var k=0u; k<128u; k++) {
    let key=parent.rng ^ ci ^ (k*0x9e3779b9u) ^ params.tick;
    let mutation=select(0.0,(random01(key)-0.5)*0.06,random01(key ^ 0xb5297a4du)<0.1);
    let bound=select(4.0,1.0,k<16u);
    child.genome[k]=clamp(parent.genome[k]+mutation,-bound,bound);
  }
  child.rng=hash_u32(parent.rng ^ ci ^ params.tick); child.alive=1u;
  child.generation=agents[ci].generation+1u;
  child.lineage_id=atomicAdd(&stats[10],1u)+100001u;
  child.parent_lineage=parent.lineage_id;
  child.birth_tick=params.tick;
  child.birth_parent_slot=pi;
  child.ancestry_depth=parent.ancestry_depth+1u;
  child.guide_id=INVALID; child.target_id=INVALID; child.event_actor=INVALID;
  agents[pi]=parent; agents[ci]=child;
  var blank:NeuralState; neural_state[ci]=blank;
  for (var k=0u; k<8u; k++) { social_memory[ci*8u+k]=Relation(INVALID,0u,0.0,0.0,0.0,params.tick,0.0,0.0,0.0,0.0,0u,0u); }
  atomicAdd(&stats[3],1u);
}
