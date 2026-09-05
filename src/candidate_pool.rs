//! Bounded cross-world storage. Admission never inspects behavior or world reward.
use crate::{
    life_record::LifeRecord,
    neural_selector::random,
    reproduction_archive::{CAPACITY, Entry, Snapshot},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Source {
    /// Zero identifies the initial source world; experimental rounds start at one.
    pub round: u32,
    pub batch: u32,
    pub population: usize,
    pub seed: u32,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub source: Source,
    pub source_tick: u32,
    /// Admission is not renewed when this record is selected for founding copies.
    pub admitted_round: u32,
    pub entry: Entry,
}
impl Candidate {
    fn validate(&self) -> Result<(), String> {
        crate::brain::validate(&self.entry.genome)?;
        self.entry.life.validate()?;
        if self.entry.body.lineage_id == 0
            || self
                .entry
                .body
                .observed_tick
                .is_none_or(|t| t > self.source_tick)
            || self.entry.life.initialized != 1
            || self.entry.alive != 0
            || !matches!(self.entry.life.death_cause, 1 | 2)
            || self.entry.life.death_tick > self.source_tick
        {
            return Err("Candidate must have a completed natural lifetime and source".into());
        }
        Ok(())
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub version: u32,
    pub entries: Vec<Candidate>,
}
impl Pool {
    pub fn from_archive(archive: &Snapshot) -> Result<Self, String> {
        archive.validate()?;
        if archive.extinction_tick.is_none() {
            return Err("Round training requires an extinct source world".into());
        }
        let mut entries = archive.entries.clone();
        entries.sort_by_key(|e| {
            crate::reproduction_archive::priority(e.body.lineage_id, archive.source_seed)
        });
        let pool = Self {
            version: 1,
            entries: entries
                .into_iter()
                .map(|entry| Candidate {
                    source: Source {
                        round: 0,
                        batch: 0,
                        population: 0,
                        seed: archive.source_seed,
                    },
                    source_tick: archive.source_tick,
                    admitted_round: 1,
                    entry,
                })
                .collect(),
        };
        pool.validate()?;
        Ok(pool)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.entries.is_empty() || self.entries.len() > CAPACITY {
            return Err("Invalid bounded candidate pool".into());
        }
        let mut identities = std::collections::HashSet::new();
        for c in &self.entries {
            c.validate()?;
            if c.admitted_round == 0
                || c.source.round.checked_add(1) != Some(c.admitted_round)
                || !identities.insert((c.source, c.entry.body.lineage_id))
            {
                return Err("Duplicate or invalid candidate provenance".into());
            }
        }
        Ok(())
    }

    pub fn records(&self) -> Vec<LifeRecord> {
        self.entries.iter().map(|c| c.entry.life).collect()
    }

    pub fn genomes(&self) -> Vec<Vec<f32>> {
        self.entries
            .iter()
            .map(|c| c.entry.genome.clone())
            .collect()
    }

    /// Remove oldest records first, including all expired records. Small pools may
    /// shrink when experimental worlds contain fewer distinct sampled individuals.
    pub fn refresh(
        &mut self,
        intake: &mut Intake,
        round: u32,
        retention: u32,
    ) -> Result<(), String> {
        self.validate()?;
        intake.validate()?;
        if round < 2
            || !(1..=16).contains(&retention)
            || intake.entries.is_empty()
            || intake.limit != CAPACITY.div_ceil(retention as usize)
            || intake
                .entries
                .iter()
                .any(|c| c.candidate.admitted_round != round)
            || self.entries.iter().any(|c| c.admitted_round >= round)
        {
            return Err("Pool refresh needs a completed round and new candidates".into());
        }
        self.entries.sort_by_key(|c| c.admitted_round);
        let remove = self
            .entries
            .len()
            .div_ceil(retention as usize)
            .min(intake.limit);
        self.entries.drain(..remove);
        self.entries
            .retain(|c| round.saturating_sub(c.admitted_round) < retention);
        self.entries
            .extend(intake.entries.drain(..).map(|c| c.candidate));
        // Pool position must not encode archive age or source-world reward.
        shuffle(&mut self.entries, &mut intake.rng);
        self.validate()
    }
}

pub fn shuffle<T>(items: &mut [T], rng: &mut u64) {
    for i in (1..items.len()).rev() {
        let j = random(rng) as usize % (i + 1);
        items.swap(i, j);
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Ticket {
    candidate: Candidate,
    rank: usize,
    world_ticket: u64,
}
impl Ticket {
    fn key(&self) -> (usize, u64, Source) {
        (self.rank, self.world_ticket, self.candidate.source)
    }
}

/// Streaming equal-world allocation: first ticket from every world precedes any
/// world's second ticket. Random world order decides indivisible remainders.
/// Retains only `limit` genomes, regardless of the number or length of trials.
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intake {
    pub limit: usize,
    pub rng: u64,
    entries: Vec<Ticket>,
}
impl Intake {
    pub fn new(limit: usize, rng: u64) -> Self {
        Self {
            limit,
            rng,
            entries: vec![],
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn sources(&self) -> impl Iterator<Item = Source> + '_ {
        self.entries.iter().map(|t| t.candidate.source)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.limit == 0 || self.limit > CAPACITY || self.entries.len() > self.limit {
            return Err("Invalid candidate intake size".into());
        }
        let mut identities = std::collections::HashSet::new();
        for t in &self.entries {
            t.candidate.validate()?;
            if t.rank >= self.limit
                || t.candidate.source.round.checked_add(1) != Some(t.candidate.admitted_round)
                || !identities.insert((t.candidate.source, t.candidate.entry.body.lineage_id))
            {
                return Err("Invalid incoming candidate ticket".into());
            }
        }
        Ok(())
    }

    pub fn observe(&mut self, archive: &Snapshot, source: Source) -> Result<(), String> {
        archive.validate()?;
        if archive.extinction_tick.is_none()
            || archive.source_seed != source.seed
            || source.round == 0
        {
            return Err("Intake requires a completed experimental world".into());
        }
        let world_ticket = random(&mut self.rng);
        let mut entries: Vec<_> = archive.entries.iter().collect();
        shuffle(&mut entries, &mut self.rng);
        for (rank, entry) in entries.into_iter().take(self.limit).enumerate() {
            let candidate = Candidate {
                source,
                source_tick: archive.source_tick,
                admitted_round: source.round.checked_add(1).ok_or("Round overflow")?,
                entry: entry.clone(),
            };
            candidate.validate()?;
            self.entries.push(Ticket {
                candidate,
                rank,
                world_ticket,
            });
        }
        self.entries.sort_by_key(Ticket::key);
        self.entries.truncate(self.limit);
        self.validate()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{model::MAX_AGENTS, survivor_observer::SampledBody};

    pub fn archive(seed: u32, count: usize, duration: u32) -> Snapshot {
        Snapshot {
            policy: crate::reproduction_archive::POLICY.into(),
            source_seed: seed,
            source_tick: duration,
            started_tick: 0,
            extinction_tick: Some(duration),
            eligible_count: count as u32,
            qualified_lineages: vec![0; MAX_AGENTS as usize],
            entries: (0..count)
                .map(|i| {
                    let mut life = LifeRecord::initial(65.0, 2.0, 0, 0);
                    life.ticks = duration;
                    life.actions[0] = duration;
                    life.death_tick = duration;
                    life.death_cause = 1;
                    life.collected = i as f32;
                    let mut genome = crate::brain::blank(1).to_vec();
                    genome[crate::model::NODE_BIAS] = i as f32 / 1000.0;
                    Entry {
                        body: SampledBody {
                            slot: i,
                            lineage_id: i as u32 + 1,
                            parent_lineage: 0,
                            ancestry_depth: 0,
                            founder_family: i as u32,
                            age: duration as f32,
                            energy: 0.0,
                            food: 0.0,
                            brain_nodes: 1,
                            brain_edges: 0,
                            node_change: 0,
                            edge_change: 0,
                            observed_tick: Some(duration),
                        },
                        genome,
                        life,
                        alive: 0,
                        lifetime_births: 0,
                        distance_travelled: 0.0,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn equal_world_intake_ignores_performance_and_includes_small_failed_worlds() {
        let mut a = Intake::new(64, 19);
        let mut b = Intake::new(64, 19);
        for world in 0..3 {
            let source = Source {
                round: 1,
                batch: 1,
                population: world,
                seed: 20 + world as u32,
            };
            let count = if world == 0 { 2 } else { 256 };
            a.observe(&archive(source.seed, count, 1), source).unwrap();
            b.observe(&archive(source.seed, count, 10000), source)
                .unwrap();
            assert!(a.len() <= 64);
        }
        let ids = |intake: &Intake| {
            intake
                .entries
                .iter()
                .map(|t| (t.candidate.source, t.candidate.entry.body.lineage_id))
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(&a), ids(&b));
        let counts: Vec<_> = (0..3)
            .map(|p| a.sources().filter(|s| s.population == p).count())
            .collect();
        assert_eq!(counts, vec![2, 31, 31]);
    }

    #[test]
    fn round_refresh_is_bounded_expires_original_records_and_preserves_mapping() {
        let mut pool = Pool::from_archive(&archive(7, 256, 1)).unwrap();
        let mut intake = Intake::new(64, 9);
        for round in 2..=8 {
            intake
                .observe(
                    &archive(8, 256, round),
                    Source {
                        round: round - 1,
                        batch: 1,
                        population: 0,
                        seed: 8,
                    },
                )
                .unwrap();
            pool.refresh(&mut intake, round, 4).unwrap();
            assert_eq!(pool.entries.len(), 256);
            assert!(pool.entries.iter().all(|c| round - c.admitted_round < 4));
            if round <= 5 {
                assert_eq!(
                    pool.entries.iter().filter(|c| c.source.round == 0).count(),
                    (5 - round) as usize * 64
                );
            }
            for (life, genome) in pool.records().iter().zip(pool.genomes()) {
                assert_eq!(genome[crate::model::NODE_BIAS], life.collected / 1000.0);
            }
        }
        // Identical genomes from a later real lifetime are allowed; the old record is gone.
        assert!(pool.entries.iter().all(|c| c.source.round >= 4));
    }

    #[test]
    fn live_external_and_duplicate_records_cannot_enter_training_pool() {
        let mut a = archive(1, 2, 10);
        a.extinction_tick = None;
        assert!(Pool::from_archive(&a).is_err());
        a.extinction_tick = Some(10);
        a.entries[0].life.death_cause = 3;
        assert!(Pool::from_archive(&a).is_err());
        let mut pool = Pool::from_archive(&archive(1, 2, 10)).unwrap();
        pool.entries.push(pool.entries[0].clone());
        assert!(pool.validate().is_err());
    }
}
