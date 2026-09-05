# Actual-survivor loop: first bounded pilot

2026-09-05. Completed, unpromoted. The feedback loop works and there is a useful
survival signal, but it has NOT produced a demonstrated self-sustaining population.

Run: `v3-survivor-loop-20260905`. Four independent random-origin lines, eight
transfers each: 32 training worlds plus 24 separate evaluation worlds. All56
completed. The [frozen protocol](../training/SURVIVOR_LOOP_PLAN.md) used unchanged
normal-world physics, distinct training seeds, and the same two separate evaluation
seeds at rounds0/4/8. Evaluations never supplied selection genomes.

## Matched evaluation results

Each row averages eight worlds (four lines × two seeds). Survival is capped at8192;
these are mean observed survival times, NOT uncensored mean extinction times.

| Transfer round | Mean observed survival | Worlds alive at8192 | Mean food collected | Mean births | Mean matured descendants |
| --- | ---: | ---: | ---: | ---: | ---: |
| 0, random origin | 2400 | 0/8 | 293.43 | 653.875 | 55.0 |
| 4 | 4504 | 2/8 | 1499.98 | 130.75 | 40.875 |
| 8 | 4680 | 2/8 | 1536.56 | 129.375 | 39.875 |

Observed survival increased95%, and total collection increased5.24×. Collection
totals also reflect longer lives; these are not normalized feeding-efficiency
measurements. Births and the absolute number of matured descendants fell.
Births to descendant parents totaled0,9,5 across the eight evaluations at the
three stages respectively: some later reproduction, not continuing replacement.

No cherry-picked winner: mean observed survival for every line:

| Line | Random origin | Round4 | Round8 |
| --- | ---: | ---: | ---: |
| 0 | 2592 | 8192 | 8192 |
| 1 | 2224 | 2448 | 2480 |
| 2 | 2448 | 5136 | 5760 |
| 3 | 2336 | 2240 | 2288 |

The two final worlds still alive each contained **one original founder**, not a
continuing population of descendants (living ancestry depth0). Their energies
were99.932 and72.661. Line1 had zero births in both midpoint and final evaluations.
Longer-lived, better-fed bodies and species persistence are different outcomes.

## What actually crossed world boundaries

The recorder retained up to64 bodies at the newest nonempty128-tick observation.
It copied current slot genomes, including mutations, rather than substituting
original ancestors ranked by family performance. Extinction did not clear the sample.

Across32 transfers,56 bodies were sampled, including8 actual descendants in7
transfers. Twenty-two transfers retained only one body. The largest sample held6.
This was therefore a severe one-to-few-body bottleneck, not a diverse64-survivor
population. Repeated founder carryover is real late survival, not an implementation
reversion to ancestral-family scoring; ancestry metadata makes that visible.

Each sample contributed one exact copy per body, then balanced mutated replicas
filled256 genomes. New bodies received ordinary initial reserves/age/hidden state.
External mutation used .02 probability/weight and uniform±.03, clipped to±4.
All transfers record source file hashes, per-body f32 genome hashes, current
ancestry, parent identities for replicas, and counts of changed weights.

## What we learned—and what we did not

The cross-world feedback loop can retain variants that survive longer and acquire
more food than its random origin on these matched worlds. That is useful evidence;
the earlier independent-world ecology diagnostic could not answer this question.

But terminal survival plus external rejuvenation need not favor reproduction.
The observed loss of births and severe bottlenecks are consistent with that
selection tradeoff; this pilot does not isolate each cause. We should not fix the
interpretation by relabeling a long-lived sterile body an intelligent species.

Eight transfers and two evaluation seeds do not establish broad generalization,
long-run learning saturation, or migration across relocating resources. The8192
horizon ends before major habitat relocation and before maximum founder lifespan.
The next evidence gap is persistence after original founders age out, alongside
whether terminal bottlenecks are discarding productive lineages—not a demonstrated
need for more senses, an authored eating policy, or different energy costs.

## Verification and handoff

- Rust:60 passed,0 failed,1 pre-existing opt-in test ignored. Targeted GPU test
  confirmed a real newborn mutation was exported exactly and survived extinction.
- Python:23 passed. Checked exact carryover, mutation provenance/bounds, balanced
  replication, one-body bottlenecks, empty-bank refusal, changed-bank rejection.
- Formatting, Clippy with warnings denied, and git diff whitespace checks passed.
- All56 reports passed fixed-physics, input-bank, population-accounting and
  zero-invalid-output checks. No sampled near-capacity population.
- Frozen completed resume verified all reports/receipts and regenerated all32
  next banks identically without rerunning a world; final summary hash unchanged.
- Runtime SHA256: `9f29365bef56036d33f0669d3554c8aa1c70b15be54e33ac5ea34676ecc5a650`.

The pilot stopped normally. No background successor, default promotion, main
merge, GUI manipulation, save overwrite or deletion. Previous campaigns and user
checkpoints remain intact. Final banks are available for inspection, not endorsed
as validated intelligent agents. The run's `summary.json` separates training from
evaluation and retains all four lines.
