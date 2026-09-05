struct ArchiveState {
 count:u32, eligible:u32, pad0:u32, pad1:u32,
 keys:array<u32,256>, slots:array<u32,256>, ticks:array<u32,256>, identities:array<u32,256>, origins:array<u32,256>,
};
@group(0) @binding(0) var<storage,read> candidates:array<u32>;
@group(0) @binding(1) var<storage,read> agents:array<Agent>;
@group(0) @binding(2) var<storage,read_write> archive:ArchiveState;
@group(0) @binding(3) var<uniform> params:SimParams;
fn worst_slot()->u32 {
 var worst=0u;for(var i=1u;i<archive.count;i++){if(archive.keys[i]>archive.keys[worst]){worst=i;}}
 return worst;
}
@compute @workgroup_size(1)
fn main(){
 if(candidates[1]==0u && archive.pad0==0u){archive.pad0=params.tick+1u;}
 for(var i=0u;i<256u;i++){archive.slots[i]=INVALID;}
 let count=candidates[0];archive.eligible+=min(count,0xffffffffu-archive.eligible);
 var worst=worst_slot();
 for(var n=0u;n<count;n++){
  let slot=candidates[4u+n];let lineage=agents[slot].lineage_id;let key=hash_u32(lineage^params.lifecycle.x);
  var chosen=worst;
  if(archive.count<256u){chosen=archive.count;archive.count++;}
  else if(key>=archive.keys[worst]){continue;}
  archive.keys[chosen]=key;archive.slots[chosen]=slot;archive.origins[chosen]=slot;
  archive.identities[chosen]=lineage;archive.ticks[chosen]=params.tick+1u;
  worst=worst_slot();
 }
}
