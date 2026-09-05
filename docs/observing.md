# Observe without steering

## Bounded headless world

```sh
cargo run --release -- --headless --random-founders --seed 42 --ticks 200000 --sample 1024 --output reports/seed42.json
```

`--ticks` is an additional tick budget, including when resuming a checkpoint.
The run ends at its horizon or extinction; extinction is detected within a GPU
batch of at most 32 ticks. A bounded diagnostic is not the unlimited visible loop.
Choose new output files. Reports include the build/model, settings, sampled
history, ancestry observations, and an explicit termination reason.

Optional `--families` adds read-only founder-family accounting for fresh worlds
of at most 200,000 ticks. It does not rank founders or train the brains.
`--survivors path.json` records late living genomes without changing selection
inside the world. See `cargo run --release -- --help` for the complete interface.

## Individual journeys

```sh
cargo run --release -- --headless --checkpoint path/to/world.checkpoint --ticks 16384 --sample 1024 --journeys reports/journeys.jsonl --journey-sample 32 --output reports/journey-world.json
```

This is a separate continuation of a save, not an observer attached to an already
running window. It uses GPU resources and can slow a concurrent viewer. It never
overwrites the source checkpoint or feeds observations back to brains.

The sampled journey definition requires collection at a source, depletion to
at most 25% of its observed peak and 0.02 food, departure by at least 48 units,
a sampled food-poor crossing of at least 48 net units, collection at least 96
units from the source, subsequent ingestion, and reproduction near the destination.
These are observer thresholds, not agent rules or rewards.

The observer samples every 32 ticks by default. It misses between-sample events;
food footprints are not whole ecological regions; a lost identity is not a
diagnosed death. Unfinished tracks are censored, not failed. A qualifying journey
does not prove planning, causal response to depletion, offspring survival, or
successful adaptation to a major geographic relocation.

Optional Python 3.11+ tools:

```sh
python tools/analyze_departures.py reports/journeys.jsonl --metabolic-cost 0.06 --movement-cost 0.01 --output reports/departures.json
```

Supply the actual checkpoint costs, not those example values if you changed them.
The tool uses only the standard library. `tools/audit_checkpoint_communication.py`
additionally requires NumPy (`python -m pip install -r tools/requirements.txt`).
It audits checkpoint counters and provable action suppression; it does not
establish that communication helps receivers or that unsuppressed actions occur.

## Back up an evolution run

```sh
python tools/backup_run.py --run runs/my-first-run --backup reports/my-backups
```

The standard-library tool incrementally archives completed worlds and full saves,
checks archive bytes against source SHA-256 values, and leaves originals alone.
It never controls the viewer, changes difficulty, restarts a run, or deletes old
files. It is a one-shot command, not a scheduler; the viewer itself handles its
five-minute loop autosaves. Incomplete `.partial` saves are ignored.

Checkpoint header/layout validation is not a GPU semantic load test. ZIP archives
are local copies, not off-device protection. Keep space available, copy valuable
archives elsewhere yourself, and preserve checksums/source settings when sharing.

## Limits worth keeping visible

- Five-minute saves are usually too far apart to reconstruct individual lives.
- Population relocation can reflect birth/death turnover rather than the same
  individuals crossing the map. Use identity-aware journey traces for that claim.
- Several cumulative GPU counters are 32-bit and can wrap on long, dense runs.
  Do not interpret a wrapped count as a decrease in activity or perfect accounting.
- GPU contention can vary population trajectories. Seeded does not promise
  bitwise replay across devices or schedules.
- Changed physical settings or manual food interventions confound simple
  before/after comparisons. Preserve that context instead of labeling it learning.
