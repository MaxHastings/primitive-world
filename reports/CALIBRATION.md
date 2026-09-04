# Ecology calibration — 2026-09-04

**Superseded movement behavior:** these population results were collected before the destination-pacing bug was found. They do not establish competent foraging. See [movement regression](MOTION_FIX.md) for the diagnosis and correction. Defaults are unchanged, but the corrected movement changes population trajectories.

## Scope

Regeneration now defaults to 0.025 instead of 0.1. Reproduction, old age, starvation, weather, force, social weights, and the population capacity remain in place. There is no artificial population floor or ceiling correction.

The decision changes are:

- Harvest utility uses the fraction of an attainable harvest, so a patch containing one complete harvest is useful even if it is visually sparse.
- Eat utility scales with the available bite, so a crumb does not receive the utility of a full meal. Emergency eating still interrupts travel.
- Continuing an existing trip competes with new destinations; it no longer overwrites the destination selected by a higher score.

The UI adds reserve and hunger counts, plus sufficient decimal precision for regeneration. Headless reports add initial settings, intervention timing, stocked/hungry/moving/eating counts, and cumulative vegetation harvesting. Restoration uses the configured regeneration value rather than a hardcoded 0.1.

## Matched food supply comparison

NVIDIA RTX 4070 SUPER, Vulkan. Initial population 1,000; seed 1; regeneration 0.025 for both builds; vegetation removed and growth stopped at tick 6,000, then vegetation replenished and growth restored at tick 8,000. Rows at intervention times show state immediately before applying that intervention.

| Observation | Previous decisions | Revised decisions |
| --- | ---: | ---: |
| Population at tick 6,000 | 14,475 | 4,734 |
| Carried food per agent at tick 6,000 | 0.04 | 1.56 |
| Agents carrying at least 1.5 food at tick 6,000 | 82 | 1,559 |
| Learned helping ties within sensory range at tick 6,000 | 7 | 52 |
| Population at tick 8,000 | 334 | 1,074 |
| Population at tick 16,000 | 17,228 | 8,400 |
| Cumulative gifts at tick 16,000 | 5,388 | 5,040 |
| Cumulative force incidents at tick 16,000 | 798 | 21,563 |

The revision supports greater carried reserves and survival through this shock, but force increases substantially. Different population sizes, lifetimes, and food holdings change interaction opportunities; these totals do not isolate aggression or cooperation rates.

Raw reports: [before](calibration-before-seed1.json), [after](calibration-after-seed1.json).

## Recovery and longer run

With revised decisions, seed 2 fell from 5,667 at tick 6,000 to 1,021 at tick 8,000 and reached 8,738 at tick 16,000. [Report](calibration-after-seed2.json).

A seed-2 run with concern, reciprocity, social steering, and reports disabled fell from 4,696 to 927 through the same shock and recovered to 7,369. It recorded no gifts and 42,100 force incidents, compared with 5,263 gifts and 28,493 force incidents with those social features enabled. Both recovered. This broad ablation changes several mechanisms and population trajectories at once; it does not establish that a particular relationship caused survival. [Control report](calibration-no-social-seed2.json).

An undisturbed seed-1 run sampled every 2,000 ticks stayed between 4,446 and 6,943 agents from tick 4,000 through 24,000. Average carried food at those samples ranged from 1.12 to 1.69 units. It ended with 4,895 living, 41,076 cumulative births, 12,552 gifts, and 79 nearby helping ties. This covers more than two initial maximum lifespans. [Report](calibration-natural-seed1.json).

These are sampled observations, not exact extrema or evidence of permanent equilibrium. Parallel atomic ordering permits divergent population histories even for a repeated seed. The open visual application may share GPU time; throughput numbers are not isolated performance benchmarks.

## Verification and remaining gaps

All nine GPU tests passed. New checks exercise gathering instead of eating a crumb, changing direction toward a better observed patch, and a gift selected by the complete decision pipeline. Existing checks cover conservation, birth costs, recovery, remembered travel, local relationship effects, reports, force, contention, batched clocks, and checkpoint replay. Cargo check and diff whitespace checks passed.

The dashboard's helping ties require familiarity at least 0.2 and learned food benefit at least 0.1. The counts exclude navigation-only ties; proximity means sensory range, not exchange range. Cumulative harvested food counts vegetation only and is settled in the next resource pass. It is not a complete energy conservation ledger.

Persistent social groups are not established by these tests. Only two seeds received famine trials, the pre/post comparison uses one seed, and identity-specific survival benefits need stronger controlled experiments. Giving and local remembered benefit work, but ties remain sparse and force needs further observation. Four compass food samples also impose visible directional bias; straight trails should not be described as emergent roads.
