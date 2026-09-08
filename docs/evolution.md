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

At effective environment age zero, worlds begin with movement and collection only. Each additional action unlocks
independently for an increasing, neutral hash-selected share of living bodies:
reproduction over ticks 0–2,500. Social actions wait through the 50,000-tick
survival bootstrap: transfer unlocks over 52,500–65,000, signals over
65,000–80,000, and force over 80,000–100,000. At each interval's midpoint,
roughly half of the bodies can select that action; at its end, every body can.
The draw is stable for each body and has no connection to its energy, behavior,
genome quality, ancestry, or success. Their ordinary food patches start 100%
wider and recede to normal size while metabolism rises from .01 to .06. From
50,000 to 250,000, the normal-sized food territory becomes progressively more
mobile without changing metabolism or total normalized habitat. Habitat then
fragments through 500,000 while
preserving its keyframe mean; regional lean seasons strengthen through 750,000
while moving abundance elsewhere. Each pressure then remains capped. This shared
deterministic environmental schedule is not a reward, score, or different
comparison condition. Every new world begins at effective environment age zero,
so actions and ecological pressures restart from the same baseline.

World duration and effective environment age both begin at zero for every
comparison. A matched challenger shares that same fresh age with its incumbent;
no earned environmental floor carries difficulty between worlds.

The same seed controls geography, weather and starting bodies in each pair. The
ecology orientation is held constant for the incumbent/challenger pair, then
advances through the four quarter-turns for the next independent comparison.
This makes a fixed world-axis policy less reusable without giving the controller
the rotation as an input. GPU
competition can still vary trajectories. One paired seed is a deliberately small
comparison, not proof of general improvement. Moving to a fresh seed can produce
shorter worlds even after a candidate has won an earlier comparison.

## What is inherited

The saved founding group is the genotype being selected, including each founder
position's genome and the group's mixture and multiplicities. Default New Game
and headless runs create one seed-specific random genome per founding body.
An explicitly imported current-format bank repeats across founding positions.

Every challenger founder inherits an existing genome and then varies from it.
After an incumbent world ends naturally, a uniform sample of its terminal
descendant genomes supplies up to 10% of the inherited sources (floor, minimum
one); those sampled descendants remain unchanged as viable anchors. Up to 5% of
the remaining sources are fresh random genomes, and the rest are mutated from
the incumbent. Neither anchors nor immigrants use an individual score,
direction, or authored behavior. There is no identity ranking, genetic
clustering, novelty score or separate learned selector.

The same mutation law applies to challenger founders and paid births. Each
inherited genome independently samples a continuous log-uniform mutation
temperature from 1/8 to 8. That temperature scales both the chance that each
parameter changes (base .02) and its step size (base .03), bounded to [-4,4].
Thus most inherited brains receive slight variation while a few receive much
larger changes. At least one parameter changes in every inherited genome.

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
terrain are rebuilt. Manual ecological intervention is saved as part of the world
and remains eligible for population comparison; outcomes record whether they were
assisted. Read-only diagnostic observers do not alter eligibility.

There is one current model, `primitive-v24-delayed-social-fresh-worlds`: checkpoint 37,
game receipt 4, founder bank 13. Noncurrent files are rejected, never converted,
executed through a compatibility path, overwritten or deleted.

The viewer saves on explicit Save, menu, close and every five minutes of changed
state. Each experiment retains its six newest complete snapshots, and the whole
library is capped at 16 GiB while protecting each experiment's newest valid
snapshot. `--prune-saves` applies that retention policy on demand. A crash can lose
progress since the last save, including completed worlds. Raw headless checkpoints
are written when requested.

`--purge-legacy-saves` is an explicit cleanup for paired receipts that identify a
different model. Current-format retention never removes incompatible data on its own.

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
