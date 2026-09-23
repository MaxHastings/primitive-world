# Fast evolution: reference baseline

The preservation reference is commit `c453308`, model
`primitive-v45-depth-retention`, checkpoint version 59. Development uses the
separate `codex/fast-evolution` worktree. The original checkout and saves were
left untouched.

## Current architecture

- 16,384 shared organism/packet slots; live passes compact organism indices,
  while other work and scratch storage still depend on capacity.
- 512 × 512 ecology, updated every biological tick; 256 × 256 spatial grid.
- 107 inputs, including sixteen body-relative regions; 1–16 expressed recurrent
  units and 14 outputs. The 2,494-float genome is copied into packets.
- Learned weights include input, recurrent, gate and output connections.
- Newborns mature at roughly age 1,800; normal worlds last roughly 9,000 ticks.

## Reference measurements

The [saved-world profile](gpu-tick-performance.md#saved-world-layer-profile)
measured 2,660 synchronized headless ticks/s and 2,353 wallpaper ticks/s on an
RTX 4070 SUPER, with 582 organisms and 53 packets. Its separately sampled GPU
costs were approximately 66 µs sensing, 53 µs inheritance, 51 µs ecology, 23 µs
plasticity and 18 µs decisions per tick. GPU batch time was 308 µs/tick, CPU
command encoding/finish 65 µs/tick, submission 1.7 µs/tick. The viewer measured
about 144 FPS at native refresh. These measurements are historical, not a paired
comparison with the new model.

A fresh release run on 2026-09-23 used seed 42, 750 random founders, 1,024 ticks,
one end sample, and an isolated report. It completed in 0.358 wall seconds
(2,857 ticks/s), ending with 67 organisms and 3 packets. The adapter was NVIDIA
GeForce RTX 4070 SUPER, driver 616.92, Vulkan. This short, declining-population
fixture is a smoke baseline; it is not suitable for a sustained-throughput claim.
Raw output: `reports/fast-evolution/baseline-750.json` (ignored).

The [controlled sensor witness](sensor-reachability-audit.md) showed a naturally
born offspring maturing and contributing to subsequent births. The earlier
[unassisted million-tick chain](evolutionary-accessibility-audit.md) produced
1,115 births but no juvenile maturation. It is an early negative result, not a
summary of the much later owner-run experiment. A read-only local audit of a
later v45 save reported 322,180,302 cumulative receipt ticks, 541 organisms and
54 packets at the saved instant, and 833 within-world depth-front steps across
saved world-450 endpoints. Its retained 64 completed worlds plus current world
establish a lower bound of 10,049,004 births; lifetime birth and successful
descendant-transition totals are unavailable. Source: the original checkout's
ignored `reports/waiting-time-audit-20260912/analysis.md`. These observations
are not a controlled performance comparison with v46.

## Comparison rule

Future reference/fast comparisons must restore identical declared fixture
conditions, report living organisms and packets throughout, and separate
macro-steps/s from biological units/s. A CPU prototype or a shrinking population
does not establish the target throughput.
