@group(0) @binding(0)
var<storage, read> prefix: array<u32>;

@group(0) @binding(1)
var<storage, read_write> cursors: array<atomic<u32>>;

const CELL_COUNT: u32 = 65536u;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= CELL_COUNT) {
        return;
    }

    let start = select(0u, prefix[id.x - 1u], id.x > 0u);
    atomicStore(&cursors[id.x], start);
}
