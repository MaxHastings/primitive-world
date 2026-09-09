//! Masked sixteen-unit gated recurrent brains.  Controller weights are
//! inherited; runtime memory and learned deltas are deliberately not.
//! Mutation draw order and arithmetic match the GPU inheritance pass.
use crate::model::*;
pub fn random_packet_size(rng: &mut u32) -> f32 {
    1.0 + 47.0 * draw(rng)
}
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

/// Fresh founders choose capacity uniformly, then choose unit locations without
/// giving low-numbered units any special meaning.
pub fn random_active_mask(rng: &mut u32) -> u32 {
    let capacity = 1 + (draw(rng) * HIDDEN as f32) as usize % HIDDEN;
    let mut slots: Vec<_> = (0..HIDDEN).collect();
    for i in 0..capacity {
        let j = i + ((draw(rng) * (HIDDEN - i) as f32) as usize % (HIDDEN - i));
        slots.swap(i, j);
    }
    slots[..capacity]
        .iter()
        .fold(0u32, |mask, &h| mask | (1u32 << h))
}

pub fn random_plasticity(rng: &mut u32) -> ([f32; HIDDEN], f32, f32, f32, f32, f32) {
    let rates = std::array::from_fn(|_| (draw(rng) * 2.0 - 1.0) * 0.01);
    // Traces and learned state begin reasonably persistent but evolution owns
    // their exact time scale.
    let trace_retention = 0.8 + draw(rng) * 0.19;
    let learned_weight_retention = 0.9 + draw(rng) * 0.099;
    // Log-uniform founders give evolution both conservative and exploratory
    // lineages without treating either as the privileged default.
    let parameter_mutation_rate = 0.25 * 16.0f32.powf(draw(rng));
    let parameter_mutation_step = 0.25 * 16.0f32.powf(draw(rng));
    let topology_mutation_rate = 0.25 * 16.0f32.powf(draw(rng));
    (
        rates,
        trace_retention,
        learned_weight_retention,
        parameter_mutation_rate,
        parameter_mutation_step,
        topology_mutation_rate,
    )
}
pub fn validate(g: &[f32]) -> Result<(), String> {
    if g.len() != GENOME_SIZE || g.iter().any(|v| !v.is_finite() || v.abs() > 4.0) {
        return Err(format!(
            "Invalid masked brain: expected {GENOME_SIZE} finite parameters in [-4, 4]"
        ));
    }
    Ok(())
}

#[cfg(test)]
pub fn active(mask: u32, unit: usize) -> bool {
    mask & (1u32 << unit) != 0
}

/// Apply an occasional, bounded mutation over expressed circuitry only. A birth
/// is allowed to be an exact inherited copy; there is no temperature redraw or
/// compulsory per-birth change.
#[cfg(test)]
fn clone_activated_unit(
    genome: &mut [f32],
    donor: usize,
    new_unit: usize,
    active_mask: u32,
    rng: &mut u32,
) {
    let jitter =
        |value: f32, rng: &mut u32| (value + (draw(rng) * 2.0 - 1.0) * 0.01).clamp(-4.0, 4.0);
    genome[NODE_BIAS + new_unit] = jitter(genome[NODE_BIAS + donor], rng);
    genome[GATE_BIAS + new_unit] = jitter(genome[GATE_BIAS + donor], rng);
    for input in 0..INPUTS {
        genome[INPUT_BASE + new_unit * INPUTS + input] =
            jitter(genome[INPUT_BASE + donor * INPUTS + input], rng);
    }
    for other in 0..HIDDEN {
        if active_mask & (1 << other) == 0 {
            continue;
        }
        genome[RECURRENT_BASE + new_unit * HIDDEN + other] =
            jitter(genome[RECURRENT_BASE + donor * HIDDEN + other], rng);
        genome[GATE_BASE + new_unit * HIDDEN + other] =
            jitter(genome[GATE_BASE + donor * HIDDEN + other], rng);
    }
    for other in 0..HIDDEN {
        if !active(active_mask, other) {
            continue;
        }
        let recurrent = genome[RECURRENT_BASE + other * HIDDEN + donor] * 0.5;
        genome[RECURRENT_BASE + other * HIDDEN + donor] = recurrent;
        genome[RECURRENT_BASE + other * HIDDEN + new_unit] = recurrent;
        let gate = genome[GATE_BASE + other * HIDDEN + donor] * 0.5;
        genome[GATE_BASE + other * HIDDEN + donor] = gate;
        genome[GATE_BASE + other * HIDDEN + new_unit] = gate;
    }
    for base in [RECURRENT_BASE, GATE_BASE] {
        let value = genome[base + new_unit * HIDDEN + donor] * 0.5;
        genome[base + new_unit * HIDDEN + donor] = value;
        genome[base + new_unit * HIDDEN + new_unit] = value;
    }
    for output in 0..OUTPUTS {
        let value = genome[OUTPUT_BASE + output * HIDDEN + donor] * 0.5;
        genome[OUTPUT_BASE + output * HIDDEN + donor] = value;
        genome[OUTPUT_BASE + output * HIDDEN + new_unit] = value;
    }
}

#[cfg(test)]
fn mutate_traits(genome: &mut [f32], traits: &mut CognitiveTraits, rng: &mut u32) {
    let capacity = traits.active_mask.count_ones();
    if capacity > 1 && draw(rng) < (0.01 * traits.topology_mutation_rate).min(1.0) {
        let nth = (draw(rng) * capacity as f32) as u32;
        let mut seen = 0;
        for h in 0..HIDDEN {
            if traits.active_mask & (1 << h) != 0 {
                if seen == nth {
                    traits.active_mask &= !(1 << h);
                    break;
                }
                seen += 1;
            }
        }
    }
    let capacity = traits.active_mask.count_ones();
    if capacity < HIDDEN as u32 && draw(rng) < (0.01 * traits.topology_mutation_rate).min(1.0) {
        let donor_nth = (draw(rng) * capacity as f32) as u32;
        let empty_nth = (draw(rng) * (HIDDEN as u32 - capacity) as f32) as u32;
        let (mut donor, mut target, mut seen_on, mut seen_off) = (0usize, 0usize, 0u32, 0u32);
        for h in 0..HIDDEN {
            if traits.active_mask & (1 << h) != 0 {
                if seen_on == donor_nth {
                    donor = h;
                }
                seen_on += 1;
            } else {
                if seen_off == empty_nth {
                    target = h;
                }
                seen_off += 1;
            }
        }
        clone_activated_unit(genome, donor, target, traits.active_mask, rng);
        traits.active_mask |= 1 << target;
        traits.plasticity_rate[target] = traits.plasticity_rate[donor];
    }
}
#[cfg(test)]
fn mutate_expressed(g: &mut [f32], traits: &mut CognitiveTraits, rng: &mut u32) {
    let mask = traits.active_mask;
    let CognitiveTraits {
        plasticity_rate: plasticity,
        trace_retention,
        learned_weight_retention,
        parameter_mutation_rate,
        parameter_mutation_step,
        topology_mutation_rate,
        packet_size,
        ..
    } = traits;
    assert!(mask != 0 && g.len() == GENOME_SIZE);
    let magnitude = (BASE_MUTATION_MAGNITUDE * *parameter_mutation_step).max(0.000_001);
    let draws =
        usize::from(draw(rng) < (BASE_MUTATION_PROBABILITY * *parameter_mutation_rate).min(1.0));
    let mut expressed = Vec::with_capacity(GENOME_SIZE);
    for h in 0..HIDDEN {
        if active(mask, h) {
            expressed.push(NODE_BIAS + h);
            expressed.push(GATE_BIAS + h);
            for k in 0..INPUTS {
                expressed.push(INPUT_BASE + h * INPUTS + k);
            }
            for k in 0..HIDDEN {
                if active(mask, k) {
                    expressed.push(RECURRENT_BASE + h * HIDDEN + k);
                    expressed.push(GATE_BASE + h * HIDDEN + k);
                }
            }
            for o in 0..OUTPUTS {
                expressed.push(OUTPUT_BASE + o * HIDDEN + h);
            }
        }
    }
    expressed.extend(OUTPUT_BIAS..OUTPUT_BIAS + OUTPUTS);
    for _ in 0..draws {
        let choice = (draw(rng) * expressed.len() as f32) as usize;
        let index = expressed[choice];
        let old = g[index];
        g[index] = (old + (draw(rng) * 2.0 - 1.0) * magnitude).clamp(-4.0, 4.0);
    }
    if draws != 0 {
        let units: Vec<_> = (0..HIDDEN).filter(|&h| active(mask, h)).collect();
        let h = units[(draw(rng) * units.len() as f32) as usize];
        plasticity[h] = (plasticity[h] + (draw(rng) * 2.0 - 1.0) * magnitude).clamp(-0.2, 0.2);
        *trace_retention =
            (*trace_retention + (draw(rng) * 2.0 - 1.0) * magnitude).clamp(0.0, 0.9999);
        *learned_weight_retention =
            (*learned_weight_retention + (draw(rng) * 2.0 - 1.0) * magnitude).clamp(0.0, 0.9999);
        *parameter_mutation_rate =
            (*parameter_mutation_rate * (0.97 + 0.06 * draw(rng))).clamp(0.25, 4.0);
        *parameter_mutation_step =
            (*parameter_mutation_step * (0.97 + 0.06 * draw(rng))).clamp(0.25, 4.0);
        *topology_mutation_rate =
            (*topology_mutation_rate * (0.97 + 0.06 * draw(rng))).clamp(0.25, 4.0);
        *packet_size = (*packet_size * (0.9 + 0.2 * draw(rng))).clamp(1.0, 48.0);
    }
}

#[cfg(test)]
pub fn mutate_inherited(g: &mut [f32], traits: &mut CognitiveTraits, seed: u32) {
    let mut rng = seed;
    mutate_traits(g, traits, &mut rng);
    mutate_expressed(g, traits, &mut rng);
}

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
    fn equivalent_unit_permutations_preserve_controller_behavior() {
        let original = random_genome(&mut 719);
        let mut permuted = original;
        let inputs = std::array::from_fn(|i| ((i as f32) * 0.31).sin());
        let previous = std::array::from_fn(|i| ((i as f32) * 0.43).cos());
        let p = |i: usize| HIDDEN - 1 - i;
        for h in 0..HIDDEN {
            for base in [NODE_BIAS, GATE_BIAS] {
                permuted[base + p(h)] = original[base + h];
            }
            for k in 0..INPUTS {
                permuted[INPUT_BASE + p(h) * INPUTS + k] = original[INPUT_BASE + h * INPUTS + k];
            }
            for k in 0..HIDDEN {
                for base in [RECURRENT_BASE, GATE_BASE] {
                    permuted[base + p(h) * HIDDEN + p(k)] = original[base + h * HIDDEN + k];
                }
            }
            for o in 0..OUTPUTS {
                permuted[OUTPUT_BASE + o * HIDDEN + p(h)] = original[OUTPUT_BASE + o * HIDDEN + h];
            }
        }
        let (a, x) = evaluate(&original, &inputs, &previous);
        let (b, y) = evaluate(&permuted, &inputs, &std::array::from_fn(|i| previous[p(i)]));
        for h in 0..HIDDEN {
            assert!((a[h] - b[p(h)]).abs() < 0.00001);
        }
        for o in 0..OUTPUTS {
            assert!((x[o] - y[o]).abs() < 0.00001);
        }
    }

    #[test]
    fn inheritance_mutation_is_bounded_reproducible_and_can_be_exact() {
        let parent = random_genome(&mut 17);
        let mut child = parent;
        mutate_inherited(&mut child, &mut AgentGpu::default().cognitive_traits(), 9);
        validate(&child).unwrap();
        let mut again = parent;
        mutate_inherited(&mut again, &mut AgentGpu::default().cognitive_traits(), 9);
        assert_eq!(again, child);
        assert!(validate(&vec![0.0; 1686]).is_err());
        child[0] = f32::NAN;
        assert!(validate(&child).is_err());
    }

    #[test]
    fn founder_masks_are_nonempty_and_not_position_biased() {
        let mut rng = 71;
        let mut seen = 0u32;
        for _ in 0..4096 {
            let mask = random_active_mask(&mut rng);
            assert_ne!(mask, 0);
            seen |= mask;
        }
        assert_eq!(seen, ACTIVE_MASK_ALL);
    }

    #[test]
    fn expressed_mutation_leaves_inactive_circuitry_latent() {
        let mut rng = 91;
        let mut genome = random_genome(&mut rng);
        let before = genome;
        let mut traits = CognitiveTraits {
            active_mask: 1,
            padding: [0; 2],
            packet_size: 16.0,
            plasticity_rate: [0.0; HIDDEN],
            trace_retention: 0.9,
            learned_weight_retention: 0.99,
            parameter_mutation_rate: 1.0,
            parameter_mutation_step: 1.0,
            topology_mutation_rate: 1.0,
        };
        mutate_expressed(&mut genome, &mut traits, &mut 7);
        for h in 1..HIDDEN {
            assert_eq!(genome[NODE_BIAS + h], before[NODE_BIAS + h]);
            assert_eq!(genome[GATE_BIAS + h], before[GATE_BIAS + h]);
            assert_eq!(traits.plasticity_rate[h], 0.0);
        }
    }

    #[test]
    fn topology_mutation_rate_is_independent_of_parameter_mutation_controls() {
        let traits = |parameter_rate, parameter_step, topology_rate| CognitiveTraits {
            active_mask: 0b11,
            padding: [0; 2],
            packet_size: 16.0,
            plasticity_rate: [0.0; HIDDEN],
            trace_retention: 0.9,
            learned_weight_retention: 0.99,
            parameter_mutation_rate: parameter_rate,
            parameter_mutation_step: parameter_step,
            topology_mutation_rate: topology_rate,
        };
        let mut rare_changes = 0;
        let mut frequent_changes = 0;
        for seed in 0..4_096 {
            let mut rare = traits(0.25, 0.25, 0.25);
            let mut frequent = traits(0.25, 0.25, 4.0);
            let mut parameter_variant = traits(4.0, 4.0, 4.0);
            let mut rare_genome = blank();
            let mut frequent_genome = blank();
            let mut parameter_genome = blank();
            mutate_inherited(&mut rare_genome, &mut rare, seed);
            mutate_inherited(&mut frequent_genome, &mut frequent, seed);
            mutate_inherited(&mut parameter_genome, &mut parameter_variant, seed);
            rare_changes += usize::from(rare.active_mask != 0b11);
            frequent_changes += usize::from(frequent.active_mask != 0b11);
            assert_eq!(frequent.active_mask, parameter_variant.active_mask);
        }
        assert!(frequent_changes > rare_changes * 5);
    }
}
