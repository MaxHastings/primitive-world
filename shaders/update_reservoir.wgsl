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

struct CognitiveTraits { active_mask:u32,padding:array<u32,2>,packet_size:f32,plasticity_rate:array<f32,HIDDEN_COUNT>,trace_retention:f32,learned_weight_retention:f32,parameter_mutation_rate:f32,parameter_mutation_step:f32,topology_mutation_rate:f32, };
@group(0) @binding(10) var<storage,read_write> reservoir_traits:array<CognitiveTraits>;
@group(0) @binding(11) var<storage,read_write> reservoir_rng:atomic<u32>;

fn copy_traits(a:Agent)->CognitiveTraits {
 var t:CognitiveTraits;t.active_mask=a.active_mask;t.padding=array<u32,2>(0u,0u);t.packet_size=a.packet_size;t.plasticity_rate=a.plasticity_rate;
 t.trace_retention=a.trace_retention;t.learned_weight_retention=a.learned_weight_retention;t.parameter_mutation_rate=a.parameter_mutation_rate;t.parameter_mutation_step=a.parameter_mutation_step;t.topology_mutation_rate=a.topology_mutation_rate;return t;
}
@group(0) @binding(12) var<storage,read_write> claims:array<atomic<u32>>;

fn valid_birth(slot:u32)->bool {
 if(slot>=INVALID){return false;}let child=agents[slot];
 return child.alive==ORGANISM && child.ancestry_depth>0u && child.birth_tick==params.tick && child.lived_ticks==0u;
}
fn replacement_slot(slot:u32)->u32 {return hash_u32(atomicLoad(&reservoir_rng)+slot)%4096u;}
@compute @workgroup_size(64)
fn claim(@builtin(global_invocation_id) id:vec3<u32>) {
 if(valid_birth(id.x)){atomicMax(&claims[replacement_slot(id.x)],id.x+1u);atomicAdd(&claims[4096],1u);}
}
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>) {
 let ci=id.x;if(!valid_birth(ci)){return;}let slot=replacement_slot(ci);
 if(atomicLoad(&claims[slot])!=ci+1u){return;}
 for(var k=0u;k<GENOME_BANK_STRIDE;k++){reservoir0[slot*GENOME_BANK_STRIDE+k]=genomes0[ci*GENOME_BANK_STRIDE+k];reservoir1[slot*GENOME_BANK_STRIDE+k]=genomes1[ci*GENOME_BANK_STRIDE+k];}
 reservoir_traits[slot]=copy_traits(agents[ci]);
}
@compute @workgroup_size(1)
fn advance() {atomicAdd(&reservoir_rng,atomicLoad(&claims[4096]));}
