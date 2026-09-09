// Successful births alone enter the rolling hereditary reservoir. The copied
// state is strictly the child's genome and inherited cognitive traits.
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> free_indices:array<u32>;
@group(0) @binding(2) var<storage,read> free_prefix:array<u32>;
@group(0) @binding(3) var<storage,read> parents:array<u32>;
@group(0) @binding(4) var<storage,read> birth_prefix:array<u32>;
@group(0) @binding(5) var<uniform> params:SimParams;
@group(0) @binding(6) var<storage,read> genomes0:array<f32>;
@group(0) @binding(7) var<storage,read> genomes1:array<f32>;
@group(0) @binding(8) var<storage,read_write> reservoir0:array<f32>;
@group(0) @binding(9) var<storage,read_write> reservoir1:array<f32>;

struct CognitiveTraits { active_mask:u32,padding:array<u32,3>,plasticity_rate:array<f32,HIDDEN_COUNT>,trace_retention:f32,learned_weight_retention:f32,mutation_scale:f32, };
@group(0) @binding(10) var<storage,read_write> reservoir_traits:array<CognitiveTraits>;
@group(0) @binding(11) var<storage,read_write> reservoir_rng:atomic<u32>;

fn copy_traits(a:Agent)->CognitiveTraits {
 var t:CognitiveTraits;t.active_mask=a.active_mask;t.padding=array<u32,3>(0u,0u,0u);t.plasticity_rate=a.plasticity_rate;
 t.trace_retention=a.trace_retention;t.learned_weight_retention=a.learned_weight_retention;t.mutation_scale=a.mutation_scale;return t;
}
@group(0) @binding(12) var<storage,read_write> claims:array<atomic<u32>>;

fn valid_birth(rank:u32)->bool {
 let births=birth_prefix[INVALID-1u];let free=free_prefix[INVALID-1u];
 if(rank>=min(births,free)){return false;}
 let pi=parents[(rank+hash_u32(params.tick)%births)%births];let child=agents[free_indices[rank]];
 return child.alive!=0u && child.birth_tick==params.tick && child.birth_parent_slot==pi;
}
fn replacement_slot(rank:u32)->u32 {return hash_u32(atomicLoad(&reservoir_rng)+rank)%4096u;}
// Each birth draws a slot independently. For collisions, the last birth in the
// deterministic allocation order wins, just as sequential replacement would.
@compute @workgroup_size(64)
fn claim(@builtin(global_invocation_id) id:vec3<u32>) {
 if(valid_birth(id.x)){atomicMax(&claims[replacement_slot(id.x)],id.x+1u);}
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let rank=id.x;if(!valid_birth(rank)){return;}let slot=replacement_slot(rank);
 if(atomicLoad(&claims[slot])!=rank+1u){return;}let ci=free_indices[rank];
 for(var k=0u;k<GENOME_BANK_STRIDE;k++){reservoir0[slot*GENOME_BANK_STRIDE+k]=genomes0[ci*GENOME_BANK_STRIDE+k];reservoir1[slot*GENOME_BANK_STRIDE+k]=genomes1[ci*GENOME_BANK_STRIDE+k];}
 reservoir_traits[slot]=copy_traits(agents[ci]);
}
@compute @workgroup_size(1)
fn advance() {atomicAdd(&reservoir_rng,min(birth_prefix[INVALID-1u],free_prefix[INVALID-1u]));}
