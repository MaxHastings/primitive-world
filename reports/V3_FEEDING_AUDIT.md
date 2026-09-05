# V3 feeding and juvenile survival audit — September 4, 2026

Status: diagnostic replay complete;100-round development campaign launched.
No candidate promoted, no broad-goal or intelligence claim. Main and personal
checkpoints are untouched. The new observer is not an agent input or fitness term.

## What changed for this audit

Relative to the frozen six-round pilot source hashes, only family_observer.rs,
observe_families.wgsl and simulation_tests.rs changed in the Rust/WGSL source set.
Controller, physiology, initialization, heredity and resource rules are identical.
Family report schema2 adds read-only per-tick feeding, investment, maturation and
death counters. Paired32-bit accumulation preserves long-run food/energy totals.
The checkpoint/body layout remains15. Trainer changes expose diagnostics and
strengthen resume ancestry-bank validation; selection and curriculum are unchanged.

Verification:58 Rust tests passed,0 failed,1 optional diagnostic ignored. Six
Python trainer tests passed. Observer-isolation fixture gives identical body bytes
with/without observation. Controlled fixtures verify feeding across maturity and
terminal juvenile starvation without stale-death recounting. These are integrity
checks, not proof that every population run is bitwise deterministic.

## Registered diagnostic replay

Local artifacts: reports/v3-feeding-audit-20260904.
Executable SHA256:66831d1c6b202825e96ea2ee6d4fa464fc8ec7bf4b44325a517a0d520538a85a.
Initial and candidate banks come from the completed six-round pilot, with hashes
checked against its prior evaluation. Seed1964496970,1000 founders, full habitat
contrast, regeneration.01, metabolism.06, movement cost.01, gain4;8192-tick cap.
All bank hashes/commands are in registration.json and per-arm command files.
This seed was already observed. It is diagnostic development data, not a holdout.

| Measurement | Initial pool | Six-round candidate |
| --- | ---: | ---: |
| Extinction detected at tick | 2016 | 1632 |
| Food collected, world total | 169.954 | 950.681 |
| Births | 638 | 920 |
| Mean birth energy | 19.13 | 16.39 |
| Born below24 stationary no-food maturity energy | 484 | 904 |
| Juvenile food collected, total | 24.346 | 194.229 |
| Juvenile collection choices / food-present ticks | 2404 / 24356 | 22074 / 28535 |
| Descendants entering maturity alive | 60 | 71 |
| Juvenile starvation deaths | 578 | 848 |
| Adult descendant starvation deaths | 60 | 72 |
| Births to descendant parents | 0 | 1 |

The candidate's food-present collection fraction is77.4%, versus9.9% initially.
Its92.2% juvenile starvation rate remains severe. Most births have too little
energy to mature without feeding even if stationary, but that is a diagnostic
lower bound, not proof they are physically incapable of survival. Food-present
counts use local vegetation before competition, not guaranteed personal access.
The candidate's juvenile collection averages only.211 food per birth (about1.69
energy at conversion8), with an unknown distribution across individual children.
Family-level distributions remain in the raw reports; averages hide outliers.

The single grandchild differs from the original pilot replay's zero. Parallel
mixed-population simulation is not guaranteed bitwise deterministic. This is
evidence that a second generation is possible, not a robust capability. One
descendant died on its maturity tick, explaining72 adult-class deaths versus71
recorded alive maturity entries. Death class uses terminal age, whereas entry
requires being alive after the tick. No descendant deaths were unexplained.

## Interpretation and next experiment

Feeding behavior is not universally absent. The selected pool collects more food,
including as juveniles, but invests less energy per birth and still loses nearly
all offspring before they can reproduce. This is consistent with a short-horizon
investment tradeoff; it does not establish causality or optimal scoring changes.

The authorized [100-round campaign](../training/FEEDING_CAMPAIGN.md) holds biology,
fitness and curriculum fixed. Root seed9042602;4 islands;64 families per island;
8 founder replicas;3 contexts per round; fixed benchmarks every5 rounds; separate
four-case initial/final200k-or-extinction comparison. Artifacts are under
reports/v3-feeding-100-20260904. This report does not anticipate its outcome.

Do not make feeding automatic, reward collection, alter child endowment or change
pressure in mid-campaign. Observe whether more search discovers repeatable family
persistence. If not, retain the failure and identify the measured bottleneck.
