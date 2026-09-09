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
  let delta = torus_delta(center,position,vec2<f32>(f32(GRID)));
  let distance_squared = dot(delta,delta);
  return exp(-distance_squared / (2.0 * radius * radius));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= GRID || id.y >= GRID) { return; }
  let index = id.y * GRID + id.x;
  let old_value = resources[index];
  let environment_tick=params.lifecycle.w;
  if (params.mutation.z!=0.0) {
    let phase=f32(environment_tick%8192u)/8192.0;
    let blend=phase*phase*(3.0-2.0*phase);
    ground[index].habitat=mix(terrain[index].x,terrain[index].y,blend);
    ground[index].productivity=mix(terrain[index].z,terrain[index].w,blend);
  }
  // Evaluate weather in canonical coordinates so the entire environmental
  // history rotates, not just the initial food picture. Brains keep world axes.
  var canonical = id.xy;
  for (var turn=0u; turn<params.lifecycle.z; turn++) {
    canonical=vec2<u32>(canonical.y, GRID-1u-canonical.x);
  }
  let position = vec2<f32>(canonical);
  let canonical_index = canonical.y * GRID + canonical.x;

  let event_id = environment_tick / EVENT_LENGTH;
  let event_phase = f32(environment_tick % EVENT_LENGTH) / f32(EVENT_LENGTH);
  let event_blend = event_phase * event_phase * (3.0 - 2.0 * event_phase);
  let grid_size=vec2<f32>(f32(GRID));
  let rain_start=center_for(event_id ^ 0x1f123bb5u);let rain_end=center_for((event_id + 1u) ^ 0x1f123bb5u);
  let drought_start=center_for(event_id ^ 0x9e3779b9u);let drought_end=center_for((event_id + 1u) ^ 0x9e3779b9u);
  let rain_center = wrap_world(rain_start+torus_delta(rain_start,rain_end,grid_size)*event_blend,grid_size);
  let drought_center = wrap_world(drought_start+torus_delta(drought_start,drought_end,grid_size)*event_blend,grid_size);
  let rain_active = mix(select(0.0, 1.0, unit(event_id ^ 0x62a9d9edu) > 0.34),
    select(0.0, 1.0, unit((event_id + 1u) ^ 0x62a9d9edu) > 0.34), event_blend);
  let drought_active = mix(select(0.0, 1.0, unit(event_id ^ 0x7f4a7c15u) > 0.72),
    select(0.0, 1.0, unit((event_id + 1u) ^ 0x7f4a7c15u) > 0.72), event_blend);
  let rain = rain_active * patch_strength(position, rain_center, 70.0);
  let drought = drought_active * patch_strength(position, drought_center, 105.0);

  let spatial_wave = 0.5 + 0.5 * sin(position.x * 6.2831853 * 3.0 / f32(GRID)) * cos(position.y * 6.2831853 * 2.0 / f32(GRID));
  // Regional lean seasons operate from tick zero. Their spatial
  // mean remains constant: one region's lean period is another's abundance,
  // independently of agent behavior.
  let seasonality = params.environment.z;
  let seasonal_phase = mix(
    spatial_wave,
    0.5 + 0.5 * sin((position.x + position.y) * 6.2831853 / f32(GRID) + f32(environment_tick) * 0.00017),
    seasonality,
  );
  // Regional seasons span the full cycle instead of putting nearly every
  // source into abundance and scarcity at the same time.
  let season = 0.65
    + (0.35 + 0.30 * seasonality)
      * sin(f32(environment_tick) * 0.0011 + seasonal_phase * 6.2831853);
  let harvested = atomicExchange(&ground[index].extracted, 0u);
  // Collection accounting is recorded atomically at physical collection time.
  let depletion = f32(harvested) / MAX_RESOURCE;

  var soil = fertility[index];
  soil = clamp(
    soil + params.environment.x * (rain * 0.004 + (0.55 - soil) * 0.00008 - depletion * 0.004),
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
  let jitter = mix(unit(canonical_index ^ event_id), unit(canonical_index ^ (event_id + 1u)), event_blend);
  let spatial_signal = mix(0.5, 0.55 * spatial_wave + 0.45 * jitter, heterogeneity);
  // Persistent geography separates fertile patches from barren travel space.
  // Productivity is normalized at world creation to concentrate growth, rather
  // than simply deleting most of the world's potential food supply.
  // The temporary coverage floor reaches zero by world tick 100,000.
  let opening_cover = params.time_and_costs.x;
  // Temporary ground cover makes travel space useful for feeding too. It
  // fades with world age without changing the underlying map.
  let geography = max(ground[index].habitat, opening_cover);
  let productivity = max(ground[index].productivity, opening_cover);
  let growth = params.time_and_costs.y * (0.2 + 0.8 * spatial_signal) * (0.25 + 0.75 * habitat) * productivity;
  let capacity = habitat * MAX_RESOURCE * geography;
  // Capacity is a target, not an instantaneous deletion threshold. Vegetation
  // recedes over environmental time, including when a patch vanishes. Carry
  // fractional losses so slow ecology does not round up to one unit per tick.
  let old_food = f32(old_value);
  let delta = select(min(growth * 14.0, max(capacity - old_food, 0.0)),
    -(old_food - capacity) * 0.01 * params.environment.x, old_food > capacity);
  let accumulation = ground[index].remainder + delta;
  let whole = floor(accumulation);
  // Tiny negative deltas can round their fractional part to 1 in float32.
  ground[index].remainder = min(accumulation - whole, 0.99999994);
  var next = old_value;
  if (whole < 0.0) { next -= min(old_value, u32(-whole)); }
  else { next += u32(whole); }
  // Dropped/manual food lives in ground[].dropped and is unaffected.
  if (next>=old_value) { ground[index].produced += next-old_value; }
  else { ground[index].weather_loss += old_value-next; }
  resources[index]=next;
}
