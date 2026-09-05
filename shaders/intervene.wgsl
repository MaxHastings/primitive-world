struct InterventionParams {
  center: vec2<f32>,
  radius: f32,
  delta: f32,
};

@group(0) @binding(0) var<storage, read_write> resource_values: array<u32>;
@group(0) @binding(1) var<uniform> intervention: InterventionParams;
@group(0) @binding(2) var<storage, read_write> ground: array<Ground>;

const GRID: u32 = 512u;
const WORLD_SIZE: f32 = 2048.0;
const SCALE: f32 = 1000.0;

@compute @workgroup_size(8, 8, 1)
fn apply(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= GRID || id.y >= GRID) { return; }
  let world = (vec2<f32>(id.xy) + vec2<f32>(0.5)) / f32(GRID) * WORLD_SIZE;
  if (length(world - intervention.center) > intervention.radius) { return; }
  let index = id.y * GRID + id.x;
  // Hand-painted food is dropped supply: it remains harvestable anywhere and
  // is never clipped by the moving vegetation capacity on the next tick.
  if (intervention.delta > 0.0) {
    atomicAdd(&ground[index].dropped, u32(round(intervention.delta * SCALE)));
  } else {
    let old_value = f32(resource_values[index]);
    resource_values[index] = u32(clamp(old_value + intervention.delta * SCALE, 0.0, SCALE));
    // The erase brush also removes manually painted and death-dropped food.
    // This pass owns each cell and is dispatched separately from agent actions.
    let dropped = atomicLoad(&ground[index].dropped);
    let removed = min(dropped, u32(round(-intervention.delta * SCALE)));
    atomicStore(&ground[index].dropped, dropped - removed);
  }
}
