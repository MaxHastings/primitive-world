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
