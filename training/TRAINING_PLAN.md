# Persistent evolutionary preparation — primitive-v3

This replaces the endpoint-only founding protocol. Its16-world extinction results
are preserved in reports/v3-founding-20260904 with the prior runner/protocol.
Extinction is a valid evaluation outcome, not an infrastructure failure.

## Two separate processes

Inside a world, agents gather, reproduce and die under the same rules as play.
No observer score changes actions, mutation, costs or survival. Death stays death.

Outside the world, the trainer preserves initial candidate genomes and evaluates
their genetic families, even when every test body dies. It selects candidates,
creates mutated copies, and launches new independent evaluations. This is an
explicit optimization process with an authored outcome objective, not a claim
that offline selection is natural or bias-free.

Default:100 selection rounds,4 independent islands,64 candidate genomes per
island,8 fresh founder bodies per genome per trial. No adult weight updates.
Default full-world costs and capacities remain fixed. Current node/body sizes
and mutation during births are unchanged.

By default initial genomes are random. --initial-bank PATH instead archives a
compatible named bank and partitions it cyclically across islands, without adding
noise or changing fitness, body rules or curriculum. With256 source genomes and
four64-family islands, every starting genotype is used exactly once. The authored
starter's origin is disclosed in STARTER_PLAN.md; it is not a learned default.

## What is measured and selected

An observer-only family tag follows each founder genome slot into all descendants.
GPU counters accumulate post-step living-body counts EVERY tick, not just at
report snapshots. Counters survive family extinction and body-slot reuse.

Observer schema2 additionally records birth investment, juvenile collection and
digestion, collection choices while local vegetation is present, maturity entries,
terminal descendant death classes and births to descendant parents. These are
diagnostics ONLY, not selection or curriculum inputs. Source and destination body
buffers distinguish this tick's deaths from stale slots. Food/energy totals round
each contribution to thousandths and use paired32-bit counters with64-bit carry.
The stationary maturity energy comparison ignores feeding, movement and all other
spending: being born below it is not inherently nonviable. Headless fresh worlds
start with founders, so birth/death accounting includes every measured descendant.

For each family, divide by initial founder count and the requested evaluation
window (never by its shorter time to extinction). Rank lexicographically by:

1. Descendant body-ticks in the second half of the requested trial.
2. Mature-descendant body-ticks over the whole requested trial.
3. All descendant body-ticks over the whole requested trial.
4. Original-founder body-ticks, as a fallback.

Average each component over three matched-context trials before ranking.
Raw birth counts, actions, communication, force, food gradients and routes do not
select a genome. The first objective prioritizes late family persistence; later
components provide partial credit even before sustainable families establish.
These are proxies, not a proof of long-term fitness: an early reproductive burst
may still earn partial credit and subsequently fail an endurance test.

A family is rooted in its ORIGINAL candidate genome. Within-world descendants
mutate normally, so family outcomes assess that candidate's reproductive process,
not identical clones forever. The outer loop retains the tested original candidate,
not the genome of a cherry-picked dying body or an unrecorded descendant.
This is deliberate fixed-genome evolutionary search with ordinary in-world
inheritance; claims of a single continuous evolutionary lineage would be false.

## Variation, fairness and persistence

Within each island, preserve the top quarter unchanged (at least two genomes).
Fill most remaining slots from uniformly chosen parents in the top half:
independently mutate2% of weights by uniform[-.1,.1], clipped to[-4,4].
This offline mutation scale differs explicitly from within-world birth mutation
(.03). About5% are fresh random candidates (at least one); there is no live-world
reseed. Shuffle exact fitness ties rather than preferring low array indices.

All candidates in a trial get equal founder counts and coexist in a mixed world.
Shuffle genome-to-body-slot assignment each trial and map results back correctly.
Repeat on three different seeds/orientations; this reduces placement luck but
does not eliminate ecological interactions or stochastic variation. Separate
islands protect alternatives without a behavioral diversity reward.

Every candidate bank, trial configuration, family outcome, selection ranking,
parentage and mutation-produced next bank is preserved. A round still selects
and mutates when all its trial worlds end in extinction. Invalid numbers, failed
population/birth accounting, changed artifacts, or corrupt reports are errors.

## Adaptive curriculum BETWEEN rounds

| Level | Tick cap | Habitat contrast |
| --- | ---: | ---: |
| 0 | 2,048 | 0 |
| 1 | 4,096 | 0 |
| 2 | 8,192 | .25 |
| 3 | 16,384 | .5 |
| 4 | 32,768 | 1 |
| 5 | 65,536 | 1 |

Each round uses rehearsal(level-1), frontier(level), and stretch(level+1), clipped
to the table. All three use distinct seeds. Starting at level0 intentionally
includes two comparable low-level trials and one longer stretch trial.
Whole-environment quarter-turns vary independently; brains receive no flag.

Starting at level3, rehearsal/frontier/stretch regeneration is .012/.01/.008;
earlier levels retain .01. Harder does not mean changing body costs, disabling
abilities, forcing travel, or constantly worsening one live world.

Advance an island one level only after TWO consecutive rounds in which at least
25% of its frontier families average at least one living descendant per founder
over the late window AND mature-descendant body-ticks per founder reach at least
10% of the full horizon. These thresholds are explicit provisional scheduler
choices, not biological laws or release acceptance thresholds.

If no frontier family has late-window descendants for two rounds, step down one
level, without reverting its genomes. Otherwise hold. Extinction alone neither
throws away selection data nor automatically changes difficulty.
Long uninterrupted user-played worlds remain useful complementary evidence:
changing resource pressure there supplies a manual curriculum to established
families. The trainer is not a replacement for observing that ecology.

## Fixed benchmarks and final development comparison

Before training, register all training seeds, two benchmark seeds, and four
separate evaluation seeds. The two8,192-tick full-contrast benchmark worlds run
at round0, every fifth round and the final round. These outcomes NEVER select
parents or set difficulty. Compare on this fixed task, not on raw scores from
changing curricula. All observed seeds become development data, not final holdouts.

Pool at most256 genomes with equal per-island quotas (the first genomes are the
retained elites, followed by mutants). Compare the FINAL pool with the frozen
initial pool on the four reserved development seeds, matched initial bodies,
rotation and settings. Default duration is200,000 ticks; --endurance-ticks allows
explicitly labelled shorter engineering pilots. It is not a success threshold.

No auto-promotion or replacement of the shipped default. Newer is not automatically
better. The broad eight-seed/migration/fun completion contract is unchanged.

## Running, stopping and resuming

python training/prepare.py --exe target/release/primitive_world.exe --directory reports/run-name

Use --plan-only to freeze a configuration without simulation. Default maximum
training budget is100×4×3×65,536 ticks, plus benchmarks/evaluations; actual budget
depends on curriculum and early extinction.100 rounds is a starting budget, not
a promise of competence. Check cumulative actual ticks and wall time in reports.

Headless trials stop at detected extinction, within one at-most32-tick GPU batch,
independent of metric intervals. Initially empty worlds stop at tick0. Trial
caps prevent endless easy-world runs; evaluation uses the SAME cap for competing
candidates. Outcomes retain extinction vs tick-limit status.

--resume --directory reports/run-name uses saved configuration, frozen executable,
completed reports and banks. It validates trainer/protocol/executable/artifact
hashes. It never silently overwrites a corrupt or partial report: such an error
needs explicit inspection. Interrupted logs are kept. No infinite retry loop.
The final bank is a candidate even if every test world died: results determine
its merit, not the existence of the output file.

New registrations include replay/training/prepare.py and its protocol files.
If the active trainer changes, invoke that frozen script with --resume and the
original --directory. Supplied initial banks are also frozen and hash checked;
resume rejects a replacement --initial-bank. The already-running random100-round
campaign retains its original in-memory code and has an exact replay snapshot.
