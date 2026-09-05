@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> genomes:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];let p=perceptions[i];var d:Decision;d.target_id=INVALID;
 if(a.alive==0u){decisions[i]=d;return;}
 var x:array<f32,INPUT_COUNT>;
 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;
 x[3]=a.age/10000.0;x[4]=a.velocity.x/1.2;x[5]=a.velocity.y/1.2;
 x[6]=a.collected;x[7]=a.ingested;x[8]=a.spent;x[9]=a.received;
 x[10]=a.moved.x/1.2;x[11]=a.moved.y/1.2;
 x[12]=p.nearby_count/16.0;
 x[13]=f32(a.next_birth-min(a.next_birth,params.tick))/240.0;
 for(var k=0u;k<6u;k++){x[14u+k]=f32(a.action==k);}
 for(var k=0u;k<16u;k++){x[20u+k*2u]=p.regions[k].food;x[21u+k*2u]=p.regions[k].bodies/16.0;}
 for(var k=0u;k<8u;k++){let b=p.bodies[k];let n=52u+k*7u;
  if(b.slot<INVALID){x[n]=b.offset.x/a.sensor_radius;x[n+1u]=b.offset.y/a.sensor_radius;x[n+2u]=b.velocity.x/1.2;x[n+3u]=b.velocity.y/1.2;x[n+4u]=b.signal;x[n+5u]=1.0;x[n+6u]=b.signal_present;}
 }
 for(var k=0u;k<INPUT_COUNT;k++){if(!finite(x[k])){d.invalid=1u;}x[k]=clamp(x[k],-8.0,8.0);d.inputs[k]=x[k];}
 // Element copies avoid an array-valued temporary that crashes the Windows
 // shader compiler through Naga 24's HLSL path.
 var previous:array<f32,HIDDEN_COUNT>;
 for(var h=0u;h<HIDDEN_COUNT;h++){previous[h]=a.hidden[h];}
 var candidate:array<f32,HIDDEN_COUNT>;
 for(var h=0u;h<HIDDEN_COUNT;h++){
  let row=h*RECURRENT_ROW;var v=genomes[i*GENOME_SIZE+row+RECURRENT_ROW-1u];
  for(var k=0u;k<INPUT_COUNT;k++){v+=genomes[i*GENOME_SIZE+row+k]*x[k];}
  for(var k=0u;k<HIDDEN_COUNT;k++){v+=genomes[i*GENOME_SIZE+row+INPUT_COUNT+k]*previous[k];}
  if(!finite(v)){d.invalid=1u;v=0.0;}candidate[h]=tanh(v);
 }
 // Evolved gates read proposed features. Zero retains exactly; one replaces.
 // No forced forgetting, semantic memory slots, or mandatory gate bias.
 for(var h=0u;h<HIDDEN_COUNT;h++){
  let row=GATE_BASE+h*(HIDDEN_COUNT+1u);var v=genomes[i*GENOME_SIZE+row+HIDDEN_COUNT];
  for(var k=0u;k<HIDDEN_COUNT;k++){v+=genomes[i*GENOME_SIZE+row+k]*candidate[k];}
  if(!finite(v)){d.invalid=1u;v=0.0;}
  let gate=clamp(v,0.0,1.0);d.update_gates[h]=gate;
  d.hidden[h]=(1.0-gate)*previous[h]+gate*candidate[h];
  if(!finite(d.hidden[h])){d.invalid=1u;}
 }
 var out:array<f32,OUTPUT_COUNT>;
 for(var o=0u;o<OUTPUT_COUNT;o++){
  let row=OUTPUT_BASE+o*(HIDDEN_COUNT+1u);var v=genomes[i*GENOME_SIZE+row+HIDDEN_COUNT];
  for(var h=0u;h<HIDDEN_COUNT;h++){v+=genomes[i*GENOME_SIZE+row+h]*d.hidden[h];}
  if(!finite(v)){d.invalid=1u;v=0.0;}out[o]=v;
 }
 // Direct bounded requests; eight weight units span the genome interval [-4,4].
 d.mutation_probability=clamp(out[MUTATION_OUTPUT],0.0,1.0);
 d.mutation_magnitude=clamp(out[MUTATION_OUTPUT+1u],0.0,8.0);
 var best=-3.4e38;
 for(var k=0u;k<6u;k++){d.scores[k]=out[k];if(out[k]>best){best=out[k];d.selected_action=k;}}
 // Continuous actuator calibration: no minimum motion or preferred heading.
 let raw=vec2<f32>(out[6],out[7]);d.movement=unit_vector(raw)*tanh(length(raw)*params.physical.z);
 d.amount=1.0/(1.0+exp(-clamp(out[8],-20.0,20.0)));d.payload=tanh(out[9]);
 let force_raw=vec2<f32>(out[FORCE_OUTPUT],out[FORCE_OUTPUT+1u]);d.force=unit_vector(force_raw)*tanh(length(force_raw));
 best=-3.4e38;
 for(var k=0u;k<8u;k++){if(p.bodies[k].slot<INVALID && out[10u+k]>best){best=out[10u+k];d.target_id=p.bodies[k].slot;d.target_generation=p.bodies[k].generation;}}
 // Fault containment only: do not replace finite but ineffective intentions.
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;d.force=vec2<f32>(0);d.mutation_probability=0.0;d.mutation_magnitude=0.0;for(var h=0u;h<HIDDEN_COUNT;h++){d.hidden[h]=0.0;}}
 decisions[i]=d;
}
