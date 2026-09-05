use super::*;
use crate::candidate_pool::tests::archive;

fn training() -> Training {
    let settings = SimSettings {
        population: 3,
        ..SimSettings::default()
    };
    let mut state = Training {
        version: 2,
        plan: Plan {
            rounds: 3,
            batches: 2,
            retention: 2,
            compositions: 2,
            seeds: vec![11, 22],
            bootstrap_seed: 42,
            settings,
        },
        round: 1,
        batch: 1,
        completed: false,
        comparison: None,
        bootstrap_learner: None,
        bootstrap_paused: None,
        bootstrap_duration: None,
        intake: Intake::new(128, 123),
        executed_ticks: 0,
    };
    state
        .start(
            Pool::from_archive(&archive(42, 8, 1)).unwrap(),
            Learner::new(10),
        )
        .unwrap();
    state.validate().unwrap();
    state
}

fn observe_one(state: &mut Training) {
    let c = state.comparison.as_mut().unwrap();
    let (population, index) = c.next().unwrap();
    let seed = c.seeds[index];
    // One extremely poor proposal must not replace the source pool.
    let duration = if population == 0 { 1 } else { 100 + seed };
    state
        .intake
        .observe(
            &archive(seed, 8, duration),
            Source {
                round: state.round,
                batch: state.batch,
                population,
                seed,
            },
        )
        .unwrap();
    c.durations[population][index] = Some(duration);
    state.validate().unwrap();
}

fn finish_batch(state: &mut Training) {
    while state.comparison.as_ref().unwrap().next().is_some() {
        observe_one(state);
    }
    state.comparison.as_mut().unwrap().complete().unwrap();
}

fn encoded(state: &Training) -> serde_json::Value {
    serde_json::to_value(state).unwrap()
}

#[test]
fn failed_proposals_and_learning_batches_leave_pool_fixed_until_round_boundary() {
    let mut state = training();
    let original = serde_json::to_value(&state.comparison.as_ref().unwrap().pool).unwrap();
    observe_one(&mut state);
    assert!(state.comparison.as_mut().unwrap().complete().is_err());
    assert_eq!(state.learner().updates, 0);
    finish_batch(&mut state);
    assert_eq!(state.learner().updates, 1);
    state.advance().unwrap();
    assert_eq!((state.round, state.batch), (1, 2));
    assert_eq!(
        serde_json::to_value(&state.comparison.as_ref().unwrap().pool).unwrap(),
        original
    );
    finish_batch(&mut state);
    state.advance().unwrap();
    assert_eq!((state.round, state.batch), (2, 1));
    assert_ne!(
        serde_json::to_value(&state.comparison.as_ref().unwrap().pool).unwrap(),
        original
    );
    assert_eq!(state.intake.len(), 0);
}

#[test]
fn resume_mid_batch_and_after_credit_produces_identical_future_choices_and_refresh() {
    let mut uninterrupted = training();
    observe_one(&mut uninterrupted);
    let mut resumed: Training =
        serde_json::from_slice(&serde_json::to_vec(&uninterrupted).unwrap()).unwrap();
    while !uninterrupted.completed {
        finish_batch(&mut uninterrupted);
        finish_batch(&mut resumed);
        let updates = resumed.learner().updates;
        // This is the durable receipt between applying credit and advancing.
        resumed = serde_json::from_slice(&serde_json::to_vec(&resumed).unwrap()).unwrap();
        resumed.comparison.as_mut().unwrap().complete().unwrap();
        assert_eq!(resumed.learner().updates, updates);
        uninterrupted.advance().unwrap();
        resumed.advance().unwrap();
        assert_eq!(encoded(&uninterrupted), encoded(&resumed));
    }
    assert_eq!(resumed.learner().updates, 6);
    let CandidatePool::Retained(pool) = &resumed.comparison.as_ref().unwrap().pool else {
        panic!()
    };
    assert!(pool.entries.iter().all(|c| c.source.round != 0));
}

#[test]
fn frozen_and_uniform_rounds_refresh_without_learning() {
    for uniform in [false, true] {
        let mut state = training();
        // Construct the batch under the control policy so proposal gradients agree.
        let mut learner = Learner::new(10);
        if uniform {
            learner.policy = crate::neural_selector::Policy::Uniform;
        } else {
            learner.frozen = true;
        }
        state
            .start(Pool::from_archive(&archive(42, 8, 1)).unwrap(), learner)
            .unwrap();
        let initial = state.learner().weights.clone();
        while !state.completed {
            finish_batch(&mut state);
            state.advance().unwrap();
        }
        assert_eq!(state.learner().updates, 0);
        assert_eq!(state.learner().weights, initial);
        assert_eq!(state.learner().completed_worlds, 24);
    }
}

#[test]
fn state_validation_rejects_expired_candidates_and_configuration_mismatches() {
    let mut state = training();
    state.round = 3;
    assert!(state.validate().is_err());
    let mut state = training();
    state.plan.seeds[1] = 11;
    assert!(state.validate().is_err());
    let mut state = training();
    state.plan.compositions = 3;
    assert!(state.validate().is_err());
}

#[test]
fn round_cli_requires_explicit_mode_and_resume_preserves_configuration() {
    let parse = |s: &str| {
        headless::arguments(&s.split_whitespace().map(str::to_string).collect::<Vec<_>>())
    };
    assert!(parse("app --train-loop out --rounds 3 --compositions 2").is_ok());
    assert!(parse("app --train-loop out --round-resume save --ticks 9").is_ok());
    for invalid in [
        "app --rounds 3",
        "app --train-loop out --rounds 3 --worlds 4",
        "app --train-loop out --rounds 3 --selector-horizon 2",
        "app --train-loop out --round-resume save --rounds 4",
        "app --train-loop out --rounds 3 --comparison-resume save",
        "app --train-loop out --round-resume save --population 2",
    ] {
        assert!(parse(invalid).is_err(), "{invalid}");
    }
}
