# Random-origin founding ecology: 8 paired seeds x 3 conditions

Question: can random V3 lineages establish continued reproduction without an
external genome-ranking loop, and does changing food access help?

This is a bounded experiment, not a training campaign or a release validation.
No score selects parents, no action earns a reward, and no genome is transferred
between worlds. Only ordinary births inherit and mutate the actual parent's
weights. Extinction ends a world; it does not trigger revival or replacement.

## Registered before execution

- Eight world seeds: 9043101 through 9043108.
- Eight random-bank seeds: 9043201 through 9043208, paired by index.
- Each bank has 256 independent random genomes using prepare.random_genome.
  No authored initializer or previously trained bank. The same exact bank seeds
  all three conditions for its world seed, with 1000 bodies cycling through it.
- Same seed/rotation within each triple; rotations are index modulo 4. Execute
  the three condition labels in cyclic order by seed index to vary run order.
- All cases: 16384 ticks or extinction, sample every 256 ticks, family observer
  enabled ONLY for diagnostics, --static-landscape, full endpoint checkpoint.
- The simulator executable is frozen from v3-feeding-100-20260904 and must match
  its registration SHA256. Its source files are also checked/copied from the
  working tree against that registration, allowing historical provenance without
  a new runtime build. No changes to Rust, shaders or biological constants.

| Condition | Habitat contrast | Regeneration |
| --- | ---: | ---: |
| baseline | 1.0 | .01 |
| uniform | 0.0 | .01 |
| higher_regeneration | 1.0 | .03 |

Threefold regeneration is a disclosed diagnostic setting, not a claim of optimal
difficulty. Uniform contrast preserves mean habitat, not necessarily realized
food stock or carrying capacity. Static landscape freezes geography, NOT weather,
seasonality, soil, depletion or regrowth. No further parameter tuning mid-suite.

All other settings remain normal V3: metabolism .06; movement .01; maximum speed
1.2; motor gain 4; collection maximum .025 food/tick; digestion 8 energy/food;
founders 65 energy and 2 food; offspring energy 40*amount, no food; construction
cost 10; maturity 400; recovery 240. Force and signals remain enabled.

## Measurements, not selection criteria

Record extinction time, population history, births, matured descendants, births
to descendant parents, maximum ancestry depth, family survival, capacity exposure,
food collected/digested, juvenile food encounters and collection decisions,
mean birth energy and energy at maturity, energy among living agents over time.
Juvenile digestion per processed juvenile tick excludes founders' supplied food.
Compare that energy-income rate to basic metabolic expenditure .06. This is not
a full individual energy balance: movement, interactions, investment, selection
of survivors and initial energy need separate accounting. Do not infer causal
food-seeking just from frequent collection choices.

Endpoint survivors at 16384 ticks must be descendants: founders live at most
11000 ticks. Report this separately from a stronger founding indicator requiring
living adults, depth >=3 and births to descendant parents. The indicator is only
a readout; it never changes a world. A passing endpoint is not proof of indefinite
stability, adaptation, communication utility or migration. Capacity-limited cases
are flagged, not silently counted as unrestricted ecological success.

Compare paired seeds across conditions; do not treat different environments as
evidence of genetic improvement. Retain failures and per-seed detail alongside
condition summaries. Eight seeds are development evidence, not final holdouts.

## Preservation and scope

Full endpoint checkpoints (even extinct ones), raw reports, exact commands, logs,
banks, source/protocol/runner snapshots and hashes are preserved. A living world
can later continue from its actual state rather than restarting from a founder
bank with fresh energy. No continuation, difficulty escalation, default promotion
or automatic resume of retired campaigns is part of this initial suite.

Output paths are new. A failed/interrupted case stops the runner. Resume verifies
all frozen files and completed results; a partial raw report/checkpoint is never
deleted or overwritten. Corruption requires inspection rather than silent retry.
Only one runner may hold the directory lock. Summary replacement is atomic.

## Engineering preflight disclosure

The first registration in reports/v3-founding-ecology-20260904 executed only
seed0-baseline, ending in extinction at tick2464. Its runner then rejected a
checkpoint/report comparison because the two serializers print float32 settings
with different decimal precision. The values, including all genes, matched at
float32 precision. All of that registration and its outputs are preserved.

The corrected runner compares settings at exact float32 precision, not a loose
tolerance. The full suite uses a new directory ending in -validated. It repeats
the preflight case transparently; all seed choices, banks, conditions, biological
rules and readouts remain unchanged. The earlier observation is development
evidence and was not used to choose a condition, seed or genome.
