use crate::*;

pub fn agent(ui: &mut egui::Ui, state: &mut AppState) {
    if let Some(s) = state.inspection.snapshot {
        ui.separator();
        ui.small(format!(
            "Inspector snapshot: tick {} (before this frame's steps)",
            state.inspection.tick
        ));
        if !state.inspection.notice.is_empty() {
            ui.label(&state.inspection.notice);
        }
        ui.label(format!(
            "Agent {} · incarnation {} · ancestry {}",
            s.agent_id(),
            s.agent.generation,
            s.agent.ancestry_depth
        ));
        ui.label(format!(
            "Energy {:.2} · inventory {:.3} · age {:.0} · lived {} ticks",
            s.agent.energy, s.agent.food, s.agent.age, s.agent.lived_ticks
        ));
        ui.label(format!(
            "Position {:.1}, {:.1} · fixed compass sensors",
            s.agent.position[0], s.agent.position[1]
        ));
        ui.collapsing("Actual feedback (last body update)",|ui|{ui.label(format!("Collected {:.3} · ingested {:.3} · spent {:.3} · received {:.3} · displacement {:?}",s.agent.collected,s.agent.ingested,s.agent.spent,s.agent.received,s.agent.moved));});
        if state.inspection.has_decision_trace() {
            ui.small(
                "Inputs were sensed before the last step's movement; body feedback is after it.",
            );
            ui.label(format!(
                "Last body action: {} · requested movement {:.2}, {:.2}",
                action_name(s.agent.action),
                s.decision.movement[0],
                s.decision.movement[1]
            ));
            ui.small(format!(
                "Requested amount {:.3} · signal {:.3} · target {}:{}",
                s.decision.amount,
                s.decision.payload,
                s.decision.target,
                s.decision.target_generation
            ));
            ui.small(format!(
                "Requested contact displacement: {:?} × 3 units",
                s.decision.force
            ));
            ui.small(format!(
                "Fixed brain: {} memory units, {} inherited parameters",
                model::HIDDEN,
                model::GENOME_SIZE
            ));
            ui.collapsing("Surrounding sensory field", |ui| {
                ui.label(format!("Underfoot {:.3}", s.perception.resource_here));
                ui.small("Mean food per grid cell; body counts include every in-range neighbor.");
                for (i, p) in s.perception.regions.iter().enumerate() {
                    ui.small(format!(
                        "{} {}: food {:.3} · bodies {:.0}",
                        if i < 8 { "Near" } else { "Far" },
                        model::SECTOR_NAMES[i % 8],
                        p.food,
                        p.bodies
                    ));
                }
            });
            ui.collapsing("Observed bodies", |ui| {
                ui.small("Nearest in each sector; identifiers below are inspector-only.");
                for (i, b) in s
                    .perception
                    .bodies
                    .iter()
                    .enumerate()
                    .filter(|(_, b)| b.slot < MAX_AGENTS)
                {
                    ui.small(format!(
                        "{} · {}:{} offset {:?} · {}",
                        model::SECTOR_NAMES[i],
                        b.slot,
                        b.generation,
                        b.offset,
                        if b.signal_present != 0.0 {
                            format!("signal {:.3}", b.signal)
                        } else {
                            "silent".into()
                        }
                    ));
                }
            });
            ui.collapsing("Action logits (not rewards)", |ui| {
                for (name, v) in model::ACTION_NAMES.iter().zip(s.decision.scores) {
                    ui.label(format!("{name}: {v:.3}"));
                }
            });
            ui.collapsing("Internal recurrent state", |ui| {
                for (i, v) in s.agent.hidden.iter().enumerate() {
                    ui.small(format!(
                        "{i}: {v:.4} · update {:.3}",
                        s.decision.update_gates[i]
                    ));
                }
                ui.small("Numeric state has no assigned semantic labels.");
                ui.small(
                    "Update 0 retains the previous value; 1 replaces it with the proposed value.",
                );
            });
            ui.collapsing("Raw controller input vector", |ui| {
                for (i, v) in s.decision.inputs.iter().enumerate() {
                    ui.small(format!("{i}: {v:.4}"));
                }
            });
        } else if s.agent.alive == 0 {
            ui.small("Terminal body data only: decision/perception buffers may have been cleared after death.");
        } else {
            ui.small("No controller decision yet for this body; first update pending.");
        }
    } else {
        ui.heading("Follow an individual");
        ui.label("Click an agent in the world to inspect its energy, actions, and senses.");
    }
}

pub fn history(ui: &mut egui::Ui, state: &mut AppState) {
    ui.collapsing("Population history", |ui| {
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 80.0), egui::Sense::hover());
        let peak = state
            .history
            .iter()
            .map(|m| m.living)
            .max()
            .unwrap_or(1)
            .max(1) as f32;
        let points: Vec<_> = state
            .history
            .iter()
            .enumerate()
            .map(|(i, m)| {
                egui::pos2(
                    rect.left() + rect.width() * i as f32 / (state.history.len().max(2) - 1) as f32,
                    rect.bottom() - rect.height() * m.living as f32 / peak,
                )
            })
            .collect();
        if points.len() > 1 {
            ui.painter().add(egui::Shape::line(
                points,
                egui::Stroke::new(1.5, egui::Color32::LIGHT_GREEN),
            ));
        }
    });
}

pub fn physics(ui: &mut egui::Ui, state: &mut AppState) {
    ui.collapsing("World rules", |ui| {
        let s = &state.simulation.settings;
        ui.small("Configure a New Game to change world rules.");
        ui.small("Saves keep all revisions (~115–120 MB each at 1,000 founders); autosave every five minutes. No automatic deletion.");
        ui.label(format!("Founding bodies: {}", s.population));
        ui.label(format!("Food regeneration: {:.3}", s.resource_regeneration));
        ui.label(format!(
            "Metabolism: .010 → {:.3} over {} ticks · movement cost: {:.3}",
            s.metabolic_cost, s.metabolic_ramp_ticks, s.movement_energy_cost
        ));
        ui.small(format!(
            "Social actions: {}",
            if s.social_actions_enabled {
                format!(
                    "unlock progressively after the {}-tick survival bootstrap",
                    s.metabolic_ramp_ticks
                )
            } else {
                "disabled".into()
            }
        ));
        ui.small("Food patches start broader and recede to their normal footprint by the ramp cap.");
        ui.small("Metabolism includes brain upkeep. Reproduction overhead includes construction.");
    });
}

pub fn events(ui: &mut egui::Ui, state: &mut AppState, command: &mut controls::Command) {
    ui.collapsing("Recent physical events", |ui| {
        if ui.button("Refresh events").clicked() {
            *command = controls::Command::RefreshEvents;
        }
        for e in state.recent_events.iter().rev().take(30) {
            ui.small(format!(
                "{}: {} {} → {} ({:.3})",
                e.tick,
                e.actor,
                action_name(e.action),
                e.other,
                e.amount
            ));
        }
    });
}

pub fn stats(ui: &mut egui::Ui, state: &mut AppState) {
    ui.label(format!(
        "Deaths: {} starvation / {} age",
        state.starvation_deaths, state.age_deaths
    ));
    let capacity_percent = 100.0 * state.living_agents as f32 / MAX_AGENTS as f32;
    ui.small(format!(
        "Slots: {} / {} ({capacity_percent:.1}%)",
        state.living_agents, MAX_AGENTS
    ));
    if u64::from(state.living_agents) * 100 >= u64::from(MAX_AGENTS) * 95 {
        ui.colored_label(
            egui::Color32::YELLOW,
            "Capacity warning: slots nearly full; births need free slots.",
        );
    }
    if let Some(m) = state.history.back() {
        ui.label(format!(
            "Food: {:.1} vegetation / {:.1} dropped / {:.1} carried",
            m.vegetation, m.dropped_food, m.carried_food
        ));
        ui.small(format!(
            "Invalid outputs: {} · force: {} · signals: {}",
            m.invalid_outputs, m.events[5], m.signals
        ));
        ui.collapsing("Reproduction resolution", |ui| {
            for (name, count) in [
                "Immature attempts",
                "Insufficient energy",
                "Recovery active",
                "Requested",
                "Eligible before interactions",
                "Resolved",
            ]
            .iter()
            .zip(m.birth_gates)
            {
                ui.label(format!("{name}: {count}"));
            }
        });
    }
}
