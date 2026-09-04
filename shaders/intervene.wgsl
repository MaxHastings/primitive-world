struct InterventionParams {
  center: vec2<f32>,
  radius: f32,
  delta: f32,
};

@group(0) @binding(0) var<storage, read_write> resource_values: array<u32>;
@group(0) @binding(1) var<uniform> intervention: InterventionParams;

const GRID: u32 = 512u;
const WORLD_SIZE: f32 = 2048.0;
const SCALE: f32 = 1000.0;

@compute @workgroup_size(8, 8, 1)
fn apply(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= GRID || id.y >= GRID) { return; }
  let world = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / f32(GRID) * WORLD_SIZE;
  if (length(world - intervention.center) > intervention.radius) { return; }
  let old_value = f32(resource_values[id.y * GRID + id.x]);
  resource_values[id.y * GRID + id.x] = u32(clamp(old_value + intervention.delta * SCALE, 0.0, SCALE));
}
