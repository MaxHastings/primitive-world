// primitive-world Rust/WGSL storage contract; covered by layout and replay tests.
const INVALID:u32=16384u;
const FOOD_CAPACITY:f32=8.0;
const INTERACTION_RADIUS:f32=6.0;
const GATHER_EFFORT_COST:f32=0.005;
const ANGULAR_DAMPING:f32=0.85;
const TORQUE_STRENGTH:f32=0.0375;
const TURN_EFFORT_COST:f32=0.02;
const SIGNAL_ACTIVATION_COST:f32=0.01;
const SIGNAL_AMPLITUDE_COST:f32=0.02;
const ORGANISM:u32=1u; const PACKET:u32=2u;
const NONE:u32=0u;
const TRANSFER:u32=2u; const APPLY_FORCE:u32=3u; const EMIT:u32=4u; const PRODUCE_PACKET:u32=5u; const SIGNAL_OBSERVED:u32=6u; const MEMORY_SAMPLE:u32=8u;
struct Agent {
 position:vec2<f32>, velocity:vec2<f32>, energy:f32, age:f32, max_speed:f32, sensor_radius:f32,
 food:f32, action:u32, alive:u32,
 heading:f32,rng:u32,generation:u32,packet_size:f32,
 max_age:f32,signal_payload:f32,signal_tick:u32,physical_previous:array<u32,2>,
 collected:f32,ingested:f32,
 spent:f32,received:f32,moved:vec2<f32>,
 lineage_id:u32,parent_lineage:u32,birth_tick:u32,birth_parent_slot:u32,
 ancestry_depth:u32,packets_produced:u32,distance_travelled:f32,founder_family:u32,
 active_mask:u32,plasticity_rate:array<f32,HIDDEN_COUNT>,trace_retention:f32,learned_weight_retention:f32,
 hidden:array<f32,HIDDEN_COUNT>,lived_ticks:u32,parameter_mutation_rate:f32,parameter_mutation_step:f32,topology_mutation_rate:f32,angular_velocity:f32,
};
// Every local sample has the same physical channels.  It deliberately carries
// no body id, nearest-body record, absolute bearing, or social classification.
struct Region {food:f32,bodies:f32,velocity:vec2<f32>,signal:f32,pressure:f32,};
struct Perception {resource_here:f32,nearby_count:f32,padding:vec2<f32>,regions:array<Region,16>,};
struct Decision {
 scores:array<f32,6>,selected_action:u32,evaluated:u32,movement:vec2<f32>,amount:f32,
 payload:f32,invalid:u32,decision_padding:u32,force:vec2<f32>,placement:vec2<f32>,candidate:array<f32,HIDDEN_COUNT>,hidden:array<f32,HIDDEN_COUNT>,update_gates:array<f32,HIDDEN_COUNT>,
 outputs:array<f32,OUTPUT_COUNT>,memory_write_cost:f32,inputs:array<f32,INPUT_COUNT>,
};
struct SimParams {
 world_size:vec4<f32>,resource_grid_size:u32,agent_count:u32,tick:u32,world_padding:u32,
 time_and_costs:vec4<f32>,resource_and_noise:vec4<f32>,sensor_and_padding:vec4<f32>,physical:vec4<f32>,lifecycle:vec4<u32>,mutation:vec4<f32>,environment:vec4<f32>,
};
struct Ground {dropped:atomic<u32>,extracted:atomic<u32>,remainder:f32,produced:u32,
 weather_loss:u32,collected:atomic<u32>,habitat:f32,productivity:f32,};
struct InteractionEvent {tick:u32,actor:u32,other:u32,action:u32,amount:f32,sequence:u32,actor_lineage:u32,other_lineage:u32,position:vec2<f32>,context:vec2<f32>,actual_action:u32,padding:u32,};
fn unit_vector(v:vec2<f32>)->vec2<f32>{return v/max(length(v),0.0001);}
fn body_to_world(v:vec2<f32>,heading:f32)->vec2<f32>{let c=cos(heading);let s=sin(heading);return vec2<f32>(c*v.x-s*v.y,s*v.x+c*v.y);}
fn world_to_body(v:vec2<f32>,heading:f32)->vec2<f32>{return body_to_world(v,-heading);}
fn hash_u32(input:u32)->u32{var v=input;v=(v^61u)^(v>>16u);v=v+(v<<3u);v=v^(v>>4u);v=v*0x27d4eb2du;return v^(v>>15u);}
fn random01(seed:u32)->f32{return f32(hash_u32(seed)&65535u)/65535.0;}
// The simulation space is a torus: positions always remain in the half-open
// world rectangle, while crossing an edge continues at the opposite edge.
fn wrap_world(position:vec2<f32>,world_size:vec2<f32>)->vec2<f32>{return position-world_size*floor(position/world_size);}
// Shortest displacement from `from` to `to` on the world torus. Exact
// half-world ties consistently choose the negative direction.
fn torus_delta(start:vec2<f32>,end:vec2<f32>,world_size:vec2<f32>)->vec2<f32>{let delta=end-start;return delta-world_size*floor(delta/world_size+vec2<f32>(0.5));}
fn wrap_grid_index(index:i32,size:i32)->u32{return u32(index-size*i32(floor(f32(index)/f32(size))));}
fn ground_index(position:vec2<f32>,world_size:vec2<f32>)->u32{let wrapped=wrap_world(position,world_size);let c=vec2<u32>(clamp(floor(wrapped/world_size*512.0),vec2<f32>(0),vec2<f32>(511)));return c.y*512u+c.x;}
fn finite(v:f32)->bool{return v==v && abs(v)<=3.4e38;}
fn unit_active(mask:u32,h:u32)->bool{return (mask&(1u<<h))!=0u;}
// Clockwise body-relative bearings starting forward.
// Half-open angular boundaries; coincident bodies use bearing zero.
fn sensory_sector(v:vec2<f32>)->u32 {
 if(all(v==vec2<f32>(0))){return 0u;}
 let angle=atan2(v.y,v.x)+6.283185307+0.392699082;
 return u32(floor(angle/0.785398163))%8u;
}

// Ontogeny is physiology only: no identity, social policy, or feeding counter.
fn growth_fraction(age:f32,maturity:f32)->f32{return clamp(age/max(maturity,1.0),0.0,1.0);}
fn gathering_fraction(age:f32,maturity:f32,floor:f32)->f32{
 let x=growth_fraction(age,maturity);return floor+(1.0-floor)*x*x*x*x*x*x;
}
fn reserve_capacity(age:f32,maturity:f32)->f32{return 48.0+52.0*growth_fraction(age,maturity);}
fn inventory_capacity(age:f32,maturity:f32)->f32{return 1.0+7.0*growth_fraction(age,maturity);}
