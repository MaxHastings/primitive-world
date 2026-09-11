@group(1) @binding(3) var<storage, read> ground_words: array<vec4<u32>>;
struct Camera {
  center: vec2<f32>,
  zoom: f32,
  aspect: f32,
  lens: u32,
  point_size: f32,
  selected_id: u32,
  selected_generation: u32,
  world_size: vec2<f32>,
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

fn resource_at(cell: vec2<u32>) -> f32 {
  return f32(resources[cell.y * GRID + cell.x]) / SCALE;
}

// The simulation stores food per cell, but the display should not expose the
// cell lattice as hard bands when a rectangular wallpaper scales the field.
// Bilinear interpolation keeps the underlying values unchanged while making
// patch edges continuous in screen space.
fn sample_resource(uv: vec2<f32>) -> f32 {
  let grid = clamp(
    uv * f32(GRID) - vec2<f32>(0.5),
    vec2<f32>(0.0),
    vec2<f32>(f32(GRID - 1u) - 0.001),
  );
  let lower = vec2<u32>(floor(grid));
  let upper = min(lower + vec2<u32>(1u), vec2<u32>(GRID - 1u));
  let blend = fract(grid);
  let top = mix(
    resource_at(vec2<u32>(lower.x, lower.y)),
    resource_at(vec2<u32>(upper.x, lower.y)),
    blend.x,
  );
  let bottom = mix(
    resource_at(vec2<u32>(lower.x, upper.y)),
    resource_at(vec2<u32>(upper.x, upper.y)),
    blend.x,
  );
  return mix(top, bottom, blend.y);
}

@vertex
fn vs(@builtin(vertex_index) index: u32) -> VertexOutput {
  var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
  let p = positions[index];
  return VertexOutput(vec4<f32>(p, 0.0, 1.0), p);
}

@fragment
fn fs(input: VertexOutput) -> @location(0) vec4<f32> {
  let world = camera.center + vec2<f32>(input.ndc.x * camera.aspect, -input.ndc.y) * params.world_size.y / (2.0 * camera.zoom);
  let inside_world = world.x >= 0.0 && world.x < params.world_size.x && world.y >= 0.0 && world.y < params.world_size.y;
  if (!inside_world) {
    return vec4<f32>(0.003, 0.005, 0.009, 1.0);
  }
  let uv = world / params.world_size.xy;
  let cell = vec2<u32>(uv * f32(GRID));
  let value = sample_resource(uv);
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
    let occupancy_cell = vec2<u32>(uv * f32(AGENT_GRID));
    let density = min(f32(atomicLoad(&occupancy[occupancy_cell.y * AGENT_GRID + occupancy_cell.x])) / 24.0, 1.0);
    tint = mix(vec3<f32>(0.01, 0.02, 0.10), vec3<f32>(0.92, 0.12, 0.04), smoothstep(0.0, 0.9, density));
  }
  let dropped = f32(ground_words[(cell.y*GRID+cell.x)*2u].x)/1000.0;
  // Painted/death-dropped food shares the natural food palette. Add a small
  // brightness lift for visibility without turning it into a different color.
  tint = min(tint + vec3<f32>(0.06, 0.10, 0.04) * clamp(dropped / 2.0, 0.0, 0.8), vec3<f32>(1.0));
  if (camera.lens==9u) {
    let habitat=clamp(bitcast<f32>(ground_words[(cell.y*GRID+cell.x)*2u+1u].z),0.0,1.0);
    let potential=sqrt(habitat);
    tint=mix(vec3<f32>(0.004,0.008,0.016),vec3<f32>(0.10,0.32,0.17),smoothstep(0.0,0.6,potential));
    tint=mix(tint,vec3<f32>(0.65,0.55,0.16),smoothstep(0.6,1.0,potential));
  }
  let vignette = 1.0 - 0.16 * length(input.ndc);
  return vec4<f32>(tint * vignette, 1.0);
}
