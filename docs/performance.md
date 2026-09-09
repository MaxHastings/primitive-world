# Performance and limits

The model has 16 potential recurrent units, with 1–16 expressed per organism.
Decisions and local plasticity use a cooperative GPU workgroup per living body.
Each lane evaluates a unit or output, sharing intermediate activities through
barriers. All biological ticks and active connections are evaluated at every speed.

Inherited genomes and lifetime learned deltas each use two GPU banks. At 16,384
slots they occupy 155.875 MiB and 153 MiB respectively; paired body buffers use
9.25 MiB, perception 6.25 MiB, decisions 11.75 MiB, and activity traces 8.5625 MiB.
The 4,096-record hereditary pool adds 38.96875 MiB of GPU genome storage plus traits.
These are allocations, not total process-memory measurements. Sparse descendant
sampling reads only selected genomes; learned-magnitude observation reduces on
GPU before reading compact totals.

The viewer adaptively batches up to 32 ticks so GPU readback stays bounded while
timer pacing can still reach the selected rate. 1x requests
60 ticks/s; 32x requests 1,920 ticks/s; MAX is uncapped.
Requested speed does not override hardware throughput. Rendering, other GPU
applications, body count, dense neighbors, and reproduction all affect speed.
Full saves can pause playback while complete state is read and written.

## v35 optimization probe

The current model rebuilds spatial indexing after motion for contact correctness,
uses generic body-relative area samples, and proportionally shares gathering.
Population and expressed brain capacity also change over time; comparing requested
speed alone does not isolate an execution regression.

Execution optimizations preserve the physical model: distribute sensory input
assembly/validation across the existing decision workgroup, cache shared learning
traces and output activations once per body, and skip empty food cells plus exact
zero/full-share integer division cases. No tick, sample,
learning update, cost, or contact opportunity is removed. Persistence is unchanged.

A local release probe on the RTX 4070 SUPER measured the following batch-32 rates:

| Starting bodies | Before (ticks/s) | After (ticks/s) |
| --- | ---: | ---: |
| 32 | 690 | 807 |
| 1,000 | 532 | 612 |
| 4,096 | 286 | 341 |

The viewer was running concurrently. These are preliminary shared-load measurements,
not isolated benchmark claims or directly comparable to the historical table.
Single-tick GPU timestamps identify decisions/plasticity as major costs at higher
population; gathering decreased from roughly 32 to 12 microseconds in the sampled
4,096-body tick, but individual timings are noisy. A clean benchmark requires
pausing competing simulation work deliberately and repeating paired measurements.

For interactive use, 16x requests 960 ticks/s and 32x requests 1,920. Lowering
presentation FPS (for example `--view-fps 30`, particularly in wallpaper mode on a
high-refresh display) can free rendering budget without changing biology. It will
not overcome a compute-bound controller/learning workload. Larger execution
changes should focus on neural memory access and dispatch overhead, with parity
and accounting tests; do not reduce learning frequency or sensory coverage as a
performance shortcut.

## Historical v26 measurements on September 8, 2026

Windows, RTX 4070 SUPER, NVIDIA 591.86, Vulkan, release build, seed 42. Each case
resets, warms for 32 ticks, then measures 512 ticks. The population grows during
measurement. These are headless measurements on the user's machine, not a
promise of the same interactive rate at every population size.

| Starting bodies | Batch 32, ticks/s | Batch 8 + telemetry, ticks/s |
| --- | ---: | ---: |
| 32 | 1,768 | 2,009 |
| 1,000 | 1,194 | 1,250 |
| 4,096 | 536 | 544 |

The preceding serial 32-unit implementation measured around 149–179 ticks/s
in the same diagnostic session. At the smallest population, decision and
plasticity passes consumed about 2.6 and 3.4 milliseconds. With 16 units and
cooperative evaluation those passes measured about 24 and 28 microseconds.
Both architecture and execution changed, so this is not an isolated capacity
comparison. The measurements establish that 1,000 ticks/s is possible with
1,000 starting bodies; they do not guarantee it for large or crowded worlds.

Reproduce with the ignored `simulation::tests::profile_tick_throughput` test.
The profiler includes all cognitive passes and keeps telemetry separate from
GPU timestamps. Full survivor snapshots every eight ticks add substantial
readback overhead and are diagnostic, not the ordinary playback path.

These historical throughput values predate the composable hereditary-pool model.
Use the long-run reports for current timing; they include checkpoint/restart overhead.
