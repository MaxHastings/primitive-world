use super::*;
#[test]
fn streamed_birth_mutation_matches_cpu_and_preserves_parent_parameters() {
    let (d, q) = gpu();
    let mut s = scene(&d, &q);
    s.settings.mutation_probability = 0.4;
    s.settings.mutation_magnitude = 0.25;
    let parent = fixed(5, [0.0; 2]);
    put(&s, &q, 0, body([200.0, 200.0]), &parent);
    step(&mut s, &d, &q, 1);
    let agents = s.agent_snapshot(&d, &q).unwrap();
    let slot = agents
        .iter()
        .position(|a| a.alive != 0 && a.ancestry_depth == 1)
        .unwrap();
    let mut expected = parent;
    crate::brain::mutate(&mut expected, agents[0].rng ^ slot as u32, &s.settings);
    let genes = read::<f32>(&d, &q, &s.genome_buffer, (slot + 1) * GENOME_SIZE);
    assert_eq!(&genes[..GENOME_SIZE], &parent);
    for (x, y) in genes[slot * GENOME_SIZE..(slot + 1) * GENOME_SIZE]
        .iter()
        .zip(expected)
    {
        near(*x, y);
    }
    assert_eq!(agents[slot].hidden, [0.0; HIDDEN]);
}
