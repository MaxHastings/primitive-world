//! Observer-only GPU funnel and dual-parent life-cycle certificates.
use super::*;
use serde_json::json;

pub(crate) struct FunnelObserver {
    counters: wgpu::Buffer,
    contacts: wgpu::Buffer,
    contact_pre: Compute,
    lives: wgpu::Buffer,
    events: wgpu::Buffer,
    scratch: wgpu::Buffer,
    pre: Compute,
    summary: Compute,
    post: Compute,
}

#[test]
fn observer_is_physically_neutral_and_counts_real_births() {
    let (d, q) = super::tests::gpu();
    let mut baseline = Simulation::new(&d, &q, 2001);
    let mut observed = Simulation::new(&d, &q, 2001);
    for s in [&mut baseline, &mut observed] {
        s.settings.population = 0;
        s.reset(&q);
        let mut genes = crate::brain::blank();
        genes[OUTPUT_BIAS + 5] = 2.0;
        genes[OUTPUT_BIAS + 8] = 3.0;
        for slot in 0..2 {
            let a = AgentGpu {
                position: [100.0 + slot as f32, 100.0],
                energy: 35.0,
                age: 1800.0,
                max_speed: 1.2,
                sensor_radius: 24.0,
                max_age: 11000.0,
                alive: 1,
                lineage_id: slot as u32 + 1,
                generation: 1,
                active_mask: 1,
                ..Default::default()
            };
            for b in &s.agent_buffers {
                q.write_buffer(
                    b,
                    slot as u64 * std::mem::size_of::<AgentGpu>() as u64,
                    bytemuck::bytes_of(&a),
                );
            }
            s.write_genome_slot(&q, slot, &genes);
        }
    }
    observed.funnel_observer = Some(FunnelObserver::new(&d, &observed));
    for n in [0, 1, 1] {
        super::tests::step(&mut baseline, &d, &q, n);
        super::tests::step(&mut observed, &d, &q, n);
        let mut a = baseline.agent_snapshot(&d, &q).unwrap();
        let mut b = observed.agent_snapshot(&d, &q).unwrap();
        // Concurrent packet creation assigns diagnostic lineage labels through
        // an atomic counter; GPU scheduling may permute those labels. At this
        // two-tick horizon only unchanged founder IDs can be packet producers.
        // All other physical and cognitive fields must match bit-for-bit.
        for item in a.iter_mut().chain(b.iter_mut()) {
            item.lineage_id = 0;
        }
        let first = bytemuck::cast_slice::<AgentGpu, u32>(&a)
            .iter()
            .zip(bytemuck::cast_slice::<AgentGpu, u32>(&b))
            .position(|(x, y)| x != y);
        if let Some(at) = first {
            eprintln!(
                "first mismatch tick={} slot={} word={} values={} / {}",
                baseline.tick,
                at / 74,
                at % 74,
                bytemuck::cast_slice::<AgentGpu, u32>(&a)[at],
                bytemuck::cast_slice::<AgentGpu, u32>(&b)[at]
            );
        }
        assert!(
            first.is_none(),
            "observer comparison differs at tick{}",
            baseline.tick
        );
    }
    for (a, b) in [
        (&baseline.resource_buffer, &observed.resource_buffer),
        (&baseline.ecology_buffer, &observed.ecology_buffer),
        (&baseline.death_stats_buffer, &observed.death_stats_buffer),
    ] {
        assert!(
            observability::read_buffer(&d, &q, a).unwrap()
                == observability::read_buffer(&d, &q, b).unwrap()
        );
    }
    let report = observed.funnel_observer.as_ref().unwrap().report(&d, &q);
    let metrics = observed.metrics(&d, &q).unwrap();
    assert_eq!(report["births"], metrics.events[3]);
    assert_eq!(report["packets_produced"], metrics.birth_gates[2]);
    assert!(report["births"].as_u64().unwrap() > 0);
}

#[test]
#[ignore = "diagnostic for pre-existing dense GPU replay variability, with and without observer"]
fn dense_replay_order_probe() {
    let (d, q) = super::tests::gpu();
    for enabled in [false, false, false, false, true, true] {
        let mut baseline = Simulation::new(&d, &q, 2001);
        let mut s = Simulation::new(&d, &q, 2001);
        if enabled {
            s.funnel_observer = Some(FunnelObserver::new(&d, &s));
        }
        super::tests::step(&mut baseline, &d, &q, 1);
        super::tests::step(&mut s, &d, &q, 1);
        let mut b = baseline.agent_snapshot(&d, &q).unwrap();
        for item in &mut b {
            item.lineage_id = 0;
        }
        let mut a = s.agent_snapshot(&d, &q).unwrap();
        for item in &mut a {
            item.lineage_id = 0;
        }
        {
            let left = bytemuck::cast_slice::<AgentGpu, u32>(&b);
            let right = bytemuck::cast_slice::<AgentGpu, u32>(&a);
            let differences: Vec<_> = left
                .iter()
                .zip(right)
                .enumerate()
                .filter(|(_, (x, y))| x != y)
                .map(|(i, (x, y))| (i / 74, i % 74, *x, *y))
                .collect();
            eprintln!(
                "observer={enabled} differing words={} first={:?}",
                differences.len(),
                differences.first()
            );
        }
    }
}
impl FunnelObserver {
    pub fn new(d: &wgpu::Device, s: &Simulation) -> Self {
        Self::with_capacity(d, s, 65536)
    }
    pub fn with_capacity(d: &wgpu::Device, s: &Simulation, capacity: u32) -> Self {
        assert!(capacity > 0 && capacity <= 2_000_000);
        let counters = buffer(d, "funnel counters", 32 * 4);
        let lives = buffer(d, "funnel incarnations", MAX_AGENTS as u64 * 32);
        let events = buffer(d, "funnel life events", u64::from(capacity) * 48);
        let scratch = buffer(d, "funnel packet scratch", 16);
        let contacts = buffer(d, "juvenile contact opportunities", MAX_AGENTS as u64 * 32);
        let contact_pre = Compute::new(
            d,
            "read-only contact opportunities",
            include_str!("opportunity_contacts.wgsl"),
            "main",
            "rurrrw",
            (0..2)
                .map(|i| {
                    vec![
                        &s.agent_buffers[i],
                        &s.params_buffer,
                        &s.decision_buffer,
                        &s.audit_cell_offsets,
                        &s.audit_indices,
                        &contacts,
                    ]
                })
                .collect(),
        );
        let make = |entry| {
            Compute::new(
                d,
                "read-only funnel",
                include_str!("funnel_audit.wgsl"),
                entry,
                "rrurrwwwwrr",
                (0..2)
                    .map(|i| {
                        vec![
                            &s.agent_buffers[i],
                            &s.agent_buffers[1 - i],
                            &s.params_buffer,
                            &s.perception_buffer,
                            &s.claims_buffer,
                            &counters,
                            &lives,
                            &events,
                            &scratch,
                            &s.decision_buffer,
                            &contacts,
                        ]
                    })
                    .collect(),
            )
        };
        let pre = make("pre_fusion");
        let summary = make("coexistence");
        let post = make("post_tick");
        Self {
            contacts,
            contact_pre,
            counters,
            lives,
            events,
            scratch,
            pre,
            summary,
            post,
        }
    }
    pub fn before_contacts(&self, e: &mut wgpu::CommandEncoder, source: usize) {
        self.contact_pre
            .dispatch(e, source, MAX_AGENTS.div_ceil(64), 1);
    }
    pub fn contact_counts(&self, d: &wgpu::Device, q: &wgpu::Queue) -> Vec<[u32; 8]> {
        let bytes = observability::read_buffer(d, q, &self.contacts).unwrap();
        bytemuck::cast_slice(&bytes).to_vec()
    }
    pub fn before_fusion(&self, e: &mut wgpu::CommandEncoder, source: usize) {
        e.clear_buffer(&self.scratch, 0, None);
        self.pre.dispatch(e, source, MAX_AGENTS.div_ceil(64), 1);
        self.summary.dispatch(e, source, 1, 1);
    }
    pub fn after_tick(&self, e: &mut wgpu::CommandEncoder, source: usize) {
        self.post.dispatch(e, source, MAX_AGENTS.div_ceil(64), 1);
    }
    pub fn counts(&self, d: &wgpu::Device, q: &wgpu::Queue) -> [u32; 32] {
        let bytes = observability::read_buffer(d, q, &self.counters).unwrap();
        let values: [u32; 32] = *bytemuck::from_bytes(&bytes);
        assert_eq!(values[1], 0, "funnel observer capacity exceeded");
        values
    }
    pub fn report(&self, d: &wgpu::Device, q: &wgpu::Queue) -> serde_json::Value {
        let words = observability::read_buffer(d, q, &self.counters).unwrap();
        let totals: &[u32] = bytemuck::cast_slice(&words);
        assert_eq!(
            totals[1], 0,
            "funnel observer capacity exceeded; cohort is incomplete"
        );
        assert!(u64::from(totals[0]) <= self.events.size() / 48);
        let bytes = observability::read_buffer(d, q, &self.events).unwrap();
        let all: &[[u32; 12]] = bytemuck::cast_slice(&bytes);
        let events = &all[..totals[0] as usize];
        let mut fusion = std::collections::BTreeMap::new();
        for e in events.iter().filter(|e| e[0] == 6) {
            fusion.insert((e[1], e[4]), ([e[5], e[6]], [e[2], e[9]]));
        }
        let packet_qualified: std::collections::BTreeMap<_, _> = events
            .iter()
            .filter(|e| e[0] == 9)
            .map(|e| (e[2], e[6] != 0))
            .collect();
        let mut ordered = events.to_vec();
        ordered.sort_by_key(|e| (e[1], e[0]));
        let mut individuals = std::collections::BTreeMap::<u32, serde_json::Value>::new();
        let mut births_involving_descendants = 0u32;
        let mut closed_depth = 0u32;
        for e in &ordered {
            let lineage = e[2];
            let tick = e[1];
            match e[0] {
                1 => {
                    let (parents, packets) = *fusion
                        .get(&(tick, e[10]))
                        .expect("birth missing dual-parent fusion evidence");
                    let mut depth = 0u32;
                    let mut descendant_parent = false;
                    for (parent, packet) in parents.into_iter().zip(packets) {
                        if let Some(p) = individuals.get(&parent) {
                            descendant_parent = true;
                            if *packet_qualified
                                .get(&packet)
                                .expect("fusion packet missing production evidence")
                            {
                                depth = depth
                                    .max(p["closed_depth_at_birth"].as_u64().unwrap() as u32 + 1);
                            }
                        }
                    }
                    births_involving_descendants += u32::from(descendant_parent);
                    closed_depth = closed_depth.max(depth);
                    individuals.insert(lineage,json!({"lineage":lineage,"generation":e[3],"birth_tick":tick,"parents":parents,"birth_energy":f32::from_bits(e[8]),"closed_depth_at_birth":depth,"maturity_tick":null,"adult_gather_tick":null,"qualified_packet_tick":null,"adult_packets":0,"juvenile_received_milli":0,"dead":false}));
                }
                2 => {
                    let p = individuals
                        .get_mut(&lineage)
                        .expect("maturation missing birth");
                    p["maturity_tick"] = json!(tick);
                    p["juvenile_received_milli"] = json!(e[9]);
                    p["juvenile_gathered_milli"] = json!(e[10]);
                }
                3 => {
                    let p = individuals
                        .get_mut(&lineage)
                        .expect("adult gathering missing birth");
                    p["adult_gather_tick"] = json!(tick);
                }
                4 => {
                    let p = individuals
                        .get_mut(&lineage)
                        .expect("adult production missing birth");
                    p["adult_packets"] = json!(e[10]);
                    if p["maturity_tick"].is_u64() && p["adult_gather_tick"].is_u64() {
                        p["qualified_packet_tick"] = json!(tick);
                    }
                }
                5 => {
                    let p = individuals.get_mut(&lineage).expect("death missing birth");
                    p["dead"] = json!(true);
                    p["death_tick"] = json!(tick);
                    p["death_age"] = json!(f32::from_bits(e[7]));
                    p["juvenile_received_milli"] = json!(e[9]);
                    p["adult_gathered_milli"] = json!(e[10]);
                }
                12 => {
                    let p = individuals
                        .get_mut(&lineage)
                        .expect("contact record missing birth");
                    p["contact_opportunity"] = json!({"observed_ticks":e[10],"nearby_organism_ticks":e[5],"nearby_food_ticks":e[6],"nearby_funded_transfer_intent_ticks":e[9]});
                }
                _ => {}
            }
        }
        let life_bytes = observability::read_buffer(d, q, &self.lives).unwrap();
        let lives: &[[u32; 8]] = bytemuck::cast_slice(&life_bytes);
        for l in lives {
            if let Some(p) = individuals.get_mut(&l[0])
                && p["generation"] == l[1]
            {
                p["adult_gathered_milli"] = json!(l[6]);
                p["juvenile_received_milli"] = json!(l[4]);
            }
        }
        for c in self.contact_counts(d, q) {
            if let Some(p) = individuals.get_mut(&c[0])
                && p["generation"] == c[1]
            {
                p["contact_opportunity"] = json!({"observed_ticks":c[5],"nearby_organism_ticks":c[2],"nearby_food_ticks":c[3],"nearby_funded_transfer_intent_ticks":c[4]});
            }
        }
        json!({"opportunities":{"mature_manufacture_blocked_actual_energy":totals[23],"mature_manufacture_blocked_fraction_only":totals[24],"mature_manufacture_affordable_but_not_produced":totals[25],"pre_fusion_packet_ticks":totals[26],"packet_ticks_with_compatible_proposal":totals[27],"packet_ticks_with_viable_proposal":totals[28],"packet_ticks_above_32_energy":totals[29],"packet_ticks_with_proposal_above_48_birth_energy":totals[30],"scope":"Post-interaction, pre-fusion observations. Failed manufacture observations partition mature selected actions with no manufacture. The fraction-only bucket describes an insufficient hypothetical fraction budget; it is a causal veto only when the legacy gate is explicitly installed. Under production actual-energy funding it can instead include allocation failures. Proposal counts are packet-ticks, can count both sides and do not equal distinct pairs. New packets contribute one pre-fusion tick before eligibility begins."},"packets_produced":totals[2],"compatible_packet_coexist_ticks":totals[3],"births":totals[4],"juvenile_transfer_ticks":totals[5],"juvenile_received_milli":totals[6],"juveniles_matured":totals[7],"matured_descendants_gathered":totals[8],"matured_descendant_packets":totals[9],"matured_descendants_produced_packets":totals[10],"births_involving_descendant_parents":births_involving_descendants,"maximum_closed_life_cycle_depth":closed_depth,"maximum_genealogical_depth":totals[22],"signals":{"emissions":totals[11],"nonzero_aggregate_exposure_ticks":totals[12],"transfer_selected_during_exposure":totals[13],"reproduction_selected_during_exposure":totals[14],"transfer_inventory_debits_after_recent_exposure_including_possible_death_drops":totals[15],"actual_packet_productions_with_exposure_within_16_ticks":totals[16]},"adult_descendant_gathered_milli":totals[17],"juvenile_gathered_milli":totals[18],"founder_food_seen_ticks":totals[19],"founder_harvest_ticks":totals[20],"all_harvested_milli":totals[21],"individuals":individuals,"life_events":events,"scope":"Per-tick test-only observer. Per-child contact_opportunity is measured after movement and before contact allocation: surviving immature ticks, any nearby organism, any neighbor with food while receiver has headroom, and any such neighbor selecting transfer. A nearby transfer intention need not target this child or win allocation. Compatible coexistence means positive-energy packets from different producers exist simultaneously before fusion anywhere in the world; it does not require proximity or sufficient combined energy. Both fusion parents retained. Closed depth counts consecutive non-founder links that matured, gathered as adults, produced a subsequent packet and contributed to a birth; founders start at depth0 and a merely born child does not close a loop. Signal association is observational, not a causal effect or success requirement. The transfer inventory debit proxy can include terminal food drops and is not an actual delivery count. Event ticks are completed one-based ticks. Overflow fails explicitly."})
    }
}
