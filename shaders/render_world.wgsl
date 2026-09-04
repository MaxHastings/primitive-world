struct Camera {
  center: vec2<f32>,
  zoom: f32,
  aspect: f32,
  lens: u32,
  point_size: f32,
  selected_id: u32,
  padding: u32,
};
struct SimParams {
  world_size: f32, resource_grid_size: u32, agent_count: u32, tick: u32,
  time_and_costs: vec4<f32>, resource_and_noise: vec4<f32>, sensor_and_padding: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var<storage, read> resources: array<u32>;
@group(1) @binding(1) var<uniform> params: SimParams;
@group(1) @binding(2) var<storage, read> occupancy: array<atomic<u32>>;

const GRID: u32 = 512u;
const AGENT_GRID: u32 = 256u;
const SCALE: f32 = 1000.0;

struct VertexOutput {
  @builtin(position) position: vec4<f32>,
  @location(0) ndc: vec2<f32>,
};

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VertexOutput {
  let positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  let p = positions[index];
  return VertexOutput(vec4<f32>(p, 0.0, 1.0), p);
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
  let world = camera.center + vec2<f32>(input.ndc.x * camera.aspect, -input.ndc.y) * params.world_size / (2.0 * camera.zoom);
  let inside_world = world.x >= 0.0 && world.x < params.world_size && world.y >= 0.0 && world.y < params.world_size;
  if (!inside_world) {
    return vec4<f32>(0.003, 0.005, 0.009, 1.0);
  }
  let uv = world / params.world_size;
  let cell = vec2<u32>(uv * f32(GRID));
  let value = f32(resources[cell.y * GRID + cell.x]) / SCALE;
  let occupancy_cell = vec2<u32>(uv * f32(AGENT_GRID));
  let density = min(f32(atomicLoad(&occupancy[occupancy_cell.y * AGENT_GRID + occupancy_cell.x])) / 24.0, 1.0);
  var tint = mix(vec3<f32>(0.004, 0.008, 0.016), vec3<f32>(0.12, 0.27, 0.14), smoothstep(0.02, 0.78, value));
  if (camera.lens == 1u) {
    let v = clamp(value, 0.0, 1.0);
    if (v < 0.5) {
      tint = mix(vec3<f32>(0.015, 0.02, 0.16), vec3<f32>(0.02, 0.72, 0.86), v * 2.0);
    } else {
      tint = mix(vec3<f32>(0.02, 0.72, 0.86), vec3<f32>(1.0, 0.72, 0.04), (v - 0.5) * 2.0);
    }
  }
  if (camera.lens == 2u) {
    tint = mix(vec3<f32>(0.01, 0.02, 0.10), vec3<f32>(0.92, 0.12, 0.04), smoothstep(0.0, 0.9, density));
  }
  let vignette = 1.0 - 0.16 * length(input.ndc);
  return vec4<f32>(tint * vignette, 1.0);
}
