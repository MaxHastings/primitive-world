//! External transfer selection. GPU bookkeeping never changes a living organism.
use crate::{
    founders::FounderBank,
    model::*,
    simulation::{Compute, Simulation, observability::read_buffer},
    survivor_observer::{SampledBody, SurvivorSample},
};

pub const POLICY: &str = "life-reservoir-v3";
pub const CAPACITY: usize = 256;
const STATE_WORDS: usize = 4 + CAPACITY * 5;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub body: SampledBody,
    pub genome: Vec<f32>,
    pub life: crate::life_record::LifeRecord,
    pub alive: u32,
    pub lifetime_births: u32,
    pub distance_travelled: f32,
}
#[derive(Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub policy: String,
    pub source_seed: u32,
    pub source_tick: u32,
    pub started_tick: u32,
    pub extinction_tick: Option<u32>,
    pub eligible_count: u32,
    pub qualified_lineages: Vec<u32>,
    pub entries: Vec<Entry>,
}
pub fn priority(lineage: u32, seed: u32) -> u32 {
    let mut v = lineage ^ seed;
    v = (v ^ 61) ^ (v >> 16);
    v = v.wrapping_add(v << 3);
    v ^= v >> 4;
    v = v.wrapping_mul(0x27d4eb2d);
    v ^ (v >> 15)
}
impl Snapshot {
    pub fn empty(sim: &Simulation) -> Self {
        Self {
            policy: POLICY.into(),
            source_seed: sim.seed,
            source_tick: sim.tick,
            started_tick: sim.tick,
            extinction_tick: None,
            eligible_count: 0,
            qualified_lineages: vec![0; MAX_AGENTS as usize],
            entries: vec![],
        }
    }
    /// Capture every currently living individual before the first simulation tick.
    /// Attach at initialization for complete coverage; attaching mid-world is explicit.
    pub fn initial(sim: &Simulation, d: &wgpu::Device, q: &wgpu::Queue) -> Result<Self, String> {
        let mut saved = Self::empty(sim);
        let bodies = sim.agent_snapshot(d, q)?;
        let bytes = read_buffer(d, q, &sim.genome_buffer)?;
        let genes: &[f32] = bytemuck::cast_slice(&bytes);
        let mut candidates: Vec<_> = bodies
            .iter()
            .enumerate()
            .filter(|(_, a)| a.alive != 0)
            .collect();
        saved.eligible_count = candidates.len() as u32;
        for &(slot, a) in &candidates {
            saved.qualified_lineages[slot] = a.lineage_id;
        }
        candidates.sort_by_key(|(_, a)| priority(a.lineage_id, sim.seed));
        for (slot, a) in candidates.into_iter().take(CAPACITY) {
            saved.entries.push(Entry {
                body: SampledBody {
                    slot,
                    lineage_id: a.lineage_id,
                    parent_lineage: a.parent_lineage,
                    ancestry_depth: a.ancestry_depth,
                    founder_family: a.founder_family,
                    age: a.age,
                    energy: a.energy,
                    food: a.food,
                    brain_nodes: a.brain_nodes,
                    brain_edges: a.brain_edges,
                    node_change: a.node_change,
                    edge_change: a.edge_change,
                    observed_tick: Some(sim.tick),
                },
                genome: genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE].to_vec(),
                life: if a.life.initialized == 0 {
                    crate::life_record::LifeRecord::initial(
                        a.energy,
                        a.food,
                        a.birth_tick,
                        sim.tick,
                    )
                } else {
                    a.life
                },
                alive: a.alive,
                lifetime_births: a.lifetime_births,
                distance_travelled: a.distance_travelled,
            });
        }
        saved.validate()?;
        Ok(saved)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.policy != POLICY
            || self.started_tick > self.source_tick
            || self.qualified_lineages.len() != MAX_AGENTS as usize
            || self
                .extinction_tick
                .is_some_and(|t| t < self.started_tick || t > self.source_tick)
            || self.entries.len() != (self.eligible_count as usize).min(CAPACITY)
        {
            return Err("Invalid neural selection archive header".into());
        }
        let mut ids = std::collections::HashSet::new();
        for e in &self.entries {
            crate::brain::validate(&e.genome)?;
            e.life.validate()?;
            let a = &e.body;
            if e.alive > 1
                || a.lineage_id == 0
                || !ids.insert(a.lineage_id)
                || a.slot >= MAX_AGENTS as usize
                || a.observed_tick
                    .is_none_or(|t| t < self.started_tick || t > self.source_tick)
                || !a.age.is_finite()
                || a.age < 0.0
                || !a.energy.is_finite()
                || !a.food.is_finite()
                || a.food < 0.0
                || !e.distance_travelled.is_finite()
                || e.distance_travelled < 0.0
                || a.brain_nodes != e.genome[0] as u32
                || a.brain_edges != e.genome[1] as u32
            {
                return Err("Invalid neural selection archive entry".into());
            }
        }
        Ok(())
    }
    pub fn records(&self) -> Vec<crate::life_record::LifeRecord> {
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by_key(|e| priority(e.body.lineage_id, self.source_seed));
        entries.into_iter().map(|e| e.life).collect()
    }
    pub fn sample(&self) -> Option<SurvivorSample> {
        if self.entries.is_empty() {
            return None;
        }
        let mut entries: Vec<_> = self.entries.iter().collect();
        entries.sort_by_key(|e| priority(e.body.lineage_id, self.source_seed));
        Some(SurvivorSample {
            bank:FounderBank {version:FOUNDER_BANK_VERSION,model:MODEL_ID.into(),name:format!("candidate-reservoir-seed{}-tick{}",self.source_seed,self.source_tick),source_seed:self.source_seed,source_tick:self.source_tick,genomes:entries.iter().map(|e|e.genome.clone()).collect()},
            source_population:self.eligible_count as usize,bodies:entries.iter().map(|e|e.body.clone()).collect(),
            selection:"Uniform identity-hash reservoir across observed individuals, including dead founders and descendants; this is the candidate pool, not the neural selection result.".into()
        })
    }
}

pub struct Archive {
    qualify: Compute,
    select: Compute,
    capture: Compute,
    qualified: wgpu::Buffer,
    candidates: wgpu::Buffer,
    state: wgpu::Buffer,
    bodies: wgpu::Buffer,
    genomes: wgpu::Buffer,
    started_tick: u32,
}
fn buffer(d: &wgpu::Device, name: &str, size: u64) -> wgpu::Buffer {
    d.create_buffer(&wgpu::BufferDescriptor {
        label: Some(name),
        size,
        usage: wgpu::BufferUsages::STORAGE
            | wgpu::BufferUsages::COPY_SRC
            | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
impl Archive {
    pub fn new(
        d: &wgpu::Device,
        q: &wgpu::Queue,
        sim: &Simulation,
        saved: &Snapshot,
    ) -> Result<Self, String> {
        saved.validate()?;
        if saved.source_seed != sim.seed || saved.source_tick > sim.tick {
            return Err("Candidate archive belongs to another world or a future tick".into());
        }
        let qualified = buffer(
            d,
            "registered candidate identities",
            u64::from(MAX_AGENTS) * 4,
        );
        let candidates = buffer(
            d,
            "new individual candidates",
            u64::from(MAX_AGENTS + 4) * 4,
        );
        let state = buffer(d, "candidate reservoir", STATE_WORDS as u64 * 4);
        let bodies = buffer(
            d,
            "archived candidate bodies",
            (CAPACITY * std::mem::size_of::<AgentGpu>()) as u64,
        );
        let genomes = buffer(
            d,
            "archived candidate genomes",
            (CAPACITY * GENOME_SIZE * 4) as u64,
        );
        let qualify = Compute::new(
            d,
            "qualify candidate",
            include_str!("../shaders/archive_qualify.wgsl"),
            "main",
            "rwwur",
            (0..2)
                .map(|i| {
                    vec![
                        &sim.agent_buffers[i],
                        &qualified,
                        &candidates,
                        &sim.params_buffer,
                        &sim.agent_buffers[1 - i],
                    ]
                })
                .collect(),
        );
        let select = Compute::new(
            d,
            "select candidate reservoir",
            include_str!("../shaders/archive_select.wgsl"),
            "main",
            "rrwu",
            (0..2)
                .map(|i| {
                    vec![
                        &candidates,
                        &sim.agent_buffers[i],
                        &state,
                        &sim.params_buffer,
                    ]
                })
                .collect(),
        );
        let capture = Compute::new(
            d,
            "capture candidate genomes",
            include_str!("../shaders/archive_capture.wgsl"),
            "main",
            "rrwwwu",
            (0..2)
                .map(|i| {
                    vec![
                        &sim.agent_buffers[i],
                        &sim.genome_buffer,
                        &state,
                        &bodies,
                        &genomes,
                        &sim.params_buffer,
                    ]
                })
                .collect(),
        );
        let mut words = [0u32; STATE_WORDS];
        words[0] = saved.entries.len() as u32;
        words[1] = saved.eligible_count;
        words[2] = saved.extinction_tick.unwrap_or(0);
        for (i, entry) in saved.entries.iter().enumerate() {
            let a = &entry.body;
            words[4 + i] = priority(a.lineage_id, sim.seed);
            words[4 + CAPACITY + i] = MAX_AGENTS;
            words[4 + CAPACITY * 2 + i] = a.observed_tick.unwrap();
            words[4 + CAPACITY * 3 + i] = a.lineage_id;
            words[4 + CAPACITY * 4 + i] = a.slot as u32;
            let body = AgentGpu {
                life: entry.life,
                alive: entry.alive,
                lifetime_births: entry.lifetime_births,
                distance_travelled: entry.distance_travelled,
                lineage_id: a.lineage_id,
                parent_lineage: a.parent_lineage,
                ancestry_depth: a.ancestry_depth,
                founder_family: a.founder_family,
                age: a.age,
                energy: a.energy,
                food: a.food,
                brain_nodes: a.brain_nodes,
                brain_edges: a.brain_edges,
                node_change: a.node_change,
                edge_change: a.edge_change,
                ..Default::default()
            };
            q.write_buffer(
                &bodies,
                (i * std::mem::size_of::<AgentGpu>()) as u64,
                bytemuck::bytes_of(&body),
            );
            q.write_buffer(
                &genomes,
                (i * GENOME_SIZE * 4) as u64,
                bytemuck::cast_slice(&entry.genome),
            );
        }
        q.write_buffer(&state, 0, bytemuck::cast_slice(&words));
        q.write_buffer(
            &qualified,
            0,
            bytemuck::cast_slice(&saved.qualified_lineages),
        );
        Ok(Self {
            qualify,
            select,
            capture,
            qualified,
            candidates,
            state,
            bodies,
            genomes,
            started_tick: saved.started_tick,
        })
    }
    pub fn encode(&self, e: &mut wgpu::CommandEncoder, group: usize) {
        e.clear_buffer(&self.candidates, 0, Some(8));
        self.qualify.dispatch(e, group, MAX_AGENTS.div_ceil(64), 1);
        self.select.dispatch(e, group, 1, 1);
        self.capture.dispatch(e, group, CAPACITY as u32, 1);
    }
    pub fn snapshot(
        &self,
        sim: &Simulation,
        d: &wgpu::Device,
        q: &wgpu::Queue,
    ) -> Result<Snapshot, String> {
        let sources = [&self.state, &self.qualified, &self.bodies, &self.genomes];
        let packed = buffer(
            d,
            "candidate archive readback",
            sources.iter().map(|b| b.size()).sum(),
        );
        let mut encoder = d.create_command_encoder(&Default::default());
        let mut offset = 0;
        for source in sources {
            encoder.copy_buffer_to_buffer(source, 0, &packed, offset, source.size());
            offset += source.size();
        }
        q.submit(Some(encoder.finish()));
        let bytes = read_buffer(d, q, &packed)?;
        let state: &[u32] = bytemuck::cast_slice(&bytes[..STATE_WORDS * 4]);
        let qualified_end = STATE_WORDS * 4 + MAX_AGENTS as usize * 4;
        let qualified_lineages =
            bytemuck::cast_slice::<u8, u32>(&bytes[STATE_WORDS * 4..qualified_end]).to_vec();
        let bodies_end = qualified_end + CAPACITY * std::mem::size_of::<AgentGpu>();
        let bodies: &[AgentGpu] = bytemuck::cast_slice(&bytes[qualified_end..bodies_end]);
        let genes: &[f32] = bytemuck::cast_slice(&bytes[bodies_end..]);
        let entries = (0..(state[0] as usize).min(CAPACITY))
            .map(|i| {
                let a = bodies[i];
                Entry {
                    body: SampledBody {
                        slot: state[4 + CAPACITY * 4 + i] as usize,
                        lineage_id: a.lineage_id,
                        parent_lineage: a.parent_lineage,
                        ancestry_depth: a.ancestry_depth,
                        founder_family: a.founder_family,
                        age: a.age,
                        energy: a.energy,
                        food: a.food,
                        brain_nodes: a.brain_nodes,
                        brain_edges: a.brain_edges,
                        node_change: a.node_change,
                        edge_change: a.edge_change,
                        observed_tick: Some(state[4 + CAPACITY * 2 + i]),
                    },
                    genome: genes[i * GENOME_SIZE..(i + 1) * GENOME_SIZE].to_vec(),
                    life: if a.life.initialized == 0 {
                        crate::life_record::LifeRecord::initial(
                            a.energy,
                            a.food,
                            a.birth_tick,
                            sim.tick,
                        )
                    } else {
                        a.life
                    },
                    alive: a.alive,
                    lifetime_births: a.lifetime_births,
                    distance_travelled: a.distance_travelled,
                }
            })
            .collect();
        let result = Snapshot {
            policy: POLICY.into(),
            source_seed: sim.seed,
            source_tick: sim.tick,
            started_tick: self.started_tick,
            extinction_tick: (state[2] != 0).then_some(state[2]),
            eligible_count: state[1],
            qualified_lineages,
            entries,
        };
        result.validate()?;
        Ok(result)
    }
}
