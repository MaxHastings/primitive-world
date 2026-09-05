# Random-origin founding ecology: 24/24 completed, no established population

Protocol: [FOUNDING_ECOLOGY_PLAN.md](../training/FOUNDING_ECOLOGY_PLAN.md).
Authoritative artifacts: v3-founding-ecology-20260904-validated/registration.json,
summary.json, per-case reports, commands, banks, logs and full checkpoints.

Eight random genome banks were each tested in three matched-seed conditions.
No family fitness was computed, no external parent selection or breeding ran,
and no genomes moved between worlds. Ordinary reproduction and mutation inside
worlds were unchanged. Geography stayed fixed; weather, regrowth and depletion
continued. This is founding evidence, not a demonstration of trained adaptation.

## Outcomes

| Condition | Worlds alive at 16384 | Mean extinction tick | Mean offspring reaching adulthood | Mean food collected |
| --- | ---: | ---: | ---: | ---: |
| Patchy baseline, regeneration .01 | 0/8 | 2300 | 53.875 | 251.308 |
| Uniform habitat, regeneration .01 | 0/8 | 2732 | 91.375 | 1031.968 |
| Patchy habitat, regeneration .03 | 0/8 | 2768 | 54.125 | 268.750 |

All cases ended naturally in extinction before3745 ticks, well below16384.
All recorded invalid-output counts and capacity exposure were zero. Only one
grandchild birth occurred across all24 worlds (seed9043101 baseline); every
other case reached only first-generation offspring. No great-grandchildren.
There is no viable endpoint population to continue.

### Paired extinction times

| Seed | Baseline | Uniform | Higher regeneration |
| --- | ---: | ---: | ---: |
| 9043101 | 2368 | 3168 | 3008 |
| 9043102 | 2048 | 3488 | 2432 |
| 9043103 | 2080 | 2848 | 1888 |
| 9043104 | 1984 | 2272 | 1920 |
| 9043105 | 1760 | 2656 | 2080 |
| 9043106 | 1888 | 2016 | 3424 |
| 9043107 | 3264 | 2528 | 3744 |
| 9043108 | 3008 | 2880 | 3648 |

Each treatment outlasted baseline on6/8 seeds, not universally. There is one
GPU realization per case; contention can vary trajectories. Differences between
conditions are environmental effects, not evidence of inherited improvement.

## Food and offspring energy

The following fractions/rates pool actual processed juvenile ticks across the
eight worlds per condition (they are not unweighted means of percentages).

| Condition | Juvenile ticks with food underfoot | Choose collect on those ticks | Digested energy per juvenile tick | Mean energy at birth |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 20.59% | 11.50% | .001704 | 19.169 |
| Uniform | 79.62% | 11.16% | .005366 | 19.186 |
| Higher regeneration | 21.44% | 13.10% | .001916 | 19.210 |

Basic metabolism alone costs .06 energy per tick. Even uniform-world juvenile
digestion averaged only8.94% of that rate. Starting birth energy bridges the
shortfall temporarily. About77% of births in each condition received less than
24 energy, the stationary no-feeding cost of reaching maturity at400 ticks.
Being below24 is not inherently impossible: a child can eat before maturity.
These children mostly failed to obtain enough additional energy.

Collected food excludes the2 food units supplied to each founder. Juvenile
digestion excludes founders' direct consumption, but food transferred from a
founder could still enter a juvenile; it is not an independent resource-origin
tracer. Rates are aggregate observations, not per-individual causal estimates.
Food presence means at least .001 local food, NOT a full meal or exclusive access.
Uniform habitat preserves its spatial mean, not realized calories per agent.
The conditional collection rate alone does not establish intelligence or intent.

## What this tells us

The tested environmental changes improved food encounters and some survival,
but did not establish a self-replacing random-origin population. Uniform habitat
produced roughly4.1x more collection than baseline, without a single grandchild.
Access alone, as manipulated here, was not sufficient. Feeding throughput,
collection decisions and reproductive provisioning remain plausible bottlenecks;
the experiment does not isolate which intervention would solve them.

The next diagnostic should account for requested/available/received food and
energy flows around births, rather than declaring that harder pressure, longer
runs, more regeneration, a new brain, or a different winner score must solve it.
No additional parameter changes or successor experiments were launched.

## Integrity and preservation

The exact prior V3 executable was reused (SHA256
66831d1c6b202825e96ea2ee6d4fa464fc8ec7bf4b44325a517a0d520538a85a).
All registered runtime source hashes matched and were archived. The only shared
Python validator changes allow an explicitly static landscape and suppress
fitness computation; its existing trainer behavior remains the default.

17 Python tests passed, including paired-condition constraints, body-rule flags,
observer-only validation, precision-sensitive checkpoint comparison, and error
handling. Every completed case passed population/descendant accounting, fixed
settings and bank checks, and checkpoint header/settings/hash validation.
All24 endpoint checkpoints are kept (3570117282 bytes total). A zero-tick
headless load of seed0-uniform.checkpoint also succeeded; no saved world was
advanced or overwritten. The simulations reported72.18 total headless wall-seconds,
including final readback/checkpoint writing but excluding process startup,
initial setup, Python orchestration and subsequent verification.

An earlier preflight in v3-founding-ecology-20260904 ran seed0-baseline once,
then rejected equivalent JSON float formatting in the checkpoint validator.
Its original runner and artifacts are untouched. The corrected suite repeats
the same predefined case; the protocol documents this before the full suite.
The repeated outcome differed (2464 versus2368 extinction ticks), consistent with
the simulator's stated non-bitwise-deterministic GPU contention. No seed or
condition was chosen in response to the preflight outcome.

The retired family-scoring and authored-starter campaigns remain stopped.
The old scheduled follow-up remains paused. No promotion or merge to main.
