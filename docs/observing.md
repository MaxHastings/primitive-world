# Observe without steering

## Bounded headless world

```sh
cargo run --release -- --headless --single-world --seed 42 --ticks 200000 --sample 1024 --output reports/seed42.json
```

`--ticks` is an additional tick budget, including when resuming a checkpoint.
The run ends at its horizon or extinction; extinction is detected within a GPU
batch of at most 32 ticks. A bounded diagnostic is not the unlimited visible loop.
Choose new output files. Reports include the build/model, settings, sampled
history, ancestry, and an explicit termination reason.

## Rolling hereditary worlds

```sh
cargo run --release -- --headless --seed 42 --ticks 200000 --sample 4096 --output reports/evolution-seed42.json
```

`--ticks` bounds work across worlds. Natural extinction starts a fresh world from
unchanged pool records. `--comparisons` is unsupported. Samples include ancestry,
mean movement plus horizontal direction counts and bias (-1 all left, +1 all
right). It also reports the local food-gradient direction and the alignment of
movement with that gradient. These measures can expose directional lock-in and
resource avoidance; they do not reward, penalize, or alter agents.

In single-world diagnostics, optional `--families` adds read-only founder-family accounting for fresh worlds
of at most 200,000 ticks. It does not rank founders or train the brains.
`--survivors path.json` records late living genomes without changing selection
inside the world. See `cargo run --release -- --help` for the complete interface.

## Individual journeys

```sh
cargo run --release -- --headless --single-world --checkpoint path/to/world.checkpoint --ticks 16384 --sample 1024 --journeys reports/journeys.jsonl --journey-sample 32 --output reports/journey-world.json
```

This is a separate continuation of a save, not an observer attached to an already
running window. It uses GPU resources and can slow a concurrent viewer. It never
overwrites the source checkpoint or feeds observations back to brains.

The sampled journey definition requires collection at a source, depletion to
at most 25% of its observed peak and 0.02 food, departure by at least 48 units,
a sampled food-poor crossing of at least 48 net units, collection at least 96
units from the source, subsequent ingestion, and packet production near the destination.
These are observer thresholds, not agent rules or rewards. Packet production
is not proof of fusion or offspring survival. Journey files use schema 3.

The observer samples every 32 ticks by default. It misses between-sample events;
food footprints are not whole ecological regions; a lost identity is not a
diagnosed death. Unfinished tracks are censored, not failed. A qualifying journey
does not prove planning, causal response to depletion, offspring survival, or
successful adaptation to a major geographic relocation.

Optional Python 3.11+ tools:

```sh
python tools/analyze_departures.py reports/journeys.jsonl --metabolic-cost 0.06 --movement-cost 0.01 --output reports/departures.json
```

Supply the actual checkpoint costs, including metabolism for body and brain.
Range estimates remain optimistic bounds rather than full budgets.
The tools use only the standard library.
additionally requires NumPy (`python -m pip install -r tools/requirements.txt`).
It audits checkpoint counters and provable action suppression; it does not
establish that communication helps receivers or that unsuppressed actions occur.

## Measure tick throughput

```sh
cargo test --release profile_tick_throughput -- --ignored --nocapture --test-threads=1
```

This manual diagnostic prints the GPU/driver, batch throughput for worlds starting
with 32, 1,000 and 4,096 bodies, the cost of taking full survivor snapshots every small
batch, the current telemetry path, and individual GPU pass timestamps.
Each throughput case resets the same seeded world and warms up for 32 ticks;
populations can change during the following 512 ticks. Per-pass timings are from
one subsequent tick, not an average. Rendering and Windows event-loop waits are
excluded, so these numbers are not a promise of visible playback speed. Run without
other GPU workloads when comparing builds. GPU timestamp support is required.

Playback 128x requests 7,680 ticks per second (60 x 128); it cannot guarantee that
rate. Sensing every in-range food cell, recurrent decisions, ecology, GPU dispatch
and synchronization all cost time even when presentation is infrequent.

## Back up an evolution run

```sh
python tools/backup_run.py --run path/to/experiment --backup reports/my-backups
```

The standard-library tool incrementally archives complete game receipts and their checkpoints,
checks archive bytes against source SHA-256 values, and leaves originals alone.
It never controls the viewer, changes difficulty, restarts a run, or deletes old
files. It is a one-shot command, not a scheduler; the viewer itself handles its
five-minute autosaves. Incomplete `.partial` saves are ignored. Headless evolution saves a complete checkpoint directly with --save-checkpoint.

Checkpoint header/layout validation is not a GPU semantic load test. ZIP archives
are local copies, not off-device protection. Keep space available, copy valuable
archives elsewhere yourself, and preserve checksums/source settings when sharing.

## Limits worth keeping visible

- Five-minute saves are usually too far apart to reconstruct individual lives.
- Population relocation can reflect birth/death turnover rather than the same
  individuals crossing the map. Use identity-aware journey traces for that claim.
- Accounting counters have explicit finite horizons. Overflow latches an engine
  stop; do not interpret the censored boundary as ecological extinction. Food
  ingestion uses a paired low/high counter.
- GPU contention can vary population trajectories. Seeded does not promise
  bitwise replay across devices or schedules.
- Changed physical settings or manual food interventions confound simple
  before/after comparisons. Preserve that context instead of labeling it learning.

The current controller has 1–16 active units. Evolution snapshots include the
capacity distribution and per-living-body sums of absolute recurrent and learned
state. Cognitive upkeep and write-energy counters are quantized to thousandths.
Memory samples use the effective inherited-plus-learned readout; their context
contains carried food and underfoot resource, and actual_action records the
selected action. None of these diagnostics feeds the controller or hereditary pool.
