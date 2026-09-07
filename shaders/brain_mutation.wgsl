fn mutation_draw(rng:ptr<function,u32>)->f32 {
 *rng=(*rng)*1664525u+1013904223u;return f32((*rng)>>8u)/16777216.0;
}
fn mutation_temperature(rng:ptr<function,u32>)->f32 {
 return 0.125*pow(64.0,mutation_draw(rng));
}
fn mutate_parameter(value:f32,rng:ptr<function,u32>,probability:f32,magnitude:f32)->f32 {
 if(mutation_draw(rng)<probability) {
  return clamp(value+(mutation_draw(rng)*2.0-1.0)*magnitude,-4.0,4.0);
 }
 return value;
}
fn mutate_brain(g:ptr<function,array<f32,GENOME_SIZE>>,seed:u32,law:vec4<f32>) {
 var rng=seed;
 let scale=sqrt(mutation_temperature(&rng));
 let probability=clamp(law.x*scale,0.0,1.0);
 let magnitude=max(law.y*scale,0.000001);
 var changed=false;
 for(var k=0u;k<GENOME_SIZE;k++) {
  let old=(*g)[k];let next=mutate_parameter(old,&rng,probability,magnitude);
  changed=changed || next!=old;(*g)[k]=next;
 }
 if(!changed) {
  let k=min(u32(mutation_draw(&rng)*f32(GENOME_SIZE)),GENOME_SIZE-1u);
  let old=(*g)[k];let direction=select(-1.0,1.0,mutation_draw(&rng)>=0.5);
  let next=clamp(old+direction*magnitude,-4.0,4.0);
  (*g)[k]=select(next,clamp(old-direction*magnitude,-4.0,4.0),next==old);
 }
}
