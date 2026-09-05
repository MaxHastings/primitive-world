# Actual-survivor serial transfer

This replaces neither the controller nor the physics. It implements the missing
cross-world feedback loop. No authored starter, family ranking, action reward,
energy ranking, original-founder substitution, or automatic difficulty adjustment.

## Frozen first pilot

- Four independent random-origin lines, eight transfers each, 1024 bodies/world.
- Normal dynamic geography and defaults: metabolism .06, movement .01, regeneration
  .01, habitat contrast 1. Each world ends on extinction or 8192 ticks.
- Every 128 ticks (also initial/end), sample up to 64 living bodies using hash
  ordering independent of behavior, energy and family. Retain only the newest
  nonempty sample. This is sampled late survival, NOT the exact last 64 deaths.
  It may retain only one body. Founders are eligible alongside descendants;
  depth and source identity are exposed, never passed off as descendant progress.
- Copy the actual current genomes of those bodies. Extinction cannot erase the
  last nonempty sample. The observer is read-only; it never changes world state.
- Seed the next world with every sampled genome exactly once, then balanced
  mutated replicas to fill 256 genomes. Each weight independently mutates with
  probability .02 by uniform ±.03, clipped to ±4, rounded to f32. This is explicitly
  EXTERNAL variation, matching the in-world mutation scale, not lifetime learning.
  No random immigrants or crossing between lines. Each genome gets four founders.
- Body age, energy, food and hidden state reset through ordinary initialization.
  We carry genes, not learned memories. New-world ancestry counters reset; cross-
  world provenance records sampled identities and exact f32 genome hashes.
- Evaluate all four lines at rounds 0, 4 and 8 on the same two separate world
  seeds, at rotations 0 and 1. These worlds never feed selection. Train seeds and
  evaluation seeds are unique and frozen before execution. No cherry-picked line.

## Interpretation and limits

Compare matched evaluation extinction times, food acquired, births, matured
descendants and descendant-parent births. Report every line, including regressions.
Later training-world survival alone is not comparable across changing seeds.
An 8192-tick survivor is right-censored, not immortal. The pilot cannot establish
survival across major resource relocations (which begin later), intelligence,
communication use, or a statistically secure generalization claim with two seeds.

Serial transfer is an artificial selection experiment, not untouched evolution:
sampling and rejuvenation can favor long-lived nonreproducers, and terminal
bottlenecks can discard useful diversity. These are measured limitations, not
hidden fixes. If it improves only founder survival, say so. If it plateaus, do not
quietly add priors, make food easier, or declare the model trained. Inspect that
evidence before choosing a longer run or a different selection mechanism.

## Run and evidence

Build the research worktree, then use a new directory:

```powershell
cargo build --release --target-dir target/feeding-audit
python training/survivor_loop.py --directory reports/my-survivor-loop
```

`summary.json` updates after each world and contains training and evaluation
separately. Each world has a report/log, immutable completion receipt and input
hash. Training worlds also retain the actual sampled genome bank. Every transfer
has exact-source hashes and per-child parent/mutation provenance. No full endpoint
checkpoint is necessary: extinct endpoint bodies cannot seed the next world.

The runner freezes the executable, source, scripts and protocol. Resume with the
frozen script and `--resume --directory ABSOLUTE_RUN_DIRECTORY`; verified completed
cases are not rerun. Partial cases are refused, not overwritten or silently retried.
Failure leaves its evidence and an explicit status for inspection. A cleanly
completed pilot stops; no endless background campaign or automatic promotion.

Headless `--survivors PATH [--survivor-sample N]` saves the last nonempty sample
as a compatible bank with extra provenance; N is 1..1024 (default 128). Output
paths must be new. This differs from GUI/export-founders, which still exports
only living descendants at the current instant. Sampling is held in memory until
normal run completion; interruption may leave an empty output, not a valid bank.
