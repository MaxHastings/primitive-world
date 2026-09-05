//! Shared selector configuration and headless round dispatch.
use crate::{
    headless,
    neural_selector::{Learner, Policy},
};
use std::collections::HashMap;
pub fn configure_selector(args: &HashMap<String, String>, seed: u32) -> Result<Learner, String> {
    let frozen = args.contains_key("--selector-frozen");
    let mut learner = if let Some(path) = args.get("--selector-model") {
        let loaded: Learner =
            serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        loaded.fork_model(u64::from(seed) ^ 0x53454c454354, frozen)?
    } else {
        Learner::new(u64::from(seed) ^ 0x53454c454354)
    };
    learner.frozen = frozen;
    if let Some(policy) = args.get("--selector-policy") {
        learner.policy = match policy.as_str() {
            "population" => Policy::Population,
            "individual" => Policy::Individual,
            "uniform" => Policy::Uniform,
            _ => return Err("Selector policy must be population, individual, or uniform".into()),
        };
    }
    learner.validate()?;
    Ok(learner)
}

pub fn run(args: &[String]) -> Result<(), String> {
    let options = headless::arguments(args)?;
    if options.contains_key("--single-batch") || options.contains_key("--comparison-resume") {
        crate::selector_comparison::run(&options, args)
    } else {
        crate::selector_rounds::run(&options, args)
    }
}
