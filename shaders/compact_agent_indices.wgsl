@group(0) @binding(0) var<storage, read> flags: array<u32>;
@group(0) @binding(1) var<storage, read> prefix: array<u32>;
@group(0) @binding(2) var<storage, read_write> indices: array<u32>;
@group(0) @binding(3) var<storage, read> free_prefix: array<u32>;
@group(0) @binding(4) var<storage, read_write> dispatch: array<u32>;
@group(0) @binding(5) var<storage,read_write> stats:array<atomic<u32>>;
@group(0) @binding(6) var<storage,read_write> inheritance_dispatch:array<u32>;
@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  if (id.x == 0u) {
    let births=prefix[INVALID-1u];
    if(births>free_prefix[INVALID-1u]){counter_add(39u,births-free_prefix[INVALID-1u]);}
    if(atomicLoad(&stats[10])>0xffffffffu-2u*INVALID-1u){atomicStore(&stats[10],0u);atomicAdd(&stats[50],1u);}
    // Reserve headroom for this tick in one identity epoch before parallel allocation.
    dispatch[0] = (min(births,free_prefix[INVALID-1u])+63u)/64u;
    dispatch[1] = 1u; dispatch[2] = 1u;
    inheritance_dispatch[0]=min(births,free_prefix[INVALID-1u]);
    inheritance_dispatch[1]=1u;inheritance_dispatch[2]=1u;
  }
  if (id.x >= INVALID || flags[id.x] == 0u) { return; }
  indices[prefix[id.x] - 1u] = id.x;
}

fn counter_add(index:u32,value:u32)->u32 {
 let prior=atomicAdd(&stats[index],value);
 if(index!=0u && prior>0xffffffffu-value){atomicAdd(&stats[40u+index],1u);}
 return prior;
}
