@group(0) @binding(3) var<storage, read_write> ground: array<Ground>;
@group(0) @binding(0) var<storage, read_write> resources: array<u32>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<storage, read_write> fertility: array<f32>;
@group(0) @binding(4) var<storage, read> terrain: array<vec4<f32>>;

const GRID: u32 = 512u;
const MAX_RESOURCE: f32 = 1000.0;
const EVENT_LENGTH: u32 = 640u;



fn unit(seed: u32) -> f32 {
  return f32(hash_u32(seed) & 65535u) / 65535.0;
}

fn center_for(seed: u32) -> vec2<f32> {
  return vec2<f32>(
    unit(seed ^ params.lifecycle.x ^ 0xa341316cu),
    unit(seed ^ params.lifecycle.x ^ 0xc8013ea4u),
  ) * f32(GRID - 1u);
}

fn patch_strength(position: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
  let delta = position-center;
  let distance_squared = dot(delta,delta);
  return exp(-distance_squared / (2.0 * radius * radius));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= GRID || id.y >= GRID) { return; }
  let index = id.y * GRID + id.x;
  let old_value = resources[index];
  if (params.lifecycle.w!=0u) {
    let phase=f32(params.tick%8192u)/8192.0;
    let blend=phase*phase*(3.0-2.0*phase);
    ground[index].habitat=mix(terrain[index].x,terrain[index].y,blend);
    ground[index].productivity=mix(terrain[index].z,terrain[index].w,blend);
  }
  let position = vec2<f32>(f32(id.x), f32(id.y));

  let event_id = params.tick / EVENT_LENGTH;
  let event_phase = f32(params.tick % EVENT_LENGTH) / f32(EVENT_LENGTH);
  let rain_center = mix(center_for(event_id ^ 0x1f123bb5u), center_for((event_id + 1u) ^ 0x1f123bb5u), event_phase);
  let drought_center = mix(center_for(event_id ^ 0x9e3779b9u), center_for((event_id + 1u) ^ 0x9e3779b9u), event_phase);
  let rain_active = select(0.0, 1.0, unit(event_id ^ 0x62a9d9edu) > 0.34);
  let drought_active = select(0.0, 1.0, unit(event_id ^ 0x7f4a7c15u) > 0.72);
  let rain = rain_active * patch_strength(position, rain_center, 70.0);
  let drought = drought_active * patch_strength(position, drought_center, 105.0);

  let spatial_wave = 0.5 + 0.5 * sin(f32(id.x) * 0.037) * cos(f32(id.y) * 0.029);
  // Regional seasons span the full cycle instead of putting nearly every
  // source into abundance and scarcity at the same time.
  let season = 0.65 + 0.35 * sin(f32(params.tick) * 0.0011 + spatial_wave * 6.2831853);
  let harvested = atomicExchange(&ground[index].extracted, 0u);
  // Collection accounting is recorded atomically at physical collection time.
  let depletion = f32(harvested) / MAX_RESOURCE;

  var soil = fertility[index];
  soil = clamp(
    soil + rain * 0.004 + (0.55 - soil) * 0.00008 - depletion * 0.004,
    0.02,
    1.0,
  );
  fertility[index] = soil;

  let habitat = clamp(
    0.12 + soil * (0.55 + 0.45 * season) + rain * 0.28 - drought * 0.52,
    0.02,
    1.0,
  );
  let heterogeneity = clamp(params.resource_and_noise.z, 0.0, 1.0);
  let jitter = unit(index ^ event_id);
  let spatial_signal = mix(0.5, 0.55 * spatial_wave + 0.45 * jitter, heterogeneity);
  // Persistent geography separates fertile patches from barren travel space.
  // Productivity is normalized at world creation to concentrate growth, rather
  // than simply deleting most of the world's potential food supply.
  let geography = ground[index].habitat;
  let growth = params.time_and_costs.y * (0.2 + 0.8 * spatial_signal) * (0.25 + 0.75 * habitat) * ground[index].productivity;
  let capacity = habitat * MAX_RESOURCE * geography;
  let accumulation=ground[index].remainder + growth*14.0;
  let increment=u32(accumulation);
  ground[index].remainder=fract(accumulation);
  // Food painted into barren space remains harvestable, but does not regrow.
  let next=select(old_value,min(u32(capacity),old_value+increment),geography>0.0);
  if (next>=old_value) { ground[index].produced += next-old_value; }
  else { ground[index].weather_loss += old_value-next; }
  resources[index]=next;
}
