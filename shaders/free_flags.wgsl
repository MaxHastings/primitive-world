@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read_write> flags: array<u32>;
@group(0) @binding(2) var<uniform> params: SimParams;
@group(0) @binding(3) var<storage, read_write> reservoir_claims: array<atomic<u32>>;
const CLEAR_CLAIMS:bool=true;
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  // Clear next admission's claims during existing slot classification, before
  // any newborn can challenge the pool. Includes the final RNG advance count.
  if (CLEAR_CLAIMS && id.x < arrayLength(&reservoir_claims)) { atomicStore(&reservoir_claims[id.x], 0u); }
  if (id.x >= INVALID) { return; }
  flags[id.x] = u32(id.x < params.agent_count && agents[id.x].alive == 0u);
}
