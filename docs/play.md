# Playing Primitive World

## Start a world

Run commands from the repository root:

```sh
cargo run --release -- --seed 42
```

The default is a frozen set of 256 reproducible **random**, untrained genomes,
cycled across 1,000 initial bodies. `--random-founders` instead generates a genome
for each initial body from the chosen seed. Neither option supplies a survival
template. `--founders path/to/pool.bank.json` explicitly uses a compatible gene pool.

Windows users can run `Play.cmd` or `Play.ps1`; arguments pass to the simulator.
If rebuilding an executable that is already running fails with an access error,
leave that process alone and use `cargo run --release --target-dir target/another-build`.
This changes the build location, not the simulation model.

## Continuous evolution

```sh
cargo run --release -- --random-founders --seed 42 --watch-loop runs/my-first-run --view-speed 16x
```

Use a new directory for each launch. Parent directories are created automatically.
`1x`, `2x`, `4x`, `8x`, `16x`, and `MAX` are available; actual throughput depends on
your GPU, population, and observation workload.

At extinction, survivor genes seed a new world in the same window. Speed, camera,
lens, and final physical settings carry forward. There is no world-length or
round limit. Loading another world and resetting are disabled in this mode.
Closing the viewer saves the current world and stops; it does not secretly reopen.

The viewer writes a full checkpoint at startup and every five wall-clock minutes
while a loop is active and its state changes. Saving briefly stalls the viewer.
Autosaves appear in the run’s `checkpoints/` folder; closing writes
`world-NNNNNN/paused.checkpoint`. Completed worlds also contain survivor banks,
reports, and the parent-to-child handoff records. There is no automatic retention
deletion: long runs accumulate files. These are local backups, not protection
against disk failure.

## Controls

| Control | Effect |
| --- | --- |
| Space | Pause/resume |
| WASD / arrows | Pan |
| Mouse wheel | Zoom |
| Home | Fit the world |
| L | Change information lens |
| Click a body | Inspect that individual |

The inspector follows identity, not just a reusable storage slot. Death ends
tracking and leaves a labeled snapshot. A newborn is not silently substituted.
The displayed observation has a tick label; it precedes the next frame’s steps.

Physical controls affect the live world. Seed and initial population apply at
the next reset/world creation. Adding/removing food and killing bodies are manual
experiments, not hidden assistance. Record interventions when comparing results.

## Saves versus gene pools

**Save new checkpoint** preserves bodies, genes, private state, settings, and the
world in a new file under `reports/checkpoints/`. The status message shows its path.
**Load checkpoint** restores the entered path paused in ordinary play.

To open a checkpoint from the command line:

```sh
cargo run --release -- --checkpoint path/to/world.checkpoint
```

To continue its evolution loop:

```sh
cargo run --release -- --checkpoint path/to/world.checkpoint --watch-loop runs/resumed-run --view-speed 16x
```

CLI checkpoint launches begin running. The checkpoint’s settings take precedence;
physical overrides cannot accompany `--checkpoint`. The new loop’s world numbering
starts at 1, but the loaded tick, bodies, genes, and state are preserved.

**Export living descendants** saves a sample of descendant genomes for fresh
bodies in another world. It does not save settings or memories. No living
descendants means there is nothing eligible for this particular export. The
automatic evolution loop separately samples late survivors, including founders.

**Export history** writes a uniquely named JSON file under `reports/history/`.
It contains the latest 400 metric samples, not the entire run. Old exports are not
overwritten. Use headless sampling for a complete bounded diagnostic history.

Checkpoints use format 15; founder banks use format 4. Unsupported formats are
rejected without modifying the file. See [release status](release.md).

## Reading the behavior

Collection is a chosen action. Digestion of carried food is automatic. An agent
walking through food need not collect it. Force displaces another body; it does
not directly inflict damage or spill food. Signals are local numbers with no
built-in meaning. No ability is required to become useful.

A sparse population may recover or die. A long world may contain many generations
or a lingering bottleneck. Look at births, population history, feeding, and actual
journeys together—not just the final survival tick.
