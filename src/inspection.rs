//! Read-only inspector bookkeeping, never controller or simulation state.
use crate::model::SelectionOutput;

#[derive(Default)]
pub struct Inspection {
    pub snapshot: Option<SelectionOutput>,
    pub tick: u32,
    pub following: bool,
    pub notice: String,
}

impl Inspection {
    pub fn select(&mut self, snapshot: Option<SelectionOutput>, tick: u32) {
        *self = Self {
            following: snapshot.is_some(),
            snapshot,
            tick,
            notice: String::new(),
        };
    }

    pub fn refresh(&mut self, result: Result<Option<SelectionOutput>, String>, tick: u32) {
        if !self.following {
            return;
        }
        match result {
            Ok(Some(current)) => {
                self.snapshot = Some(current);
                self.tick = tick;
                self.following = current.agent.alive != 0;
                self.notice = if self.following {
                    String::new()
                } else {
                    "Body died — terminal snapshot; no longer following.".into()
                };
            }
            Ok(None) => {
                self.following = false;
                self.notice =
                    "Body no longer available — last observed snapshot, not its replacement."
                        .into();
            }
            Err(error) => {
                // A failed read is not evidence of death. Retry next frame.
                self.notice =
                    format!("Read unavailable — last observed snapshot; retrying: {error}");
            }
        }
    }

    pub fn highlight(&self) -> (u32, u32) {
        match self.snapshot {
            Some(s) if self.following && self.notice.is_empty() && s.agent.alive != 0 => {
                (s.selected - 1, s.agent.generation)
            }
            _ => (u32::MAX, 0),
        }
    }

    pub fn has_decision_trace(&self) -> bool {
        self.snapshot.is_some_and(|s| {
            s.agent.alive != 0
                && self.tick > 0
                && (s.agent.ancestry_depth == 0 || self.tick > s.agent.birth_tick.saturating_add(1))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> SelectionOutput {
        let mut s = SelectionOutput {
            selected: 4,
            ..Default::default()
        };
        s.agent.alive = 1;
        s.agent.generation = 7;
        s
    }

    #[test]
    fn refresh_tracks_values_and_keeps_a_labelled_terminal_snapshot() {
        let mut view = Inspection::default();
        let mut s = snapshot();
        view.select(Some(s), 3);
        assert_eq!(view.highlight(), (3, 7));
        s.agent.position = [400.0, 600.0];
        view.refresh(Ok(Some(s)), 20);
        assert_eq!(view.snapshot.unwrap().agent.position, [400.0, 600.0]);
        assert_eq!(view.tick, 20);
        s.agent.alive = 0;
        view.refresh(Ok(Some(s)), 21);
        assert!(!view.following);
        assert_eq!(view.highlight().0, u32::MAX);
        assert!(view.notice.contains("died"));
        assert_eq!(view.snapshot.unwrap().agent.alive, 0);
        assert!(!view.has_decision_trace());
    }

    #[test]
    fn failed_reads_retry_but_reused_slots_end_tracking_without_relabelling_old_data() {
        let mut view = Inspection::default();
        view.select(Some(snapshot()), 3);
        view.refresh(Err("test read failure".into()), 4);
        assert!(view.following);
        assert_eq!(view.tick, 3);
        assert_eq!(view.highlight().0, u32::MAX);
        view.refresh(Ok(Some(snapshot())), 5);
        assert!(view.notice.is_empty());
        assert_eq!(view.highlight(), (3, 7));
        view.refresh(Ok(None), 6);
        assert!(!view.following);
        assert_eq!(view.tick, 5);
        assert!(view.notice.contains("last observed"));
        view.select(None, 7);
        assert!(view.snapshot.is_none());
        assert!(view.notice.is_empty());
    }

    #[test]
    fn newborn_and_initial_snapshots_do_not_claim_old_slot_decisions() {
        let mut view = Inspection::default();
        let mut s = snapshot();
        view.select(Some(s), 0);
        assert!(!view.has_decision_trace());
        view.refresh(Ok(Some(s)), 1);
        assert!(view.has_decision_trace());
        s.agent.ancestry_depth = 3;
        s.agent.birth_tick = 10;
        view.select(Some(s), 11);
        assert!(!view.has_decision_trace());
        view.refresh(Ok(Some(s)), 12);
        assert!(view.has_decision_trace());
    }
}
