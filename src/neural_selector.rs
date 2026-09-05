//! Factual-life-record population policy. Genomes never enter this module.
#![allow(clippy::needless_range_loop)] // Matrix indices align forward/backward equations.
use crate::{
    life_record::{FEATURES, LifeRecord},
    model::MAX_AGENTS,
};
use serde::{Deserialize, Serialize};
const D: usize = 16;
const ENC: usize = D * (FEATURES + 1);
const MAT: usize = D * D;
const SELECTED: usize = ENC + 2 * MAT;
const POOL: usize = SELECTED + MAT;
const BIAS: usize = POOL + MAT;
const STEP: usize = BIAS + D;
const OUT: usize = STEP + D;
const LINEAR: usize = OUT + D;
const WEIGHTS: usize = LINEAR + FEATURES;
const EPSILON: f64 = 0.1;
type V = [f64; D];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Policy {
    Population,
    Individual,
    Uniform,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Action {
    pub policy_version: u64,
    pub selected: Vec<usize>,
    /// Exchangeable, behavior-independent composition exploration, fixed across draws.
    pub composition_bias: Vec<f64>,
    pub probabilities: Vec<f64>,
    pub log_probability: f64,
    pub gradient: Vec<f64>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Learner {
    pub version: u32,
    pub policy: Policy,
    pub frozen: bool,
    pub weights: Vec<f64>,
    pub first_moment: Vec<f64>,
    pub second_moment: Vec<f64>,
    pub updates: u64,
    pub completed_worlds: u64,
    pub baseline_ticks: f64,
    pub rng: u64,
}
pub fn random(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}
fn unit(state: &mut u64) -> f64 {
    (random(state) >> 11) as f64 / (1u64 << 53) as f64
}
fn dot(a: &V, b: &V) -> f64 {
    a.iter().zip(b).map(|(a, b)| a * b).sum()
}
fn softmax(values: &[f64]) -> Vec<f64> {
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut p: Vec<_> = values.iter().map(|v| (v - max).exp()).collect();
    let total: f64 = p.iter().sum();
    for v in &mut p {
        *v /= total;
    }
    p
}
fn transform(w: &[f64], off: usize, x: &V) -> V {
    std::array::from_fn(|i| (0..D).map(|j| w[off + i * D + j] * x[j]).sum())
}
fn transform_back(w: &[f64], g: &mut [f64], off: usize, x: &V, dy: &V) -> V {
    let mut dx = [0.0; D];
    for i in 0..D {
        for j in 0..D {
            g[off + i * D + j] += dy[i] * x[j];
            dx[j] += w[off + i * D + j] * dy[i];
        }
    }
    dx
}
struct Attention {
    input: Vec<V>,
    p: Vec<Vec<f64>>,
    context: Vec<V>,
    output: Vec<V>,
}
fn attention(w: &[f64], off: usize, input: Vec<V>) -> Attention {
    let p: Vec<_> = input
        .iter()
        .map(|a| {
            softmax(
                &input
                    .iter()
                    .map(|b| dot(a, b) / (D as f64).sqrt())
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let context: Vec<V> = p
        .iter()
        .map(|p| std::array::from_fn(|k| p.iter().zip(&input).map(|(p, h)| p * h[k]).sum()))
        .collect();
    let output = input
        .iter()
        .zip(&context)
        .map(|(h, c)| {
            let t = transform(w, off, c);
            std::array::from_fn(|k| (h[k] + t[k]).tanh())
        })
        .collect();
    Attention {
        input,
        p,
        context,
        output,
    }
}
fn attention_back(w: &[f64], g: &mut [f64], off: usize, c: &Attention, dy: &[V]) -> Vec<V> {
    let n = dy.len();
    let mut dx = vec![[0.0; D]; n];
    for i in 0..n {
        let dz: V = std::array::from_fn(|k| dy[i][k] * (1.0 - c.output[i][k].powi(2)));
        let dc = transform_back(w, g, off, &c.context[i], &dz);
        for k in 0..D {
            dx[i][k] += dz[k];
        }
        let dp: Vec<_> = c.input.iter().map(|h| dot(&dc, h)).collect();
        let average: f64 = dp.iter().zip(&c.p[i]).map(|(d, p)| d * p).sum();
        for j in 0..n {
            let p = c.p[i][j];
            let ds = p * (dp[j] - average) / (D as f64).sqrt();
            for k in 0..D {
                dx[j][k] += p * dc[k] + ds * c.input[i][k];
                dx[i][k] += ds * c.input[j][k];
            }
        }
    }
    dx
}
impl Learner {
    pub fn new(seed: u64) -> Self {
        let mut rng = seed;
        let mut weights = vec![0.0; WEIGHTS];
        for (i, w) in weights[..OUT].iter_mut().enumerate() {
            *w = (unit(&mut rng) * 2.0 - 1.0)
                / if i < ENC {
                    (FEATURES as f64).sqrt()
                } else {
                    (D as f64).sqrt()
                };
        }
        // Zero output gives uniform initial selection, without preferred behaviors.
        Self {
            version: 4,
            policy: Policy::Population,
            frozen: false,
            weights,
            first_moment: vec![0.0; WEIGHTS],
            second_moment: vec![0.0; WEIGHTS],
            updates: 0,
            completed_worlds: 0,
            baseline_ticks: 0.0,
            rng,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let finite = |v: &[f64]| v.len() == WEIGHTS && v.iter().all(|x| x.is_finite());
        if self.version != 4
            || !finite(&self.weights)
            || !finite(&self.first_moment)
            || !finite(&self.second_moment)
            || self.second_moment.iter().any(|x| *x < 0.0)
            || !self.baseline_ticks.is_finite()
            || self.baseline_ticks < 0.0
        {
            return Err("Invalid life-record selector state (requires version 4)".into());
        }
        Ok(())
    }
    /// Explicit model import for a separate run. Resume instead restores all state.
    pub fn fork_model(&self, seed: u64, frozen: bool) -> Result<Self, String> {
        self.validate()?;
        let mut l = self.clone();
        l.rng = seed;
        l.frozen = frozen;
        Ok(l)
    }
    fn encoded(&self, records: &[LifeRecord]) -> (Vec<Vec<f64>>, Vec<V>) {
        // Fixed invertible numerical compression, identical for every encoded value.
        let x: Vec<Vec<f64>> = records
            .iter()
            .map(|g| g.features().iter().map(|v| v.asinh() / 10.0).collect())
            .collect();
        let h = x
            .iter()
            .map(|g| {
                std::array::from_fn(|k| {
                    let start = k * (FEATURES + 1);
                    (self.weights[start + FEATURES]
                        + g.iter()
                            .zip(&self.weights[start..start + FEATURES])
                            .map(|(x, w)| x * w)
                            .sum::<f64>())
                    .tanh()
                })
            })
            .collect();
        (x, h)
    }
    /// Exact joint log probability, including gradients through selected encodings.
    fn action(
        &self,
        records: &[LifeRecord],
        draws: usize,
        rng: &mut u64,
        forced: Option<&[usize]>,
        composition_bias: &[f64],
    ) -> Action {
        let n = records.len();
        let (x, initial) = self.encoded(records);
        let mut layers = vec![];
        let mut h = initial.clone();
        if self.policy == Policy::Population {
            for layer in 0..2 {
                let cache = attention(&self.weights, ENC + layer * MAT, h);
                h = cache.output.clone();
                layers.push(cache);
            }
        }
        let pool: V = std::array::from_fn(|k| h.iter().map(|v| v[k]).sum::<f64>() / n as f64);
        let mut selected_sum = [0.0; D];
        let mut counts = vec![0usize; n];
        let mut dh = vec![[0.0; D]; n];
        let mut gradient = vec![0.0; WEIGHTS];
        let mut selected = Vec::with_capacity(draws);
        let mut probabilities = Vec::with_capacity(draws);
        let mut log_probability = 0.0;
        for t in 0..draws {
            let previous: V = std::array::from_fn(|k| selected_sum[k] / t.max(1) as f64);
            let mut context = [0.0; D];
            if self.policy == Policy::Population {
                let a = transform(&self.weights, SELECTED, &previous);
                let b = transform(&self.weights, POOL, &pool);
                for k in 0..D {
                    context[k] = a[k] + b[k] + self.weights[STEP + k] * t as f64 / draws as f64;
                }
            }
            let z: Vec<V> = h
                .iter()
                .map(|v| {
                    std::array::from_fn(|k| (v[k] + context[k] + self.weights[BIAS + k]).tanh())
                })
                .collect();
            let mut logits: Vec<_> = z
                .iter()
                .map(|v| {
                    if self.policy == Policy::Uniform {
                        0.0
                    } else {
                        v.iter()
                            .zip(&self.weights[OUT..LINEAR])
                            .map(|(h, w)| h * w)
                            .sum()
                    }
                })
                .collect();
            if self.policy == Policy::Individual {
                logits = x
                    .iter()
                    .map(|row| {
                        row.iter()
                            .zip(&self.weights[LINEAR..])
                            .map(|(x, w)| x * w)
                            .sum()
                    })
                    .collect();
            }
            for (logit, bias) in logits.iter_mut().zip(composition_bias) {
                *logit += bias;
            }
            let soft = softmax(&logits);
            let probs: Vec<_> = soft
                .iter()
                .map(|p| (1.0 - EPSILON) * p + EPSILON / n as f64)
                .collect();
            let chosen = forced.map_or_else(
                || {
                    let r = unit(rng);
                    let mut c = 0.0;
                    probs
                        .iter()
                        .position(|p| {
                            c += p;
                            r < c
                        })
                        .unwrap_or(n - 1)
                },
                |s| s[t],
            );
            log_probability += probs[chosen].ln();
            selected.push(chosen);
            probabilities.push(probs[chosen]);
            if self.policy != Policy::Uniform && !self.frozen {
                let factor = (1.0 - EPSILON) * soft[chosen] / probs[chosen];
                if self.policy == Policy::Individual {
                    for i in 0..n {
                        let c = factor * (f64::from(i == chosen) - soft[i]);
                        for j in 0..FEATURES {
                            gradient[LINEAR + j] += c * x[i][j];
                        }
                    }
                }
                let mut dc = [0.0; D];
                for i in 0..n {
                    if self.policy == Policy::Individual {
                        break;
                    }
                    let c = factor * (f64::from(i == chosen) - soft[i]);
                    for k in 0..D {
                        gradient[OUT + k] += c * z[i][k];
                        let dz = c * self.weights[OUT + k] * (1.0 - z[i][k].powi(2));
                        gradient[BIAS + k] += dz;
                        dh[i][k] += dz;
                        dc[k] += dz;
                    }
                }
                if self.policy == Policy::Population {
                    for k in 0..D {
                        gradient[STEP + k] += dc[k] * t as f64 / draws as f64;
                    }
                    let da = transform_back(&self.weights, &mut gradient, SELECTED, &previous, &dc);
                    let db = transform_back(&self.weights, &mut gradient, POOL, &pool, &dc);
                    for i in 0..n {
                        for k in 0..D {
                            dh[i][k] +=
                                db[k] / n as f64 + da[k] * counts[i] as f64 / t.max(1) as f64;
                        }
                    }
                }
            }
            counts[chosen] += 1;
            for k in 0..D {
                selected_sum[k] += h[chosen][k];
            }
        }
        if self.policy != Policy::Uniform && !self.frozen {
            for (layer, c) in layers.iter().enumerate().rev() {
                dh = attention_back(&self.weights, &mut gradient, ENC + layer * MAT, c, &dh);
            }
            for i in 0..n {
                for k in 0..D {
                    let dz = dh[i][k] * (1.0 - initial[i][k].powi(2));
                    let start = k * (FEATURES + 1);
                    for j in 0..FEATURES {
                        gradient[start + j] += dz * x[i][j];
                    }
                    gradient[start + FEATURES] += dz;
                }
            }
        }
        Action {
            policy_version: self.updates,
            selected,
            composition_bias: composition_bias.to_vec(),
            probabilities,
            log_probability,
            gradient,
        }
    }
    /// A random logit offset per candidate changes whole-population mixtures.
    /// Its law is independent of measurements and parameters; gradients are conditional on it.
    fn exploration(&self, count: usize, rng: &mut u64) -> Vec<f64> {
        if self.policy == Policy::Uniform || unit(rng) >= 0.25 {
            return vec![];
        }
        (0..count)
            .map(|_| {
                let radius = (-2.0 * unit(rng).max(f64::MIN_POSITIVE).ln()).sqrt();
                3.0 * radius * (std::f64::consts::TAU * unit(rng)).cos()
            })
            .collect()
    }
    pub fn propose_comparison(
        &mut self,
        records: &[LifeRecord],
        draws: usize,
        count: usize,
    ) -> Result<Vec<Action>, String> {
        self.validate()?;
        if records.is_empty()
            || records.len() > 256
            || !(2..=32).contains(&count)
            || draws == 0
            || draws > MAX_AGENTS as usize
        {
            return Err("Matched comparison requires 2..32 populations, valid life records, and no pending loop credit".into());
        }
        for record in records {
            record.validate()?;
        }
        let mut rng = self.rng;
        let actions = (0..count)
            .map(|_| {
                let bias = self.exploration(records.len(), &mut rng);
                self.action(records, draws, &mut rng, None, &bias)
            })
            .collect();
        self.rng = rng;
        Ok(actions)
    }
    /// Each row is one independently sampled population, each column the same environment seed.
    /// Only a fully observed batch trains. Other populations supply the leave-one-out baseline.
    pub fn complete_comparison(
        &mut self,
        actions: &[Action],
        ticks: &[Vec<u32>],
    ) -> Result<bool, String> {
        self.validate()?;
        if actions.len() < 2
            || actions.len() != ticks.len()
            || ticks[0].len() < 2
            || ticks.iter().any(|r| r.len() != ticks[0].len())
            || actions.iter().any(|a| {
                a.policy_version != self.updates
                    || a.gradient.len() != WEIGHTS
                    || a.gradient.iter().any(|g| !g.is_finite())
            })
        {
            return Err("Invalid, incomplete, or stale matched comparison".into());
        }
        let mut gradient = vec![0.0; WEIGHTS];
        let mut total = 0.0;
        for seed in 0..ticks[0].len() {
            let sum: f64 = ticks.iter().map(|r| f64::from(r[seed])).sum();
            total += sum;
            for (i, a) in actions.iter().enumerate() {
                let reward = f64::from(ticks[i][seed]);
                let baseline = (sum - reward) / (actions.len() - 1) as f64;
                let advantage =
                    (reward - baseline) / (10_000.0 * actions.len() as f64 * ticks[0].len() as f64);
                for (g, score) in gradient.iter_mut().zip(&a.gradient) {
                    *g += advantage * score;
                }
            }
        }
        let trained = !self.frozen && self.policy != Policy::Uniform;
        if trained {
            self.apply_gradient(&gradient);
        }
        self.completed_worlds += (actions.len() * ticks[0].len()) as u64;
        self.baseline_ticks = total / (actions.len() * ticks[0].len()) as f64;
        self.validate()?;
        Ok(trained)
    }
    fn apply_gradient(&mut self, gradient: &[f64]) {
        self.updates += 1;
        let t = self.updates as f64;
        for (i, g) in gradient.iter().enumerate() {
            self.first_moment[i] = 0.9 * self.first_moment[i] + 0.1 * g;
            self.second_moment[i] = 0.999 * self.second_moment[i] + 0.001 * g * g;
            self.weights[i] += 0.0003 * (self.first_moment[i] / (1.0 - 0.9f64.powf(t)))
                / ((self.second_moment[i] / (1.0 - 0.999f64.powf(t))).sqrt() + 1e-8);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn records() -> Vec<LifeRecord> {
        (0..3)
            .map(|i| LifeRecord {
                ticks: 10 + i,
                actions: [10 + i, 0, 0, 0, 0, 0],
                collected: (i * 7) as f32,
                distance: (i * 30) as f32,
                ..LifeRecord::initial(65.0, 2.0, 0, 0)
            })
            .collect()
    }
    fn nonuniform() -> Learner {
        let mut l = Learner::new(9);
        for i in 0..D {
            l.weights[OUT + i] = 0.2 * (i as f64 - 7.0);
        }
        l
    }
    #[test]
    fn joint_gradient_matches_finite_differences() {
        let mut l = nonuniform();
        let gs = records();
        let choices = [0, 2, 2, 1];
        let a = l.action(&gs, 4, &mut 1, Some(&choices), &[]);
        for i in [
            0,
            2,
            FEATURES,
            3 * (FEATURES + 1) + 20,
            ENC,
            ENC + MAT + 7,
            SELECTED + 9,
            POOL + 17,
            BIAS + 3,
            STEP + 2,
            OUT + 3,
        ] {
            let w = l.weights[i];
            let eps = 1e-5;
            l.weights[i] = w + eps;
            let plus = l
                .action(&gs, 4, &mut 1, Some(&choices), &[])
                .log_probability;
            l.weights[i] = w - eps;
            let minus = l
                .action(&gs, 4, &mut 1, Some(&choices), &[])
                .log_probability;
            l.weights[i] = w;
            let expected = (plus - minus) / (2.0 * eps);
            assert!(
                (a.gradient[i] - expected).abs() < 1e-6,
                "parameter {i}: {} vs {expected}",
                a.gradient[i]
            );
        }
    }
    #[test]
    fn pool_permutation_preserves_joint_probability_and_gradient() {
        let l = nonuniform();
        let gs = records();
        let mut reordered = gs.clone();
        reordered.swap(0, 2);
        let a = l.action(&gs, 4, &mut 1, Some(&[0, 2, 2, 1]), &[]);
        let b = l.action(&reordered, 4, &mut 1, Some(&[2, 0, 0, 1]), &[]);
        assert!((a.log_probability - b.log_probability).abs() < 1e-12);
        assert!(
            a.gradient
                .iter()
                .zip(b.gradient)
                .all(|(a, b)| (a - b).abs() < 1e-10)
        );
    }
    #[test]
    fn choices_condition_population_policy_and_retain_exploration() {
        let mut l = nonuniform();
        let gs = records();
        let a = l.action(&gs, 2, &mut 1, Some(&[0, 1]), &[]);
        let b = l.action(&gs, 2, &mut 1, Some(&[2, 1]), &[]);
        assert!((a.probabilities[1] - b.probabilities[1]).abs() > 1e-10);
        assert!(a.probabilities.iter().all(|p| *p >= EPSILON / 3.0));
        l.policy = Policy::Individual;
        assert_eq!(
            l.action(&gs, 2, &mut 1, Some(&[0, 1]), &[]).probabilities[1],
            l.action(&gs, 2, &mut 1, Some(&[2, 1]), &[]).probabilities[1]
        );
    }
}
