# Evolution and hereditary continuity

The [core direction](direction.md) requires a broad, reachable search space,
without an authored preferred strategy. Selection comes from ordinary survival
and paid reproduction. Evaluation never supplies online rewards or retention rules.

## Inheritance and mutation

Two resource-bearing packets from different producers must meet locally. Each
packet contains an inherited genome snapshot that survives its producer. Fusion
blindly recombines both snapshots before ordinary mutation. Packet size segregates
independently and can mutate; contribution size never determines genetic donation.
See [agents.md](agents.md#reproductive-packets) for costs and module boundaries.
Recurrent state, learned deltas, traces and physical
feedback reset at birth. Eligible topology retirement and activation each have a
probability of .01 times the inherited topology-mutation rate (.25–4). Activation
duplicates a random active unit with bounded jitter, splitting outgoing weights.
Separately, probability .25 times inherited parameter-mutation rate (.25–4)
perturbs one random expressed weight, one active plasticity rate and both
retention traits by bounded steps of .03 times inherited parameter-mutation step.
The three controls drift independently when parameter variation occurs. Exact
copies are valid. Mutation never consults behavior, lifespan, or outcomes.

## The rolling pool

A fixed 4,096-record pool is independent of the 16,384-slot organism/packet allocation.
Initial entries are uniform samples with replacement from the founding population.
An empty diagnostic initializes pool storage from one random record but never
restarts automatically.

Each successful birth draws a random replacement slot. A claim pass resolves
collisions in deterministic child-slot order before copying. Both genome
banks and traits always come from one complete child, as sequential replacement
would produce. Private learning and failed births never enter the pool. This is
rolling replacement, not a uniform sample of all historical births: older records
can be overwritten, and more births naturally contribute more records.

Admission occurs at birth, before juvenile survival or maturation is known.
The pool record is not removed if that juvenile dies, and additional feeding,
longevity or maturation does not refresh it. A juvenile that later reproduces
can contribute additional descendant records through those actual births.
Thus partial care can extend physical opportunities without directly increasing
cross-world representation until it changes subsequent reproduction. This is
the production A rule. The direct re-entry audit establishes a developmental
bypass: a dead juvenile record can return as an adult founder. A test-only
mature-parent admission comparison is documented in
[evolutionary-accessibility-audit.md](evolutionary-accessibility-audit.md).
The physical care gradient remains frozen while that comparison runs.

Natural extinction starts a newly seeded world. Fresh bodies sample unchanged
pool records uniformly with replacement, using a separate saved RNG. Their
position, age, reserves and lifetime state are freshly initialized. There are no
ranked founders, accepted candidates, mutation proposals or immigrant quotas.
Body upkeep is stationary at 0.05 energy/tick. New worlds use their seeded
habitat without opening food coverage. Juvenile dependence and reproduction
physics are permanent; ongoing climate does not observe population success.

## Observation and persistence

The latest 64 completed worlds retain duration, births, ancestry and material
observations. History length and values cannot affect hereditary draws. Completion
is idempotent and requires natural extinction. A pause, save or requested work
budget never completes a living world.

Checkpoint format 58 preserves live physics, lifetime learning, both pool banks,
traits, replacement state, founder RNG and history. Validation precedes live
writes. Prior-world durations are independent of the new world's age. Previous
models are rejected without changing their files.

## Engineering limits

Packet requests that exceed free storage are skipped without payment; a
tick-varying allocation order shares admission opportunities. In-place fusion
continues at full storage. This is an explicit gameplay limit and can influence
outcomes near capacity. Accounting horizons automatically roll into a new world
without claiming extinction. Atomic food/sensory reductions retain their existing
floating-point tolerances; controlled checkpoint replay is regression-tested.


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
Exact-copy birth counts compare inherited genes and traits with the arbitration-selected
parent (not both parents or the pre-mutation recombinant); topology counters
measure activation/retirement frequency. None is an optimization objective.
Live founder-family labels describe the current world's founding bodies; they
follow only the arbitration-selected parental branch and are not a complete
two-parent pedigree or reconstructed cross-world ancestry tree. The pool does not retain
individual lifetime histories or rank a historical lineage.

Declining diversity is allowed. A finite run showing diversity, turnover or births
is not evidence of intelligence or a reason to add novelty rewards or escape logic.
