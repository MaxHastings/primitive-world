fn mutation_draw(rng:ptr<function,u32>)->f32 {
 *rng=(*rng)*1664525u+1013904223u;return f32((*rng)>>8u)/16777216.0;
}
fn mutate_parameter(value:f32,rng:ptr<function,u32>,law:vec4<f32>)->f32 {
 if(mutation_draw(rng)<law.x) {
  return clamp(value+(mutation_draw(rng)*2.0-1.0)*law.y,-4.0,4.0);
 }
 return value;
}
fn mutate_brain(g:ptr<function,array<f32,GENOME_SIZE>>,seed:u32,law:vec4<f32>) {
 var rng=seed;
 for(var k=0u;k<GENOME_SIZE;k++) {
  (*g)[k]=mutate_parameter((*g)[k],&rng,law);
 }
}
