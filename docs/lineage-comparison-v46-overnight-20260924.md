# V46 overnight lineage comparison, 2026-09-24

## Frozen morning populations

The wallpaper continued at MAX speed on BIO_DT=4. Two adjacent autosaves were
copied and inspected without writing to the live save or interrupting the
wallpaper:

| Morning sample (CDT) | V46 macro steps | World / tick | Living | Exported living descendants | Exact distinct genomes |
| --- | ---: | --- | ---: | ---: | ---: |
| 2:51 AM | 165,972,821 | 322 / 1,884,506 | 590 | 256 | 216 |
| 3:01 AM | 167,811,759 | 323 / 1,170,010 | 700 | 256 | 238 |

The v45 inherited base was 2,654,150,367 macro steps, making the cumulative
raw step count 2,821,962,126 at 3:01 AM. The copied 2:51 AM checkpoint SHA-256
is `B15E34CDC980D9FCE4EF381A6E4D959910F149A52787DAD0DDB8B5227ED8F0A2`;
the 3:01 AM checkpoint SHA-256 is
`F1BC9FE1A0BC30D51D32E47A2F89318AB553E8BE809C5016383A6D47F5C398C6`.
The copies, founder exports, and contest reports are under
`reports/lineage-v46-overnight-20260924/`.

The morning banks contain sampled *living descendants*, not the hereditary
pool. Median inherited packet sizes were 14.20 and 13.56 for the 2:51 and
3:01 AM banks. This is descriptive, not an explanation of contest performance.

## Matched tests

Each bank was challenged against three earlier BIO_DT=4 banks: early 79.46M,
middle 90.80M, and late 107.30M v46 macro steps. See
`docs/lineage-comparison-v46-timecourse-20260923.md` for their provenance.
The same current-v46 assay used 8,192 mature founders in a fresh shared world,
128 hash-ordered genomes per bank interleaved in slots, seeds 101/202/303,
and both slot orders. Packet fusion was restricted to the same ancestry.
Each contest stopped at lineage extinction or 50,000 macro steps. Wallpaper-like
resource regeneration (0.01), metabolic upkeep (0.006008222), movement cost
(0.01), motor gain (4), habitat contrast (1), evolving landscape, and no forced
famine were used. Every run reported zero hybrid births and zero unknown or
hybrid living bodies.

The score below is **organism-body-time lead** across three seeds and two slot
orders. A lead at the 50,000-step limit is not an extinction win.

| Morning bank | Versus 79.46M early | Versus 90.80M middle | Versus 107.30M late |
| --- | --- | --- | --- |
| 2:51 AM, world 322 | Behind 6/6; extinct 6/6 | Ahead 4/6; middle extinct 2/6, morning extinct 2/6 | Ahead 4/6; late extinct 3/6, morning extinct 1/6 |
| 3:01 AM, world 323 | Behind 6/6; extinct 6/6 | Behind 6/6; extinct 4/6 | Ahead 2/6; morning extinct 2/6 |

Against early, both morning banks produced only 14–52 births with descendant
parents per run before extinction, while early produced thousands. The 2:51 AM
bank's advantage over middle and late was sensitive to seed and founder slot
order; it should not be reported as a clean victory. For example, against late
it led in organism-body time in four placements, lost one by extinction, and
had one capped run with late ahead. The 3:01 AM bank was weaker than 2:51 AM
against middle in every placement, despite being about 1.84M macro steps later.

## Interpretation

**Measured:** Overnight wallpaper evolution did not clearly surpass the
strongest sampled 79.46M cohort in this contest. It produced one morning world
competitive with the 90.80M and 107.30M banks and an adjacent world that was
much weaker. Living population size and exact genome count alone did not
predict the head-to-head result. The wallpaper itself remained running and
responsive at MAX speed after testing.

**Inference and limit:** World rollover, founder composition, ecology, and
sampling can change the available living cohort substantially. This assay
begins with dense mature founders and disallows between-bank mating. It
measures competitive persistence and closed descendant-parent births under
these shared rules; it does not directly measure the live experiment's useful
heritable strategy discovery per real-world hour. The result does not isolate
the cubic founder mutation curve as the cause. The stronger next diagnostic is
to compare samples from the hereditary pool across rollovers and run matched
restart-rule branches from the *same copied pool* over several worlds.

No simulator code or live experiment setting was changed for these tests.
