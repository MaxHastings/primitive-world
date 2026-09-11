# GPU tick performance

The ordinary tick records 44 dispatches in three compute passes. Buffer copies
and clears remain outside the passes. Optional observers flush pending dispatches
before observing state. Per-dispatch timestamp diagnostics use separate passes so
they still work without requiring timestamp writes inside compute passes.

Plasticity accounting writes energy, cumulative spending, and (when necessary)
alive status directly. It does not rewrite the remaining body fields. Genome and
learned-weight storage remain f32. This reduces body-record traffic; it does not
compress dense neural weights.

Reproduce the paired benchmark in a release build:

```sh
cargo test --release profile_tick_optimizations -- --ignored --nocapture --test-threads=1
```

The diagnostic reports its adapter and compares the reference kernel,
grouped dispatches, and direct plasticity accounting at 32, 1,000, and
4,096 initial bodies. It uses seed 42, default settings except population, a
32-tick warmup, and 512 measured ticks in batches of 32. Three repetitions reverse
configuration order on alternate runs. Results are end-to-end synchronized tick
throughput, without rendering. Population can change during the run. Performance
depends on the adapter, driver, and other GPU workloads.

The regression test `tick_optimizations_preserve_state_across_climate_epochs`
compares buffers byte for byte against separate-pass, full-record
accounting references across multiple seeds, all rotations, weather epoch
boundaries, a terrain boundary, and successive command batches. It checks bodies,
decisions, learned weights, traces, resources, fertility, ground, ecology, and
counters. Existing GPU tests cover checkpoint replay and observer boundaries.

## Measured alternatives

A prototype cached all seeded substrate values and weather endpoints in an 8 MiB
buffer, refreshing only across seed, orientation, and weather epoch changes.
Byte-for-byte ecology comparisons passed, but paired measurements on an RTX 4070
SUPER (NVIDIA 591.86, Vulkan, Windows) found this slower than recomputing the noise
with grouped dispatches. The cache was removed. Resource math remains unchanged.
Cooperative recurrent/gate weight staging also regressed the dense case and was
removed. Neither prototype changes the shipped model or save format.

The longer prototype comparison (three 512-tick repetitions, seed 42) measured
median reference versus retained grouped/in-place-accounting throughput:

| Initial bodies | Reference ticks/s | Retained changes ticks/s |
| ---: | ---: | ---: |
| 32 | 767.5 | 1,305.1 |
| 1,000 | 608.7 | 912.3 |
| 4,096 | 374.5 | 482.6 |

These are local measurements with a live wallpaper experiment also running;
absolute throughput and small differences are workload-sensitive.

## Cooperative inheritance and neural outputs

Packet inheritance assigns one 64-lane workgroup per eligible packet. Lanes copy
independent parameters contiguously instead of making one invocation copy all
2,494 parameters. The existing birth compaction produces its indirect work count;
zero eligible births dispatch zero workgroups. The allocation, identity, mutation,
fusion and overflow rules remain unchanged.

Decision workgroups also write input, hidden-state and output arrays cooperatively.
Each field has one writer. Neural accumulation order, fault handling and all f32
storage layouts are preserved. Playback at 32x and above targets 32 milliseconds
per batch, retaining the existing maximum of 32 biological ticks. Speeds through
16x retain the 8-millisecond target.

A September 11, 2026 release benchmark on Windows, RTX 4070 SUPER, NVIDIA 591.86,
Vulkan measured the following medians. The wallpaper was saved and closed for
these measurements. Each case starts from seed 42 and default settings except
population, warms for 32 ticks, then measures 512 ticks in batches of 32. Three
paired repetitions reverse configuration order on alternate runs.

| Starting bodies | Previous kernels (ticks/s) | Cooperative kernels (ticks/s) | Change |
| ---: | ---: | ---: | ---: |
| 32 | 2,817 | 2,808 | approximately unchanged |
| 1,000 | 2,021 | 2,203 | +9% |
| 4,096 | 1,014 | 1,254 | +24% |
| 8,192 | 616 | 789 | +28% |

These measurements exclude rendering and playback polling; they do not establish
a 2x improvement or guaranteed 32x/64x playback. Population and packet production
change during measurement. Do not add these gains to older published percentages.

Reproduce the kernel comparison or a read-only benchmark of the latest compatible
saved experiment (the latter also compares completion polling and batch targets):

```sh
cargo test --release profile_streaming_optimizations -- --ignored --nocapture --test-threads=1
cargo test --release profile_saved_experiment_streaming -- --ignored --nocapture --test-threads=1
```

Regression coverage compares complete simulation buffers against the previous
kernels with empty, sparse and 1,000-body juvenile scenes, including deaths and
multiple batch boundaries. Bodies are separated to exclude pre-existing GPU
neighbor-summation and birth-identity ordering variability from byte comparisons.
A 130-packet fixture checks every inherited parameter across multiple workgroups.
The full GPU suite additionally covers invalid decisions, capacity exhaustion,
mutation, packet fusion, observers and checkpoint continuation.

A prototype folded dead-record preservation and scratch clearing into the free-slot
pass to remove the full body copy and mid-tick pass boundaries. Paired measurements
found no reliable gain and a small-population regression, so it was not retained.
Ecology cadence, spatial rebuilding and biological rules are unchanged.

The same adapter also benchmarked a saved running experiment with 787 living
organisms (three alternating pairs, 32 warmup + 512 measured ticks, read-only
checkpoint loads). Median kernel throughput increased from 1,623 to 2,223 ticks/s
(+37%). A following 512-tick phase with telemetry and one-millisecond completion
polling increased from 1,392 to 2,038 ticks/s (+46%), combining the kernels and
8-to-32-millisecond batch target. This polling probe excludes rendering, periodic
statistics, inspection and requested-rate credit pacing; it is not a measurement
of sustained wallpaper playback or proof that 32x will always be attained.

Validation: 166 release tests passed, 29 manual diagnostics ignored; Clippy with
warnings denied, formatting, and 12 Python tests passed. The saved-experiment
benchmark was run separately from the full suite with competing simulation work
closed. No saved state was rewritten by benchmarks.

## Further execution audit

Additional September 11, 2026 prototypes were compared against the cooperative
inheritance/output implementation above, using the same adapter and alternating
three-pair, 32-warmup/512-measured-tick methodology. The live wallpaper was saved
and closed. These prototypes were removed after testing:

- Stable organism-only cognition queues used two packed 16-bit prefix counts in
  the existing scan. Mixed organism/packet parity passed; representative rates
  differed by roughly -1% to +2%, with no reliable saved-world improvement.
- Explicit neural scratch initialization passed parity, but removing automatic
  workgroup clearing did not produce a consistent material gain.
- Fusing signal/memory diagnostics into decisions passed state comparisons, but
  the extra work in the neural kernel offset dispatch savings. Signal-only fusion
  also regressed dense and saved-world measurements.
- Expressed-unit staging preserved sparse and full controller state. Dense random
  scenes improved about 2%, but the saved experiment regressed about 2%.
- Eight-lane newborn resets and pool copying gave small, workload-sensitive
  differences rather than a reliable saved-world gain.

These results do not rule out a larger storage or spatial redesign; they show why
estimated savings should be checked against the actual running population. Neural
identity, inherited latent parameters, f32 storage, ecology cadence and save formats
remain unchanged. The retained small scheduling correction limits pool-update
launches to 4,096 destinations; no substantial speedup is claimed for that change.
