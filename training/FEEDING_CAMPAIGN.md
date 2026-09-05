# Feeding and repeated generations — registered development campaign

Authorized after the six-round primitive-v3 pilot. Do not change agent rules,
weights-initialization distribution, action selection, mutation, scoring or physical
costs in response to intermediate results. Observer instrumentation is read-only.

Question: does a larger selection budget turn increased collection into sustainable
descendants, and where between birth, feeding, maturity and reproduction do families
currently fail? Collection itself is not a reward or promotion gate.

## Before training

Verify observer isolation and counters, including terminal juvenile deaths and
maturity transitions. Replay the pilot initial and final banks on the same already
observed evaluation seed1964496970, cap8192, default full-contrast environment.
This is a diagnostic replay, not new validation or an exact determinism claim.
Preserve its parameters and executable hash before execution. Account for the
founders'65 energy plus2 food versus offspring's0..40 energy with no carried food.

## Training registration

Run100 selection rounds,4 independent islands,64 candidate genomes per island,
8 founder replicas per candidate, root seed9042602. Use the unchanged curriculum
and fitness in TRAINING_PLAN.md. Every round has three distinct contexts per
island. Advance difficulty only under the existing repeated-family-competence
criterion, not because time elapsed. Worlds stop on extinction; candidate search
does not. Intermediate fixed benchmarks occur at round0 and every5 rounds.

All seeds, initial banks, source and executable hashes are frozen by prepare.py
before the campaign starts. No mid-run scoring edits, manual food interventions,
restored old candidates, or selection based on the benchmark results. Four
separate final development cases compare the frozen initial and final pools for
200000 ticks or extinction. This is not final eight-seed acceptance testing.

## Interpretation

Report actual ticks, extinction, harvesting, birth energy, juvenile food collection,
food-present collection choices, maturity and births to descendant parents. Compare
these at fixed benchmark settings; scores from different curriculum levels are not
directly comparable. Separate whole-pool summaries from winning-family behavior.
Increasing collection or births without repeated generations is partial progress,
not competence. If100 rounds fail, preserve the outcome and inspect the measured
bottleneck; do not silently add feeding rewards or call the newest bank smarter.

Default and personal saves remain untouched. No automatic main/default promotion.
