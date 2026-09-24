# V46 restart curve comparison, 2026-09-24

## Question and controlled fork

Does the new `u⁵` founder redraw curve preserve useful descendants better than
the previous `u³` curve? The live wallpaper was not stopped or modified. A
copy of its world-363, tick-3,028,468 save was continued read-only until the
world naturally ended at tick 3,191,488. That zero-population checkpoint was
the identical fork for both arms (SHA-256
`15883D380676895C6AB5B2B391E6B1800301E15E372930337250681B65493B94`).
The archived pre-handoff executable ran `u³`; the current release executable
ran `u⁵`. Both restored the same 4,096-record hereditary pool, progress RNG,
settings, and world-363 history. Each took its normal next-world rollover,
started world 364 with the same world seed `2034854279`, and ran exactly
1,000,000 macro steps (4,000,000 biological units) without a shock. The only
intended production difference between executables was the redraw exponent.
Inputs, hash, reports, checkpoint continuations, and founder exports are in
`reports/restart-curve-ab-20260924/`.

| World 364 at 1M macro steps | `u³` | `u⁵` |
| --- | ---: | ---: |
| Living | 498 | 602 |
| Viable births | 324,222 | 318,557 |
| Closed births | 322,432 | 316,031 |
| Natural maturations | 265,610 | 262,820 |
| Maximum closed depth | 1,007 | 996 |
| Distinct full pool genomes | 3,593 / 4,096 | 3,411 / 4,096 |
| Diagnostic wall time while sharing GPU with wallpaper | 723.7 s | 707.7 s |

Both arms sustained extensive naturally closed reproduction. The slightly
larger `u⁵` living population and slightly larger `u³` birth and pool-diversity
counts do not establish useful strategy quality. Wall-time differences here
are confounded by the continuing GPU wallpaper workload and are not a clean
throughput benchmark.

## Shared-world descendant contest

Each arm exported 256 living descendants at the matched horizon. The same
current-v46 no-hybrid assay used 128 hash-ordered genomes from each export in
8,192 interleaved mature founders, seeds 101/202/303, both slot orders,
50,000 macro-step maximum, and wallpaper-like ecology and upkeep (regeneration
0.01, metabolic cost 0.006008222, movement cost 0.01, motor gain 4, habitat
contrast 1, evolving landscape, no forced famine). Packet fusion stayed
within ancestry. Every run reported zero hybrid births and zero unknown or
hybrid living bodies.

| Seed | `u⁵` / `u³` organism-time, forward | `u⁵` / `u³`, reversed | Reading |
| --- | ---: | ---: | --- |
| 101 | 0.28× | 0.43× | `u³` dominates; `u⁵` extinct in forward placement |
| 202 | 2.38× | 1.81× | `u⁵` leads in both placements |
| 303 | 1.61× | 2.61× | `u⁵` leads in both placements |

`u⁵` led in organism-time and births involving descendant parents in four of
six placements. `u³` led in both placements of seed 101, including one
extinction win. Where both survived to the tick limit, these are leads, not
extinction wins. The contest assesses offspring competitiveness under this
particular dense, no-hybrid challenge; it does not measure all aspects of
wallpaper fitness.

## Decision limit

This is **one matched evolutionary fork**. The three contest seeds test the
same two frozen descendant banks under different ecologies; they are not three
independent restart histories. It demonstrates that `u⁵` did not break closed
reproductive chains and provides a directional, seed-dependent competitive
advantage in this fork. It does not prove `u⁵` improves retained useful
strategies per real-world hour over many natural world restarts. The current
wallpaper remains on `u⁵` at MAX speed while that longer evidence accumulates.
