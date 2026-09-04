@group(0) @binding(0)
var<storage, read> agents: array<Agent>;

@group(0) @binding(1)
var<storage, read_write> cursors: array<atomic<u32>>;

@group(0) @binding(2)
var<storage, read_write> agent_indices: array<u32>;

@group(0) @binding(3)
var<uniform> params: SimParams;

const OCCUPANCY_GRID: u32 = 256u;
const MAX_AGENTS: u32 = 100000u;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let agent_index = id.x;
    if (agent_index >= params.agent_count || agents[agent_index].alive == 0u) {
        return;
    }

    let normalized = clamp(
        agents[agent_index].position / params.world_size * f32(OCCUPANCY_GRID),
        vec2<f32>(0.0),
        vec2<f32>(f32(OCCUPANCY_GRID - 1u)),
    );
    let cell = vec2<u32>(normalized);
    let cell_index = cell.y * OCCUPANCY_GRID + cell.x;
    let slot = atomicAdd(&cursors[cell_index], 1u);

    if (slot < MAX_AGENTS) {
        agent_indices[slot] = agent_index;
    }
}
