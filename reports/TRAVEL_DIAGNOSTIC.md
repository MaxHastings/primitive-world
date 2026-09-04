# Travel diagnosis — 2026-09-04

## Outcome

The current controller can physically complete a journey to a sufficiently valued
remembered target. It does not yet demonstrate reliable discovery or repeated
inter-patch returns in this small diagnostic. Survival in ordinary population
experiments must not be interpreted as evidence of navigation competence.

No policy weights, sensing shader, navigation heuristic, physical costs, or
ordinary-world defaults were changed in this diagnostic revision.

## Evidence

### Modest-patch baseline

`python experiments/travel.py --directory reports/travel-baseline --ticks 3000`

32 conditions: two distances (120/300), two regeneration rates (0.002/0.02),
two seed/genome pairs (7/0, 19/1), discovery/known-target, and memory/erasure.
Patch radius 16, initial food 0.3 per cell, 52 cells per patch. A single mature
agent starts with 81 energy-equivalent reserves. See experiments/README.md for
the deliberate trial interventions and metric definitions.

**0 arrivals at B, 0 actual observations of food in B, and 0 survivors at the
3,000-tick limit.** All died earlier. Within each paired condition, erasing place
memory produced the same recorded aggregate outcomes. The known-target memory
was present but did not result in a journey to B. These are only two distinct
seed/genome samples, not a statistical estimate of all founders or landscapes.

For discovery seed 7, distance 120, regeneration 0.002, the agent died at tick
1,309 after collecting 1.5 food, travelling 1,452 units and changing goals 32
times. Only two changes occurred after a commitment deadline, including initial
goal establishment: this is not evidence that deadline expiry explains most
direction changes. Completed short exploration legs also change destinations.

A replay with final resource readbacks found **14.766 food left in A and 16.386
in B**. Initial stock was 15.6 per patch; regeneration added 0.666 and 0.786.
The arithmetic matches: 15.6 + 0.666 - 1.5 = 14.766. This individual failed to
access existing food rather than exhausting aggregate world supply.

### Rich remembered target: positive control

`python experiments/travel.py --directory reports/travel-positive --ticks 1600 --distances 120 300 --regenerations 0.02 --seeds 7 --modes known-target --food 1`

| Distance | Place memory | First arrival at B | Food collected | Alive at 1,600 |
| --- | --- | ---: | ---: | --- |
| 120 | retained | 103 | 9.408 | yes |
| 120 | erased | never | 0.500 | no |
| 300 | retained | 257 | 10.907 | yes |
| 300 | erased | never | 0.500 | no |

Neither retained-memory case returned to A by the endpoint. This is evidence of
memory-dependent arrival, not commuting. The positive control changes initial
food as well as supplied memory value, so it is not a matched test of valuation
alone. Ordinary resource capacity/weather rules still act on the initially
painted food; the fixture does not keep the target artificially full.

### Wider patch: staying can be viable

`python experiments/travel.py --directory reports/travel-wide --ticks 3000 --distances 300 --regenerations 0.02 --seeds 7 --modes discovery --radius 48`

Both runs survived 3,000 ticks without discovering or visiting B. With memory,
the agent collected 28.494 food and travelled 774 units; with place erasure,
28.655 food and 1,135 units. Staying around a sufficiently productive patch can
sustain the body without an inter-patch route. Increasing radius also increases
total food and productivity: this comparison does **not** isolate sensing from
resource abundance.

## Diagnosis and next change

**Follow-up:** the matched sensor-geometry experiment and destination-score
characterization are recorded in [SENSING_DIAGNOSTIC.md](SENSING_DIAGNOSTIC.md).
The hypotheses below describe what motivated that experiment, not proof that
changing sensing alone fixes travel.

1. **Destination choice is still an authored bottleneck.** `decide.wgsl` chooses
   one movement destination before evaluating inherited action weights. Fresh
   moderate-food memories lose to short exploration in the tested starting
   states. More survival selection cannot freely replace this destination rule.
2. **Local sensing is sparse, not a filled disk.** `perceive.wgsl` samples the
   body cell and four compass points at sensory radius 24. From the center of a
   radius-16 patch, every remote sample is outside the patch. Depleting the body
   cell can therefore leave nearby unobserved food. The code establishes this
   blind spot; a matched sensing intervention is still needed to measure its
   causal contribution to the failures.
3. **Fixed anchors are not learned patch yield.** Memories store observed cell
   food, not measured collection rate, patch extent or recovery time. A stored
   coordinate alone cannot tell the controller how profitable a visit would be.

The next minimal experiment should expose bounded nearby food samples **with
their actual coordinates** as movement candidates, then compare candidate
destinations using local observations instead of preselecting one with the old
rule. Repeat these same controls before changing ecology or adding social
following. Preserve a baseline/ablation path. Do not fabricate distant knowledge,
guarantee loops, or award fitness for migration.

## Reproducibility and checks

- Baseline executable SHA-256:
  `e1e28825dd3b3dabe120bc49a626f4654b822127d26fc7d3335f2c631e5fa50a`.
- Positive, wider-patch, and resource-audit executable SHA-256:
  `fe9ca522f2dbbd2ae149c6e0faf4b3be4dc67123f500c22c7f068732183081aa`.
  The later diagnostic adds radius selection and final resource accounting;
  candidate behavior is unchanged. The resource audit exactly reproduced the
  baseline individual's path, collection and death tick.
- Local raw outputs: `reports/travel-baseline`, `reports/travel-positive`,
  `reports/travel-wide`. Each batch summary records exact commands and its
  executable hash. Raw JSON remains ignored by Git; this record is durable.
- Headless suite: 23 passed, 0 failed, 1 intentionally ignored legacy motion
  diagnostic. New tests cover observer trip accounting, real GPU harvesting,
  resource-layout initialization, and refusal to overwrite existing evidence.
- A final ordinary-world release smoke (seed 505, 1,000 ticks) finished with
  1,051 living, 51 births and 17 force interactions. This checks that normal
  startup remains functional; population-level GPU runs are not promised to be
  bitwise deterministic.
- The user's existing application window was not controlled or restarted.
