# V46 restart-diversity trial

## 2026-09-24 stability-biased handoff

The live founder redraw curve is now `u⁵` instead of `u³`, where `u` is each
founder's shuffled rank between zero and one. The median gene redraw
probability falls from 12.5% to 3.125%; the 80th percentile falls from 51.2%
to 32.768%. The exact inherited and fresh endpoints remain. Brain genes,
plasticity parameters, structural traits, and inherited mutation controls
still follow the same mixing mechanism. Ordinary birth mutation, the rolling
hereditary pool, ecology, sensing, physics, rendering, and BIO_DT=4 were not
changed. The new curve will first act at the next natural world rollover.

The owner chose this stability shift without a matched fitness trial. The
morning lineage comparisons showed substantial competitive variation but did
not establish that the cubic restart caused it. The old wallpaper saved and
closed normally at world 324, tick 852,158, with 780 living and 171,392,447
v46 macro steps. Its exact checkpoint, receipt, previous executable, hashes,
and handoff manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\restart-cubic5-before-handoff-20260924-032954`.
The new executable resumed that checkpoint at MAX speed as process 7708 on
the same raised desktop wallpaper layer. The checkpoint SHA-256 is
`29E045AD319C412D73921E07CCACAE4CC4E4CEEC32B9D0A5D09F9535F307AEAD`.
The new executable SHA-256 is
`D685AA18FAB8B517D65254C31A4FF798F6E07DFB9589E6F1BAF8AEFA41DF1091`.
The release build passed; a separate comparison test was deliberately skipped
at the owner's request.

A later [matched `u³` versus `u⁵` fork and descendant contest](restart-curve-comparison-20260924.md)
found that both curves sustained closed reproduction; `u⁵` led in four of six
contest placements from one paired evolutionary fork. That result is
directional and seed-dependent, not a long-run fitness verdict.

## Two-parent reproductive ancestry check (2026-09-23)

A separate diagnostic restarted from the same copied world-268 pool and
recorded both packet producers at every viable fusion. It matched every birth
record to its fusion pair, reconstructed the full founder pedigree of each
living organism, and checked that the reconstructed first-parent tag matched
the stored `founder_family` label. The run reached tick 262,144 with 85,905
viable births and 1,706,954 observer events, below the allocated 2,000,000
event capacity. The live wallpaper and its saves were untouched.

| Trial tick | Living | One-sided tags | Founders in living two-parent pedigrees | Founders ancestral to every living organism | Median founders per living pedigree |
| ---: | ---: | ---: | ---: | ---: | ---: |
| 8,192 | 1,352 | 102 | 249 | 0 | 29 |
| 65,536 | 666 | 11 | 243 | 243 | 243 |
| 131,072 | 666 | 8 | 243 | 243 | 243 |
| 262,144 | 436 | 3 | 243 | 243 | 243 |

By tick 65,536, reproductive ancestry had mixed enough that every living
organism had the same 243 founders somewhere in its two-parent pedigree. The
visible tag count continued to fall to three, and 63,367 of 85,905 viable
births by tick 262,144 paired parents with different one-sided tags. Thus a
tag sweep is not a sweep of all reproductive ancestry. The shared pedigree
also reaches a saturation point and stops distinguishing which founders are
currently supplying useful genetic material.

This is **reproductive genealogy**, not exact gene provenance. Each fusion
uses two packets, but module-wise recombination can replace a distant
ancestor's last surviving parameters. The 243 shared founders must not be
reported as 243 active genetic contributors. Exact inherited-material
tracking would need provenance attached to each heritable module or parameter
through recombination and mutation. The compact milestone records are under
`reports/two-parent-seed-2718281828/two-parent-tick-*.json`.

## Isolated restart observations from the world-268 pool (2026-09-23)

A diagnostic loaded a **copy** of the world-268 tick-632,116 checkpoint and
used the normal cubic rollover to start a counterfactual world 269 with 8,192
founders. Three independent restart seeds used the same 4,096-record source
pool. Two ran for 65,536 world ticks; the third ran for 262,144. None wrote a
game save or changed the running wallpaper. These are trajectories, not a
real-time performance or evolutionary-value comparison.

| Trial / tick | Living | Viable births | Founder tags | Largest tag | Three-unit living | Distinct expressed brains in sample |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| A / 0 | 8,192 | 0 | 8,192 | 0.01% | 66.7% | 1,013 / 1,024 |
| A / 65,536 | 728 | 27,012 | 16 | 21.3% | 98.5% | 515 / 728 |
| B / 0 | 8,192 | 0 | 8,192 | 0.01% | 67.6% | 1,017 / 1,024 |
| B / 65,536 | 386 | 17,239 | 7 | 25.4% | 90.4% | 263 / 386 |
| C / 0 | 8,192 | 0 | 8,192 | 0.01% | 66.6% | 1,017 / 1,024 |
| C / 65,536 | 587 | 27,453 | 12 | 28.3% | 99.7% | 485 / 587 |
| C / 131,072 | 706 | 49,377 | 7 | 27.6% | 97.3% | 603 / 706 |
| C / 262,144 | 458 | 82,031 | 4 | 61.1% | 96.5% | 360 / 458 |

In C, tag count fell to 91 by tick 8,192 and 12 by tick 65,536, then more
slowly to four at tick 262,144. Viable, naturally closed reproduction
continued throughout; 80,235 of the 82,031 births by the last checkpoint
were closed births. The common `0x19` three-unit brain's mean expressed-weight
RMS pair distance fell from 0.0576 among sampled founders to 0.0081 at
262,144. That indicates narrower weights among living brains with this mask,
even though 360 of 458 living organisms still had exact distinct expressed
brains. The initial sample included strongly redrawn founders, so this change
does not isolate selection from the loss of highly disrupted founders.

The pool itself did not become a single genotype. In C, exact distinct full
pool genomes were 2,675 of 4,096 before rollover, 3,580 at tick 8,192, and
3,890 at tick 262,144. Exact distinct *expressed* pool brains were 2,586,
3,340, and 2,617, respectively. The pool remained overwhelmingly three-unit:
4,043 slots at the start and 4,018 at tick 262,144. Founders with other
topologies were therefore mostly made by restart redraw; their early loss
does not demonstrate that a larger well-adapted brain is inherently worse.

**Interpretation limit:** `founder_family` is an observer tag copied from only
one packet in a two-parent fusion. A single tag can contain genetic material
from multiple founders. It resets to the spawn slot each new world and cannot
identify a persistent cross-world lineage. Tag consolidation measures this
one-sided label's concentration, not loss of all genetic diversity. The
two-parent check above traces reproductive pedigree; exact genetic-material
contributions still need module or parameter provenance. A controlled
brain-size comparison would also need matched genome
backgrounds and starting conditions. The per-tick JSON observations are in
`reports/latest-world-20260923-w268/pool-trial-tick-*.json` and
`reports/pool-trial-seed-{3735928559,2718281828}/pool-trial-tick-*.json`.

## Previous selection: one cubic curve

The owner replaced the fixed 20/40/40 mix with one rule applied to every
founder at each *natural* world rollover. All founders independently sample
the rolling hereditary pool. A shuffled rank gives each founder a unique
position `u` from zero to one; each brain gene, including latent genes, has
redraw probability `u³` from an independent fresh genome. There are exact
endpoints: `u=0` is an unchanged pool record, and `u=1` is an entirely fresh
genome and fresh inherited traits. The ranks are shuffled across spawn slots.
At 8,192 founders, the expected gene redraw fractions at the 20th, 40th,
60th, 80th, and 90th percentiles are 0.8%, 6.4%, 21.6%, 51.2%, and 72.9%.
These fractions describe genetic replacement, not behavioral distance.

Plasticity coefficients and retention traits follow the same redraw
probability. Each active-mask bit, packet size, and inherited mutation control
uses its square, so structural disruption rises more cautiously. An empty
mixed active mask is replaced with the fresh nonempty mask. The endpoints
remain exact. Pool records are never overwritten by founder generation;
only ordinary viable births can add descendants to the pool. Ecology,
rendering, sensing, collision handling, force, packets, and BIO_DT=4 are
unchanged. The curve does not alter the current living world until it ends.

Checkpoint format 67 accepts prior v46 formats 62–66. The previous format-66
wallpaper/checkpoint were archived at handoff; see the handoff note below.
This curve increases the range of restart exploration but has no measured
fitness advantage yet. In particular, a smooth redraw probability does not
guarantee smooth changes in behavior. Judge it by retained naturally closed
reproductive chains per real hour, population stability, and visible wallpaper
continuity, not by births or world turnovers alone.

A read-only check against the evolved format-66 pool found that the 50% gene
redraw point left a median 50.1% of expressed parameters unchanged. Its median
Euclidean distance from the inherited expressed genome was 0.711 of the
distance to a wholly fresh genome (10th–90th percentiles 0.649–0.766).
These are genetic measurements; behavior and evolutionary value remain open.
The targeted endpoint, deterministic rollover/reload, format-66→67 one-step
save/reload, release build, and clippy checks passed.

**Exact handoff:** The format-66 wallpaper saved and closed normally at world
229, tick 28,003, 262 living, cumulative v46 macro step 79,462,040. Its
receipt, 417,660,257-byte checkpoint, executable, and SHA-256 manifest are
under `%LOCALAPPDATA%\PrimitiveWorldV46\archive\restart-cubic-before-format67-20260923`.
The wallpaper resumed that exact world at MAX and attached to the same
3440×1440 raised-desktop layer. The curve started at its next natural
world rollover. After a source/test cleanup, the wallpaper saved normally
again at world 233, tick 74,351, 240 living, cumulative v46 macro step
79,946,390, and resumed this format-67 checkpoint at MAX. The final installed
executable SHA-256 is
`E501724874C4A3AA128E39EFA54CF05DD69525F51242A6BFB77F14DE2C5EC53A`.
This short continuation establishes a working handoff, not long-run fitness.

## Earlier fixed-mix trial and evidence

The owner selected a continuing wallpaper experiment with a 20/40/40 founder
mix at each world rollover. The source of the retained founders is the rolling
4,096-record hereditary pool, which spans worlds; it is not a literal copy of
the just-extinct world's final organisms. The current living world is not
repopulated until it naturally ends.

For each five founder slots, one receives a completely new random genome and
random inherited traits, two receive unchanged independent pool samples, and
two receive independent pool samples whose **copies** are heavily mutated.
At the saved population setting of 8,192 founders, this gives 1,639 new,
3,277 unchanged, and 3,276 mutated founders. Slot classes repeat evenly across
the initial population, whose positions are independently seeded. The pool
records themselves are unchanged by founder sampling or restart mutation;
only ordinary viable births can introduce these new lineages into the pool.

"Heavily mutated" now means one quarter of the selected pool genome's
expressed parameters are chosen without replacement and replaced by the
corresponding parameters of an independent freshly randomized genome. The
other three quarters remain exactly inherited. One active plasticity
coefficient is redrawn; both retention traits move halfway toward fresh
random values. Topology, packet size, latent genes, and inherited
mutation controls remain rooted in the pool record. This is deliberately much
stronger than a normal birth mutation, especially because 4,095 of 4,096
records in the sampled pool had parameter-mutation-rate multipliers in the
lowest histogram bin (0.25–0.354).
Ordinary births and all physical, cognitive, ecology, food, and presentation
rules remain as in the running four-unit build.

A read-only check of 128 evenly spaced records from the saved evolved pool
compared each mutated genome with its source and the same fresh random genome,
using only expressed parameters. The ratio of mutant-to-source distance to
fresh-to-source distance was 0.500 at the median (10th–90th percentiles
0.430–0.561); the median fraction of expressed parameters left exactly intact
was 0.748. This calibrates genetic distance, not behavior or reproductive
success. The metric is computed by the ignored
`profile_restart_mutation_distance_on_saved_pool` diagnostic.

The previous four-unit executable and exact stopped checkpoint were archived
before the 20/40/40 installation. Checkpoint format 65 accepted prior v46
formats 62–64. The stronger mutant version uses format 66 and accepts formats
62–65; older executables reject format 66. Each handoff resumes the current
world exactly, and the revised founder mix begins at its next rollover.

The exact handoff saved world 158, tick 15,218, 300 living, and cumulative v46
macro step 75,849,894. The old format-64 receipt, 417,660,249-byte checkpoint,
old DT4 executable, and verified SHA-256 manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\bio-dt4-before-restart-diversity-20260923`.
The format-65 executable resumed that same world at MAX. A copied format-64
checkpoint loaded and advanced one step in the new build, saved as format 65,
and reloaded; the targeted founder-mix and mutation tests passed.

After the owner clarified that the mutants should be closer to the middle
between selected and fresh genomes, the format-65 wallpaper took its normal
save-and-close path at world 184, tick 33,183, 92 living, cumulative v46 macro
step 77,008,013. Its exact receipt, 417,660,254-byte checkpoint, executable,
and verified SHA-256 manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\restart-mix-before-strong-mutants-20260923`.
The format-66 build resumed that exact world at MAX. A copied format-65
checkpoint advanced one step, saved as format 66, and reloaded successfully.

This is an exploration trial, not an established improvement. The large novel
cohort can consume resources and dilute viable inherited behavior; heavily
mutated controllers can also lose useful coordination. Judge the run by mature
descendants, naturally closed reproductive chains and their retention across
worlds per real hour, alongside population and rollover duration. Faster
rollover or more founder diversity alone is not success.

The later [v46 checkpoint time-course contest](lineage-comparison-v46-timecourse-20260923.md)
found that descendants from just before the cubic rollout beat cohorts after
11.34M and 27.84M further macro steps in a shared no-hybrid challenge. The
newest cohort did beat the intermediate one in five of six placements. This is
evidence of a competitive drop and partial recovery, not a causal estimate of
the restart rule's effect on the continuing wallpaper experiment.
