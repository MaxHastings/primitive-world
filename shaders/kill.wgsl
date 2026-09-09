struct InterventionParams {
  center: vec2<f32>,
  radius: f32,
  delta: f32,
};
@group(0) @binding(0) var<storage, read_write> agents: array<Agent>;
@group(0) @binding(1) var<uniform> intervention: InterventionParams;
@group(0) @binding(2) var<uniform> params: SimParams;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x >= INVALID) { return; }
  if (length(torus_delta(intervention.center,agents[id.x].position,params.world_size.xy)) <= intervention.radius) {
    if(agents[id.x].alive!=0u){



    }
    agents[id.x].alive = 0u;
    agents[id.x].energy = 0.0;
  }
}
