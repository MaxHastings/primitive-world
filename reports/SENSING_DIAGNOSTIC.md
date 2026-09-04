# Matched sensing diagnosis — 2026-09-04

## Question and intervention

Does changing sparse local sensor coverage, without changing the controller's
valuation rules, improve food access or discovery in the two-patch fixture?

Three treatments keep underfoot sensing and exactly four remote food/crowd
samples per tick:

- **baseline:** cardinal points at radius 24 (unchanged ordinary behavior).
- **near:** cardinal points at radius 4.
- **sweep:** radii 4, 12, 24 in cardinal directions followed by the same radii
  in diagonal directions, repeating every six ticks.

The sweep is not simultaneous disk coverage. Changing sample placement also
changes available movement proposals and the locations that memory observes;
that is necessary to attach measurements to truthful coordinates. Candidate
score formulas, commitment, memory capacity/retention, weights, RNG draws,
energy costs and resource rules are unchanged. These remain diagnostic-only
treatments, not new application defaults or new founder preparation.

## Results

All **48 runs completed**. No body survived to 3,000 ticks, observed food in B,
or arrived at B. Across each of the 16 matched conditions, all three sensing
modes produced exactly the same total food collection, path length and action
counts. Retaining versus erasing place memory also gave the same values here.
Mean collected food was 2.545 per run (range 1.500–4.171).

Each row below therefore represents all three sensing modes and both memory
conditions. Values are rounded only for display.

| Seed / genome | Regeneration | Death tick | Food collected | Path length |
| --- | ---: | ---: | ---: | ---: |
| 7 / 0 | 0.002 | 1309 | 1.500 | 1452.008 |
| 19 / 1 | 0.002 | 1344 | 1.800 | 1480.790 |
| 31 / 2 | 0.002 | 1623 | 3.905 | 1486.785 |
| 43 / 3 | 0.002 | 1308 | 1.500 | 1455.597 |
| 7 / 0 | 0.020 | 1309 | 1.500 | 1452.008 |
| 19 / 1 | 0.020 | 1347 | 1.824 | 1480.790 |
| 31 / 2 | 0.020 | 1658 | 4.161 | 1482.003 |
| 43 / 3 | 0.020 | 1627 | 4.171 | 1675.198 |

All eight overlapping baseline conditions also exactly reproduced the earlier
travel batch's collection, path length and action counts. This is a negative
result for these sensor interventions, not support for promoting them as a fix.

## Reproduction

```powershell
python experiments/travel.py --directory reports/sensing-baseline --sensing baseline --distances 300 --regenerations 0.002 0.02 --seeds 7 19 31 43 --modes discovery --ticks 3000
python experiments/travel.py --directory reports/sensing-near --sensing near --distances 300 --regenerations 0.002 0.02 --seeds 7 19 31 43 --modes discovery --ticks 3000
python experiments/travel.py --directory reports/sensing-sweep --sensing sweep --distances 300 --regenerations 0.002 0.02 --seeds 7 19 31 43 --modes discovery --ticks 3000
```

Use new output directories when repeating these commands. Each mode contains
16 conditions: four seed/genome pairs (7/0, 19/1, 31/2, 43/3), two regeneration
rates and place memory retained/erased. There are only **four distinct founder
genomes and four paired seeds**, not 48 independent samples. The isolated
body has reproduction suppressed; this is not a population fitness experiment.

Executable SHA-256 for all three batches:
`50206d4156072b6617505f881d093ecee85e6bae69b393341eabdf464742bfa7`.
Raw per-tick-derived reports and incremental batch summaries remain local under
the three named directories; summaries include exact commands and the hash.

## Why more visible food can still be ignored

A focused GPU characterization uses bootstrap weights, energy 81, empty
inventory, no active journey, and one observed food cell four units east.
It compares 0.3 with 0.9 food without changing any physical or policy settings.

For 0.3 food, hunger is 0.19 and urgency is 0.433. The authored local destination
score is `0.3 * 0.433 - 0.08 = 0.0499`. The exploratory score is
`0.10 + 0.20 * 0.19 - 0.06 = 0.078`. The controller therefore proposes a blind
48-unit exploration trip instead of the observed four-unit food destination.
At 0.9 food the local score is 0.3097 and the observed destination wins.

These numbers are authored utility scores, **not energy or measured expected
lifetime returns**. The local score deducts the same 0.08 at radius 4 and 24;
the exploration score does not explicitly compare travel distance or expected
harvest yield. After a trip starts, the existing commitment rule can also
retain that destination over modest local alternatives.

The seven inherited action rows evaluate only the winning movement proposal.
Some earlier genome traits modulate urgency, exploration and commitment, so
evolution can influence this indirectly; it cannot freely rank all destinations
using those action rows. This is a representational restriction, not proof that
evolution is broken or that no genome could forage successfully.

## Decision

Do not promote new sensing as a demonstrated fix, or add population targets,
extra food, migration rewards or forced routes. The next controller experiment
should expose multiple truthful local/remembered/exploratory destinations to
the inherited policy, with comparable distance, confidence and food features.
Keep the current controller as a baseline and validate separately:

1. usable information and physical feasibility;
2. destination ranking and sustained execution;
3. descendant success across preparation and held-out worlds;
4. individual repeat travel, only where travel actually pays.

This experiment does not rule out better simultaneous coverage, different
genomes, better memory or different landscapes. A null result here does not
establish that sensing never matters. Nor should organized travel be required
when staying at one patch is viable.

## Verification

- Full headless suite after geometry changes: 24 passed, 0 failed, 1 explicitly
  ignored legacy motion diagnostic. Two subsequently added characterization/
  checkpoint tests passed separately (26 passing tests in total).
- GPU geometry test checks all six phases in all three modes against uniquely
  painted food cells, including the actual movement and refreshed memory
  coordinates. A rich-food positive control confirms local targeting works.
- Focused GPU valuation test confirms the 0.3-versus-0.9 comparison above.
- Sweep gives byte-identical agent/perception results in batched versus
  single-tick stepping; checkpoint replay preserves the mode and sweep phase.
- Existing conservation, founder, neural, checkpoint and locality tests pass.
- `cargo fmt --check`, separate controller-test formatting, `cargo check
  --all-targets`, Python compilation and Git whitespace checks pass.
- Ordinary release smoke, seed 505 at 1,000 ticks: 1,053 living, 53 births,
  17 force interactions. Population atomic ordering can vary outcomes; this is
  a startup/regression smoke, not a new long-term fitness validation.
- No founder weights, ordinary settings, checkpoint layouts or physics changed.
  No application window was opened, controlled, rebuilt in-place or restarted.
