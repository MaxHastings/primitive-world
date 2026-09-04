//! Optional CPU-only, sampled journey evidence. Never supplied to controllers.
//! Thresholds classify observations; they do not cause travel or reward genes.
use crate::model::{AgentGpu, RESOURCE_GRID, WORLD_SIZE};
use std::collections::HashMap;

#[derive(Clone, Debug, serde::Serialize)]
pub struct Point {
    pub tick: u32,
    pub position: [f32; 2],
    pub collection_position: [f32; 2],
    pub local_vegetation: f32,
    pub collected_last_tick: f32,
    pub ingested_last_tick: f32,
    pub energy: f32,
    pub lifetime_births: u32,
}

#[derive(Debug, serde::Serialize)]
pub struct Journey {
    pub lineage_id: u32,
    pub generation: u32,
    pub birth_tick: u32,
    pub source: Point,
    pub source_peak_vegetation: f32,
    pub source_vegetation_at_departure: f32,
    pub departure_tick: u32,
    pub poor_corridor_start: Point,
    pub poor_corridor_end: Point,
    pub destination_collection: Point,
    pub destination_ingestion: Point,
    pub reproduction_interval: [u32; 2],
    pub reproduction_count: u32,
    pub waypoints: Vec<Point>,
}

struct Track {
    birth_tick: u32,
    source: Point,
    peak: f32,
    departure: Option<(u32, f32)>,
    poor_start: Option<Point>,
    corridor: Option<(Point, Point)>,
    collection: Option<Point>,
    ingestion: Option<Point>,
    points: Vec<Point>,
}

#[derive(Default, serde::Serialize)]
pub struct Stats {
    pub samples: u64,
    pub source_anchors: u64,
    pub depleted_departures: u64,
    pub poor_corridors: u64,
    pub destination_collections: u64,
    pub destination_ingestions: u64,
    pub maximum_poor_sample_net_distance: f32,
    pub completed_journeys: u64,
    pub invalid_observations: u64,
    pub truncated_tracks: u64,
    pub lost_tracks: u64,
}

#[derive(Default)]
pub struct JourneyObserver {
    tracks: HashMap<(u32, u32), Track>,
    last_tick: Option<u32>,
    pub stats: Stats,
}

fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

/// Fixed observer footprint: center plus eight points on a radius-24 ring.
/// Vegetation only: dropped food is intentionally not counted as a food patch.
fn vegetation(resources: &[u32], position: [f32; 2]) -> f32 {
    let mut total = 0.0;
    for offset in [
        [0.0, 0.0],
        [24.0, 0.0],
        [-24.0, 0.0],
        [0.0, 24.0],
        [0.0, -24.0],
        [16.970562, 16.970562],
        [-16.970562, 16.970562],
        [16.970562, -16.970562],
        [-16.970562, -16.970562],
    ] {
        let cell = |axis: usize| {
            ((position[axis] + offset[axis]).clamp(0.0, WORLD_SIZE) / WORLD_SIZE
                * RESOURCE_GRID as f32)
                .min((RESOURCE_GRID - 1) as f32) as usize
        };
        total += resources[cell(1) * RESOURCE_GRID as usize + cell(0)] as f32 / 1000.0;
    }
    total / 9.0
}

impl JourneyObserver {
    pub fn observe(
        &mut self,
        tick: u32,
        agents: &[AgentGpu],
        resources: &[u32],
    ) -> Result<Vec<Journey>, String> {
        if resources.len() != (RESOURCE_GRID * RESOURCE_GRID) as usize {
            return Err("Journey observer needs the complete vegetation grid".into());
        }
        if self.last_tick.is_some_and(|last| tick <= last) {
            return Err("Journey sample ticks must strictly increase".into());
        }
        let previous_tick = self.last_tick.unwrap_or(tick);
        let mut current = HashMap::new();
        let mut completed = Vec::new();
        for a in agents.iter().filter(|a| a.alive != 0) {
            let key = (a.lineage_id, a.generation);
            if !a
                .position
                .iter()
                .chain(&a.moved)
                .chain(&a.velocity)
                .chain([&a.energy, &a.collected, &a.ingested])
                .all(|v| v.is_finite())
                || a.collected < 0.0
                || a.ingested < 0.0
            {
                self.stats.invalid_observations += 1;
                continue;
            }
            let p = Point {
                tick,
                position: a.position,
                collection_position: [a.position[0] - a.moved[0], a.position[1] - a.moved[1]],
                local_vegetation: vegetation(resources, a.position),
                collected_last_tick: a.collected,
                ingested_last_tick: a.ingested,
                energy: a.energy,
                lifetime_births: a.lifetime_births,
            };
            let mut track = self
                .tracks
                .remove(&key)
                .filter(|t| t.birth_tick == a.birth_tick);
            let collecting_in_patch =
                a.collected > 0.0 && vegetation(resources, p.collection_position) >= 0.04;
            if let Some(t) = &mut track
                && (t.points.len() >= 512
                    || a.lifetime_births < t.points.last().unwrap().lifetime_births)
            {
                self.stats.truncated_tracks += 1;
                track = None;
            }
            if let Some(t) = &mut track {
                t.points.push(p.clone());
                let from_source = distance(a.position, t.source.collection_position);
                let source_now = vegetation(resources, t.source.collection_position);
                if from_source <= 24.0 {
                    t.peak = t.peak.max(source_now);
                }
                if t.departure.is_none()
                    && from_source >= 48.0
                    && source_now <= (t.peak * 0.25).min(0.02)
                {
                    t.departure = Some((tick, source_now));
                    self.stats.depleted_departures += 1;
                }
                let voluntary = distance(a.moved, a.velocity) <= 0.001;
                if t.departure.is_some() && t.corridor.is_none() {
                    if p.local_vegetation <= 0.01
                        && a.collected == 0.0
                        && from_source >= 48.0
                        && voluntary
                    {
                        let start = t.poor_start.get_or_insert_with(|| p.clone());
                        let net = distance(start.position, p.position);
                        self.stats.maximum_poor_sample_net_distance =
                            self.stats.maximum_poor_sample_net_distance.max(net);
                        if net >= 48.0 {
                            t.corridor = Some((start.clone(), p.clone()));
                            self.stats.poor_corridors += 1;
                        }
                    } else {
                        t.poor_start = None;
                    }
                }
                if t.corridor.is_some()
                    && t.collection.is_none()
                    && collecting_in_patch
                    && distance(p.collection_position, t.source.collection_position) >= 96.0
                {
                    t.collection = Some(p.clone());
                    self.stats.destination_collections += 1;
                }
                if let Some(c) = &t.collection {
                    if distance(p.position, c.collection_position) > 48.0 {
                        t.collection = None;
                        t.ingestion = None;
                    } else if tick > c.tick && a.ingested > 0.0 && t.ingestion.is_none() {
                        t.ingestion = Some(p.clone());
                        self.stats.destination_ingestions += 1;
                    }
                }
                if let Some(ingestion) = &t.ingestion
                    && tick > ingestion.tick
                    && a.lifetime_births > ingestion.lifetime_births
                {
                    let (departure_tick, source_vegetation_at_departure) = t.departure.unwrap();
                    let (poor_corridor_start, poor_corridor_end) = t.corridor.clone().unwrap();
                    completed.push(Journey {
                        lineage_id: a.lineage_id,
                        generation: a.generation,
                        birth_tick: a.birth_tick,
                        source: t.source.clone(),
                        source_peak_vegetation: t.peak,
                        source_vegetation_at_departure,
                        departure_tick,
                        poor_corridor_start,
                        poor_corridor_end,
                        destination_collection: t.collection.clone().unwrap(),
                        destination_ingestion: ingestion.clone(),
                        reproduction_interval: [previous_tick, tick],
                        reproduction_count: a.lifetime_births
                            - t.points[t.points.len() - 2].lifetime_births,
                        waypoints: std::mem::take(&mut t.points),
                    });
                    track = None;
                } else if t.departure.is_none() && from_source > 24.0 && collecting_in_patch {
                    // Still foraging continuously: move the source anchor, rather
                    // than claiming travel inside a single broad food zone.
                    track = None;
                }
            }
            if track.is_none() && collecting_in_patch {
                self.stats.source_anchors += 1;
                track = Some(Track {
                    birth_tick: a.birth_tick,
                    source: p.clone(),
                    peak: vegetation(resources, p.collection_position),
                    departure: None,
                    poor_start: None,
                    corridor: None,
                    collection: None,
                    ingestion: None,
                    points: vec![p],
                });
            }
            if let Some(t) = track {
                current.insert(key, t);
            }
        }
        self.stats.lost_tracks += self.tracks.len() as u64;
        self.tracks = current;
        self.last_tick = Some(tick);
        self.stats.samples += 1;
        self.stats.completed_journeys += completed.len() as u64;
        Ok(completed)
    }

    pub fn report(&self, sample: u32) -> serde_json::Value {
        serde_json::json!({"schema": 1, "sample_ticks": sample, "stats": self.stats,
            "definition": "Sampled source collection in a vegetation footprint >=0.04; source falls to <=25% of its observed peak and <=0.02; departure >=48 units; consecutive poor-footprint samples <=0.01 with no observed collection or force displacement cross >=48 net units; collection >=96 units from source in footprint >=0.04; later ingestion and an actual birth-counter increase while sampled positions remain within48 of destination.",
            "limits": ["Only the last tick's feeding is visible at each sample. Unsampled feeding, death and route details are missed. Poor-space continuity means consecutive samples, not every intervening tick.",
                "Patch means a radius24 nine-point vegetation footprint, not a global connected-component identity. Dropped food is excluded from patch classification but can contribute to actual collection.",
                "Records demonstrate a sampled sequence, not foresight, causation of survival, successful offspring survival, or event attribution to major geography renewal. Final-goal relocation attribution needs additional verification.",
                "Tracks reset on missing/dead identities, changed birth tick, decreasing birth counters, completion, or512 points. Reproduction is located to an interval, not an invented exact tick."]})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn field(patches: &[[f32; 2]]) -> Vec<u32> {
        (0..RESOURCE_GRID * RESOURCE_GRID)
            .map(|i| {
                let p = [
                    ((i % RESOURCE_GRID) as f32 + 0.5) * 4.0,
                    ((i / RESOURCE_GRID) as f32 + 0.5) * 4.0,
                ];
                if patches.iter().any(|&q| distance(p, q) < 32.0) {
                    200
                } else {
                    0
                }
            })
            .collect()
    }
    fn body(x: f32, collected: f32, ingested: f32, births: u32) -> AgentGpu {
        AgentGpu {
            alive: 1,
            lineage_id: 1,
            position: [x, 502.0],
            energy: 60.0,
            collected,
            ingested,
            lifetime_births: births,
            ..Default::default()
        }
    }
    #[test]
    fn requires_depletion_corridor_collection_ingestion_then_birth() {
        let mut o = JourneyObserver::default();
        let origin = field(&[[502.0, 502.0]]);
        let bare = field(&[]);
        let destination = field(&[[702.0, 502.0]]);
        assert!(
            o.observe(0, &[body(502.0, 0.02, 0.0, 0)], &origin)
                .unwrap()
                .is_empty()
        );
        assert!(
            o.observe(32, &[body(550.0, 0.0, 0.0, 0)], &bare)
                .unwrap()
                .is_empty()
        );
        assert!(
            o.observe(64, &[body(598.0, 0.0, 0.0, 0)], &bare)
                .unwrap()
                .is_empty()
        );
        assert!(
            o.observe(96, &[body(702.0, 0.02, 0.0, 1)], &destination)
                .unwrap()
                .is_empty()
        );
        assert!(
            o.observe(128, &[body(702.0, 0.0, 0.08, 1)], &destination)
                .unwrap()
                .is_empty()
        );
        let events = o
            .observe(160, &[body(702.0, 0.0, 0.0, 2)], &destination)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].reproduction_interval, [128, 160]);
        assert_eq!(events[0].reproduction_count, 1);
        assert_eq!(events[0].waypoints.len(), 6);
    }
    #[test]
    fn rich_source_or_identity_gap_cannot_claim_depleted_departure() {
        for gap in [false, true] {
            let mut o = JourneyObserver::default();
            let both = field(&[[502.0, 502.0], [702.0, 502.0]]);
            o.observe(0, &[body(502.0, 0.02, 0.0, 0)], &both).unwrap();
            if gap {
                o.observe(16, &[], &both).unwrap();
            }
            for (tick, a) in [
                (32, body(550.0, 0.0, 0.0, 0)),
                (64, body(598.0, 0.0, 0.0, 0)),
                (96, body(702.0, 0.02, 0.0, 0)),
                (128, body(702.0, 0.0, 0.08, 0)),
                (160, body(702.0, 0.0, 0.0, 1)),
            ] {
                assert!(o.observe(tick, &[a], &both).unwrap().is_empty());
            }
        }
    }
    #[test]
    fn incomplete_or_discontinuous_sequences_do_not_count() {
        for missing in [
            "corridor",
            "collection",
            "ingestion",
            "birth",
            "force",
            "incarnation",
        ] {
            let mut o = JourneyObserver::default();
            let origin = field(&[[502.0, 502.0]]);
            let bare = field(&[]);
            let destination = field(&[[702.0, 502.0]]);
            o.observe(0, &[body(502.0, 0.02, 0.0, 0)], &origin).unwrap();
            o.observe(32, &[body(550.0, 0.0, 0.0, 0)], &bare).unwrap();
            let mut crossing = body(
                if missing == "corridor" { 570.0 } else { 598.0 },
                0.0,
                0.0,
                0,
            );
            if missing == "force" {
                crossing.moved = [3.0, 0.0];
            }
            if missing == "incarnation" {
                crossing.birth_tick = 33;
            }
            assert!(o.observe(64, &[crossing], &bare).unwrap().is_empty());
            assert!(
                o.observe(
                    96,
                    &[body(
                        702.0,
                        if missing == "collection" { 0.0 } else { 0.02 },
                        0.0,
                        0
                    )],
                    &destination
                )
                .unwrap()
                .is_empty()
            );
            assert!(
                o.observe(
                    128,
                    &[body(
                        702.0,
                        0.0,
                        if missing == "ingestion" { 0.0 } else { 0.08 },
                        0
                    )],
                    &destination
                )
                .unwrap()
                .is_empty()
            );
            assert!(
                o.observe(
                    160,
                    &[body(702.0, 0.0, 0.0, u32::from(missing != "birth"))],
                    &destination
                )
                .unwrap()
                .is_empty(),
                "{missing}"
            );
        }
    }

    #[test]
    fn timestamp_and_grid_errors_do_not_advance_observer() {
        let mut o = JourneyObserver::default();
        let bare = field(&[]);
        o.observe(10, &[], &bare).unwrap();
        assert!(o.observe(10, &[], &bare).is_err());
        assert!(o.observe(11, &[], &[]).is_err());
        assert_eq!(o.last_tick, Some(10));
    }
}
