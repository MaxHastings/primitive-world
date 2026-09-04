//! Compact recurrent policy used by the optional neural decision path.
//! The live simulator keeps one hidden state per agent; weights are shared.

use serde::{Deserialize, Serialize};

pub const OBSERVATIONS: usize = 12;
pub const HIDDEN: usize = 16;
pub const ACTIONS: usize = 7;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralWeights {
    pub input: Vec<f32>,
    pub recurrent: Vec<f32>,
    pub hidden_bias: Vec<f32>,
    pub output: Vec<f32>,
    pub output_bias: Vec<f32>,
}

impl NeuralWeights {
    pub fn zeros() -> Self {
        Self {
            input: vec![0.0; HIDDEN * OBSERVATIONS],
            recurrent: vec![0.0; HIDDEN * HIDDEN],
            hidden_bias: vec![0.0; HIDDEN],
            output: vec![0.0; ACTIONS * HIDDEN],
            output_bias: vec![0.0; ACTIONS],
        }
    }

    /// A neutral policy which initially chooses the authored baseline action.
    /// Neural mode is therefore safe to enable before a trained file exists.
    pub fn baseline() -> Self {
        let mut w = Self::zeros();
        for action in 0..ACTIONS {
            w.output_bias[action] = 0.01 * (ACTIONS - action) as f32;
        }
        w
    }

    pub fn validate(&self) -> Result<(), String> {
        let expected = [
            ("input", self.input.len(), HIDDEN * OBSERVATIONS),
            ("recurrent", self.recurrent.len(), HIDDEN * HIDDEN),
            ("hidden_bias", self.hidden_bias.len(), HIDDEN),
            ("output", self.output.len(), ACTIONS * HIDDEN),
            ("output_bias", self.output_bias.len(), ACTIONS),
        ];
        for (name, actual, wanted) in expected {
            if actual != wanted || !self.values(name).iter().all(|v| v.is_finite()) {
                return Err(format!("invalid neural {name} shape or value"));
            }
        }
        Ok(())
    }

    fn values(&self, name: &str) -> &[f32] {
        match name {
            "input" => &self.input,
            "recurrent" => &self.recurrent,
            "hidden_bias" => &self.hidden_bias,
            "output" => &self.output,
            _ => &self.output_bias,
        }
    }

    pub fn flat(&self) -> Vec<f32> {
        self.input
            .iter()
            .chain(&self.recurrent)
            .chain(&self.hidden_bias)
            .chain(&self.output)
            .chain(&self.output_bias)
            .copied()
            .collect()
    }

    pub fn from_flat(flat: &[f32]) -> Result<Self, String> {
        let n = HIDDEN * OBSERVATIONS + HIDDEN * HIDDEN + HIDDEN + ACTIONS * HIDDEN + ACTIONS;
        if flat.len() != n || !flat.iter().all(|v| v.is_finite()) {
            return Err(format!("expected {n} finite neural parameters"));
        }
        let mut at = 0;
        let take = |at: &mut usize, len: usize| {
            let out = flat[*at..*at + len].to_vec();
            *at += len;
            out
        };
        Ok(Self {
            input: take(&mut at, HIDDEN * OBSERVATIONS),
            recurrent: take(&mut at, HIDDEN * HIDDEN),
            hidden_bias: take(&mut at, HIDDEN),
            output: take(&mut at, ACTIONS * HIDDEN),
            output_bias: take(&mut at, ACTIONS),
        })
    }

    pub fn save_json(&self, path: &std::path::Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    }

    pub fn load_json(path: &std::path::Path) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let weights: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        weights.validate()?;
        Ok(weights)
    }
}

#[derive(Clone, Debug, Default)]
pub struct NeuralPolicy {
    pub hidden: [f32; HIDDEN],
}

impl NeuralPolicy {
    pub fn reset(&mut self) {
        self.hidden = [0.0; HIDDEN];
    }

    pub fn step(&mut self, weights: &NeuralWeights, observation: [f32; OBSERVATIONS]) -> [f32; ACTIONS] {
        let old = self.hidden;
        for h in 0..HIDDEN {
            let mut value = weights.hidden_bias[h];
            for j in 0..OBSERVATIONS {
                value += weights.input[h * OBSERVATIONS + j] * observation[j];
            }
            for j in 0..HIDDEN {
                value += weights.recurrent[h * HIDDEN + j] * old[j];
            }
            self.hidden[h] = value.tanh();
        }
        let mut logits = [0.0; ACTIONS];
        for action in 0..ACTIONS {
            let mut value = weights.output_bias[action];
            for h in 0..HIDDEN {
                value += weights.output[action * HIDDEN + h] * self.hidden[h];
            }
            logits[action] = value;
        }
        logits
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Transition {
    pub observation: [f32; OBSERVATIONS],
    pub action: u32,
    pub reward: f32,
    pub done: bool,
}

/// A small, reproducible REINFORCE trainer over recorded transitions. It is
/// deliberately offline: it never mutates policy weights inside the world.
pub fn train(transitions: &[Transition], epochs: usize, learning_rate: f32) -> NeuralWeights {
    let mut weights = NeuralWeights::baseline();
    for _ in 0..epochs {
        let mut policy = NeuralPolicy::default();
        for transition in transitions {
            let logits = policy.step(&weights, transition.observation);
            let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let mut probs = [0.0; ACTIONS];
            let mut total = 0.0;
            for (p, logit) in probs.iter_mut().zip(logits) {
                *p = (logit - max_logit).exp();
                total += *p;
            }
            for p in &mut probs { *p /= total.max(1e-6); }
            let action = transition.action as usize;
            if action < ACTIONS {
                let advantage = transition.reward.clamp(-2.0, 2.0);
                weights.output_bias[action] += learning_rate * advantage * (1.0 - probs[action]);
                for other in 0..ACTIONS {
                    if other != action { weights.output_bias[other] -= learning_rate * advantage * probs[other] / (ACTIONS as f32 - 1.0); }
                }
            }
            if transition.done { policy.reset(); }
        }
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recurrent_inference_is_deterministic_and_stateful() {
        let mut a = NeuralPolicy::default();
        let mut b = NeuralPolicy::default();
        let mut weights = NeuralWeights::baseline();
        weights.input[0] = 0.5;
        weights.recurrent[0] = 0.5;
        let observation = [0.25; OBSERVATIONS];
        assert_eq!(a.step(&weights, observation), b.step(&weights, observation));
        let first = a.hidden;
        let _ = a.step(&weights, observation);
        assert_ne!(first, a.hidden);
        a.reset();
        let mut c = NeuralPolicy::default();
        assert_eq!(a.step(&weights, observation), c.step(&weights, observation));
    }

    #[test]
    fn weight_round_trip_and_training_are_finite() {
        let transitions = vec![Transition { observation: [0.0; OBSERVATIONS], action: 2, reward: 1.0, done: true }];
        let trained = train(&transitions, 4, 0.01);
        trained.validate().unwrap();
        assert!(trained.output_bias[2] > NeuralWeights::baseline().output_bias[2]);
        assert_eq!(NeuralWeights::from_flat(&trained.flat()).unwrap().flat(), trained.flat());
    }
}
