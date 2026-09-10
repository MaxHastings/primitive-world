# Substrate audit — 2026-09-10

Historical audit: model-specific evidence and earlier proposals follow. The current
release decision is defined in [the north star](north-star.md).

Subsequent controlled evidence is in the [sensor reachability audit](sensor-reachability-audit.md):
the sensor-valid care-dependent descendant loop now has a successful witness.
The observations below remain the record of the earlier substrate investigation.

The only production change in this pass is fractional gathering request rounding.
The intended development curve, physiology, actions, transfer targeting,
reproduction, mutation, climate, initial stocks and hereditary reservoir are
unchanged. Presentation and controls are unchanged. The selected straight-corridor
control demonstrates natural adult surplus and funded reproduction; a sustainable
descendant life cycle and discoverability remain unproven.

## Numerical obstruction and correction

`shaders/consume.wgsl` previously converted each request directly to `u32`.
At maximum effort a newborn requests 25 × .01 = .25 millifood per tick, but
received zero even on uncontested abundant ground. The first nonzero integer
request occurred at age 1006 of 1800. This discarded fractional demand rather
than implementing the specified gathering curve.

New worlds use deterministic stochastic rounding: floor(request) plus one with
probability equal to its fractional part. The draw uses the existing body RNG
state, tick and a fixed salt; it does not advance another RNG, depend on storage
order, accumulate credits, or create food. Demand and allocation passes recompute
the same draw. Actual debits remain exact integer millifood. The expectation
preserves the intended curve to the 24-bit draw resolution; individual histories
remain stochastic. A fractional accumulator was considered, but requires new
lifetime state and decisions about carrying unfulfilled effort across depletion
and movement. Raising the minimum yield would change the intended physiology.

The serialized `fractional_gathering` setting defaults to true for new settings
and false when absent from historical saves. Old checkpoints therefore retain
their historical numerical behavior; the body layout and genome format are
unchanged. This option is a compatibility boundary, not a controller input.

The actual GPU regression samples 1024 independent bodies per condition:
newborn maximal effort, age-900 maximal effort, adult .01 effort, adult maximal
effort and zero effort. Every debit equals collection exactly; requests are
bounded by floor/ceiling and sample means are within .05 millifood of the curve.
The historical age-threshold test remains with truncation explicitly selected.
Full release regression: 154 passed, 0 failed, 15 diagnostic/manual tests ignored.
Checkpoint replay, permutation invariance, juvenile dependence and controlled
transfer-supported maturation passed.

Remaining numerical limits are explicit: proportional allocation under scarcity
and inventory headroom still truncate to integer millifood. A one-millifood stock
shared by multiple equal requests can remain unallocated. The present change
does not establish unbiased allocation under contention.

## Ordinary-reserve adult controls

The earlier six-adult straight-path and stationary trials failed before packet
funding. Those controllers did not choose paths from food. The added
`local_foraging_economy` experiment chooses the richest natural starting cell in
seed 91 and runs a lone adult or a pair for up to 4000 active ticks. Each begins
with 35 energy, zero carried food, a one-unit brain and size-16 packets. All
subsequent physical states are produced by normal kernels. The external
controller changes action readouts only, pays normal motor/gathering/upkeep
costs, searches exact cells within 24 units, and chooses food/(4+distance).
The pair shares a destination and attempts at most two packets per founder,
only above 60 energy. Juveniles receive no care in this adult-economy probe.

This grants stronger environmental information than the actual regional senses
and selects favorable placement and cheap cognition. It is a declared physical
feasibility probe, not a random bootstrap result or a proof of a heritable policy.
Failure of this particular greedy route is not a proof that every route fails.

Fresh ecology results:

| Control | Lifetime ticks | Peak energy | Peak carried food | Harvested food | Births |
| --- | ---: | --- | --- | --- | ---: |
| One adult | 1273 | 38.99 | .025 | 5.607 | 0 |
| Pair | 1048 until both dead | 35.62 / 36.71 | .025 / .025 | 1.008 / 3.893 | 0 |

The single adult gathered .00440 food/tick on average. Stationary upkeep plus
maximal gathering alone needs (.05 + .00025 + .005)/8 = .00690625 food/tick,
before motion. Its observed total expenditure was 79.856 energy; initial energy
plus food conversion was approximately the same, explaining extinction.

Fresh harvest is transferable before next-tick digestion, so .025 carried food
is a real transient transfer opportunity even below reserve saturation. These
trials failed to accumulate a buffer; they did not have literally zero possible
offers. No adult exceeded .1 carried food.

Raw records: `reports/substrate-audit-01/local-foraging.json`; regression log:
`reports/substrate-tests.log`. Reports are local ignored artifacts. Reproduce
with a new `PRIMITIVE_AUDIT_OUTPUT` directory and
`cargo test --release local_foraging_economy -- --ignored --nocapture`.
Reports use exclusive creation and will not overwrite earlier evidence.

A follow-up held the controller constant and advanced the full, agent-free
ecology for 10,000 normal ticks before inserting the same ordinary-reserve
founders. This was an experimental spin-up under the existing favorable phase,
not a stationary-climate initialization change or added food. The single adult
survived 1579 active ticks and peaked at 41.26 energy; the pair lasted 1264 ticks
and peaked at 35.88 / 38.58. All still peaked at .025 carried food and produced
no births. This spin-up did not rescue this controller.

The final diagnostic run corrects an observer accounting issue: dead bodies
retain their last per-tick fields, so accumulated gathering/spending must exclude
ticks where a founder was already dead. Earlier pair totals included stale fields
after the first adult died. Peak reserves, peak food, extinction timing and birth
counts are unaffected. Use `reports/substrate-audit-03` for corrected totals.

## Counterexample to an unreachable adult economy

After the greedy-route failures, a separate control selects the best eastbound
128-cell corridor in seed 91's initial natural food map. It ranks adjacent lane
pairs by the weaker lane's stock, capped at five maximal harvests per cell for
the declared .8-unit/tick speed. The selected origin is cell (313, 202), with
15.892 food available to the weaker lane under that initial scoring rule.
This is an explicitly selected existence test, not an unselected success rate.

Two adults start in adjacent lanes with 35 energy, zero food, one expressed
neural unit and size-16 packets. They move east at constant paid effort and
gather at full effort. From tick 800, both may produce a packet when both exceed
60 energy, up to two packets each. Inward packet placement uses the existing
physical actuator. Food, climate, reserves, physiology and juvenile care are
never edited. There is no ecological spin-up in this successful trial.

| Measurement over 4000 ticks | Adult 1 | Adult 2 |
| --- | ---: | ---: |
| Peak energy | 99.937 | 99.937 |
| Peak carried food | 7.992 | 7.992 |
| Total naturally gathered food | 47.743 | 47.712 |
| Energy spent, including two packets each | 285.025 | 285.025 |
| Final energy | 99.937 | 99.937 |
| Final carried food | 3.995 | 3.964 |

Both already had more than .32 carried food at the tick-500 observation. They
manufactured four packets, producing two births through actual fusion. Both
founders remained alive at tick 4000. Both unprovisioned offspring died; no
transfer was requested. This test isolates adult accumulation and funded birth,
not care or descendant sustainability. Adult maximal requests are already exact
25-millifood integers, so their successful harvest is not a subsidy from the
fractional juvenile fix.

Raw evidence: `reports/substrate-audit-03/corridor-foraging.json`. Reproduce with
a new output directory and `cargo test --release corridor_foraging_economy --
--ignored --nocapture`. The two final economy diagnostics are logged together
in `reports/substrate-economy.log`.

This counterexample changes the diagnosis: default favorable ecology can
support healthy adults with buffers and repeated packet funding along at least
one selected natural route. The greedy controller's failure therefore cannot
justify raising ecology stocks or lowering physiology costs. It does not prove
that local regional sensing can locate and remain on such routes, that random
agents can discover them, or that nearby juveniles can remain in reach. Those
are the next causal comparisons, using this successful route as the reference.
Care-specific assistance and an expanded random-seed verdict remain premature.

## Active-system map and audit limits

| Mechanism | Active implementation and interaction | Limitation relevant to this audit |
| --- | --- | --- |
| Ecology | `src/climate.rs`, `resource_update.wgsl`: smooth forcing, soil, water/mineral/detritus, vegetation and local substrate | Favorable rain does not certify harvestable production. Cells have no lateral transport; sampled sites cannot establish connectivity. |
| Sensing | `perceive.wgsl`, neural input construction: 16 body-relative regions, food, occupancy, motion, signals, pressure and private physical feedback | Aggregate anonymous measurements cannot identify an exact hungry juvenile or reproduce the test controller's cell map. |
| Brain | `src/brain.rs`, decide shaders, `plasticity.wgsl`: gated recurrence, expression mask, per-unit plasticity and lifetime traces | Minimal scripted brains demonstrate selected possibilities, not mutation accessibility. |
| Actions/economy | `consume.wgsl`, `update_agents.wgsl`, `interactions.wgsl`: independent gathering, automatic digestion, paid motion, nearest contact transfer/force, emission and packet production | Nearest full recipient can block another recipient. Carried fresh harvest and persistent buffer must be measured separately. |
| Reproduction | Interaction production, `apply_births.wgsl`, `inherit_genomes.wgsl`: paid packets, different-producer local fusion, finite reserves and upkeep | Packet manufacture alone is not a birth; a birth is not maturation or descendant reproduction. |
| Development | `common.wgsl`: fixed age curve, growing capacities; ordinary costs in update kernel | Maximum birth reserves cannot by themselves fund the default maturation interval. Rounding repair does not remove intended dependence. |
| Mutation | `inherit_genomes.wgsl`, `src/brain.rs`: modular recombination and inherited parameter/topology mutation controls | No demonstrated viable chain of partial care strategies yet. |
| Cross-world persistence | `update_reservoir.wgsl`, simulation transitions, `observability.rs`: blind hereditary reservoir and checkpointed lifetime/ecology state | Reservoir admission at birth preserves genomes without demonstrated maturation; it is unchanged pending a separate audit. |
| Presentation | Existing renderer, wallpaper integration, UI, inspector, lenses and save library | No changes in this pass; the preservation contract remains a requirement. |

The inherited diagnostic files also include selected juvenile traces, short
ecology response snapshots and a 64-site long-horizon ecology probe. Their scope
labels matter: selected traces are not an unbiased cohort; frozen forcing is
not a continuous climate history; a site sample is not a connectivity map.
The whole-system audit remains incomplete. The next controlled life-cycle test
can now start from an adult strategy that actually earns its surplus. No larger
random-seed verdict or care-specific tuning is justified by these controls alone.
