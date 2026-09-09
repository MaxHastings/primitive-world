# Play and save

On Windows, run `Build.cmd` once, then double-click **Primitive World.exe** in the
project folder to resume wallpaper. No terminal is created or required. With no
saved experiment in the current model, it creates one. Startup is opt-in and is
controlled through the tray menu; launching the app never enables it by itself.

For a regular window, double-click `Play.cmd` or run
`cargo run --release -- --viewer`. New Game creates seed-specific random founding
brains. Load Game resumes the hereditary pool and current ecology. Configure
population, seed, food growth, metabolism and available interactions before
starting; the physical rules stay fixed.

The tray icon may be inside Windows' **^** notification-area overflow. Right-click
it for:

- **Pause simulation / Resume simulation**: the label follows the current state.
  Double-clicking the icon also toggles pause.
- **Save now**: writes a resumable save and confirms through a tray notification.
- **Open saves folder**: opens the experiment library in Explorer.
- **Start this copy with Windows**: checked when the current executable is
  registered to resume wallpaper at sign-in. Enabling it from a different copy
  replaces the previously registered path; disabling removes the login entry.
- **Save and quit**: saves before exiting. A save failure displays an error and
  keeps the app paused so you can retry.

Copy the executable to its permanent location before enabling startup. Windows
Startup Apps settings or organizational policy can override login registration.
To update from source, choose **Save and quit**, then run `Build.cmd` again. Builds
protect a running executable instead of stopping its experiment to replace it.

The command `.\Play.cmd --wallpaper --resume` also builds and launches the
wallpaper, then returns immediately. You may close that terminal. Use
`--wallpaper` without `--resume` to start a fresh experiment explicitly.
`Play.cmd --install-startup` and `Play.cmd --uninstall-startup` remain available,
as does `--stop-wallpaper` to request save-and-close. Headless, help and maintenance
commands remain synchronous and keep their standard output and exit codes.

Wallpaper prevents duplicate wallpaper instances; normal viewers and headless
diagnostics can run separately. It refreshes tray registration when Explorer
changes. Startup errors are displayed in a dialog; wallpaper diagnostic output
is written to `%LOCALAPPDATA%\PrimitiveWorld\logs\wallpaper.log`. The log resets
on launch after it exceeds 2 MiB. Double-clicking an already running copy points
you to its existing tray controls.

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
to save and exits; double-click the executable to resume it again. Multiple-monitor
layouts require verification on the target desktop; hosting uses one window, not
one per monitor.

On raised Windows 11 desktops, the viewer is an opaque layered child between
Explorer's icons and its background `WorkerW`. Windows MSVC builds embed the
compatibility manifest needed for that composition path. Older desktop layouts
use the dedicated background `WorkerW`; an unsupported layout reports an error
instead of silently attaching behind an opaque background.

The overview shows the current world and physical observations. Natural extinction
starts a new world from the blind hereditary pool. See [evolution](evolution.md).

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
`save-*.json` and `save-*.checkpoint` belong together. The hereditary pool,
its RNG streams, current brains/memory, ecology and recent world history are saved.
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

The current model is `primitive-v35-body-frame-contact`: receipts 4,
checkpoints 54, founder banks 19. Noncurrent data is rejected without conversion
or deletion.
Exports require new paths. See [headless observation](observing.md),
[performance limits](performance.md), and the [implementation checklist](implementation-checklist.md).
