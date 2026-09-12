# Evolution and hereditary continuity

Evolution operates at two levels: organisms survive and reproduce within a
world, and a bounded hereditary pool carries inherited records across worlds.
The current model explicitly favors deeper within-world reproduction when
retaining pool records. It does not use a behavioral reward to train controllers.

This page specifies `primitive-v45-depth-retention`. The [design principles](direction.md)
explain the intent; the rules below describe the implemented selection preference.

| Event | What carries forward |
| --- | --- |
| Packet manufacture | Immutable inherited genome and trait snapshot, paid energy |
| Packet fusion | Recombined and possibly mutated heredity; remaining energy minus construction |
| Birth | Fresh lifetime state; one hereditary-pool admission attempt |
| Natural extinction | Pool records seed a new world; individual memories do not survive |

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

Each newborn gets one pool-admission attempt. Two slots are sampled with replacement
using separate deterministic hash draws. The entry with lower stored birth ancestry
depth is targeted; ties use a separate random bit. The newborn replaces the targeted
entry regardless of its own depth. Deep entries are protected statistically, never
permanently. Same-tick births choose against the pool before any updates; collisions
at a destination retain the highest child slot, preserving whole records.

The child's within-world ancestry depth is stored with its pool record. It is
selection metadata, not a neural input or inherited body trait. Initial pool entries
have depth zero. Founder sampling remains uniform, and new founder bodies always
start at physical ancestry depth zero. Historical pool depth is never added to new
descendants' depths. Checkpoints retain this metadata in the first reserved trait
word; the second remains zero. Model identity rejects pre-rule saves.

This explicitly favors demonstrated multigenerational reproduction. It is not pure
ecological selection, an automatic guarantee of progress, or permanent protection
for relatives. Newborn mutations still get admission attempts before maturity;
lifetime learning is never copied. Admission does not wait for juvenile maturity
or later reproduction; the retention preference uses the depth stored in pool entries.

Natural extinction starts a newly seeded world. Fresh bodies sample unchanged
pool records uniformly with replacement, using a separate saved RNG. Their
position, age, reserves and lifetime state are freshly initialized. There are no
ranked founders, accepted candidates, mutation proposals or immigrant quotas.
Body upkeep defaults to 0.05 energy/tick and does not automatically change with
world age. New worlds use their seeded habitat and saved settings. Fresh default
settings include favorable initial climate keyframes; see [ecology](world.md#accounting-and-ecology).
Juvenile physiology and reproduction costs do not ease with world age, and ongoing
climate does not observe population success.

## Observation and persistence

The latest 64 completed worlds retain duration, births, ancestry and material
observations. History length and values cannot affect hereditary draws. Completion
is idempotent and requires natural extinction. A pause, save or requested work
budget never completes a living world.

Checkpoint format 59 preserves live physics, lifetime learning, both pool banks,
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

## Implementation references

- [Pool admission and replacement](../shaders/update_reservoir.wgsl)
- [Recombination and mutation](../shaders/inherit_genomes.wgsl)
- [World continuity and history](../src/evolution.rs)
- [Founder records](../src/founders.rs) and [checkpoint persistence](../src/session.rs)

Format 58 saves from this same model load by zero-extending the new high words;
existing live biology and inherited records are preserved. New saves use format 59.
