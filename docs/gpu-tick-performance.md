# GPU tick performance

For explicit checkpoint inputs, selective GPU timestamps, observer timings and
opt-in viewer logging, see the [profiling guide](profiling.md).

The ordinary tick records 34 dispatches. Cell heads and per-slot links rebuild
spatial indexing in two dispatches, twice per tick, replacing the previous two
seven-dispatch prefix/compaction sequences. Occupancy counts remain available.
Aligned dynamic uniforms select each tick's parameters, and free-slot classification
also clears reservoir claims. A normal N-tick batch uses N+1 compute passes for
ticks plus one for its final alive count; body copies remain the per-tick boundary.
Terrain changes and observers add boundaries when needed. Observers receive the
current tick's public parameter uniform; rendering receives the final one.
Per-dispatch timestamp diagnostics retain separate passes.

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

## Spatial links and dynamic tick parameters

September 11, 2026, Windows, RTX 4070 SUPER, NVIDIA 591.86, Vulkan. A real
3440×1440 wallpaper comparison used MAX with the original monitor refresh setting
in both builds. Every run loaded an isolated copy of the same compatible save:
world 286, tick 246,036, 749 living entities. The original experiment stayed saved
and closed. Three paired runs alternated order. Each used a three-second startup
wait followed by ten one-second title samples; the first two samples were excluded
from both builds to remove checkpoint/shader startup. Values below are medians of
the three per-run medians, from the application's rounded actual-speed display.

| Wallpaper build | Actual ticks/s | Rendered FPS |
| --- | ---: | ---: |
| Previous implementation | 1,959 | 133.9 |
| Spatial links + dynamic parameters + conditional density reads | 2,199 | 138.8 |

This is about +12% actual playback throughput at unchanged refresh and resolution.
Population evolves during each run; the short windows are not a sustained-run
guarantee. These measurements include rendering, telemetry, polling and ordinary
viewer work, but exclude five-minute autosaves. No save from the original
experiment was rewritten by the comparison.

Separate 1,024-tick headless pairs (32 warmup ticks, batches of 32, three alternating
repetitions) found roughly 2–4% from removing tick boundaries and about 4–9% from
linked indexing at 1,000–8,192 starting bodies and the saved world. Do not add
these percentages to the wallpaper result. Reproduce the isolated comparisons:

```sh
cargo test --release profile_tick_boundaries -- --ignored --nocapture --test-threads=1
cargo test --release profile_linked_spatial -- --ignored --nocapture --test-threads=1
```

The linked-grid test verifies every live organism and packet occurs exactly once,
including dense cells, wrapped positions, sparse slots, and a subsequent empty
world. Wrapped crowded sensing matches exact food/body counts and floating
channels within rounding tolerance. Isolated full-state comparisons span climate
epochs and command batches. Existing recovery, gathering, contact, packet-fusion
and observer tests exercise the production path. Both old scatter and linked
insertion use GPU scheduling order: neighbor sums can round differently, so
compatible checkpoint loading does not imply identical long-term trajectories
between builds. Neural arithmetic, physical costs and sensory coverage are unchanged.

A double-buffered prefix scan and a linear ecology-cell launch layout did not
show reliable saved-world gains and were removed. A 10 FPS presentation trial
gave only a small throughput improvement; it was also removed. MAX retains the
normal presentation refresh policy.

Validation: 169 release tests passed, 31 manual diagnostics ignored; formatting,
Clippy with warnings denied, and 12 Python checks passed. The release wallpaper
build was also exercised in the paired playback runs above.

## Neural, sensing, and weather inner loops

The decision and plasticity kernels now avoid integer division and remainder for
two-bank addressing. Biases and the recurrent, gate, and output regions use
bank-specific accessors, while sensory weight staging walks active rows so it no
longer divides every flattened connection index by the input count. Per-unit
accumulation and learning-cost order are unchanged.

Perception calculates the body-frame sine and cosine once per organism rather
than once per accepted food cell or neighboring body. Ecology receives the two
weather epochs and remainders in otherwise-unused tick-parameter lanes, removing
four uniform integer division/remainder operations from each of 262,144 cells.
The interpolation arithmetic remains on the GPU in its original order.

A September 11, 2026 paired release probe ran with the Wallpaper experiment still
active. Each mode warmed for 32 ticks and measured 512 ticks in batches of 32;
five pairs alternated order. The saved-world rows repeatedly loaded one isolated
snapshot and did not rewrite it.

| Workload | Previous inner loops (ticks/s) | Retained inner loops (ticks/s) | Change |
| --- | ---: | ---: | ---: |
| 1,000 starting bodies | 1,072.0 | 1,087.9 | +1.5% |
| 4,096 starting bodies | 610.6 | 663.7 | +8.7% |
| Saved world 289, tick 91,609, 930 living | 944.7 | 969.5 | +2.6% |

These are medians under a competing GPU workload, so small differences remain
workload-sensitive and should not be added to earlier percentages. Full-state
comparisons cover bodies, perception, decisions, learned weights, traces,
resources, fertility, ground, ecology, and counters. Weather parity also spans
both weather-period boundaries and a terrain epoch boundary.

Larger 16- and 32-thread live workgroups, ordinary read-only ground loads, fast
single-wrap grid indexing, and comparison-based sensory sectors were measured and
removed. They were neutral or slower on the fresh and saved workloads; the sector
prototype also differed from `atan2` at exact angular boundaries.

Reproduce the retained comparison with:

```sh
cargo test --release profile_retained_inner_loops -- --ignored --nocapture --test-threads=1
```

Validation: 170 release tests passed and 32 manual diagnostics were ignored;
formatting, Clippy with warnings denied, and 12 Python checks passed.

## Saved-world layer profile

September 12, 2026, Windows, RTX 4070 SUPER, NVIDIA 591.86, Vulkan, release build.
The original wallpaper was saved and closed; measurements loaded isolated copies
of world 442 at tick 5,977,704. The snapshot contained 582 organisms and 53 packets;
549 organisms expressed four recurrent units. The original experiment and binary
were preserved and resumed after measurement.

Five alternating-order headless repetitions each warmed 128 ticks and measured
4,096 ticks in batches of 32. Population evolved during each measurement.

| Layer | Median ticks/s |
| --- | ---: |
| Synchronized simulation batches | 2,660 |
| Batches with telemetry | 2,654 |
| Telemetry with one-millisecond completion polling | 2,507 |
| Polling plus once-per-second metrics | 2,489 |
| Blocking batches with detailed observation every 1,024 ticks | 2,383 |

The batch-timestamp probe measured about 308 microseconds of GPU execution,
65 microseconds of CPU command encoding/finish, and 1.7 microseconds of submission
per tick. Completion waiting includes GPU execution; these overlapping timings
must not be added together. Detailed observation includes metrics, evolution and
hereditary-pool search, but the diagnostic excludes full report serialization.

Three alternating wallpaper pairs used MAX at 3440 by 1440, with a fresh copied
save for each run. Each launch ran for 24 seconds; the first five statistics
windows were discarded and the next fifteen analyzed. Native refresh requested
165 Hz and measured about 144 FPS. Median throughput was 2,353 ticks/s at native
refresh versus 2,452 at a 30-FPS limit, approximately a 4.2% difference. This
comparison includes presentation and scheduling effects, not just drawing cost.

Selective kernel captures used three repetitions of 128 measured ticks per kernel,
with the same warmup and a reset for every repetition. Median diagnostic durations
were approximately 66 microseconds/tick for sensing, 53 for fusion inheritance,
51 for ecology, 23 for plasticity and 18 for decisions. These independently
sampled intervals are not additive time shares or guaranteed removable costs.
Timing every dispatch inflated GPU batch time to about 508 microseconds/tick;
selective captures stayed near their own short-window baseline of 339. The short
capture windows and longer throughput runs do not have identical evolving loads.

Five warmed observer probes measured median hereditary-pool search at 40 ms,
evolution snapshots at 6.8 ms, ordinary synchronous metrics at 0.34 ms and durable
experiment saves at 0.39 seconds. Queued checkpoint uploads were explicitly
submitted and completed before timing observers. Separate readback probes found
large genome and learned-weight buffers dominate save transfer cost. Save timing
includes synchronization and receipt publication, not only checkpoint writes.

Three normal 30,000-tick headless continuations, sampling every 1,024 ticks and
saving an end checkpoint, measured about 2,279 ticks/s using the report's wall
timer. Complete process durations were about 14.6 seconds, including startup and
final output. No world transition occurred; restart cost was not established.
Nsight Systems produced no Vulkan events in the attempted capture, so no
hardware-counter or instruction-level conclusions are claimed.

### Rejected cooperative fusion prototype

A subsequent prototype queued successful fusions and used one workgroup per
child to copy independent genome entries. One lane retained donor draws, trait
updates and mutation ordering. Targeted tests passed byte-for-byte comparisons
against serial inheritance, including 130 disjoint offspring, topology growth
and pruning, high-word ticks, failed fusions and queue resets.

Five alternating pairs used a newer isolated snapshot of the same world at tick
6,873,224 with 738 living entities, plus seed-42 fresh worlds. Every run warmed
128 ticks and measured 4,096 ticks in batches of 32. Both diagnostic variants
retained the prototype scratch allocation/reset; this was an internal algorithm
comparison, not a clean release-binary comparison.

| Workload | Serial reference ticks/s | Queued cooperative ticks/s | Change |
| --- | ---: | ---: | ---: |
| Saved world | 2,664 | 2,751 | +3.2% |
| 32 starting bodies | 4,444 | 4,371 | -1.6% |
| 1,000 starting bodies | 3,276 | 3,129 | -4.5% |
| 4,096 starting bodies | 2,885 | 2,845 | -1.4% |
| 8,192 starting bodies | 2,537 | 2,465 | -2.8% |

The small saved-world gain and mixed/regressing fresh-world results did not
justify the additional queue and synchronization complexity. The prototype was
removed without deployment. A later queued-serial comparison was interrupted
and is not treated as completed evidence. No claim of exhaustive optimization
follows from rejecting this candidate.

The retained profiling implementation passed 174 release tests, formatting,
Clippy with warnings denied, and 12 Python tests. Three new profiling diagnostics
are ignored by default and require explicit paths. No biological optimization
was retained from this profiling pass.
