struct InterventionParams {
  center: vec2<f32>,
  radius: f32,
  delta: f32,
};
@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<uniform> intervention: InterventionParams;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= INVALID) { return; }
  if (length(agents[id.x].position - intervention.center) <= intervention.radius) {
    if(agents[id.x].alive!=0u){
      agents[id.x].life.death_cause=3u;
      agents[id.x].life.death_tick=agents[id.x].life.first_observed_tick+agents[id.x].life.ticks;
      agents[id.x].life.end_energy=0.0;
    }
    agents[id.x].alive = 0u;
    agents[id.x].energy = 0.0;
  }
}
