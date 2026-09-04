# Journey-observer development check

September 4, 2026; not final validation or an evolved candidate comparison.

The first8192-tick smoke world on seed808 completed with1284 living bodies,
zero invalid controller/observer outputs,257 samples and zero complete journey
records. That proves startup, sampling and footer output, not an inability to
migrate. It precedes the major relocation window. Raw files:
`reports/journey-smoke-20260904.json` and `.jsonl` (ignored local artifacts).

A follow-up on the same development seed808 is registered here before running:
original released founder bank, original rotating sensors, gain4, costs0.06/0.01,
regeneration0.01,1000 bodies,200k cap or extinction, ordinary metrics every1024
ticks and journey sampling every32. Purpose: locate where observed departure /
crossing / collection / ingestion / reproduction sequences stop. No behavior
change, rescue, bank export, selection or promotion. Records use the explicit
development definitions in `experiments/JOURNEY_OBSERVER.md`, not the final
major-relocation acceptance criterion.

## Completed follow-up

Extinction sampled at98,304 ticks;31,472 births, zero invalid outputs. Runtime
115.21seconds. The new observer made3,073 observations, established164,347 source
anchors, observed246 depleted departures and47 qualifying poor-space crossings,
but zero qualifying destination collections, ingestions or completed journeys.
Zero invalid observations or track truncations. Anchors are repeated track
episodes, not unique bodies, so these are not population transition rates.

This contradicts the categorical interpretation that no agent ever leaves.
It does not establish that no agent ever finds another patch:32-tick sampling
and strict footprint/event definitions can miss successes. The next diagnostic
must retain failed departures' trajectories and remaining energy, not just
completed journeys, to distinguish circling, exhausted travel budgets and missed
food encounters. Do not call this migration acceptance evidence.

Frozen executable SHA256:
`003ac4aee14db666b0de819eed131eecf55e1ead0bcebc581d4f181bdd3b97bc`.
Report `reports/journey-development-20260904.json`, SHA256:
`726ff032012fb8dd7977fc447172f1c634508ef9cd394eb8436f3a999141c921`.
JSONL SHA256:
`8618cdffd0cae54bc721438aa07d9c9e21e1d6c2bec6159f9a6e69b8de63949e`.
All artifacts remain local and ignored. No bank exported or promoted.

## Verification note

One34-test parallel run failed during pipeline creation with Naga24 HLSL
`Unimplemented(write_value_type Struct)` for a Sample struct; the isolated test
passed afterward. Replacing the perception shader's Sample-valued array assignment
with equivalent field stores avoids that struct temporary. Three consecutive
full34-test runs passed after the change. This is a compatibility workaround for
an observed compiler failure, not proof every shader/compiler failure is solved.
The rotating-sensor semantics and GPU direction diagnostic are unchanged.
