// Observer-only: production buffers are read-only. Not compiled into the game.
@group(0) @binding(0) var<storage,read> bodies:array<Agent>;
@group(0) @binding(1) var<storage,read> previous:array<Agent>;
@group(0) @binding(2) var<uniform> params:SimParams;
@group(0) @binding(3) var<storage,read> perceptions:array<Perception>;
@group(0) @binding(4) var<storage,read> claims:array<u32>;
@group(0) @binding(5) var<storage,read_write> totals:array<atomic<u32>>;
struct Lifetime {lineage:u32,generation:u32,flags:u32,last_signal:u32, juvenile_received:u32,juvenile_gathered:u32,adult_gathered:u32,padding:u32,};
@group(0) @binding(6) var<storage,read_write> lives:array<Lifetime>;
struct AuditEvent {kind:u32,tick:u32,lineage:u32,generation:u32,slot:u32,parent1:u32,parent2:u32,age:f32,energy:f32,quantity:u32,aux:u32,ancestry:u32,};
@group(0) @binding(7) var<storage,read_write> events:array<AuditEvent>;
@group(0) @binding(8) var<storage,read_write> scratch:array<atomic<u32>>;
@group(0) @binding(9) var<storage,read> decisions:array<Decision>;
struct ContactLife {lineage:u32,generation:u32,body_ticks:u32,food_ticks:u32,intent_ticks:u32,observed_ticks:u32,padding1:u32,padding2:u32,};
@group(0) @binding(10) var<storage,read> contacts:array<ContactLife>;
fn add(k:u32,n:u32) {let prior=atomicAdd(&totals[k],n);if(prior>0xffffffffu-n){atomicStore(&totals[1],1u);}}
fn emit(kind:u32,a:Agent,slot:u32,p1:u32,p2:u32,quantity:u32,aux:u32) {
 let at=atomicAdd(&totals[0],1u);if(at>=arrayLength(&events)){atomicStore(&totals[1],1u);return;}
 events[at]=AuditEvent(kind,params.tick+1u,a.lineage_id,a.generation,slot,p1,p2,a.age,a.energy,quantity,aux,a.ancestry_depth);
}
@compute @workgroup_size(64)
fn pre_fusion(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=INVALID){return;}let a=bodies[i];
 if(a.alive==ORGANISM && a.age>=params.sensor_and_padding.y){
  let d=decisions[i];let old=previous[i];
  if(d.selected_action==PRODUCE_PACKET && a.packets_produced==old.packets_produced){
   if(a.energy<a.packet_size){add(23u,1u);}
   else if(a.energy*d.amount<a.packet_size){add(24u,1u);}
   else{add(25u,1u);}
  }
 }
 if(a.alive!=PACKET){return;}
 add(26u,1u);
 let proposed=claims[INVALID+i];
 if(proposed<INVALID){
  let neighbor=bodies[proposed];
  if(neighbor.alive==PACKET && neighbor.parent_lineage!=a.parent_lineage){
   add(27u,1u);
   if(a.energy+neighbor.energy>params.sensor_and_padding.w){add(28u,1u);}
   if(a.energy+neighbor.energy>params.sensor_and_padding.w+48.0){add(30u,1u);}
  }
 }
 if(a.energy>32.0){add(29u,1u);}
 atomicAdd(&scratch[0],1u);atomicMax(&scratch[1],a.parent_lineage);atomicMax(&scratch[2],~a.parent_lineage);
 if(a.birth_tick==params.tick && a.age==0.0){
  add(2u,1u);let pi=a.birth_parent_slot;let parent=bodies[pi];let prior=previous[pi];let life=lives[pi];
  let matched=life.lineage==parent.lineage_id && life.generation==parent.generation;
  let qualified=parent.ancestry_depth>0u && matched && (life.flags&2u)!=0u && ((life.flags&1u)!=0u || (prior.age>=params.sensor_and_padding.y && parent.collected>0.0));
  emit(9u,a,i,a.parent_lineage,u32(qualified),0u,pi);
 }
 if(claims[2u*INVALID+i]!=INVALID){return;}let j=claims[INVALID+i];if(j>=INVALID){return;}let b=bodies[j];
 if(b.alive!=PACKET || a.parent_lineage==b.parent_lineage){return;}
 emit(6u,a,i,a.parent_lineage,b.parent_lineage,b.lineage_id,j);
}
@compute @workgroup_size(1)
fn coexistence() {
 if(atomicLoad(&scratch[0])>1u && atomicLoad(&scratch[1])!=~atomicLoad(&scratch[2])){add(3u,1u);}
}
@compute @workgroup_size(64)
fn post_tick(@builtin(global_invocation_id) id:vec3<u32>) {
 let i=id.x;if(i>=INVALID){return;}let a=bodies[i];let old=previous[i];
 let same=old.alive==ORGANISM && old.lineage_id==a.lineage_id && old.generation==a.generation;
 if(!same && a.alive!=ORGANISM){return;}
 var life=lives[i];
 if(life.lineage!=a.lineage_id || life.generation!=a.generation){life=Lifetime(a.lineage_id,a.generation,0u,0u,0u,0u,0u,0u);}
 if(a.alive==ORGANISM && a.ancestry_depth>0u && a.birth_tick==params.tick && a.lived_ticks==0u){
  add(4u,1u);atomicMax(&totals[22],a.ancestry_depth);emit(1u,a,i,a.parent_lineage,0u,0u,a.birth_parent_slot);
 }
 if(same){
  let harvested=u32(round(max(a.collected,0.0)*1000.0));add(21u,harvested);
  var exposed=false;var food_seen=false;
  for(var k=0u;k<16u;k++){exposed=exposed || abs(perceptions[i].regions[k].signal)>0.0;food_seen=food_seen || perceptions[i].regions[k].food>0.05;}
  if(exposed){life.last_signal=params.tick+1u;add(12u,1u);if(a.action==TRANSFER){add(13u,1u);}if(a.action==PRODUCE_PACKET){add(14u,1u);}}
  let recent=life.last_signal>0u && params.tick+1u-life.last_signal<=16u;
  if(a.signal_tick==params.tick+1u){add(11u,1u);}
  let produced=a.packets_produced-old.packets_produced;
  let donated=max(old.food-a.ingested+a.collected-a.food,0.0);
  if(recent && donated>0.000001 && a.action==TRANSFER){add(15u,1u);}
  if(recent && produced>0u){add(16u,produced);}
  if(old.ancestry_depth==0u){
   if(food_seen){add(19u,1u);}if(harvested>0u){add(20u,1u);}
  }else{
   if(old.age<params.sensor_and_padding.y){
    let received=u32(round(max(a.received,0.0)*1000.0));life.juvenile_received+=received;life.juvenile_gathered+=harvested;add(18u,harvested);
    if(a.received>0.0){add(5u,1u);add(6u,received);emit(8u,a,i,0u,0u,received,0u);}
    if(a.alive==ORGANISM && a.age>=params.sensor_and_padding.y){life.flags|=2u;add(7u,1u);emit(2u,a,i,0u,0u,life.juvenile_received,life.juvenile_gathered);}
   }else{
    life.adult_gathered+=harvested;add(17u,harvested);
    if(harvested>0u && (life.flags&1u)==0u){life.flags|=1u;add(8u,1u);emit(3u,a,i,0u,0u,harvested,0u);}
    if(produced>0u){add(9u,produced);if((life.flags&4u)==0u){life.flags|=4u;add(10u,1u);}emit(4u,a,i,0u,0u,produced,a.packets_produced);}
   }
   if(a.alive==0u){
    emit(5u,a,i,0u,0u,life.juvenile_received,life.adult_gathered);
    let c=contacts[i];if(c.lineage==a.lineage_id && c.generation==a.generation){emit(12u,a,i,c.body_ticks,c.food_ticks,c.intent_ticks,c.observed_ticks);}
   }
  }
 }
 lives[i]=life;
}
