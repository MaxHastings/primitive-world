# v46 sensing and fixed-work follow-up (2026-09-23)

Goal: increase useful closed-chain evolution per wall hour without changing
the 512×512 ecology, agent channels, physical rules, save format, or wallpaper
presentation. All measurements below used a copied world-49 checkpoint with
736 organisms and 110 packets at receipt time. The packaged wallpaper kept
running on the same RTX 4070 SUPER, so these are short paired comparisons,
not standalone wallpaper throughput measurements.

## Retained: parallel food sensing

The old `perceive_live` shader gave one GPU lane every food-cell center in an
organism's sensor disk. The new shader uses eight lanes per living slot, with
eight slots per workgroup. Each lane visits a disjoint subset of the **same**
512-grid cell centers; integer milli-food and cell counts accumulate into the
same 16 sectors. One lane still performs the original body, velocity, signal,
and pressure scan. The existing compact living-slot dispatch count is reused.
Packets still receive an empty perception record. No sensor range, region,
information channel, rendering, or biological rate changed.

Three alternating 1,024-macro-step headless checkpoint pairs (128 warmup steps,
32-step synchronous batches) gave these rates:

| Configuration | Macro steps/s by repeat | Median |
| --- | --- | ---: |
| Original serial sensing and fixed passes | 1,499; 1,372; 1,391 | 1,391 |
| Parallel sensing, original fixed passes | 1,754; 1,598; 1,652 | 1,652 |

The median gain is about 19% under these conditions. The population remained
in the hundreds after each short continuation. The concurrent wallpaper and
run order make the exact percentage uncertain. Raw paired results are in
`reports/fast-evolution/current-profile-20260923/paired-sensing-fixed-work-spatial.json`
(an ignored local diagnostic).

Mechanical validation compared all perception channels with the serial shader
at 1,000 founders/radius 24 and 256 founders/radius 48. A copied evolved
checkpoint comparison covered 736 organisms and 110 packets: maximum absolute
organism input difference was 2.38e-7, all selected actions matched on the
first step, and packet perception records remained identical.
Different floating-point summation can still cause long-horizon trajectories
to diverge. The change does not claim bit-identical replay or improved fitness.

## Rejected: compact-list rewrites for fixed passes

Two telemetry passes (`observe_signals` and `observe_memory`) were prototyped
on the compact living list. In paired runs their extra gain over parallel
sensing was inconsistent by repeat; they are not retained. `linked_link` was
also prototyped on the compact list. Its combined median fell to about 1,527
macro steps/s versus 1,715 for the preceding sensing-plus-telemetry mode.
That regression was removed. Dense spatial linking and the existing telemetry
paths remain unchanged.

The empty-world floor of roughly 0.24 ms GPU time per macro step still points
to fixed work, but it does not identify a safe single-pass win. The full
dispatch trace perturbed execution heavily. A future attempt needs a narrower
phase measurement and a paired production-path benchmark before changing
scans, copies, clears, or resource display scheduling.

## Test limitations

The full GPU release suite has pre-existing v46 failures in v45 one-unit
expectations. One failing metabolic assertion produced the identical 9.88 vs
9.94 result with the original serial sensing shader. The sensing-specific
equivalence tests and copied evolved-checkpoint comparison passed. This is a
mechanical performance result, not evidence that a long wallpaper run retains
more useful strategies per real hour.

## Packaged wallpaper handoff

The existing wallpaper took its normal save-before-close path at world 55,
local tick 2,326,066, cumulative v46 macro step 65,643,306, with 390 living
entities. Its complete receipt and checkpoint were copied byte-for-byte to an
ignored rollout archive, and the previous packaged executable was backed up
inside the isolated v46 app directory. The new shader passed an additional
one-step comparison on that exact stopped checkpoint: 334 organisms and 56
packets, maximum organism input difference 2.38e-7, zero action changes.

The packaged v46 wallpaper then resumed with `--wallpaper --resume --view-speed
MAX`. An initial window-title reading showed world 55, 115 FPS, and 7,294
biological units/s. Population was 153 at that instant, down from the stopped
receipt's 390. A subsequent short check showed 868 living in the same world
at 145 FPS and 5,310 biological units/s. Those snapshots show a rebound, not
a sustained improvement in reproductive success or throughput. The v45
checkout and saves were not touched.
