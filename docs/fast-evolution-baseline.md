# Fast evolution: reference baseline

The preservation reference is commit `c453308`, model
`primitive-v45-depth-retention`, checkpoint version 59. Development uses the
separate `codex/fast-evolution` worktree. The original checkout and saves were
left untouched.

## Current architecture

- 16,384 shared organism/packet slots; live passes compact organism indices,
  while other work and scratch storage still depend on capacity.
- 512 × 512 ecology, updated every biological tick; 256 × 256 spatial grid.
- The 2,048 × 2,048 logical world is the default, but wallpaper launch currently
  replaces habitat dimensions with monitor pixels; resume may start a new world
  from at most 64 survivor genomes when those dimensions differ. V46 must keep
  simulation dimensions independent of later window changes, while preserving
  the saved dimensions when importing an existing v45 wallpaper world.
- 107 inputs, including sixteen body-relative regions; 1–16 expressed recurrent
  units and 14 outputs. The 2,494-float genome is copied into packets.
- Learned weights include input, recurrent, gate and output connections.
- The v45 checkpoint includes live bodies/packets, per-slot genomes, learned
  weights, traces, ecology, and the hereditary reservoir; it supports a fuller
  read-only v46 transfer than the existing 64-survivor conversion path.
- Newborns mature at roughly age 1,800; individual maximum ages are roughly
  9,000–11,000 ticks. Worlds can run much longer.

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

The latest complete local v45 receipt found for the owner-run experiment is
`save-1790173625387804500.json`, world 869, world tick 2,840,754,
542 living entities, and **2,654,150,367 cumulative ticks**. Its matching
checkpoint is 417,529,598 bytes. A read-only copy was used for v46 development;
SHA-256 is `e64b7b46d4d3989389a2dd748be3196f09ac58bae065f8e6e545589d454ab351`.
The original v46 handoff used this complete receipt. Its frozen source and the
v45 files, experiments, and original checkout remain untouched. The later
512-grid v46 handoff expands the accumulated v46 continuation separately.

A short same-source headless replay of 3,000 v45 biological ticks retained
528 living entities and made 182 viable births in 1.03 seconds of simulation.
This is a fixture observation, not a wallpaper-rate measurement or a long-run
control for v46. The v46 short comparison and closed-chain continuation are
recorded in [the decision record](fast-evolution-decisions.md).

## Comparison rule

Future reference/fast comparisons must restore identical declared fixture
conditions, report living organisms and packets throughout, and separate
macro-steps/s from biological units/s. A CPU prototype or a shrinking population
does not establish the target throughput.
