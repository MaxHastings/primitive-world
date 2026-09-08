# Play and save

Run `cargo run --release` or double-click `Play.cmd` on Windows. New Game creates
seed-specific random founding brains. Load Game resumes the complete population
comparison and current ecology. Configure population, seed, food growth, metabolism
and available interactions before starting; the physical rules stay fixed.

Use `Play.cmd --wallpaper` for a fixed-camera desktop terrarium. The launcher
builds and runs `target/play/release/primitive_world.exe`, so it does not reuse
an arbitrary older target directory.

On Windows, run `.\Play.cmd --install-startup` to build the current executable
and register it to start the wallpaper automatically when you sign
in. It resumes the newest saved experiment in the current model and creates one
if no save exists. Re-run `Play.cmd` after source updates to rebuild the executable
used at login. Use `.\Play.cmd --wallpaper --resume` to resume manually and
`.\Play.cmd --uninstall-startup` to
remove that login entry. The wallpaper prevents duplicate wallpaper instances;
normal viewers and headless diagnostics can still run separately. Use its tray
menu to pause/resume or quit, or `--stop-wallpaper` to request save-and-close.
It refreshes tray registration when Explorer changes.
The habitat uses the desktop host's pixel dimensions. When resuming a differently
sized habitat, a separate descendant experiment is created; the original save,
world age, and ecology remain available in Load Game. Matching sizes resume directly.
The compact top-right strip shows the world number and living population. Click
the current view or speed to open its menu; Details shows world ticks, population
history, survival records, and performance. Click outside an open menu to dismiss
it without adding food. Manual food additions are saved as part of the experiment.

Blank desktop clicks add food and the HUD's speed and view controls are routed
through Explorer without intercepting icon or taskbar clicks. Use the tray menu
for pause and quit. If Explorer destroys the wallpaper host, the viewer attempts
to save and exits; launch it again with `--wallpaper --resume`. Multiple-monitor
layouts require verification on the target desktop; hosting uses one window, not
one per monitor.

On raised Windows 11 desktops, the viewer is an opaque layered child between
Explorer's icons and its background `WorkerW`. Windows MSVC builds embed the
compatibility manifest needed for that composition path. Older desktop layouts
use the dedicated background `WorkerW`; an unsupported layout reports an error
instead of silently attaching behind an opaque background.

The overview shows whether the current population or its candidate is being
evaluated. Both face the same environment. A living candidate replaces the current
population as soon as it outlives the incumbent, then continues running. See
[evolution](evolution.md) for inheritance and comparison.

## Viewing

Space pauses; WASD/arrows pan; wheel zooms; Home fits the world; L changes the
information lens. Click a body to inspect energy, senses, actions, gates and memory.
The inspector follows individual identity rather than a reused slot.

1x targets 60 ticks/s; MAX pursues available throughput. Viewer refresh is 10, 30,
60, 120, 144 or 240 FPS; wallpaper mode defaults to the monitor refresh. Compute
budget controls work/idle pacing, not GPU power.
Speed never skips sensing, gates or biological rules. Actual ticks/s and rendered
FPS are shown separately. Pause finishes already submitted bounded work.

```sh
cargo run --release -- --seed 42 --view-speed MAX --view-fps 30
```

## Saving

Save, Main menu, close and five-minute autosaves preserve changed state. A failed
save pauses and leaves earlier complete saves available. Snapshot pairs
`save-*.json` and `save-*.checkpoint` belong together. Complete founding populations,
comparison state, current brains/memory, ecology and recent world history are saved.
Each experiment retains its six newest complete snapshots. The complete library is
also capped at 16 GiB; if it reaches that limit, the oldest extra snapshots are
removed across experiments, while each experiment's newest valid snapshot is always
preserved. Run `primitive_world.exe --prune-saves` to apply the same cleanup now.
A crash can lose work since the last save.

The background library scan skips incomplete or noncurrent data with a notice.
Storage defaults to the platform application-data folder; `PRIMITIVE_WORLD_SAVES`
overrides it. Import a current receipt with Load Game or:

```sh
cargo run --release -- --load-game path/to/save-123.json
```

The current model is `primitive-v26-masked-plastic-16`: receipts 4, checkpoints 39,
founder banks 15. Noncurrent data is rejected without conversion or deletion.
Exports require new paths. See [headless observation](observing.md),
[performance limits](performance.md), and the [implementation checklist](implementation-checklist.md).
