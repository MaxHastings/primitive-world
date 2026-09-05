@group(0) @binding(0) var<storage,read> agents:array<Agent>;
@group(0) @binding(1) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(2) var<storage,read_write> decisions:array<Decision>;
@group(0) @binding(3) var<uniform> params:SimParams;
@group(0) @binding(4) var<storage,read> genomes:array<f32>;
@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) id:vec3<u32>){
 let i=id.x;if(i>=params.agent_count){return;}let a=agents[i];let p=perceptions[i];var d:Decision;d.target_id=INVALID;
 if(a.alive==0u){decisions[i]=d;return;}
 var x:array<f32,63>;
 x[0]=a.energy/100.0;x[1]=a.food/8.0;x[2]=p.resource_here;
 x[3]=a.age/10000.0;x[4]=a.velocity.x/1.2;x[5]=a.velocity.y/1.2;
 x[6]=a.collected;x[7]=a.ingested;x[8]=a.spent;x[9]=a.received;
 x[10]=a.moved.x/1.2;x[11]=a.moved.y/1.2;
 x[12]=select(0.0,a.event_amount,a.event_tick+1u==params.tick);
 x[13]=f32(a.next_birth-min(a.next_birth,params.tick))/240.0;
 x[14]=f32(a.action)/5.0;
 for(var k=0u;k<8u;k++){x[15u+k*3u]=p.samples[k].food;x[16u+k*3u]=p.samples[k].offset.x/a.sensor_radius;x[17u+k*3u]=p.samples[k].offset.y/a.sensor_radius;}
 for(var k=0u;k<4u;k++){let b=p.bodies[k];let n=39u+k*6u;
  if(b.slot<INVALID){x[n]=b.offset.x/a.sensor_radius;x[n+1u]=b.offset.y/a.sensor_radius;x[n+2u]=b.velocity.x/1.2;x[n+3u]=b.velocity.y/1.2;x[n+4u]=b.food/8.0;x[n+5u]=b.event;}
 }
 for(var k=0u;k<63u;k++){if(!finite(x[k])){d.invalid=1u;}x[k]=clamp(x[k],-8.0,8.0);d.inputs[k]=x[k];}
 for(var h=0u;h<16u;h++){
  let row=h*80u;var v=genomes[i*1518u+row+79u];
  for(var k=0u;k<63u;k++){v+=genomes[i*1518u+row+k]*x[k];}
  for(var k=0u;k<16u;k++){v+=genomes[i*1518u+row+63u+k]*a.hidden[k];}
  if(!finite(v)){d.invalid=1u;v=0.0;}d.hidden[h]=tanh(v);
 }
 var out:array<f32,14>;
 for(var o=0u;o<14u;o++){
  let row=1280u+o*17u;var v=genomes[i*1518u+row+16u];
  for(var h=0u;h<16u;h++){v+=genomes[i*1518u+row+h]*d.hidden[h];}
  if(!finite(v)){d.invalid=1u;v=0.0;}out[o]=v;
 }
 var best=-3.4e38;
 for(var k=0u;k<6u;k++){d.scores[k]=out[k];if(out[k]>best){best=out[k];d.selected_action=k;}}
 // Continuous actuator calibration: no minimum motion or preferred heading.
 let raw=vec2<f32>(tanh(out[6]*params.physical.z),tanh(out[7]*params.physical.z));d.movement=raw/max(1.0,length(raw));
 d.amount=1.0/(1.0+exp(-clamp(out[8],-20.0,20.0)));d.payload=tanh(out[9]);
 best=-3.4e38;
 for(var k=0u;k<4u;k++){if(p.bodies[k].slot<INVALID && out[10u+k]>best){best=out[10u+k];d.target_id=p.bodies[k].slot;d.target_generation=p.bodies[k].generation;}}
 // Fault containment only: do not replace finite but ineffective intentions.
 if(d.invalid!=0u){d.selected_action=NONE;d.movement=vec2<f32>(0);d.amount=0.0;d.payload=0.0;for(var h=0u;h<16u;h++){d.hidden[h]=0.0;}}
 decisions[i]=d;
}
