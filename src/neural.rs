//! Versioned GRU contract shared with training/policy.py and WGSL.
//!
//! The contract describes an archived alternative controller. The active
//! kernel uses the inherited local controller by default; this path is retained
//! for numerical comparison and training experiments only.
use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 3;
pub const OBSERVATIONS: usize = 24;
pub const HIDDEN: usize = 32;
pub const ACTIONS: usize = 14;
pub const INTERVAL: u32 = 8;
pub const ACTION_NAMES: [&str; ACTIONS] = [
    "wait",
    "collect",
    "ingest",
    "north",
    "north-east",
    "east",
    "south-east",
    "south",
    "south-west",
    "west",
    "north-west",
    "transfer",
    "apply force",
    "emit",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NeuralWeights {
    pub version: u32,
    pub name: String,
    pub input: Vec<f32>,
    pub recurrent: Vec<f32>,
    pub input_bias: Vec<f32>,
    pub recurrent_bias: Vec<f32>,
    pub output: Vec<f32>,
    pub output_bias: Vec<f32>,
}
impl NeuralWeights {
    pub fn baseline() -> Self {
        Self {
            version: VERSION,
            name: "untrained-zero-policy".into(),
            input: vec![0.; 3 * HIDDEN * OBSERVATIONS],
            recurrent: vec![0.; 3 * HIDDEN * HIDDEN],
            input_bias: vec![0.; 3 * HIDDEN],
            recurrent_bias: vec![0.; 3 * HIDDEN],
            output: vec![0.; ACTIONS * HIDDEN],
            output_bias: vec![0.; ACTIONS],
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.version != VERSION {
            return Err("Unsupported neural policy schema (expected GRU v3)".into());
        }
        for (v, n) in [
            (&self.input, 3 * HIDDEN * OBSERVATIONS),
            (&self.recurrent, 3 * HIDDEN * HIDDEN),
            (&self.input_bias, 3 * HIDDEN),
            (&self.recurrent_bias, 3 * HIDDEN),
            (&self.output, ACTIONS * HIDDEN),
            (&self.output_bias, ACTIONS),
        ] {
            if v.len() != n || v.iter().any(|x| !x.is_finite()) {
                return Err("Invalid neural parameter shape or nonfinite value".into());
            }
        }
        Ok(())
    }
    pub fn flat(&self) -> Vec<f32> {
        [
            &self.input,
            &self.recurrent,
            &self.input_bias,
            &self.recurrent_bias,
            &self.output,
            &self.output_bias,
        ]
        .into_iter()
        .flat_map(|x| x.iter().copied())
        .collect()
    }
    pub fn from_flat(flat: &[f32]) -> Result<Self, String> {
        let mut w = Self::baseline();
        let mut at = 0;
        for v in [
            &mut w.input,
            &mut w.recurrent,
            &mut w.input_bias,
            &mut w.recurrent_bias,
            &mut w.output,
            &mut w.output_bias,
        ] {
            let end = at + v.len();
            if end > flat.len() {
                return Err("Truncated neural weights".into());
            }
            v.copy_from_slice(&flat[at..end]);
            at = end;
        }
        if at != flat.len() {
            return Err("Extra neural parameters".into());
        }
        w.validate()?;
        Ok(w)
    }
    pub fn save_json(&self, path: &std::path::Path) -> Result<(), String> {
        self.validate()?;
        std::fs::write(path, serde_json::to_vec(self).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())
    }
    pub fn load_json(path: &std::path::Path) -> Result<Self, String> {
        let w: Self = serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        w.validate()?;
        Ok(w)
    }
}

/// Written at decision time, never reconstructed from a later body snapshot.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable, Serialize, Deserialize)]
pub struct NeuralState {
    pub generation: u32,
    pub choice: u32,
    pub tick: u32,
    pub valid: u32,
    pub hidden: [f32; HIDDEN],
    pub before: [f32; HIDDEN],
    pub after: [f32; HIDDEN],
    pub observation: [f32; OBSERVATIONS],
    pub mask: [f32; ACTIONS],
    pub logits: [f32; ACTIONS],
    pub probabilities: [f32; ACTIONS],
    pub energy: f32,
    pub food: f32,
}
impl Default for NeuralState {
    fn default() -> Self {
        Self::zeroed()
    }
}
#[derive(Clone, Debug, Default)]
#[cfg(test)]
pub struct NeuralPolicy {
    pub hidden: [f32; HIDDEN],
}
#[cfg(test)]
impl NeuralPolicy {
    pub fn step(&mut self, w: &NeuralWeights, obs: [f32; OBSERVATIONS]) -> [f32; ACTIONS] {
        let mut ix = [0.; 3 * HIDDEN];
        let mut hx = [0.; 3 * HIDDEN];
        for k in 0..3 * HIDDEN {
            ix[k] = w.input_bias[k];
            hx[k] = w.recurrent_bias[k];
            for j in 0..OBSERVATIONS {
                ix[k] += w.input[k * OBSERVATIONS + j] * obs[j];
            }
            for j in 0..HIDDEN {
                hx[k] += w.recurrent[k * HIDDEN + j] * self.hidden[j];
            }
        }
        for k in 0..HIDDEN {
            let r = 1. / (1. + (-(ix[k] + hx[k])).exp());
            let z = 1. / (1. + (-(ix[HIDDEN + k] + hx[HIDDEN + k])).exp());
            let n = (ix[2 * HIDDEN + k] + r * hx[2 * HIDDEN + k]).tanh();
            self.hidden[k] = (1. - z) * n + z * self.hidden[k];
        }
        std::array::from_fn(|a| {
            w.output_bias[a]
                + (0..HIDDEN)
                    .map(|k| w.output[a * HIDDEN + k] * self.hidden[k])
                    .sum::<f32>()
        })
    }
}
