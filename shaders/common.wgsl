// primitive-v3 Rust/WGSL storage contract; covered by layout and replay tests.
const INVALID:u32=16384u;
const FOOD_CAPACITY:f32=8.0;
const INTERACTION_RADIUS:f32=6.0;
const NONE:u32=0u; const COLLECT:u32=1u;
const TRANSFER:u32=2u; const APPLY_FORCE:u32=3u; const EMIT:u32=4u; const REPRODUCE:u32=5u;
struct Agent {
 position:vec2<f32>, velocity:vec2<f32>, energy:f32, age:f32, max_speed:f32, sensor_radius:f32,
 food:f32, action:u32, target_id:u32, alive:u32,
 body_padding:f32,rng:u32,generation:u32,next_birth:u32,
 max_age:f32,signal_payload:f32,signal_tick:u32,signal_padding:array<u32,3>,
 collected:f32,ingested:f32,
 spent:f32,received:f32,moved:vec2<f32>,
 lineage_id:u32,parent_lineage:u32,birth_tick:u32,birth_parent_slot:u32,
 ancestry_depth:u32,lifetime_births:u32,distance_travelled:f32,founder_family:u32,
 hidden:array<f32,HIDDEN_COUNT>,
};
struct Sample {offset:vec2<f32>,food:f32,padding:f32,};
struct Body {offset:vec2<f32>,velocity:vec2<f32>,food:f32,signal:f32,slot:u32,generation:u32,};
struct Perception {resource_here:f32,local_count:f32,padding:vec2<f32>,samples:array<Sample,8>,bodies:array<Body,4>,};
struct Decision {
 scores:array<f32,6>,selected_action:u32,score_padding:u32,movement:vec2<f32>,amount:f32,
 payload:f32,target_id:u32,target_generation:u32,invalid:u32,body_padding:u32,
 force:vec2<f32>,hidden:array<f32,HIDDEN_COUNT>,inputs:array<f32,INPUT_COUNT>,
};
struct SimParams {
 world_size:f32,resource_grid_size:u32,agent_count:u32,tick:u32,
 time_and_costs:vec4<f32>,resource_and_noise:vec4<f32>,sensor_and_padding:vec4<f32>,physical:vec4<f32>,lifecycle:vec4<u32>,
};
struct Ground {dropped:atomic<u32>,extracted:atomic<u32>,remainder:f32,produced:u32,
 weather_loss:u32,collected:atomic<u32>,habitat:f32,productivity:f32,};
fn unit_vector(v:vec2<f32>)->vec2<f32>{return v/max(length(v),0.0001);}
fn hash_u32(input:u32)->u32{var v=input;v=(v^61u)^(v>>16u);v=v+(v<<3u);v=v^(v>>4u);v=v*0x27d4eb2du;return v^(v>>15u);}
fn random01(seed:u32)->f32{return f32(hash_u32(seed)&65535u)/65535.0;}
fn ground_index(position:vec2<f32>)->u32{let c=vec2<u32>(clamp(position/4.0,vec2<f32>(0),vec2<f32>(511)));return c.y*512u+c.x;}
fn finite(v:f32)->bool{return v==v && abs(v)<=3.4e38;}
