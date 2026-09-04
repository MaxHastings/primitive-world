@group(0) @binding(5) var<storage,read> relations:array<Relation>;
struct Summary { values: array<u32,16>, };
@group(0) @binding(0) var<storage,read> agents: array<Agent>;
@group(0) @binding(1) var<storage,read> vegetation: array<u32>;
@group(0) @binding(2) var<storage,read> ground_words: array<vec4<u32>>;
@group(0) @binding(3) var<storage,read_write> summaries: array<Summary>;
@group(0) @binding(4) var<uniform> params: SimParams;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
  let group=id.x;
  if(group>=4096u){return;}
  var totals: Summary;
  for(var k=0u;k<64u;k++) {
    let i=group*64u+k;
    totals.values[4]+=vegetation[i];
    totals.values[5]+=ground_words[i*2u].x;
    totals.values[6]+=ground_words[i*2u].w;
    totals.values[7]+=ground_words[i*2u+1u].x;
    totals.values[14]+=ground_words[i*2u+1u].y;
    if(i<INVALID) {
      let a=agents[i];
      if(a.alive!=0u) {
        totals.values[0]+=1u;
        totals.values[11]+=u32(a.food>=1.5);
        totals.values[12]+=u32(a.energy<20.0);
        totals.values[13]+=u32(a.action==MOVE);
        totals.values[15]+=u32(a.action==EAT);
        for(var k=0u;k<8u;k++) {
          let r=relations[i*8u+k];
          if(r.target_slot>=INVALID || r.familiarity<0.2 || (r.benefit<0.1 && (r.navigation<0.2 || r.navigation_evidence<1.0))) {continue;}
          let other=agents[r.target_slot];
          if(other.alive==0u || other.generation!=r.target_generation) {continue;}
          let distance=length(other.position-a.position);
          totals.values[8]+=1u;
          totals.values[9]+=u32(distance<=a.sensor_radius);
          totals.values[10]+=u32(distance*10.0);
        }
        totals.values[1]+=u32(a.age<params.sensor_and_padding.y);
        totals.values[2]+=u32(round(a.food*1000.0));
        totals.values[3]+=u32(round(a.energy*1000.0));
      }
    }
  }
  summaries[group]=totals;
}
