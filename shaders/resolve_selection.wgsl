@group(0) @binding(6) var<storage,read> memory: array<Relation>;
struct SelectionOutput {
  agent: Agent,
  perception: Perception,
  decision: Decision,
  social: SocialPerception,
  relations: array<Relation,8>,
  selected: u32,
  padding: array<u32, 5>,
};
@group(0) @binding(0) var<storage, read> agents: array<Agent>;
@group(0) @binding(1) var<storage, read> perceptions: array<Perception>;
@group(0) @binding(2) var<storage, read> decisions: array<Decision>;
@group(0) @binding(3) var<storage, read> social: array<SocialPerception>;
@group(0) @binding(4) var<storage, read> key: atomic<u32>;
@group(0) @binding(5) var<storage, read_write> output: SelectionOutput;

@compute @workgroup_size(64, 1, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let packed = atomicLoad(&key);
  if (packed != 0xffffffffu && (packed & 0x1ffffu) == id.x) {
    output.agent = agents[id.x];
    output.perception = perceptions[id.x];
    output.decision = decisions[id.x];
    output.social = social[id.x];
    for(var k=0u;k<8u;k++) { output.relations[k]=memory[id.x*8u+k]; }
    output.selected = id.x + 1u;
  }
}
