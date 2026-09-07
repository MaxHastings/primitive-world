# Evolution across worlds

The goal is longer populated worlds through feeding, survival, reproduction and
continued ecological generations. The only selection score is the number of
biological ticks from founding through natural extinction. Feeding, births and
ancestry are observations, never added rewards.

## One current population and one candidate

1. Evaluate the current founding population in a seeded world until natural extinction.
2. Make a candidate from that saved founding group. Reset the same environment,
   body positions, starting resources, ages and physical settings, and evaluate it.
3. The instant a living candidate strictly outlasts the incumbent duration, accept
   it. Keep its live bodies, ecology and descendant genomes running; do not reset
   a winner merely because it has proved itself. A candidate that goes extinct at
   or before the incumbent duration loses.
4. When a current world later ends naturally, start its next matched comparison.

A comparison starts with an incumbent's complete world. Its challenger has no
time limit: it wins as soon as it is still alive beyond that recorded duration.
A winning world continues indefinitely, including its ordinary paid births and
mutations. Pause, saving, a headless tick budget, or the integer tick-capacity
guard never count as extinction or a losing score. GPU batches can finish up to
31 empty ticks after extinction; those ticks count as executed work but do not
increase the recorded world duration.

The same seed controls geography, weather and starting bodies in each pair. GPU
competition can still vary trajectories. One paired seed is a deliberately small
comparison, not proof of general improvement. Moving to a fresh seed can produce
shorter worlds even after a candidate has won an earlier comparison.

## What is inherited

The saved founding group is the genotype being selected, including each founder
position's genome and the group's mixture and multiplicities. Default New Game
and headless runs create one seed-specific random genome per founding body.
An explicitly imported current-format bank repeats across founding positions.

Each candidate copies the full current founding group. After an incumbent world
ends naturally, a uniform sample of its terminal descendant genomes replaces up
to 5% of founder positions (floor, minimum one). This uses the ordinary sparse
variation budget: if a world produced fewer descendants, the remaining positions
are parameter-mutated copies of the incumbent. Every fourth candidate also
replaces 1% of positions with fresh random genomes (floor, minimum one). These
positions are disjoint; a one-founder exploratory candidate is entirely random.
Other positions remain unchanged. There is no identity ranking, genetic
clustering, novelty score or separate learned selector.

Parameter mutation uses the world settings, default probability .02 and magnitude
.03, bounded to [-4,4]. For candidate construction only, if all mutation draws leave
a selected genome unchanged, one parameter is nudged toward zero by the configured
magnitude where that change is representable; otherwise it is counted unchanged.
Setting either mutation
setting to zero disables parameter changes; periodic random exploration remains.
Births keep the ordinary mutation rule without the candidate fallback.

**Within-world descendant genomes are retained as potential, not automatically
accepted, founders.** At natural extinction, each terminal descendant slot has
the same chance to enter the next challenger; no lifespan, birth count, behavior
or survivor score ranks it. A descendant genome carries forward only if its
challenger population is still living beyond the incumbent's matched natural
duration. This keeps whole-population longevity as the only selector while
preventing a reset from discarding all evolved mutations.

Fresh founders and biological newborns retain their existing different endowments.
All new bodies start with zero recurrent memory. Founding is initialization, not a
paid birth. Biological births still require maturity, energy, recovery and a slot.
Weights are fixed during each life; gated private state changes every tick.

## State and progress

The overview names the current population or candidate, comparison number,
matched baseline duration, founder changes, and accepted candidates. A bounded
history retains the latest 64 completed worlds, with seed, population and parent
population IDs, exact duration, births, highest ancestry generation (founders are
zero), collected food, digested food and the comparison result. These are not
claims of genetic diversity or intelligence.

Checkpoints preserve current bodies, hidden memory, all live genomes, ecology,
traces, counters, the complete current/candidate founding groups, paired baseline,
completed outcome, history, provenance and search RNG. Saving at extinction before
advancing is valid; resuming cannot score that world twice. Derived indexes and
terrain are rebuilt. Manual ecological intervention makes a world ineligible for
population comparison. Read-only diagnostic observers do not alter eligibility.

There is one current model, `primitive-v10-live-winner-search`: checkpoint 24,
game receipt 4, founder bank 9. Noncurrent files are rejected, never converted,
executed through a compatibility path, overwritten or deleted.

The viewer saves on explicit Save, menu, close and every five minutes of changed
state. Saves remain append-only; no automatic deletion. A default checkpoint is
roughly 115–120 MB depending on phase. At twelve autosaves/hour, allow roughly
1.4 GB/hour, plus explicit saves. A crash can lose progress since the last save,
including completed worlds. Raw headless checkpoints are written when requested.

## Command line

```sh
cargo run --release -- --headless --seed 42 --ticks 200000 --sample 1024 --output reports/evolution.json --save-checkpoint reports/evolution.checkpoint
cargo run --release -- --headless --checkpoint reports/evolution.checkpoint --ticks 200000 --output reports/continued.json --save-checkpoint reports/continued.checkpoint
```

Headless mode defaults to the same population loop as the viewer. `--ticks` is
an additional execution budget across worlds. `--sample` controls reporting,
not selection. A living world at the budget remains unscored. Existing output
paths are refused; checkpoint settings cannot be overridden.

`--headless --single-world` explicitly selects bounded diagnostics, stopping at
extinction or its horizon. Family, journey, survivor and famine options require
that mode. They are observation/intervention tools, not a second evolution engine.
