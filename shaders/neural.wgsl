// GRU v3. Inputs and traces are captured in the same invocation as the decision.
fn neural_direction(k:u32)->vec2<f32> {
  var ds=array<vec2<f32>,8>(vec2<f32>(0,-1),vec2<f32>(0.70710678,-0.70710678),vec2<f32>(1,0),vec2<f32>(0.70710678,0.70710678),vec2<f32>(0,1),vec2<f32>(-0.70710678,0.70710678),vec2<f32>(-1,0),vec2<f32>(-0.70710678,-0.70710678));
  return ds[k];
}
fn neural_food(pos:vec2<f32>)->f32 {
  let cell=ground_index(pos);
  return min(1.0,f32(neural_resources[cell]+min(atomicLoad(&neural_ground[cell].dropped),8000u))/1000.0);
}
fn neural_decision(i:u32,a:Agent,p:Perception,s:SocialPerception)->Decision {
  var d:Decision; d.target_id=INVALID;d.goal=a.position;
  var st=neural_state[i];
  let fresh=st.generation!=a.generation || st.valid==0u;
  // A newborn waits until the next shared boundary. An unrecorded recurrent
  // step between bridge frames would invalidate sequence-training parity.
  let boundary=params.tick%max(params.neural_config.z,1u)==0u;
  if (fresh && !boundary) {return d;}
  if (boundary) {
    if (fresh || (params.neural_config.w&1u)!=0u) {for(var h=0u;h<NEURAL_HIDDEN;h++){st.hidden[h]=0.0;}}
    st.generation=a.generation;st.tick=params.tick;st.valid=1u;st.energy=a.energy;st.food=a.food;
    st.observation[0]=clamp(a.energy/100.0,0.0,1.0);st.observation[1]=clamp(a.food/8.0,0.0,1.0);
    st.observation[2]=min(p.resource_here,1.0);
    for(var k=0u;k<8u;k++) {
      st.observation[3u+k]=neural_food(a.position+neural_direction(k)*a.sensor_radius);
      if ((params.neural_config.w&2u)!=0u && params.tick>=8u){st.observation[3u+k]=0.0;}
    }
    st.observation[11]=p.local_density;
    st.observation[12]=clamp(a.position.y/a.sensor_radius,0.0,1.0);
    st.observation[13]=clamp((params.world_size-a.position.x)/a.sensor_radius,0.0,1.0);
    st.observation[14]=clamp((params.world_size-a.position.y)/a.sensor_radius,0.0,1.0);
    st.observation[15]=clamp(a.position.x/a.sensor_radius,0.0,1.0);
    // Social values are perceptual consequences of prior encounters. They are
    // not a built-in instruction to cooperate, attack, or follow.
    st.observation[16]=clamp(s.companion_value,0.0,1.0);
    st.observation[17]=clamp(s.danger,0.0,1.0);
    st.observation[18]=clamp(s.give_value,0.0,1.0);
    st.observation[19]=clamp(s.force_value,0.0,1.0);
    st.observation[20]=clamp(s.report_value,0.0,1.0);
    st.observation[21]=clamp(s.known_strength,0.0,1.0);
    st.observation[22]=clamp(a.velocity.x/max(a.max_speed,0.1),-1.0,1.0);
    st.observation[23]=clamp(a.velocity.y/max(a.max_speed,0.1),-1.0,1.0);
    for(var k=0u;k<NEURAL_ACTIONS;k++){st.mask[k]=1.0;}
    st.mask[1]=f32(a.food<7.999 && p.resource_here>0.001);
    st.mask[2]=f32(a.food>0.001 && a.energy<99.0);
    for(var k=0u;k<8u;k++) {
      let dir=neural_direction(k);let to=clamp(a.position+dir,vec2<f32>(0),vec2<f32>(params.world_size));
      st.mask[3u+k]=f32(length(to-a.position)>0.001);
    }
    st.mask[11]=f32(s.give_target<INVALID && a.food>1.5 && a.energy>30.0);
    st.mask[12]=f32(params.social_weights.w>0.5 && s.force_target<INVALID && a.energy>30.0);
    st.mask[13]=f32(params.lifecycle.z!=0u && s.report_target<INVALID && a.energy>30.0 && params.tick>=a.last_communication+4u);
    st.before=st.hidden;
    var ix:array<f32,96>;var hx:array<f32,96>;
    for(var k=0u;k<96u;k++) {
      ix[k]=neural_weights.input_bias[k];hx[k]=neural_weights.recurrent_bias[k];
      for(var j=0u;j<NEURAL_OBSERVATIONS;j++){ix[k]+=neural_weights.input[k*NEURAL_OBSERVATIONS+j]*st.observation[j];}
      for(var j=0u;j<NEURAL_HIDDEN;j++){hx[k]+=neural_weights.recurrent[k*NEURAL_HIDDEN+j]*st.before[j];}
    }
    for(var h=0u;h<NEURAL_HIDDEN;h++) {
      let r=1.0/(1.0+exp(-(ix[h]+hx[h])));let z=1.0/(1.0+exp(-(ix[32u+h]+hx[32u+h])));
      let n=tanh(ix[64u+h]+r*hx[64u+h]);st.hidden[h]=(1.0-z)*n+z*st.before[h];
    }
    var best=-1e20;var total=0.0;var chosen=0u;
    for(var k=0u;k<NEURAL_ACTIONS;k++) {
      var value=neural_weights.output_bias[k];
      for(var h=0u;h<NEURAL_HIDDEN;h++){value+=neural_weights.output[k*NEURAL_HIDDEN+h]*st.hidden[h];}
      st.logits[k]=value;
      if(st.mask[k]>0.5 && value>best){best=value;chosen=k;}
    }
    for(var k=0u;k<NEURAL_ACTIONS;k++){st.probabilities[k]=select(0.0,exp(st.logits[k]-best),st.mask[k]>0.5);total+=st.probabilities[k];}
    let draw=min(random01(a.rng^params.tick^0x4638ab21u),0.999999);var accum=0.0;var sampled=false;
    for(var k=0u;k<NEURAL_ACTIONS;k++) {
      st.probabilities[k]/=total;accum+=st.probabilities[k];
      if(params.neural_config.y==0u && !sampled && draw<accum){chosen=k;sampled=true;}
    }
    st.after=st.hidden;st.choice=chosen;neural_state[i]=st;
  }
  // No authored utility, remembered destination selection, or eating override.
  d.selected_action=WAIT;
  if(st.choice==1u && a.food<7.999 && p.resource_here>0.001){d.selected_action=HARVEST;}
  if(st.choice==2u && a.food>0.001 && a.energy<99.0){d.selected_action=EAT;}
  if(st.choice>=3u && st.choice<=10u){d.selected_action=MOVE;d.goal=clamp(a.position+neural_direction(st.choice-3u)*a.max_speed,vec2<f32>(0),vec2<f32>(params.world_size));}
  if(st.choice==11u && s.give_target<INVALID && a.food>1.5 && a.energy>30.0){d.selected_action=GIVE;d.target_id=s.give_target;d.amount=min(0.5,max(0.0,a.food-1.5));}
  if(st.choice==12u && params.social_weights.w>0.5 && s.force_target<INVALID && a.energy>30.0){d.selected_action=FORCE;d.target_id=s.force_target;d.amount=1.0;}
  if(st.choice==13u && params.lifecycle.z!=0u && s.report_target<INVALID && a.energy>30.0 && params.tick>=a.last_communication+4u){d.selected_action=COMMUNICATE;d.target_id=s.report_target;d.amount=f32(s.report_place);}
  d.scores[d.selected_action]=1.0;
  return d;
}
