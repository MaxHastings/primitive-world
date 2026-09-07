# Performance and limits

## Current implementation

Population search adds no per-tick CPU population readback. Two flat CPU arrays
hold the saved current and candidate founding groups; at 1,000 founders they use
about 9.1 MiB together. Candidate construction happens once per comparison.
Completion reads compact metrics, and reset uploads the selected founding group.
The individual-parent qualification/ranking/capture GPU passes are removed.

The GPU still uses compact active-body sensing/decision/update dispatch, eight-lane
workgroups, three-pass parallel scans, streamed parent-to-child parameter mutation,
GPU clearing and active-founder uploads. Initial habitat is reused and independent
terrain rows are built on up to eight CPU workers. No senses, gates or biological
ticks are omitted at higher speed.

The viewer submits at most 32 full ticks per batch, uses compact telemetry and
schedules rendering independently. 1x targets 60 ticks/s; MAX is uncapped. Render
FPS is separate. Dense coincident populations retain quadratic neighbor work;
resource updates, scans, synchronization, terrain and full saves still cost time.
A save can pause playback while complete state is read and written.

There are 74.25 MiB of active genome storage at 16,384 slots, 5.75 MiB in paired
body buffers, 6.25 MiB of perception and 8.875 MiB of decision traces. CPU founding
snapshots add up to two population-sized arrays, with transient construction/reset
storage. These are not total process-memory figures.

## Earlier measurements, not validation of this revision

The preceding 0.8.0 intermediate implementation was measured on 2026-09-07:
Windows 11 Pro 10.0.26200, Ryzen 7 7800X3D, RTX 4070 SUPER, NVIDIA 591.86, Vulkan.
Without rendering, 32 warmup ticks followed by 512 ticks in batches of 32 gave:

| Initial bodies | Earlier optimized range, ticks/s |
| --- | ---: |
| 32 | 2,097–2,219 |
| 1,000 | 1,987–2,107 |
| 4,096 | 1,504–1,547 |

Those runs used the intermediate fixed random bank and individual-lifetime outer
loop. They are not measurements of the current population-search revision or of
interactive playback. Their initialization, active-dispatch and streamed-birth
optimizations remain in the source. There has been no new benchmark, simulation
experiment or test run since the user's code-only verification instruction.

The current revision has formatting, Rust compilation/lint and static source
review evidence only. It does not yet have runtime evidence of increased world
duration, checkpoint replay or final throughput. See the
[implementation checklist](implementation-checklist.md) for exact verification.
