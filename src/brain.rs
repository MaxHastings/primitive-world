//! Sparse inherited gated networks. The packed f32 representation has exact
//! integer endpoints/counts and canonical zero allocation padding. The mutation
//! contract is mirrored by shaders/brain_mutation.wgsl and checked on the GPU.
use crate::model::*;

pub type Genome = [f32; GENOME_SIZE];

pub fn blank(nodes: usize) -> Genome {
    assert!((1..=HIDDEN).contains(&nodes));
    let mut g = [0.0; GENOME_SIZE];
    g[0] = nodes as f32;
    g
}

pub fn encoded_size(nodes: usize, edges: usize) -> usize {
    2 + 2 * nodes + OUTPUTS + 3 * edges
}

pub fn add_edge(g: &mut [f32], source: usize, destination: usize, weight: f32) -> bool {
    let count = g[1] as usize;
    if count == MAX_EDGES {
        return false;
    }
    for e in 0..count {
        let b = EDGE_BASE + e * 3;
        if g[b] == source as f32 && g[b + 1] == destination as f32 {
            return false;
        }
    }
    append_edge(g, source, destination, weight);
    true
}

fn append_edge(g: &mut [f32], source: usize, destination: usize, weight: f32) {
    let b = EDGE_BASE + g[1] as usize * 3;
    g[b] = source as f32;
    g[b + 1] = destination as f32;
    g[b + 2] = weight;
    g[1] += 1.0;
}

fn draw(rng: &mut u32) -> f32 {
    *rng = rng.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*rng >> 8) as f32 / 16_777_216.0
}

pub fn random_genome(rng: &mut u32) -> Genome {
    let n = DEFAULT_NODES;
    let mut g = blank(n);
    for h in 0..n {
        g[NODE_BIAS + h] = (draw(rng) * 2.0 - 1.0) * 0.35;
        g[GATE_BIAS + h] = (draw(rng) * 2.0 - 1.0) * 0.35;
        // Uniform sampling without replacement; no special status for any sense.
        let mut available: Vec<_> = (0..INPUTS).collect();
        for _ in 0..8 {
            let index = (draw(rng) * available.len() as f32) as usize;
            let source = available.swap_remove(index);
            add_edge(&mut g, source, h, (draw(rng) * 2.0 - 1.0) * 0.25);
        }
        for k in 0..n {
            add_edge(&mut g, INPUTS + k, h, (draw(rng) * 2.0 - 1.0) * 0.35);
            add_edge(
                &mut g,
                INPUTS + k,
                HIDDEN + h,
                (draw(rng) * 2.0 - 1.0) * 0.35,
            );
        }
    }
    for o in 0..OUTPUTS {
        g[OUTPUT_BIAS + o] = (draw(rng) * 2.0 - 1.0) * 0.5;
        for h in 0..n {
            add_edge(
                &mut g,
                INPUTS + h,
                HIDDEN * 2 + o,
                (draw(rng) * 2.0 - 1.0) * 0.5,
            );
        }
    }
    g
}

pub fn validate(g: &[f32]) -> Result<(), String> {
    let integer = |v: f32, min: usize, max: usize| {
        v.is_finite() && v.fract() == 0.0 && v >= min as f32 && v <= max as f32
    };
    if g.len() != GENOME_SIZE || !integer(g[0], 1, HIDDEN) || !integer(g[1], 0, MAX_EDGES) {
        return Err("Invalid sparse brain counts or layout".into());
    }
    let n = g[0] as usize;
    let e = g[1] as usize;
    for h in 0..HIDDEN {
        for base in [NODE_BIAS, GATE_BIAS] {
            if !g[base + h].is_finite() || g[base + h].abs() > 4.0 || (h >= n && g[base + h] != 0.0)
            {
                return Err("Invalid node parameters or allocation padding".into());
            }
        }
    }
    if g[OUTPUT_BIAS..EDGE_BASE]
        .iter()
        .any(|v| !v.is_finite() || v.abs() > 4.0)
    {
        return Err("Invalid output bias".into());
    }
    let mut seen = std::collections::HashSet::with_capacity(e);
    for edge in g[EDGE_BASE..EDGE_BASE + 3 * e].chunks_exact(3) {
        if !integer(edge[0], 0, INPUTS + n - 1)
            || !integer(edge[1], 0, HIDDEN * 2 + OUTPUTS - 1)
            || !edge[2].is_finite()
            || edge[2].abs() > 4.0
        {
            return Err("Invalid connection".into());
        }
        let src = edge[0] as usize;
        let dst = edge[1] as usize;
        let valid = dst < n
            || (src >= INPUTS && ((HIDDEN..HIDDEN + n).contains(&dst) || dst >= HIDDEN * 2));
        if !valid || !seen.insert((src, dst)) {
            return Err("Invalid or duplicate connection endpoints".into());
        }
    }
    if g[EDGE_BASE + 3 * e..].iter().any(|v| *v != 0.0) {
        return Err("Nonzero connection allocation padding".into());
    }
    Ok(())
}

/// Preserves trajectories from equal reset state, including recurrent self loops
/// and candidate-to-gate projections. Capacity failure leaves the genome intact.
pub fn duplicate(g: &mut [f32], node: usize) -> bool {
    let n = g[0] as usize;
    let count = g[1] as usize;
    if n == HIDDEN || node >= n {
        return false;
    }
    let mut extra = 0;
    for e in 0..count {
        let b = EDGE_BASE + e * 3;
        let from = g[b] as usize == INPUTS + node;
        let to = g[b + 1] as usize == node || g[b + 1] as usize == HIDDEN + node;
        extra += usize::from(from) + usize::from(to) + usize::from(from && to);
    }
    if count + extra > MAX_EDGES {
        return false;
    }
    g[NODE_BIAS + n] = g[NODE_BIAS + node];
    g[GATE_BIAS + n] = g[GATE_BIAS + node];
    for e in 0..count {
        let b = EDGE_BASE + e * 3;
        let src = g[b] as usize;
        let dst = g[b + 1] as usize;
        let from = src == INPUTS + node;
        let to = dst == node || dst == HIDDEN + node;
        let w = if from { g[b + 2] * 0.5 } else { g[b + 2] };
        g[b + 2] = w;
        if from {
            append_edge(g, INPUTS + n, dst, w);
        }
        if to {
            let target = if dst < HIDDEN { n } else { HIDDEN + n };
            append_edge(g, src, target, w);
            if from {
                append_edge(g, INPUTS + n, target, w);
            }
        }
    }
    g[0] = (n + 1) as f32;
    true
}

pub fn delete_node(g: &mut [f32], node: usize) -> bool {
    let n = g[0] as usize;
    if n <= 1 || node >= n {
        return false;
    }
    let mut kept = 0;
    for e in 0..g[1] as usize {
        let b = EDGE_BASE + e * 3;
        let mut src = g[b] as usize;
        let mut dst = g[b + 1] as usize;
        if src == INPUTS + node || dst == node || dst == HIDDEN + node {
            continue;
        }
        if src == INPUTS + n - 1 {
            src = INPUTS + node;
        }
        if dst == n - 1 {
            dst = node;
        }
        if dst == HIDDEN + n - 1 {
            dst = HIDDEN + node;
        }
        let out = EDGE_BASE + kept * 3;
        g[out] = src as f32;
        g[out + 1] = dst as f32;
        g[out + 2] = g[b + 2];
        kept += 1;
    }
    g[EDGE_BASE + kept * 3..].fill(0.0);
    g[1] = kept as f32;
    for base in [NODE_BIAS, GATE_BIAS] {
        g[base + node] = g[base + n - 1];
        g[base + n - 1] = 0.0;
    }
    g[0] = (n - 1) as f32;
    true
}

/// Same draw order and arithmetic as the GPU birth implementation. Structure is
/// changed first, then only actual biases and encoded weights can be perturbed.
pub fn mutate(g: &mut [f32], seed: u32, settings: &SimSettings) {
    let mut rng = seed;
    let n = g[0] as usize;
    let choice = draw(&mut rng);
    let nr = settings.node_mutation_rate;
    let er = settings.edge_mutation_rate;
    if choice < nr {
        duplicate(g, (draw(&mut rng) * n as f32) as usize);
    } else if choice < 2.0 * nr {
        delete_node(g, (draw(&mut rng) * n as f32) as usize);
    } else if choice < 2.0 * nr + er {
        let target = (draw(&mut rng) * (2 * n + OUTPUTS) as f32) as usize;
        let (dst, src) = if target < n {
            (target, (draw(&mut rng) * (INPUTS + n) as f32) as usize)
        } else if target < 2 * n {
            (
                HIDDEN + target - n,
                INPUTS + (draw(&mut rng) * n as f32) as usize,
            )
        } else {
            (
                2 * HIDDEN + target - 2 * n,
                INPUTS + (draw(&mut rng) * n as f32) as usize,
            )
        };
        let weight = ((draw(&mut rng) * 2.0 - 1.0) * settings.mutation_magnitude).clamp(-4.0, 4.0);
        add_edge(g, src, dst, weight);
    } else if choice < 2.0 * (nr + er) {
        let count = g[1] as usize;
        let e = (draw(&mut rng) * count as f32) as usize;
        if count > 0 {
            g.copy_within(
                EDGE_BASE + (e + 1) * 3..EDGE_BASE + count * 3,
                EDGE_BASE + e * 3,
            );
            g[EDGE_BASE + (count - 1) * 3..EDGE_BASE + count * 3].fill(0.0);
            g[1] -= 1.0;
        }
    }
    let mut perturb = |value: &mut f32| {
        if draw(&mut rng) < settings.mutation_probability {
            *value = (*value + (draw(&mut rng) * 2.0 - 1.0) * settings.mutation_magnitude)
                .clamp(-4.0, 4.0);
        }
    };
    for h in 0..g[0] as usize {
        perturb(&mut g[NODE_BIAS + h]);
        perturb(&mut g[GATE_BIAS + h]);
    }
    for o in 0..OUTPUTS {
        perturb(&mut g[OUTPUT_BIAS + o]);
    }
    for e in 0..g[1] as usize {
        perturb(&mut g[EDGE_BASE + e * 3 + 2]);
    }
}

#[cfg(test)]
pub fn evaluate(
    g: &[f32],
    inputs: &[f32; INPUTS],
    previous: &[f32; HIDDEN],
) -> ([f32; HIDDEN], [f32; OUTPUTS]) {
    let n = g[0] as usize;
    let mut candidates = [0.0; HIDDEN];
    let mut gates = [0.0; HIDDEN];
    let mut hidden = [0.0; HIDDEN];
    let mut output = [0.0; OUTPUTS];
    candidates[..n].copy_from_slice(&g[NODE_BIAS..NODE_BIAS + n]);
    gates[..n].copy_from_slice(&g[GATE_BIAS..GATE_BIAS + n]);
    output.copy_from_slice(&g[OUTPUT_BIAS..EDGE_BASE]);
    for e in g[EDGE_BASE..EDGE_BASE + g[1] as usize * 3].chunks_exact(3) {
        let src = e[0] as usize;
        let dst = e[1] as usize;
        if dst < HIDDEN {
            candidates[dst] += e[2]
                * if src < INPUTS {
                    inputs[src]
                } else {
                    previous[src - INPUTS]
                };
        }
    }
    for x in &mut candidates[..n] {
        *x = x.tanh();
    }
    for e in g[EDGE_BASE..EDGE_BASE + g[1] as usize * 3].chunks_exact(3) {
        let src = e[0] as usize;
        let dst = e[1] as usize;
        if (HIDDEN..2 * HIDDEN).contains(&dst) {
            gates[dst - HIDDEN] += e[2] * candidates[src - INPUTS];
        }
    }
    for h in 0..n {
        let gate = gates[h].clamp(0.0, 1.0);
        hidden[h] = (1.0 - gate) * previous[h] + gate * candidates[h];
    }
    for e in g[EDGE_BASE..EDGE_BASE + g[1] as usize * 3].chunks_exact(3) {
        let src = e[0] as usize;
        let dst = e[1] as usize;
        if dst >= 2 * HIDDEN {
            output[dst - 2 * HIDDEN] += e[2] * hidden[src - INPUTS];
        }
    }
    (hidden, output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duplication_preserves_recurrent_trajectories_and_can_be_deleted() {
        for seed in 0..12 {
            let mut g = random_genome(&mut (seed + 100));
            g[GATE_BIAS..GATE_BIAS + DEFAULT_NODES].fill(0.5);
            for node in 0..DEFAULT_NODES {
                let mut copy = g;
                assert!(duplicate(&mut copy, node));
                validate(&copy).unwrap();
                let mut a = [0.0; HIDDEN];
                let mut b = a;
                for t in 0..50 {
                    let inputs = std::array::from_fn(|i| ((i + t) as f32 * 0.17).sin());
                    let (next, out) = evaluate(&g, &inputs, &a);
                    let (copied, out_copy) = evaluate(&copy, &inputs, &b);
                    for (x, y) in out.iter().zip(out_copy) {
                        assert!((*x - y).abs() < 0.00002);
                    }
                    a = next;
                    b = copied;
                }
                assert!(delete_node(&mut copy, node));
                validate(&copy).unwrap();
                assert_eq!(copy[0], g[0]);
            }
        }
    }

    #[test]
    fn structural_walk_has_both_directions_and_canonical_padding() {
        let settings = SimSettings {
            node_mutation_rate: 0.25,
            edge_mutation_rate: 0.25,
            mutation_magnitude: 8.0,
            ..Default::default()
        };
        let mut g = random_genome(&mut 17);
        let mut up = false;
        let mut down = false;
        for seed in 0u32..5000 {
            let n = g[0];
            mutate(&mut g, seed.wrapping_mul(7919), &settings);
            validate(&g).unwrap();
            up |= g[0] > n;
            down |= g[0] < n;
        }
        assert!(up && down);
        let mut ceiling = blank(HIDDEN);
        assert!(!duplicate(&mut ceiling, 0));
        let mut floor = blank(1);
        assert!(!delete_node(&mut floor, 0));
        let mut bad = blank(4);
        bad[NODE_BIAS + 4] = 0.1;
        assert!(validate(&bad).is_err());
        bad = blank(4);
        add_edge(&mut bad, INPUTS + 4, 0, 1.0);
        assert!(validate(&bad).is_err());
    }
}
