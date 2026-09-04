# Playable-world validation, registered before evaluation

Freeze the motor-calibration selection and its exported bank before evaluating
new seeds. Physical calibration phase tests gains 4,8,16 on preparation seed1
for131072 ticks with original energy costs. Its declared selection rule chooses
the smallest gain that persists. Do not choose a different gain after evaluation.

Evaluate seeds808,909,1001, each up to200000 ticks, sample1024,1000 fresh bodies:

1. historical: released bank, gain1;
2. calibrated: same released bank, selected gain;
3. prepared: selected gain's exported descendants, same selected gain.

Metabolism0.06, movement0.01, regeneration0.01 and changing geography for all.
No population floor, rescue, external fitness, extra sensor or action rule.
All arms continue ordinary birth mutation. This tests starting gene pools,
not frozen single-agent weights. Personal memories are cleared at creation.
Maximum9 runs/1800000 ticks. Preserve every failure; no retries. Fixed order:
808 historical/calibrated/prepared;909 prepared/calibrated/historical;
1001 calibrated/prepared/historical. No evaluation founder exports.

Primary evidence: survival at200k, capped time to extinction. Secondary:
births, death causes, population-time, sampled peak-to-trough decline, capacity
exposure, living ancestry and sampled travel path/net progress. Travel metrics
are survivor-biased interval observations, not destination arrival or prediction.
Same seed does not guarantee GPU bitwise replay; this three-seed pilot cannot
establish statistical significance or general intelligence.

Separate conclusions: calibrated-vs-historical measures a body parameter;
prepared-vs-calibrated measures the incremental preparation/sampling package.
Do not call the former learning. Do not call more births alone intelligence.

Conservative bank promotion gate: prepared must survive at200k on at least two
more evaluation seeds than calibrated, must not have a shorter capped survival
time on any seed, and must have no numerical faults. If the gate fails, keep
the released bank; expose the candidate only as a clearly experimental option.
No automatic relaxation of this gate to obtain a winner. Capability and
playability improvements can ship independently with explicit evidence limits.
