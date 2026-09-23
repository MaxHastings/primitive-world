# V45 and v46 descendant competition, 2026-09-23

## Question and result

Did descendants evolved during the live v46 run gain competitive persistence
relative to the v45 population from which that run was imported? In nine
isolated contests across three seeds, current v46 descendants drove the v45
lineage extinct. This held against both the frozen 2.654B-tick v45 handoff and
the earlier 2.403B-tick v45 checkpoint that beat the latest v45 lineage in the
last v45 ladder match. Reversing the even/odd founder-bank placement preserved
all three wins against the 2.654B handoff.

The strongest interpretation is improvement **for competition in the current
v46 habitat and physics**. It does not isolate learned behavior from changed
genotypes or show that the v46 rules would outperform v45 rules in a reciprocal
v45-engine contest. It is one frozen v46 snapshot, three seeds, and one fixed
ecological challenge; it is not a general fitness or intelligence score.

## Frozen inputs

- V45 handoff: the read-only converted 2,654,150,367-tick v45 source, exported
  at tick zero into a 256-genome founder bank. Original v45 files were not
  modified.
- Earlier v45 challenger: the existing ladder's 2,402,553,362-tick archived
  founder bank. Only its bank header was translated from v45 format 21/model
  v45 to v46 format 22/model v46 for fresh-world loading; its genomes and
  cognitive traits were used as recorded.
- V46: read-only copy of the 2026-09-23 15:16:29 CDT live save, world 41,
  1,672,800 macro-steps in that world, 47,864,372 accumulated v46 macro-steps
  (95,728,744 biological units). SHA-256 of the frozen checkpoint:
  `b7f52164392f4b8212db356d6f0f8c398a772a7c5034eeba555169bb4681ec43`.
  Exported 256 living descendant genomes from that copy. The wallpaper
  continued running independently at MAX.

Input-bank hashes and full command settings are in the ignored
`reports/lineage-v45-v46-20260923/*/manifest.json` files. Raw reports, ancestry
counts, family outcomes, and logs are alongside them. The script
`tools/compare_lineage_banks.py` reproduces the bank mixing and scoring.
For a separate diagnostic executable, build with
`cargo build --release --features lineage-contest --target-dir target-lineage-contest`.
The script defaults to that executable path. A normal `cargo build --release`
selects the unchanged production birth/fusion shader.

## Assay

Each fresh world starts with 8,192 mature bodies cloned from 128 genomes per
bank, interleaved evenly through founder slots. Both lineages share the same
v46 environment, food, sensing, force, signaling, learning, and physical packet
reproduction. The diagnostic fusion gate only rejects packets with different
ancestry masks. It checks child and living-body ancestry at the end; every
reported contest had zero hybrid births and zero unknown/hybrid living bodies.
The run stops within 32 macro-steps of either pure lineage's extinction, or at
50,000 macro-steps (100,000 biological units).

Fixed settings match the old ladder's ecological challenge except that the
famine/restore timing is halved in macro-steps to keep 5,000/6,000 biological
units: population 8,192; regeneration 0.01; metabolic cost 0.005; movement
cost 0.01; motor gain 4; habitat contrast 1; rotation 0; centered radius-256
food shock of -10, followed by restoration. Seeds were 101, 202, and 303.
The score is founder plus descendant organism-body time, multiplied by two to
express v46 biological units. Births and descendant-parent births were also
read from the family observer, so victory was not inferred from packet count.

| Contest | Seed | V46 / v45 organism-time | V45 extinction at biological unit |
| --- | ---: | ---: | ---: |
| 2.654B v45 vs v46 | 101 | 2.88× | 33,920 |
| 2.654B v45 vs v46 | 202 | 2.78× | 23,936 |
| 2.654B v45 vs v46 | 303 | 2.48× | 28,032 |
| 2.403B v45 vs v46 | 101 | 2.12× | 27,520 |
| 2.403B v45 vs v46 | 202 | 2.16× | 19,968 |
| 2.403B v45 vs v46 | 303 | 2.31× | 24,768 |
| V46 in even slots vs 2.654B v45 | 101 | 3.11× | 31,168 |
| V46 in even slots vs 2.654B v45 | 202 | 2.26× | 20,352 |
| V46 in even slots vs 2.654B v45 | 303 | 2.46× | 25,728 |

The 2.654B v45 arm had 16–175 births to descendant parents in the first
three contests; v46 had 3,904–6,939. Thus the outcome involved multiple
reproductive generations, not merely survival of the initial mature clones.
The same direction held with the founder slots reversed.

## Production boundary and limitations

The live packaged wallpaper and its saves were not changed. The default v46
birth/fusion shader is byte-for-byte unchanged. The ancestry-gated shader is
selected only with the `lineage-contest` Cargo feature; a default build rejects
`--ancestry-audit`. The default build still allocates a small inert ancestry
buffer at reset, but performs no diagnostic ancestry work per simulation tick.

This contest deliberately changes mating eligibility and begins with a dense
cloned population, so it does not predict the wallpaper's natural long-run
ecology or the real-hour rate of useful closed chains. It also evaluates both
banks under v46 rules; the changed macro-step and reproductive burst may favor
genes adapted under v46. Longer wallpaper tracking and, if needed, a reciprocal
v45-engine contest remain distinct questions.

Mechanical checks: `cargo check --release` and
`cargo check --release --features lineage-contest` and both test-target checks
passed; a 1,000-step
feature-build smoke contest completed with zero hybrid births; all nine main
contests had zero hybrids and zero unknown/hybrid living organisms. The
production shader matched HEAD, and the v46 wallpaper process was still
running with `--wallpaper --resume --view-speed MAX` afterward.
