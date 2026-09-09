# Performance and limits

The model has 16 potential recurrent units, with 1–16 expressed per organism.
Decisions and local plasticity use a cooperative GPU workgroup per living body.
Each lane evaluates a unit or output, sharing intermediate activities through
barriers. All biological ticks and active connections are evaluated at every speed.

Inherited genomes and lifetime learned deltas each use two GPU banks. At 16,384
slots they occupy 163.25 MiB and 160 MiB respectively; paired body buffers use
9.25 MiB, perception 6.25 MiB, decisions 12.5 MiB, and activity traces 9 MiB.
The 4,096-record hereditary pool adds 40.8125 MiB of GPU genome storage plus traits.
These are allocations, not total process-memory measurements. Sparse descendant
sampling reads only selected genomes; learned-magnitude observation reduces on
GPU before reading compact totals.

The viewer adaptively batches up to 32 ticks so GPU readback stays bounded while
timer pacing can still reach the selected rate. 1x requests
60 ticks/s; 32x requests 1,920 ticks/s; MAX is uncapped.
Requested speed does not override hardware throughput. Rendering, other GPU
applications, body count, dense neighbors, and reproduction all affect speed.
Full saves can pause playback while complete state is read and written.

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
