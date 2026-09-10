# Long-term wallpaper experiment

Status: frozen experiment candidate. The owner will run the 20–50 million tick
experiment; unassisted life-cycle closure is an outcome to measure.

## Launch the accepted build

From the project directory, start a fresh wallpaper experiment with:

```powershell
.\Play.ps1 --wallpaper --seed 3001 --view-speed 32x
```

The command selects 32x (target 1,920 ticks/second; actual speed depends on load).
Choose MAX only if you want an uncapped run; 1x targets 60 ticks/second. The tick counter in Details tracks the current world; experiment
progress also records cumulative ticks across natural restarts. There is no automatic
wallpaper stop at 20 or 50 million ticks.

The script builds and updates `Primitive World.exe`, launches it and returns.
With no arguments, `Play.ps1` opens the viewer instead. The explicit seed records
the starting random initialization; it is not a curated controller or founder bank.
Wallpaper habitat dimensions follow the desktop, so its results need not match a
square headless world even with the same seed.

Use the tray's Save and quit command before closing an experiment deliberately.
To resume the most recently saved experiment:

```powershell
.\Play.ps1 --wallpaper --resume --view-speed 32x
```

Resume restores saved state and settings. It is not a fresh random trial.
For an exact experiment, use its saved receipt through the existing load-game
flow instead of relying on which save happens to be most recent.

## Record before the long run

Record the accepted main commit, executable SHA-256, model, initial seed, desktop
size, playback rate and experiment receipt. Retain the original starting save.
Current model: `primitive-v45-depth-retention`; checkpoint layout: 58. Historical
model saves are preserved and rejected rather than silently reinterpreted.

Do not paint food, move bodies, change settings or import founders during an
unassisted measurement. Observe and inspect freely. Keep biology frozen and let the long experiment reveal persistence or extinction;
there is no requirement for a preferred social strategy to emerge.

Engineering validation should include the complete regression suite and an actual desktop
check of launch, rendering, controls, saving, quitting and resuming. Short replay
checks do not establish multi-day stability. Record the measured operational
horizon and any remaining limits with the release.
