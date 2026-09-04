//! Sampled, read-only headless diagnostics. Only the previous live snapshot is
//! retained; no result is fed back into simulation state or controller inputs.
use crate::simulation::AgentGpu;
use std::collections::HashMap;

#[derive(Debug, Default, serde::Serialize)]
pub struct TravelStats {
    pub samples: u64,
    pub sample_intervals: u64,
    pub first_tick: Option<u32>,
    pub last_tick: Option<u32>,
    pub min_interval_ticks: Option<u32>,
    pub max_interval_ticks: Option<u32>,
    /// Each matched agent contributes once per consecutive sample interval.
    pub tracked_agent_intervals: u64,
    pub tracked_agent_ticks: u64,
    /// Sum of the existing distance_travelled counter deltas, in world units.
    pub path_distance: f64,
    /// Sum of endpoint Euclidean distances, not run-wide displacement.
    pub net_displacement: f64,
    pub intervals_net_at_least_sensor_radius: u64,
    /// Lifetime-birth counter deltas for the same matched agents only.
    pub reproduction_delta: u64,
    pub invalid_observations: u64,
    pub reset_counter_intervals: u64,
}

#[derive(Clone, Copy)]
struct Observation {
    position: [f32; 2],
    distance_travelled: f32,
    lifetime_births: u32,
    sensor_radius: f32,
    birth_tick: u32,
}

#[derive(Default)]
pub struct TravelObserver {
    previous: HashMap<(u32, u32), Observation>,
    pub stats: TravelStats,
}

impl TravelObserver {
    /// Supply the complete current agent buffer at each report sample, including
    /// the initial baseline and final partial interval. Missing/dead identities
    /// and new incarnations establish fresh baselines. Non-increasing timestamps
    /// are rejected without changing the observer.
    pub fn observe(&mut self, tick: u32, agents: &[AgentGpu]) -> Result<(), String> {
        let elapsed = match self.stats.last_tick {
            Some(previous_tick) => Some(
                tick.checked_sub(previous_tick)
                    .filter(|elapsed| *elapsed > 0)
                    .ok_or("Travel sample ticks must strictly increase")?,
            ),
            None => None,
        };
        let mut current = HashMap::new();
        for agent in agents.iter().filter(|agent| agent.alive != 0) {
            if !agent.position.iter().all(|value| value.is_finite())
                || !agent.distance_travelled.is_finite()
                || agent.distance_travelled < 0.0
                || !agent.sensor_radius.is_finite()
                || agent.sensor_radius <= 0.0
            {
                self.stats.invalid_observations += 1;
                continue;
            }
            let identity = (agent.lineage_id, agent.generation);
            if let (Some(elapsed), Some(previous)) = (elapsed, self.previous.get(&identity))
                && previous.birth_tick == agent.birth_tick
            {
                if agent.distance_travelled >= previous.distance_travelled
                    && agent.lifetime_births >= previous.lifetime_births
                {
                    let dx = agent.position[0] as f64 - previous.position[0] as f64;
                    let dy = agent.position[1] as f64 - previous.position[1] as f64;
                    let net = dx.hypot(dy);
                    self.stats.tracked_agent_intervals += 1;
                    self.stats.tracked_agent_ticks += elapsed as u64;
                    self.stats.path_distance +=
                        agent.distance_travelled as f64 - previous.distance_travelled as f64;
                    self.stats.net_displacement += net;
                    self.stats.intervals_net_at_least_sensor_radius +=
                        u64::from(net >= previous.sensor_radius as f64);
                    self.stats.reproduction_delta +=
                        (agent.lifetime_births - previous.lifetime_births) as u64;
                } else {
                    self.stats.reset_counter_intervals += 1;
                }
            }
            current.insert(
                identity,
                Observation {
                    position: agent.position,
                    distance_travelled: agent.distance_travelled,
                    lifetime_births: agent.lifetime_births,
                    sensor_radius: agent.sensor_radius,
                    birth_tick: agent.birth_tick,
                },
            );
        }
        self.previous = current;
        self.stats.samples += 1;
        self.stats.first_tick.get_or_insert(tick);
        self.stats.last_tick = Some(tick);
        if let Some(elapsed) = elapsed {
            self.stats.sample_intervals += 1;
            self.stats.min_interval_ticks = Some(
                self.stats
                    .min_interval_ticks
                    .map_or(elapsed, |n| n.min(elapsed)),
            );
            self.stats.max_interval_ticks = Some(
                self.stats
                    .max_interval_ticks
                    .map_or(elapsed, |n| n.max(elapsed)),
            );
        }
        Ok(())
    }

    pub fn report(&self, sample_ticks: u32) -> serde_json::Value {
        serde_json::json!({
            "stats": self.stats,
            "requested_sample_ticks": sample_ticks,
            "scope": "Consecutive live observations matched by lineage_id and generation, with birth_tick guard. Initial baseline and final partial interval included. Distances are world units; the threshold uses the starting sensory radius. Aggregate observations only; no behavior score or simulation feedback.",
            "limits": [
                "Agents dead before the next sample, including births and deaths between samples, are missed. Reproduction deltas cover matched survivors only, not total population births.",
                "Endpoint displacement misses intervening excursions and round trips. The path counter can record such movement, but samples cannot reconstruct routes or timing.",
                "distance_travelled records the existing movement counter; force pushes affect positions but are not included in that counter. Floating-point counter precision also limits small deltas.",
                "Missing identities, new incarnations, invalid observations and decreasing counters break interval matching. Only the previous sample is retained."
            ]
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(lineage_id: u32, generation: u32, x: f32, path: f32, births: u32) -> AgentGpu {
        AgentGpu {
            alive: 1,
            lineage_id,
            generation,
            position: [x, 0.0],
            distance_travelled: path,
            lifetime_births: births,
            sensor_radius: 5.0,
            ..Default::default()
        }
    }

    #[test]
    fn never_mixes_incarnations_or_lineages() {
        let mut observer = TravelObserver::default();
        observer.observe(0, &[agent(1, 1, 0.0, 0.0, 0)]).unwrap();
        observer.observe(10, &[agent(1, 2, 20.0, 20.0, 4)]).unwrap();
        observer.observe(20, &[agent(2, 2, 40.0, 40.0, 8)]).unwrap();
        let mut newborn = agent(2, 2, 50.0, 50.0, 10);
        newborn.birth_tick = 21;
        observer.observe(30, &[newborn]).unwrap();
        assert_eq!(observer.stats.tracked_agent_intervals, 0);
        assert_eq!(observer.stats.path_distance, 0.0);
        assert_eq!(observer.stats.reproduction_delta, 0);
        newborn.position[0] += 5.0;
        newborn.distance_travelled += 5.0;
        newborn.lifetime_births += 1;
        observer.observe(40, &[newborn]).unwrap();
        assert_eq!(observer.stats.tracked_agent_intervals, 1);
        assert_eq!(observer.stats.reproduction_delta, 1);
    }

    #[test]
    fn circling_adds_path_without_endpoint_displacement() {
        let mut observer = TravelObserver::default();
        observer.observe(0, &[agent(1, 0, 2.0, 10.0, 3)]).unwrap();
        observer.observe(100, &[agent(1, 0, 2.0, 50.0, 5)]).unwrap();
        assert_eq!(observer.stats.path_distance, 40.0);
        assert_eq!(observer.stats.net_displacement, 0.0);
        assert_eq!(observer.stats.intervals_net_at_least_sensor_radius, 0);
        assert_eq!(observer.stats.reproduction_delta, 2);
    }

    #[test]
    fn absent_or_dead_sample_resets_matching() {
        for gap in [vec![], vec![AgentGpu::default()]] {
            let mut observer = TravelObserver::default();
            observer.observe(0, &[agent(1, 0, 0.0, 0.0, 0)]).unwrap();
            observer.observe(10, &gap).unwrap();
            observer.observe(20, &[agent(1, 0, 10.0, 10.0, 3)]).unwrap();
            observer.observe(30, &[agent(1, 0, 15.0, 15.0, 4)]).unwrap();
            assert_eq!(observer.stats.tracked_agent_intervals, 1);
            assert_eq!(observer.stats.path_distance, 5.0);
            assert_eq!(observer.stats.reproduction_delta, 1);
        }
    }

    #[test]
    fn arbitrary_intervals_partial_final_and_threshold_equality() {
        let mut observer = TravelObserver::default();
        observer.observe(7, &[agent(1, 0, 0.0, 12.0, 6)]).unwrap();
        assert_eq!(observer.stats.tracked_agent_intervals, 0);
        observer.observe(10, &[agent(1, 0, 5.0, 17.0, 7)]).unwrap();
        observer.observe(20, &[agent(1, 0, 8.0, 20.0, 7)]).unwrap();
        observer.observe(22, &[agent(1, 0, 9.0, 21.0, 7)]).unwrap();
        assert_eq!(observer.stats.samples, 4);
        assert_eq!(observer.stats.sample_intervals, 3);
        assert_eq!(observer.stats.tracked_agent_intervals, 3);
        assert_eq!(observer.stats.tracked_agent_ticks, 15);
        assert_eq!(observer.stats.min_interval_ticks, Some(2));
        assert_eq!(observer.stats.max_interval_ticks, Some(10));
        assert_eq!(observer.stats.path_distance, 9.0);
        assert_eq!(observer.stats.net_displacement, 9.0);
        assert_eq!(observer.stats.intervals_net_at_least_sensor_radius, 1);
        assert_eq!(observer.stats.reproduction_delta, 1);
        assert!(observer.observe(22, &[]).is_err());
        assert!(observer.observe(21, &[]).is_err());
        assert_eq!(observer.stats.samples, 4);
        assert_eq!(observer.previous.len(), 1);
    }

    #[test]
    fn matching_uses_identity_and_euclidean_distance() {
        let mut observer = TravelObserver::default();
        let first = agent(1, 0, 0.0, 0.0, 0);
        let second = agent(2, 0, 10.0, 0.0, 0);
        observer.observe(0, &[first, second]).unwrap();
        let mut moved = first;
        moved.position = [3.0, 4.0];
        moved.distance_travelled = 5.0;
        moved.sensor_radius = 100.0;
        observer.observe(10, &[second, moved]).unwrap();
        assert_eq!(observer.stats.tracked_agent_intervals, 2);
        assert_eq!(observer.stats.tracked_agent_ticks, 20);
        assert_eq!(observer.stats.net_displacement, 5.0);
        assert_eq!(observer.stats.intervals_net_at_least_sensor_radius, 1);
    }

    #[test]
    fn invalid_observations_and_decreasing_counters_rebaseline() {
        let mut observer = TravelObserver::default();
        observer.observe(0, &[agent(1, 0, 0.0, 10.0, 2)]).unwrap();
        observer.observe(10, &[agent(1, 0, 5.0, 1.0, 3)]).unwrap();
        observer.observe(20, &[agent(1, 0, 7.0, 3.0, 1)]).unwrap();
        observer
            .observe(30, &[agent(1, 0, f32::NAN, 4.0, 2)])
            .unwrap();
        observer.observe(40, &[agent(1, 0, 9.0, 5.0, 3)]).unwrap();
        observer.observe(50, &[agent(1, 0, 11.0, 7.0, 4)]).unwrap();
        assert_eq!(observer.stats.reset_counter_intervals, 2);
        assert_eq!(observer.stats.invalid_observations, 1);
        assert_eq!(observer.stats.tracked_agent_intervals, 1);
        assert_eq!(observer.stats.path_distance, 2.0);
        assert_eq!(observer.stats.reproduction_delta, 1);
        assert!(serde_json::to_string(&observer.report(10)).is_ok());
    }
}
