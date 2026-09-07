//! Fixed eight-unit gated recurrent brains. Only weights and biases are inherited.
//! Mutation draw order and arithmetic match shaders/brain_mutation.wgsl.
use crate::model::*;
pub type Genome = [f32; GENOME_SIZE];
#[cfg(test)]
pub fn blank() -> Genome {
    [0.0; GENOME_SIZE]
}
fn draw(rng: &mut u32) -> f32 {
    *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*rng >> 8) as f32 / 16_777_216.0
}
pub fn random_genome(rng: &mut u32) -> Genome {
    let mut g = [0.0; GENOME_SIZE];
    for (i, value) in g.iter_mut().enumerate() {
        let scale = if (INPUT_BASE..RECURRENT_BASE).contains(&i) {
            0.1
        } else {
            0.35
        };
        *value = (draw(rng) * 2.0 - 1.0) * scale;
    }
    // Initially open memory updates halfway; inherited gates can evolve freely.
    for value in &mut g[GATE_BIAS..OUTPUT_BIAS] {
        *value += 0.5;
    }
    g
}
pub fn validate(g: &[f32]) -> Result<(), String> {
    if g.len() != GENOME_SIZE || g.iter().any(|v| !v.is_finite() || v.abs() > 4.0) {
        return Err("Invalid fixed brain: expected 1188 finite parameters in [-4, 4]".into());
    }
    Ok(())
}
/// Apply one individual, continuously scaled mutation. The temperature is
/// log-uniform, so most variation stays local while rare genomes take a larger
/// step. This exact draw order is shared with the birth shader.
pub fn mutate(g: &mut [f32], seed: u32, base_probability: f32, base_magnitude: f32) {
    assert!(!g.is_empty());
    let mut rng = seed;
    let temperature = MUTATION_TEMPERATURE_MIN
        * (MUTATION_TEMPERATURE_MAX / MUTATION_TEMPERATURE_MIN).powf(draw(&mut rng));
    let scale = temperature.sqrt();
    let probability = (base_probability * scale).clamp(0.0, 1.0);
    let magnitude = (base_magnitude * scale).max(0.000_001);
    let mut changed = false;
    for value in g.iter_mut() {
        if draw(&mut rng) < probability {
            let next = (*value + (draw(&mut rng) * 2.0 - 1.0) * magnitude).clamp(-4.0, 4.0);
            changed |= next != *value;
            *value = next;
        }
    }
    if !changed {
        let index = (draw(&mut rng) * g.len() as f32) as usize % g.len();
        let direction = if draw(&mut rng) < 0.5 { -1.0 } else { 1.0 };
        let value = g[index];
        let next = (value + direction * magnitude).clamp(-4.0, 4.0);
        g[index] = if next == value {
            (value - direction * magnitude).clamp(-4.0, 4.0)
        } else {
            next
        };
        debug_assert_ne!(g[index], value);
    }
}
/// Test fixtures address connections by their logical endpoints.
#[cfg(test)]
pub fn add_edge(g: &mut [f32], source: usize, destination: usize, weight: f32) -> bool {
    let index = if destination < HIDDEN {
        if source < INPUTS {
            INPUT_BASE + destination * INPUTS + source
        } else {
            RECURRENT_BASE + destination * HIDDEN + source - INPUTS
        }
    } else if destination < 2 * HIDDEN {
        GATE_BASE + (destination - HIDDEN) * HIDDEN + source - INPUTS
    } else {
        OUTPUT_BASE + (destination - 2 * HIDDEN) * HIDDEN + source - INPUTS
    };
    g[index] = weight;
    true
}
#[cfg(test)]
pub fn evaluate(
    g: &[f32],
    inputs: &[f32; INPUTS],
    previous: &[f32; HIDDEN],
) -> ([f32; HIDDEN], [f32; OUTPUTS]) {
    let mut candidate = [0.0; HIDDEN];
    for h in 0..HIDDEN {
        let mut sum = g[NODE_BIAS + h];
        for k in 0..INPUTS {
            sum += g[INPUT_BASE + h * INPUTS + k] * inputs[k];
        }
        for k in 0..HIDDEN {
            sum += g[RECURRENT_BASE + h * HIDDEN + k] * previous[k];
        }
        candidate[h] = sum.tanh();
    }
    let mut hidden = [0.0; HIDDEN];
    for h in 0..HIDDEN {
        let mut sum = g[GATE_BIAS + h];
        for k in 0..HIDDEN {
            sum += g[GATE_BASE + h * HIDDEN + k] * candidate[k];
        }
        let gate = sum.clamp(0.0, 1.0);
        hidden[h] = (1.0 - gate) * previous[h] + gate * candidate[h];
    }
    let output = std::array::from_fn(|o| {
        let mut sum = g[OUTPUT_BIAS + o];
        for h in 0..HIDDEN {
            sum += g[OUTPUT_BASE + o * HIDDEN + h] * hidden[h];
        }
        sum
    });
    (hidden, output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inheritance_mutation_is_bounded_reproducible_and_never_exact() {
        let parent = random_genome(&mut 17);
        let mut child = parent;
        mutate(
            &mut child,
            9,
            BASE_MUTATION_PROBABILITY,
            BASE_MUTATION_MAGNITUDE,
        );
        validate(&child).unwrap();
        assert_ne!(child, parent);
        let mut again = parent;
        mutate(
            &mut again,
            9,
            BASE_MUTATION_PROBABILITY,
            BASE_MUTATION_MAGNITUDE,
        );
        assert_eq!(again, child);
        assert!(validate(&vec![0.0; 1686]).is_err());
        child[0] = f32::NAN;
        assert!(validate(&child).is_err());
    }
}
