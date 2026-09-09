# Primitive World finish-line evidence

[direction.md](direction.md) is the design contract. This ledger separates verified
implementation from operational evidence that cannot be established by a short
local test. **The overall model is not declared done until the outstanding
operational checks below pass.** Boring runs are not a reason to redesign biology.

## Current model contract

- Model: `primitive-v35-body-frame-contact`.
- Checkpoint: 55 (`PRIMWORLD055`); founder bank: 20.
- Controller: 107 inputs, 1-16 expressed recurrent units, 14 outputs; 2,494 genes.
- Stationary body upkeep: 0.05. The metabolism ramp and its persistence field are removed.
- Retained lifecycle assumptions and reasons: [world.md](world.md).
- No compatibility execution of earlier biological representations.

These are the identities selected for this generation. Do not increment them for
UI, diagnostics or documentation work. Revisit biological identity only if the
model changes; do not reinterpret an older saved layout under the current ID.

## Migration closure

| Phase | Implementation and regression evidence |
| --- | --- |
| 1: torus | Wrapped sensing, ecology/weather centers, contact, picking and interventions; seam and rectangular-grid tests. |
| 2: small biases | No self-signal history or semantic success inputs; independent paid gathering; activation plus amplitude signal cost; accounting tests. |
| 3: mutation | Independently inherited parameter rate, step and topology rate; blind mutation and exact copies; CPU/GPU parity across capacities. |
| 4: generic perception | Heading, body-relative velocity/displacement and sixteen identical six-channel area samples; no nearest-body records or reserved inputs; rotation/locality/count tests. |
| 5: contact | No controller body-slot target; nearest physical contact and immutable proposals; equal-and-opposite paid impulses; proportional gathering removes contention-based allocation. |
| 6: birth placement | Bounded parent-controlled body-frame placement; relative randomized newborn heading, zero velocity/learning; placement and reserve tests. |

Square resource-grid quarter turns and grid-aligned translations are the exact
field symmetries. Continuous body geometry also has arbitrary-angle tests. Float32
sample reductions use a documented 1e-6 regression tolerance; discrete counts are
exact. Arbitrary subcell food-field transformations have raster aliasing, not a
claimed exact continuous symmetry.

## Philosophy and hereditary continuity

- [x] No rewards, observer feedback, merit-ranked founders, directed mutations, novelty incentives or optimizer rescue.
- [x] Lifetime recurrent state, traces and learned weights do not enter heredity.
- [x] Optional active capacity and local learning pay physical costs.
- [x] Fixed 4,096 whole-record rolling pool; only successful births replace records.
- [x] Extinction draws unchanged records uniformly without ranking or reset mutation.
- [x] Pool and founder RNG continuation are checkpoint-tested.
- [x] Assistance persists through extinction, 64-world history eviction, checkpoints and exports; imported founder banks mark external founding choice.
- [x] Engine saturation and accounting horizons stop/censor; they cannot complete or reseed a world.

Tests include `reservoir_and_world_transitions_resume_without_observer_selection`,
`simultaneous_births_replace_whole_reservoir_records_deterministically`,
`assisted_provenance_survives_restarts_history_eviction_and_checkpoint`, and
`capacity_exhaustion_is_not_extinction_or_hereditary_selection`.

## Reachability and search observations

Three fixed-seed final-default probes ran 20,000 ticks from 1,000 random founders,
without interventions. Seeds 7/42/123 ended with 2,766/1,694/3,514 living bodies and
23,632/10,330/19,473 births, respectively, with zero invalid outputs. All exceeded
the maximum founder lifespan. This establishes reachable reproductive life cycles;
it does not establish intelligence, adaptation or guaranteed success. No further
viability tuning is called for. Raw report names and hardware are in [long-run.md](long-run.md).

Rolling reports now include reservoir genome diversity, mutation-control and
active-capacity histograms, current-world family representation, changed reservoir
records between samples, exact-copy births and topology-change counts. They are
read-only. Changed slots undercount intervening replacements, and family labels
are not a cross-world reconstructed pedigree. Finite observations must not become
novelty bonuses or retention rules.

## Validation record

Commands are run on the target Windows machine with RTX 4070 SUPER / NVIDIA 591.86
and Vulkan. Local logs stay under ignored `reports/`.

- Formatting, compile/check and strict all-target Clippy pass.
- Full debug suite: 109 passed, zero failed, six manual diagnostics not run by default.
- Python tooling suite: 12 passed.
- Full release suite: 109 passed, zero failed, six manual diagnostics not run by default.
- Headless soak: 50,000 ticks, 60,037 births, 2,282 living, zero invalid outputs; checkpoint saved and reloaded successfully.
- Separate process resumed that checkpoint for 32 ticks, saved again, and validated the continuation checkpoint.
- Sampled soak peak process working set: 679 MiB; validation process: 90 MiB. This is short-run evidence, not a multi-day bound.

The suites cover mutation parity, torus seams, rotation, generic perception,
contact/recoil, reproduction/accounting, observer isolation, pool replacement,
checkpoint continuation, engine saturation and multi-world extinction/restart.
A regression drives 66 world transitions and resumes after checkpointing, so
bounded-history eviction is exercised rather than inferred.

The six ignored diagnostics require an existing experiment library, image-output
paths, throughput profiling, a supplied founder bank, or exposed desktop UI. They
are not known failing unit tests and are not evidence for unperformed desktop work.
No test failure may be called expected without revising and documenting its contract.

## Bounded operation and remaining freeze gates

Event storage is a 65,536-record ring; world history is capped at 64; headless
report history is capped at 4,096 samples per invocation. The soak runner retains
two validated checkpoints and 256 diagnostic files, stops before starting a chunk
when the run directory exceeds 8 GiB, verifies executable hashes and checkpoints,
and times out a hung child while preserving the last valid receipt. Body, pool,
learning and index buffers are fixed-size. These limits are engineering facts.

- [x] Complete and inspect the 50,000-tick soak and a separate-process checkpoint continuation.
- [ ] Exercise the soak runner across enough chunks to empirically verify pruning of expired checkpoints (two-checkpoint retention is implemented).
- [ ] Establish multi-day memory stability and restart/resume on the final binary.
- [ ] Exercise viewer autosave/recovery, wallpaper composition, monitor changes and sleep/wake on the target desktop.
- [ ] Reconcile those operational results here before declaring the entire model done.

A short headless soak cannot prove sleep/wake, Explorer hosting or multi-day viewer
stability. Keep these unchecked until they are actually exercised. Remaining
physiology choices are explicit assumptions in world.md, not pending invitations
to tune toward interesting behavior.


### Observed search health at 50,000 ticks

Seed 42's pool changed from 983 to 1,682 distinct genomes. Of 60,037 births,
30,434 were exact inherited copies; 188 activation and 333 retirement events
were observed. There were 807 changed pool-record slots in the final sample
interval. Two current-world founder families remained represented among living
bodies. These are descriptive finite-run observations. They establish continuing
variation/turnover, not intelligence or long-run evolutionary health.

The copied soak binary SHA-256 was
`6891056E7BC18E30236AB7EAD4AB178A14B14F87B3BD00CD57E335BD450FC62C`.
The run stopped at its requested tick budget, without an engine fault. Raw logs,
reports and checkpoints remain ignored local artifacts, not committed data.
