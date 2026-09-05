# Departure attrition and feeding-prior audit

September 4, 2026. Development evidence, not completion of the goal.

## What the longer trace showed

The registered seed808 schema2 trace ended in extinction at196,608 ticks with
63,055 births and zero invalid outputs. It retained800 ended departure attempts:
395 matched dead bodies, all with zero energy;405 missing/reused identities whose
death cause is not established. No completed journeys;238 attempts crossed the
poor-space threshold,2 destination-collection milestones and1 ingestion milestone.
Milestones can reset and are not counts of successful migrants.

For the237 ended attempts whose last retained stage was poor crossing:

| Measurement | Median |
| --- | ---: |
| Energy plus convertible inventory at departure | 29.15 |
| Remaining lifetime | 7,906 ticks |
| Subsequent path speed | 0.193 units/tick |
| Net displacement / voluntary path | 0.989 |
| Nearest qualifying food footprint at departure | 126.12 units |
| Optimistic no-new-food range at max body speed | 485.88 units |
| Illustrative range at subsequent observed speed | 85.19 units |

Only5/237 had the nearest qualifying destination beyond the optimistic maximum
speed range;171/237 had it beyond the illustrative observed-speed range. These
are per-attempt comparisons, not comparisons of medians. They exclude unfinished
stages outside this subset and do not establish discoverability or durable food.

The observed poor-space travelers mostly made straight progress, not circles.
Their actual pace and depleted departure reserves usually gave them much less
range than the body's maximum permits. This argues against explaining everything
as a hard speed cap or insufficient maximum lifetime. It does not prove changing
motor gain solves adaptation, and not every agent is represented in this subset.

## Important uncertainty update

The same seed/settings previously died near98k in multiple runs. This trace
lasted196k without any intended behavioral change. GPU resource competition and
subsequent evolution are not bitwise reproducible across runs; added readback and
an equivalent shader-expression change alter execution timing. We have not
isolated the contribution of those changes. Therefore, single-run survival-time
differences in the earlier sensor ablation do **not** establish a reliable effect
size. The controlled GPU sensor-frame defect is still reproduced directly.

## A separate, directly reproduced feeding defect in the starting policy

`released_bank_empty_inventory_feeding_diagnostic` dispatches the production
decision shader for all128 released genomes, empty inventory, uniform0.2 food
underfoot and at correctly offset sensors, zero previous state, no neighbors.

| Energy | Collect | Ingest carried food |
| --- | ---: | ---: |
| 10 | 0 | 128 |
| 30 | 94 | 34 |
| 50 | 128 | 0 |
| 80 | 128 | 0 |

The controller chooses the impossible carried-food action precisely when most
hungry. The body correctly does nothing with an empty inventory. This is a flaw
in the authored/retained starting disposition, not a missing global route input
or corrupted energy calculation. This isolated probe does not establish the
same action choice for every evolved descendant in a moving environment.

## Next model decision

Test a smaller, versioned body/controller contract: gathering remains a choice,
but rate-limited conversion of food already held in the body becomes automatic
physiology, like metabolism. Remove the ingest decision rather than add a hidden
action fallback or hand-written hunger policy. Also remove sensor rotation and
its actuator/input, giving the eight point probes and movement one fixed compass
frame. Keep energy/food conservation, finite capacity, movement costs, actual
birth selection and controller-owned memory. Do not add migration destinations,
departure timers, rewards or population rescue. Evaluate this separately from
whether preparation improves inherited behavior.

## Artifacts

`reports/departure-attrition-20260904/` retains frozen `world.exe`, `world.json`,
15MB `attempts.jsonl`, and the analysis produced by
`experiments/analyze_departures.py`. The committed plan is
`experiments/DEPARTURE_ATTRITION_PLAN.md`.

Executable SHA256: `a72f7a2acb43895dea79ae4d96969e642dc9d2741eed21c40f537e933a1ef020`.
World report SHA256: `c7f8847074a7e866c7aee71672711e0191d6cb09b6384ca92e923c54204d3492`.
Analysis SHA256: `999577e23f780aedb7058829ed3af6b8afeef79d0856d257c05c158a73365e97`.
