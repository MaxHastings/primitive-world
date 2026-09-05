// Inclusive integer scan, up to 65,536 elements. Three dispatches instead of
// log2(N) full-buffer passes. All lanes participate in workgroup barriers.
@group(0) @binding(0) var<storage,read> source:array<u32>;
@group(0) @binding(1) var<storage,read_write> destination:array<u32>;
@group(0) @binding(2) var<storage,read_write> totals:array<u32>;
var<workgroup> values:array<u32,256>;
fn local_scan(lane:u32){
 for(var stride=1u;stride<256u;stride*=2u){
  var previous=0u;if(lane>=stride){previous=values[lane-stride];}
  workgroupBarrier();values[lane]+=previous;workgroupBarrier();
 }
}
@compute @workgroup_size(256)
fn blocks(@builtin(global_invocation_id) id:vec3<u32>,@builtin(local_invocation_index) lane:u32,@builtin(workgroup_id) group:vec3<u32>){
 var value=0u;if(id.x<arrayLength(&source)){value=source[id.x];}
 values[lane]=value;workgroupBarrier();local_scan(lane);
 if(id.x<arrayLength(&source)){destination[id.x]=values[lane];}
 if(lane==255u){totals[group.x]=values[lane];}
}
@compute @workgroup_size(256)
fn sums(@builtin(local_invocation_index) lane:u32){
 var value=0u;if(lane<arrayLength(&totals)){value=totals[lane];}
 values[lane]=value;workgroupBarrier();local_scan(lane);
 if(lane<arrayLength(&totals)){totals[lane]=values[lane];}
}
@compute @workgroup_size(256)
fn add(@builtin(global_invocation_id) id:vec3<u32>,@builtin(workgroup_id) group:vec3<u32>){
 if(id.x<arrayLength(&destination)&&group.x>0u){destination[id.x]+=totals[group.x-1u];}
}
