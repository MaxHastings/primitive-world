# Evolution and hereditary continuity

The [core direction](direction.md) requires a broad, reachable search space,
without an authored preferred strategy. Selection comes from ordinary survival
and paid reproduction. Evaluation never supplies online rewards or retention rules.

## Inheritance and mutation

Offspring inherit controller weights, active masks, plasticity rates, retention
traits, parameter-mutation rate, parameter-mutation step, and topology-mutation
rate. Recurrent state, learned deltas, traces and physical
feedback reset at birth. Eligible topology retirement and activation each have a
probability of .01 times the inherited topology-mutation rate (.25â€“4). Activation
duplicates a random active unit with bounded jitter, splitting outgoing weights.
Separately, probability .25 times inherited parameter-mutation rate (.25â€“4)
perturbs one random expressed weight, one active plasticity rate and both
retention traits by bounded steps of .03 times inherited parameter-mutation step.
The three controls drift independently when parameter variation occurs. Exact
copies are valid. Mutation never consults behavior, lifespan, or outcomes.

## The rolling pool

A fixed 4,096-record pool is independent of the 16,384-body engine allocation.
Initial entries are uniform samples with replacement from the founding population.
An empty diagnostic initializes pool storage from one random record but never
restarts automatically.

Each successful birth draws a random replacement slot. A claim pass resolves
collisions in deterministic birth-allocation order before copying. Both genome
banks and traits always come from one complete child, as sequential replacement
would produce. Private learning and failed births never enter the pool. This is
rolling replacement, not a uniform sample of all historical births: older records
can be overwritten, and more births naturally contribute more records.

Natural extinction starts a newly seeded world. Fresh bodies sample unchanged
pool records uniformly with replacement, using a separate saved RNG. Their
position, age, reserves and lifetime state are freshly initialized. There are no
ranked founders, accepted candidates, mutation proposals or immigrant quotas.
Environmental dynamics have fixed strength from tick zero, without an earned
age floor or a staged curriculum. Body upkeep is stationary at 0.015 energy/tick. Random founder probes establish
reachable reproductive life cycles without a founding curriculum.

## Observation and persistence

The latest 64 completed worlds retain duration, births, ancestry and material
observations. History length and values cannot affect hereditary draws. Completion
is idempotent and requires natural extinction. A pause, save or requested work
budget never completes a living world.

Checkpoint format 55 preserves live physics, lifetime learning, both pool banks,
traits, replacement state, founder RNG and history. Validation precedes live
writes. Prior-world durations are independent of the new world's age. Previous
models are rejected without changing their files.

## Engineering limits

Insufficient body slots or impending lineage-ID overflow latches an engine fault
and disables birth allocation. The host stops at the submitted batch boundary
(at most 32 ticks in normal/headless operation). This boundary is censored:
it cannot count as extinction or seed another world. Save it for diagnosis and
start a separate experiment. The world tick horizon also stops explicitly.
These are engine limitations, not biological rules. Atomic contention means
population-wide bitwise replay is not guaranteed.


## Provenance and search-health observations

Assistance is sticky across extinction and bounded-history eviction. Any explicit
founder import is an externally chosen founding experiment and is marked assisted;
exported banks also record whether their source was assisted. No observer flag
changes an action, mutation, retention draw or reset draw.

Headless rolling reports measure exact distinct reservoir genomes, distributions
of the three mutation controls, expressed capacity, live founder-family counts,
and changed-record reservoir slots between samples. The latter uses 64-bit record
fingerprints and can miss multiple replacements or replacement by an identical
record. Successful births count replacement attempts, including collisions.
Exact-copy birth counts compare inherited genes and traits; topology counters
measure activation/retirement frequency. None is an optimization objective.
Live founder-family labels describe the current world's founding bodies; they
are not a reconstructed cross-world ancestry tree. The pool does not retain
individual lifetime histories or rank a historical lineage.

Declining diversity is allowed. A finite run showing diversity, turnover or births
is not evidence of intelligence or a reason to add novelty rewards or escape logic.
