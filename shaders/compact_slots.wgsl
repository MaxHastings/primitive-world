@group(0) @binding(0) var<storage,read> free_flags:array<u32>;
@group(0) @binding(1) var<storage,read> prefix:array<u32>;
@group(0) @binding(2) var<storage,read_write> free_indices:array<u32>;
@group(0) @binding(3) var<storage,read_write> live_slots:array<u32>;
@group(0) @binding(4) var<storage,read_write> perceptions:array<Perception>;
@group(0) @binding(5) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(6) var<storage,read_write> cognitive_dispatch:array<u32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=INVALID){return;}
 if(i==0u){let count=INVALID-prefix[INVALID-1u];live_slots[0]=(count+LIVE_WORKGROUP_SIZE-1u)/LIVE_WORKGROUP_SIZE;live_slots[1]=1u;live_slots[2]=1u;live_slots[3]=count;cognitive_dispatch[0]=count;cognitive_dispatch[1]=1u;cognitive_dispatch[2]=1u;}
 if(free_flags[i]!=0u){
  free_indices[prefix[i]-1u]=i;
  // Clear an expired trace once, rather than writing large empty records every tick.
  if(decisions[i].evaluated!=0u){
   var d:Decision;d.target_id=INVALID;decisions[i]=d;
   var p:Perception;for(var k=0u;k<8u;k++){p.bodies[k].slot=INVALID;}perceptions[i]=p;
  }
 }else{live_slots[4u+i-prefix[i]]=i;}
}
