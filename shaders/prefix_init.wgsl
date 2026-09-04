@group(0) @binding(0)
var<storage, read> occupancy: array<atomic<u32>>;

@group(0) @binding(1)
var<storage, read_write> prefix: array<u32>;

const CELL_COUNT: u32 = 65536u;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x < CELL_COUNT) {
        prefix[id.x] = atomicLoad(&occupancy[id.x]);
    }
}
