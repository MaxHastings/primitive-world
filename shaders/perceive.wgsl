@group(0) @binding(5) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> resources: array<u32>;
@group(0) @binding(2) var<storage, read> occupancy: array<atomic<u32>>;
@group(0) @binding(3) var<storage, read_write> perceptions: array<Perception>;
@group(0) @binding(4) var<uniform> params: SimParams;
fn food_at(pos: vec2<f32>) -> f32 {
  let cell = vec2<u32>(clamp(pos / params.world_size * 512.0, vec2<f32>(0.0), vec2<f32>(511.0)));
  return f32(resources[cell.y * 512u + cell.x] + min(atomicLoad(&ground[cell.y * 512u + cell.x].dropped), 8000u)) / 1000.0;
}
fn crowd_at(pos: vec2<f32>) -> f32 {
  let cell = vec2<u32>(clamp(pos / params.world_size * 256.0, vec2<f32>(0.0), vec2<f32>(255.0)));
  return f32(atomicLoad(&occupancy[cell.y * 256u + cell.x]));
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let i = id.x;
  if (i >= params.agent_count) { return; }
  var p: Perception;
  let a = agents[i];
  if (a.alive == 0u) { perceptions[i] = p; return; }
  let r = a.sensor_radius;
  var dirs = array<vec2<f32>, 4>(vec2<f32>(0,-1), vec2<f32>(1,0), vec2<f32>(0,1), vec2<f32>(-1,0));
  var foods: array<f32, 4>;
  for (var k=0u; k<4u; k++) {
    let pos = a.position + dirs[k] * r;
    foods[k] = food_at(pos);
    if ((params.neural_config.w&2u)!=0u && params.tick>=8u) {foods[k]=0.0;}
    p.crowd[k] = crowd_at(pos);
  }
  p.resource_here = food_at(a.position);
  p.resource_north = foods[0]; p.resource_east = foods[1];
  p.resource_south = foods[2]; p.resource_west = foods[3];
  p.local_count = crowd_at(a.position);
  p.local_density = clamp((p.local_count - 1.0) / 8.0, 0.0, 1.0);
  // An occupancy cell covers four food cells: this is a deliberately coarse forecast.
  let demand = max(0.0, p.local_count - 1.0) * 0.25 * params.resource_and_noise.x / 1000.0 * 8.0;
  p.projected_food = max(0.0, p.resource_here - demand);
  p.competition_pressure = clamp(demand / max(p.resource_here, 0.02), 0.0, 1.0);
  p.gradient = vec2<f32>(foods[1]-foods[3], foods[2]-foods[0]);
  perceptions[i] = p;
}
