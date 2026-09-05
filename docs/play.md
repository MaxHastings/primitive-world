# Playing Primitive World

## Start a world

Run this from the repository root:

```sh
cargo run --release
```

The home screen opens first. Choose **New Game**, name the experiment, choose its
seed and conditions, and press **Start evolution**. New games use seed-specific
random, untrained brains and immediately begin running. No starter policy or
survival template is supplied. The app saves experiment receipts under its local
data folder (on Windows, `%LOCALAPPDATA%\PrimitiveWorld\experiments`).

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
round limit. Closing the viewer saves the current world and stops; it does not
secretly reopen.

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
| Lens menu, lower-left | Change information lens |
| Click a body | Inspect that individual |

The inspector follows identity, not just a reusable storage slot. Death ends
tracking and leaves a labeled snapshot. A newborn is not silently substituted.
The displayed observation has a tick label; it precedes the next frame’s steps.

Physical controls affect the live world. Seed and initial population apply at
the next reset/world creation. Adding/removing food and killing bodies are manual
experiments, not hidden assistance. Record interventions when comparing results.

## Saves and gene pools

**Save** preserves bodies, genes, private state, settings, and the current
survivor-transfer archive. The status line shows the result. **Menu → Load Game**
lists the latest complete save for each experiment; interrupted or incompatible
saves are left untouched and skipped. Continuing a save opens it paused. **Use
brains in a new world** branches from its living or archived survivors.

**Import save…** can import a compatible experiment receipt or a raw checkpoint.
The imported data becomes a new local experiment, leaving the source untouched.

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

**Export living descendants** in the Experiment tab saves a sample of descendant
genomes for fresh bodies in another world. It does not save settings or memories.
No living descendants means there is nothing eligible for this particular export.
The automatic evolution loop separately samples late survivors, including founders.

**Export history** writes a uniquely named JSON file under `reports/history/`.
It contains the latest 400 metric samples, not the entire run. Old exports are not
overwritten. Use headless sampling for a complete bounded diagnostic history.

Checkpoints use format 16; founder banks use format 5. Primitive-v4 intentionally
rejects V3 saves and banks because its expanded brain has a different genome
layout. Unsupported formats are
rejected without modifying the file. See [release status](release.md).

## Reading the behavior

Collection is a chosen action. Digestion of carried food is automatic. An agent
walking through food need not collect it. Force displaces another body; it does
not directly inflict damage or spill food. Signals are local numbers with no
built-in meaning. No ability is required to become useful.

A sparse population may recover or die. A long world may contain many generations
or a lingering bottleneck. Look at births, population history, feeding, and actual
journeys together—not just the final survival tick.
