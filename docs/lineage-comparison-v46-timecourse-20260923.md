# V46 checkpoint time-course contest, 2026-09-23

## Question and frozen checkpoints

Are the descendants in the current BIO_DT=4 wallpaper more competitive than
descendants about 25 million v46 macro steps earlier? Three read-only source
checkpoints from the same continuing experiment were exported as founder banks:

| Cohort | Cumulative v46 macro steps | World / tick | What had changed |
| --- | ---: | --- | --- |
| Early | 79,462,040 | 229 / 28,003 | BIO_DT=4 adapted; just before the cubic restart curve |
| Middle | 90,803,097 | 261 / 1,476,972 | 11.34M steps after cubic curve; before cognition costs were removed |
| Late | 107,299,630 | 276 / 975,558 | 27.84M steps after early; cubic curve and free cognition in effect |

The early checkpoint is the frozen format-66 handoff in
`%LOCALAPPDATA%/PrimitiveWorldV46/archive/restart-cubic-before-format67-20260923`.
The middle checkpoint is the frozen format-67 handoff in
`%LOCALAPPDATA%/PrimitiveWorldV46/archive/pre-free-cognition-20260923-195016`.
The late checkpoint was copied, hash-verified, and exported from the 21:25:56
CDT autosave. No live save was opened for writing. The late copied checkpoint
SHA-256 is
`126EC68F0A6314740381B2D018A84B6656024FC39250F3BA49E3FBC29BB9ECE8`.
All three source checkpoints are already in the four-biological-unit regime.

The exporter samples *living descendants*, not the 4,096-record hereditary
pool: early 255 bodies, middle 211, late 256. The contest uses the first 128
hash-ordered bodies from each export. Among all exported bodies, exact distinct
full genomes were 165, 101, and 228 respectively. Median inherited packet
sizes were 17.12, 14.00, and 13.77. These describe the cohorts; they do not
by themselves establish fitness or the cause of change.

## Matched contests

Each fresh current-v46 world began with 8,192 mature founders, interleaving
128 genomes from each bank across even and odd slots. The diagnostic feature
forbids between-bank packet fusion and verifies zero hybrid births. Both arms
share BIO_DT=4, 512-cell ecology, sensing, plasticity, signaling, force, packet
physics, habitat, food, and mutation rules. The comparison uses seeds 101,
202, and 303, then swaps bank parity for each seed. It stops on lineage
extinction or at 50,000 macro steps. The native-like assay uses the wallpaper's
resource regeneration 0.01, metabolic cost 0.006008222, movement cost 0.01,
motor gain 4, habitat contrast 1, and evolving landscape, without a forced
food shock. Family records count organism-body time, viable births, maturation,
and births involving descendant parents. The observer reported zero unknown
or hybrid living bodies and zero hybrid births in every contest.

| Comparison | Wins across 3 seeds × 2 slot orders | Organism-body-time ratio, median | Main reading |
| --- | --- | ---: | --- |
| Early vs late | Early 6, late 0 | Early / late 3.07× | Late cohort loses strongly |
| Early vs middle | Early 6, middle 0 | Early / middle 3.03× | Drop already visible before cost removal |
| Late vs middle | Late 5, middle 1 | Late / middle 2.46× | Partial recovery after the middle checkpoint |

Early displaced late within 5,856–9,440 macro steps in the native-like
contests. In seed 101 with early in even slots, early had 3,986 births involving
descendant parents versus late's 74; in the reversed placement, early had
5,071 versus late's 84. The result therefore concerns reproductive chain
continuation, not only founder survival or packet count. A second assay with
the earlier standardized food shock and metabolic cost 0.005 also had early
winning all six placements against late. The native-like assay is the primary
comparison because its upkeep matches the wallpaper more closely.

The source files, exported banks, input hashes, per-seed reports, and summary
JSON are under `reports/lineage-v46-early-vs-late-20260923/`. The diagnostic
executable was rebuilt from current source with `--features lineage-contest`;
the installed wallpaper executable and ongoing experiment were unchanged.
`tools/compare_lineage_banks.py` now reads BIO_DT from each report when
computing biological organism-time and accepts an optional no-shock assay.

## What this does and does not establish

**Measured:** For these frozen living cohorts, shared current-v46 rules, and
this no-hybrid contest, the 79.46M cohort is far more competitive than the
107.30M cohort. The 90.80M cohort is also much weaker than 79.46M, while the
107.30M cohort usually beats 90.80M. Thus this particular competitive assay
shows an early decline followed by partial recovery, not monotonic improvement.

**Inference:** The timing makes the cubic restart curve a plausible contributor
to the early decline. The curve changed the distribution of founder disruption,
including topology mixing, immediately after the early checkpoint. But this
time-course is observational. Population bottlenecks, habitat history, random
drift, packet-size evolution, and cohort sampling also changed. The earlier
fixed mix already contained fresh and strongly mutated founders; the cubic
curve is not simply a higher mean parameter mutation rate. The later removal
of cognition costs cannot explain the initial early-to-middle drop because it
had not happened at the middle checkpoint.

This challenge begins with dense mature clones and prevents cross-bank mating.
It therefore measures competitive persistence under the chosen v46 assay,
not the wallpaper's natural real-hour rate of useful closed chains. It cannot
prove the cubic curve harmed long-term evolution or justify changing the live
wallpaper on its own. A causal test would branch one copied checkpoint into
several matched worlds using the old fixed restart rule versus the cubic rule,
then compare retained closed chains and viable descendant cohorts over
multiple rollovers with matched seeds and the same current biology.
