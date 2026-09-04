struct Camera {
  center: vec2<f32>, zoom: f32, aspect: f32, lens: u32, point_size: f32, selected_id: u32, padding: u32,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<storage, read> agents: array<Agent>;
@group(1) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(1) @binding(2) var<storage, read> occupancy: array<atomic<u32>>;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
};

fn heat(value: f32, a: vec3<f32>, b: vec3<f32>) -> vec3<f32> {
  return mix(a, b, clamp(value, 0.0, 1.0));
}

@vertex
fn vs(@builtin(vertex_index) vertex: u32, @builtin(instance_index) index: u32) -> VertexOutput {
  let agent = agents[index];
  if (agent.alive == 0u) {
    return VertexOutput(vec4<f32>(2.0, 2.0, 0.0, 1.0), vec4<f32>(0.0));
  }
  let agent_offset = agent.position - camera.center;
  let ndc = vec2<f32>(agent_offset.x * 2.0 * camera.zoom / 2048.0 / camera.aspect, -agent_offset.y * 2.0 * camera.zoom / 2048.0);
  let p = perceptions[index];
  let cell = clamp(agent.position / 2048.0 * 256.0, vec2<f32>(vec2(0.0)), vec2<f32>(255.0));
  let density = min(f32(atomicLoad(&occupancy[u32(cell.y) * 256u + u32(cell.x)])) / 24.0, 1.0);
  var color = vec3<f32>(0.86, 0.94, 0.98);
  if (camera.lens == 1u) { color = heat(p.resource_here, vec3<f32>(0.15, 0.22, 0.94), vec3<f32>(1.0, 0.85, 0.18)); }
  if (camera.lens == 2u) { color = heat(density, vec3<f32>(0.12, 0.20, 0.9), vec3<f32>(1.0, 0.18, 0.12)); }
  if (camera.lens == 3u) { color = heat(agent.energy / 100.0, vec3<f32>(0.9, 0.12, 0.08), vec3<f32>(0.10, 0.95, 0.45)); }
  if (camera.lens == 4u) { color = heat(length(agent.velocity), vec3<f32>(0.15, 0.25, 0.85), vec3<f32>(0.95, 0.78, 0.16)); }
  if (camera.lens == 5u) { color = heat(fract(agent.age / 5000.0), vec3<f32>(0.15, 0.75, 0.95), vec3<f32>(0.94, 0.28, 0.72)); }
  if (camera.lens == 6u) { color = heat((agent.attention + 3.14159265) / 6.2831853, vec3<f32>(0.18, 0.18, 0.85), vec3<f32>(1.0, 0.5, 0.08)); }
  if (camera.lens == 7u) { color=heat(agent.food/8.0,vec3<f32>(0.7,0.15,0.1),vec3<f32>(0.2,0.95,0.9)); }
  if (camera.lens == 8u) {
    var colors=array<vec3<f32>,7>(vec3<f32>(0.5),vec3<f32>(0.3,0.65,1.0),vec3<f32>(0.2,0.9,0.3),vec3<f32>(1.0,0.75,0.1),vec3<f32>(0.95,0.4,0.9),vec3<f32>(1.0,0.12,0.1),vec3<f32>(0.4,1.0,1.0));
    color=colors[min(agent.action,6u)];
  }
  if (index == camera.selected_id) { color = vec3<f32>(1.0, 1.0, 1.0); }
  var corners = array<vec2<f32>, 6>(
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
    vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0));
  let pixel = camera.point_size * 2.0 * camera.zoom / 2048.0;
  let corner_offset = corners[vertex] * vec2<f32>(pixel / camera.aspect, pixel);
  return VertexOutput(vec4<f32>(ndc + corner_offset, 0.0, 1.0), vec4<f32>(color, 0.90));
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> { return input.color; }
