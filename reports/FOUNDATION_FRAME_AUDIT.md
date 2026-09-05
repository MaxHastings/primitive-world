# Foundation audit: rotating sensor slots

September 4, 2026. Goal remains unfinished; this is a diagnostic, not a release
or successful migration claim.

## Reproduced on the actual GPU

`cargo test --release released_bank_sensor_frame_diagnostic -- --nocapture`
passes against the released 128-genome bank. Identical adult bodies, zero hidden
state, 50 energy, 1 inventory, no neighbors, gain4. Food underfoot is 0.4. Compare
an east-rising food field to its west-rising mirror; both slopes are 0.01 food per
world unit. Report half the difference in requested movement, so unconditional
drift largely cancels. Only food changes within each pair.

| Attention radians | Mean response x | Mean response y | Positive east response |
| --- | ---: | ---: | ---: |
| 0 | 0.130683 | -0.001033 | 128/128 |
| +pi/2 | -0.000373 | -0.144378 | 54/128 |
| pi | -0.133714 | 0.001069 | 0/128 |
| -pi/2 | 0.000401 | 0.142424 | 73/128 |

These are normalized motor requests, not actual speed or migration success.
The test dispatches production perception and decision shaders without executing
the body action, so resource collection and competing agents cannot change the
paired food cue. It verifies that the reported sample offsets rotate.

## Cause and limits

`src/model.rs::bootstrap_genome` relays the four near food sample slots into
state units 3..6, then subtracts opposed pairs to steer x/y. In
`shaders/perceive.wgsl` these slots rotate with attention. In
`shaders/decide.wgsl` the motor vector is world-aligned. The mutable initial
disposition therefore works at zero angle but reverses at pi. The released
descendants have not compensated in this controlled probe. Actual offsets and
attention are inputs, so compensation is representable in principle; this is
not a disconnected sensor or an invalid physics calculation.

This does **not** establish the cause of long-run extinction, diagnose behavior
of every evolved descendant, prove memory is unnecessary, or establish the
right final coordinate convention. The diagnostic is specific to the frozen
historical bank and should not constrain future controllers to preserve failure.

## Next causal test

In a separate research worktree, hold food sampling compass-aligned. Keep
controller weights, physical costs, regeneration, movement resolution and birth
rules unchanged. This is an explicitly disclosed sensor-attention ablation, not
a final clean model: attention still changes its scalar input but cannot rotate
the food probes. Compare survival with the original on already-used development
seeds 808, 909 and 1001 at 200k ticks. No new holdouts are consumed. Do not promote
an ablation with a dead actuator as the final solution; use it to decide whether
coordinate coherence warrants a versioned controller redesign.

## Foundation still to establish

The 64-input/16-state/16-output controller has fixed lifetime weights, tanh state
updates and argmax body actions; actual births copy and mutate weights. There is
no gradient loss or within-life weight learning. Sixteen stored numbers alone
do not prove useful retention. Local point probes do not reveal remote food.
Travel must fit both energy and lifetime budgets, and must lead to feeding and
reproduction to benefit descendants. Aggregate displacement misses those events.
The existing food, physiology and birth-accounting tests establish narrower
invariants, not adaptive capability. These distinctions remain required in all
future reporting.

## Completed development comparison

Six worlds, 588,800 actual ticks. Frozen bank SHA256
`b99a0682a3f9bfc4593446a3d297ad3bbc879060e1cb52c8d41d8b864e3edd0a`;
gain4, costs0.06/0.01, regeneration0.01, 1000 initial bodies, 200k cap.

| Seed | Original extinction sample | Fixed-sensor extinction sample |
| --- | ---: | ---: |
| 808 | 98,304 | 98,304 |
| 909 | 49,152 | 122,880 |
| 1001 | 73,728 | 146,432 |

All six populations died; none produced a 200k survivor. No invalid outputs or
95%-capacity samples. Counts are sampled extinction endpoints, not exact death
times. Parallel GPU resource competition means repeated worlds need not have
identical birth counts. These were previously used development seeds, not fresh
heldouts. Two longer lifetimes suggest the mismatch matters ecologically; this
single paired run per seed does not estimate effect uncertainty or establish it
as the sole cause of extinction.

Subsequent evidence strengthens that caution: an unmodified-controller seed808
trace lasted196k rather than98k without an intended behavior change. The apparent
survival benefit of the sensor ablation needs replication; do not present it as
a reliable causal effect size. See `DEPARTURE_ATTRITION.md`. The controlled
single-decision coordinate mismatch remains directly reproduced.

The fixed-sensor GPU probe produced a positive paired food response for128/128
genomes at all four angles (mean x0.13055..0.13087). The intended manipulation
worked. It is still not a clean final model: the attention actuator no longer
affects food sensing. Do not promote it unchanged or claim training improved.

Artifacts and registered plan are preserved in the sibling `ClownSimulator-frame`
worktree, branch `codex/sensor-frame-diagnostic`, commit `d4f2fbe`, under
`reports/sensor-frame-20260904`. Raw reports and frozen executables remain local;
plan/summary are committed. The runner's decimal-precision verification error
and continuation without rerunning the completed world are documented in that
worktree's `reports/SENSOR_FRAME_RUNNER_NOTE.md`.

Next: use a versioned, coherent sensory/motor contract instead of leaving the
ablation's dead actuator in production; separately measure whether surviving
descendant banks improve survival after preparation under that contract. The
new sampled journey observer provides sequence evidence, but major-relocation
attribution and final heldout validation are still outstanding.
