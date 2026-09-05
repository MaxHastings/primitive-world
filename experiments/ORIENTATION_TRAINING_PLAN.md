# Equal-budget orientation experience trial — development, not final validation

Question: does broader directional experience improve adaptability more than an
equal amount of ordinary continued preparation? No new brain, within-life weight
updates, language semantics, navigation reward or forced travel. Only the
orientation of whole training environments differs between arms.

## Frozen comparison

Both arms start from the completed16-world bank, SHA256
`89790e952e2e91fc1b5af0c0173e95724ecda7fd767fec840af0fedd1f995fc2`.
Each receives four65,536-tick worlds (262,144 additional preparation ticks),
using seeds1201,1202,1203,1204 in that order. These are training seeds from now on.

- Ordinary continuation: environment quarter turns0,0,0,0.
- Varied orientation: quarter turns1,2,3,0 (90°,180°,270°,0°).

The same map, resource renewal timeline, weather and initial body placement are
rotated together. Brain weights, world-aligned sensors/actions, body reserves,
age distribution, costs, mutation, reproduction and export selection are not
rotated or changed. Thus the manipulation is directional experience, not a
different resource quantity/difficulty/time schedule. Exact grid permutations
are tested; continuous position/grid-boundary rounding and nondeterministic
population competition preclude a claim of identical complete world histories.

Run arms interleaved per episode; ordinary first on episodes1/3, varied first on
2/4. Export up to128 living descendant bodies using the existing abundance-
weighted lineage-hash sampling. Each arm passes only its own endpoint bank to
its next world. If it has no living descendants to export, stop that arm as a
training failure: no rollback, mixing, extra budgets or rescue. Finish the other
arm if possible. Never choose an intermediate bank based on test outcomes.

## Evaluation

Before any training/evaluation runs, freeze executable, source/runner/plan hashes,
start bank, settings, execution order, and two randomly drawn seeds absent from
available project seed records (scan all local Git worktrees). These two seeds
are this development trial's untouched evaluation seeds, NOT the final goal's
eight holdouts; once observed they are development data permanently.

Evaluate three fixed banks: unchanged starting bank, completed ordinary arm,
completed varied arm. For each evaluation seed, use both0° and180° environments:
four cases per bank, twelve200,000-tick-or-extinction worlds if both arms finish.
The paired orientations of one seed are not independent environment seeds.
An extinct training arm has no endpoint bank and is reported as failure, not
silently replaced with the start bank for evaluation.

All evaluations use1000 fresh bodies,65energy,2inventory,age0–300,empty private
state,metabolism0.06,movement cost0.01,motor gain4,regeneration0.01,evolving
geography,force/signals enabled. Bank loading adds no extra genetic noise.
Ordinary within-world reproduction/mutation is identical in all cases.
Evaluation never exports genes back into training.

Rotate bank evaluation order by case so no bank is always first. Preserve all
commands, results and failures. Metrics1024ticks; existing schema2 journeys32ticks.
Report capped survival times, surviving cases AND distinct seeds, births,
accounting/invalid/cap exposure, sampled journeys and post-hoc directional
responses of the endpoint banks. No directional assay enters survivor selection.

Primary pilot comparison: varied versus ordinary survival across the same four
cases. Also compare both to the unchanged start bank: more training may regress.
If both arms survive equally, call survival tied; journey counts or prettier
paths cannot be substituted afterward as the primary win condition. With two
seeds, outcomes are exploratory, not significance or broad generalization.
Do not promote a bank or claim the broad goal from this trial.

## Integrity and limits

Verify loaded float32 genes, orientation/settings, exact budgets or recorded
extinction, no interventions, per-sample population accounting, numerical and
observer validity, and all input/output hashes. The only between-arm setting
difference during training is registered orientation. Both arms use the same
new executable; historical campaigns keep their original frozen binaries.
Any integrity failure stops the runner and preserves partial evidence.

Schema2 remains limited sampled evidence, not final proof of migration across
three separate major relocations. That measurement upgrade is still required
before the goal's final eight-seed registration. No acceptance criterion changes.
