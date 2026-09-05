# Playing Primitive World

Run `cargo run --release`, or double-click `Play.cmd` on Windows. The main menu
opens with **New Game** and **Load Game**. Both use round-based evolution.

## New Game

Name your experiment and choose its world rules. Evolution setup controls the
number of populations, matched environments, batches, and retention rounds.
Defaults are three populations, two environments, three batches per round, and
four-round retention. Start evolution begins running immediately.

The initial world gathers lifetime records. After natural extinction, the selector
proposes different founding populations from those records. Each population faces
the same environment seeds. The pool stays available across attempts; the selector
learns after an entire batch ends. Between rounds, a bounded sample from the trial
worlds replaces old pool entries. The game keeps advancing rounds until you stop.
No target behavior, rescue population, or artificial extinction deadline is added.

The overview displays the current round, batch, population, environment, candidate
pool, incoming candidates, and selector updates. The last completed comparison
shows each population's world durations. World rules remain fixed throughout a
save so the comparisons mean the same thing. Configure a New Game to change them.

## Controls

| Control | Effect |
| --- | --- |
| Space / Pause | Pause or resume the current trial |
| Step | Run one simulation tick |
| Speed menu | 1x through 128x, or MAX |
| WASD / arrows | Pan |
| Mouse wheel | Zoom |
| Home | Fit the world |
| L / Lens menu | Change information lens |
| Click a body | Inspect that individual |
| Escape / Menu | Save and return to the menu |

1x targets 60 simulation ticks per second. Display FPS and compute budget control
presentation and pacing, not the learning reward. The inspector follows the exact
individual; death does not substitute a new organism reusing the same slot.

## Save and Load Game

Save preserves the physical world, private neural state, current lifetime archive,
fixed candidate pool, incoming samples, proposed populations, completed durations,
selector weights, optimizer, and random state. Closing saves too. Autosaves occur
every five minutes while running and around natural world transitions. A pause or
close does not award an extinction reward or refresh the pool.

Load Game lists the latest complete save for each experiment. It opens paused at
the saved trial. Resume continues the current world, including a saved extinction
transition, without awarding the same batch twice. Import save accepts the complete
`save-*.json` receipt. Its paired checkpoint must remain beside it.

```sh
cargo run --release -- --load-game path/to/save-123.json
```

On Windows, managed experiments live in `%LOCALAPPDATA%/PrimitiveWorld/experiments`.
`PRIMITIVE_WORLD_SAVES` can select another folder. Receipts are published only after
their complete checkpoint. Incomplete or incompatible saves are skipped and a notice
appears. Older models are not supported or migrated. Saved files are never deleted
automatically; back up an experiment folder and its checkpoints together.

Game receipts use format 2, viewer round snapshots format 1, training state format
2, selector weights format 4, and physical checkpoints format 20. Founder genomes
remain format 7. These format identities are separate from application version.

## Command-line starts

```sh
cargo run --release -- --random-founders --seed 42 --view-speed 16x
```

This opens a fresh round-based evolution directly in the viewer. Headless training
uses the same engine through `--train-loop`; see [evolution](evolution.md).

## Reading behavior

Collection is chosen; digestion of carried food is automatic. Force displaces
another body. Signals are local numbers without a prescribed meaning. None of
these abilities is required to become useful. Longer world duration is the
selector's evidence, not proof of intelligence or sustained improvement.

Export living descendants saves genes, not the complete game. Export history
writes the latest 400 metric samples under `reports/history/`. Neither replaces
Save. See [release status](release.md) for model limits.
