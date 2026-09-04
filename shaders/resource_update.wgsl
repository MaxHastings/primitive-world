struct SimParams {
  world_size: f32,
  resource_grid_size: u32,
  agent_count: u32,
  tick: u32,
  time_and_costs: vec4<f32>,
  resource_and_noise: vec4<f32>,
  sensor_and_padding: vec4<f32>,
};

@group(0) @binding(0) var<storage, read_write> resources: array<u32>;
@group(0) @binding(1) var<uniform> params: SimParams;
@group(0) @binding(2) var<storage, read_write> fertility: array<f32>;

const GRID: u32 = 512u;
const MAX_RESOURCE: f32 = 1000.0;
const EVENT_LENGTH: u32 = 640u;

fn hash_u32(input: u32) -> u32 {
  var value = input;
  value = (value ^ 61u) ^ (value >> 16u);
  value = value + (value << 3u);
  value = value ^ (value >> 4u);
  value = value * 0x27d4eb2du;
  return value ^ (value >> 15u);
}

fn unit(seed: u32) -> f32 {
  return f32(hash_u32(seed) & 65535u) / 65535.0;
}

fn center_for(seed: u32) -> vec2<f32> {
  return vec2<f32>(
    unit(seed ^ 0xa341316cu),
    unit(seed ^ 0xc8013ea4u),
  ) * f32(GRID - 1u);
}

fn torus_delta(a: f32, b: f32) -> f32 {
  let direct = abs(a - b);
  return min(direct, f32(GRID) - direct);
}

fn patch_strength(position: vec2<f32>, center: vec2<f32>, radius: f32) -> f32 {
  let dx = torus_delta(position.x, center.x);
  let dy = torus_delta(position.y, center.y);
  let distance_squared = dx * dx + dy * dy;
  return exp(-distance_squared / (2.0 * radius * radius));
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= GRID || id.y >= GRID) { return; }
  let index = id.y * GRID + id.x;
  let old_value = resources[index];
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
  let season = 0.65 + 0.35 * sin(f32(params.tick) * 0.0011 + spatial_wave * 2.0);
  let food_fraction = f32(old_value) / MAX_RESOURCE;
  let depletion = smoothstep(0.0, 0.6, 1.0 - food_fraction);

  var soil = fertility[index];
  soil = clamp(
    soil + rain * 0.004 + (0.55 - soil) * 0.00008 - depletion * 0.00012,
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
  let growth = params.time_and_costs.y * (0.2 + 0.8 * spatial_signal) * (0.25 + 0.75 * habitat);
  let capacity = habitat * MAX_RESOURCE;
  resources[index] = u32(min(capacity, f32(old_value) + growth * 14.0));
}
