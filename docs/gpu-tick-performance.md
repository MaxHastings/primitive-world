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
