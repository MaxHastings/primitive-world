struct ArchiveState {
 count:u32, eligible:u32, pad0:u32, pad1:u32,
 keys:array<u32,256>, slots:array<u32,256>, ticks:array<u32,256>, identities:array<u32,256>, origins:array<u32,256>,
};
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> genomes:array<f32>;
@group(0) @binding(2) var<storage,read_write> archive:ArchiveState;
@group(0) @binding(3) var<storage,read_write> bodies:array<Agent>;
@group(0) @binding(4) var<storage,read_write> genes:array<f32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@compute @workgroup_size(64)
fn main(@builtin(workgroup_id) group:vec3<u32>,@builtin(local_invocation_index) lane:u32){
 let entry=group.x;if(entry>=archive.count){return;}
 let slot=archive.origins[entry];let fresh=archive.slots[entry]<INVALID;
 if(lane==0u && agents[slot].lineage_id==archive.identities[entry] && (fresh || bodies[entry].alive!=0u)){
  bodies[entry]=agents[slot];archive.ticks[entry]=params.tick+1u;
 }
 if(!fresh){return;}
 for(var k=lane;k<GENOME_SIZE;k+=64u){genes[entry*GENOME_SIZE+k]=genomes[slot*GENOME_SIZE+k];}
}
