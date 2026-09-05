// Logical genome operations. Mirror src/brain.rs; allocation padding stays zero.
fn brain_draw(rng:ptr<function,u32>)->f32 {
 *rng=(*rng)*1664525u+1013904223u;return f32((*rng)>>8u)/16777216.0;
}
fn append_edge(g:ptr<function,array<f32,GENOME_SIZE>>,src:u32,dst:u32,w:f32){
 let b=EDGE_BASE+u32((*g)[1])*3u;(*g)[b]=f32(src);(*g)[b+1u]=f32(dst);(*g)[b+2u]=w;(*g)[1]+=1.0;
}
fn duplicate_node(g:ptr<function,array<f32,GENOME_SIZE>>,node:u32){
 let n=u32((*g)[0]);let count=u32((*g)[1]);if(n==HIDDEN_COUNT){return;}
 var extra=0u;
 for(var e=0u;e<count;e++){
  let b=EDGE_BASE+e*3u;let outgoing=u32((*g)[b])==INPUT_COUNT+node;
  let dst=u32((*g)[b+1u]);let to=dst==node||dst==HIDDEN_COUNT+node;
  extra+=u32(outgoing)+u32(to)+u32(outgoing&&to);
 }
 if(count+extra>MAX_EDGES){return;}
 (*g)[NODE_BIAS+n]=(*g)[NODE_BIAS+node];(*g)[GATE_BIAS+n]=(*g)[GATE_BIAS+node];
 for(var e=0u;e<count;e++){
  let b=EDGE_BASE+e*3u;let src=u32((*g)[b]);let dst=u32((*g)[b+1u]);
  let outgoing=src==INPUT_COUNT+node;let to=dst==node||dst==HIDDEN_COUNT+node;
  let w=select((*g)[b+2u],(*g)[b+2u]*0.5,outgoing);(*g)[b+2u]=w;
  if(outgoing){append_edge(g,INPUT_COUNT+n,dst,w);}
  if(to){
   let destination=select(HIDDEN_COUNT+n,n,dst<HIDDEN_COUNT);append_edge(g,src,destination,w);
   if(outgoing){append_edge(g,INPUT_COUNT+n,destination,w);}
  }
 }
 (*g)[0]=f32(n+1u);
}
fn delete_node(g:ptr<function,array<f32,GENOME_SIZE>>,node:u32){
 let n=u32((*g)[0]);if(n<=1u){return;}let count=u32((*g)[1]);var kept=0u;
 for(var e=0u;e<count;e++){
  let b=EDGE_BASE+e*3u;var src=u32((*g)[b]);var dst=u32((*g)[b+1u]);let w=(*g)[b+2u];
  if(src==INPUT_COUNT+node||dst==node||dst==HIDDEN_COUNT+node){continue;}
  if(src==INPUT_COUNT+n-1u){src=INPUT_COUNT+node;}
  if(dst==n-1u){dst=node;}if(dst==HIDDEN_COUNT+n-1u){dst=HIDDEN_COUNT+node;}
  let out=EDGE_BASE+kept*3u;(*g)[out]=f32(src);(*g)[out+1u]=f32(dst);(*g)[out+2u]=w;kept++;
 }
 for(var k=EDGE_BASE+kept*3u;k<EDGE_BASE+count*3u;k++){(*g)[k]=0.0;}
 (*g)[1]=f32(kept);(*g)[0]=f32(n-1u);
 (*g)[NODE_BIAS+node]=(*g)[NODE_BIAS+n-1u];(*g)[NODE_BIAS+n-1u]=0.0;
 (*g)[GATE_BIAS+node]=(*g)[GATE_BIAS+n-1u];(*g)[GATE_BIAS+n-1u]=0.0;
}
fn perturb(value:f32,rng:ptr<function,u32>,law:vec4<f32>)->f32 {
 if(brain_draw(rng)<law.x){return clamp(value+(brain_draw(rng)*2.0-1.0)*law.y,-4.0,4.0);}return value;
}
fn mutate_brain(g:ptr<function,array<f32,GENOME_SIZE>>,seed:u32,law:vec4<f32>){
 var rng=seed;let n=u32((*g)[0]);let choice=brain_draw(&rng);
 if(choice<law.z){duplicate_node(g,u32(brain_draw(&rng)*f32(n)));}
 else if(choice<2.0*law.z){delete_node(g,u32(brain_draw(&rng)*f32(n)));}
 else if(choice<2.0*law.z+law.w){
  let destination=u32(brain_draw(&rng)*f32(2u*n+OUTPUT_COUNT));var dst=0u;var src=0u;
  if(destination<n){dst=destination;src=u32(brain_draw(&rng)*f32(INPUT_COUNT+n));}
  else if(destination<2u*n){dst=HIDDEN_COUNT+destination-n;src=INPUT_COUNT+u32(brain_draw(&rng)*f32(n));}
  else{dst=2u*HIDDEN_COUNT+destination-2u*n;src=INPUT_COUNT+u32(brain_draw(&rng)*f32(n));}
  let weight=clamp((brain_draw(&rng)*2.0-1.0)*law.y,-4.0,4.0);let count=u32((*g)[1]);var exists=false;
  for(var e=0u;e<count;e++){let b=EDGE_BASE+e*3u;if(u32((*g)[b])==src&&u32((*g)[b+1u])==dst){exists=true;}}
  if(!exists&&count<MAX_EDGES){append_edge(g,src,dst,weight);}
 }else if(choice<2.0*(law.z+law.w)){
  let count=u32((*g)[1]);let edge=u32(brain_draw(&rng)*f32(count));
  if(count>0u){
   for(var k=EDGE_BASE+edge*3u;k<EDGE_BASE+(count-1u)*3u;k++){(*g)[k]=(*g)[k+3u];}
   for(var k=EDGE_BASE+(count-1u)*3u;k<EDGE_BASE+count*3u;k++){(*g)[k]=0.0;}
   (*g)[1]-=1.0;
  }
 }
 for(var h=0u;h<u32((*g)[0]);h++){
  (*g)[NODE_BIAS+h]=perturb((*g)[NODE_BIAS+h],&rng,law);(*g)[GATE_BIAS+h]=perturb((*g)[GATE_BIAS+h],&rng,law);
 }
 for(var o=0u;o<OUTPUT_COUNT;o++){(*g)[OUTPUT_BIAS+o]=perturb((*g)[OUTPUT_BIAS+o],&rng,law);}
 for(var e=0u;e<u32((*g)[1]);e++){let b=EDGE_BASE+e*3u+2u;(*g)[b]=perturb((*g)[b],&rng,law);}
}
