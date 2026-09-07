# Play and save

Run `cargo run --release` or double-click `Play.cmd` on Windows. New Game creates
seed-specific random founding brains. Load Game resumes the complete population
comparison and current ecology. Configure population, seed, food growth, metabolism
and available interactions before starting; the physical rules stay fixed.

The overview shows whether the current population or its candidate is being
evaluated. Both face the same environment. A living candidate replaces the current
population as soon as it outlives the incumbent, then continues running. See
[evolution](evolution.md) for inheritance and comparison.

## Viewing

Space pauses; WASD/arrows pan; wheel zooms; Home fits the world; L changes the
information lens. Click a body to inspect energy, senses, actions, gates and memory.
The inspector follows individual identity rather than a reused slot.

1x targets 60 ticks/s; MAX pursues available throughput. Viewer refresh is 10, 30
or 60 FPS, default 30. Compute budget controls work/idle pacing, not GPU power.
Speed never skips sensing, gates or biological rules. Actual ticks/s and rendered
FPS are shown separately. Pause finishes already submitted bounded work.

```sh
cargo run --release -- --seed 42 --view-speed MAX --view-fps 30
```

## Saving

Save, Main menu, close and five-minute autosaves preserve changed state. A failed
save pauses and leaves earlier complete saves available. Append-only pairs
`save-*.json` and `save-*.checkpoint` belong together. Complete founding populations,
comparison state, current brains/memory, ecology and recent world history are saved.
No files are automatically removed. Default checkpoints are roughly 115–120 MB;
allow about 1.4 GB per hour of autosaves. A crash can lose work since the last save.

The background library scan skips incomplete or noncurrent data with a notice.
Storage defaults to the platform application-data folder; `PRIMITIVE_WORLD_SAVES`
overrides it. Import a current receipt with Load Game or:

```sh
cargo run --release -- --load-game path/to/save-123.json
```

The current model is `primitive-v10-live-winner-search`: receipts 4, checkpoints 24,
founder banks 9. Noncurrent data is rejected without conversion or deletion.
Exports require new paths. See [headless observation](observing.md),
[performance limits](performance.md), and the [implementation checklist](implementation-checklist.md).
