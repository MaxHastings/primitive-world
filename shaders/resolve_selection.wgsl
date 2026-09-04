struct SelectionOutput{agent:Agent,perception:Perception,decision:Decision,selected:u32,padding:u32,};
@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read> decisions:array<Decision>;
@group(0) @binding(3) var<storage,read> key:atomic<u32>;
@group(0) @binding(4) var<storage,read_write> output:SelectionOutput;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let packed=atomicLoad(&key);
 if(packed!=0xffffffffu && (packed&0x1ffffu)==id.x){
  output.agent=agents[id.x];output.perception=perceptions[id.x];output.decision=decisions[id.x];output.selected=id.x+1u;
 }
}
