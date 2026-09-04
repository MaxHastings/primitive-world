# Primitive World

**Playable build 0.2.0:** [quick play guide](PLAY_GUIDE.md) ·
[calibration and held-out results](reports/PLAYABLE_RELEASE.md).

One GPU artificial-life model: **recurrent-v1**, checkpoint **12**.
Agents inherit neural weights, keep private recurrent state, and directly request
movement, attention, interactions and reproduction. There is no destination
scorer, place-memory manager, automatic birth lottery or alternate controller.

The clean cutover is implemented. The bundled bank contains actual descendants
prepared on seeds 11 and 22. Held-out results and limitations are in
[the validation record](reports/RECURRENT_VALIDATION.md).
The newer prepared bank did not meet the separate promotion gate; it remains
experimental rather than silently replacing these released weights.

## Run

```powershell
.\Play.ps1
```

Requires Rust and a GPU supported by wgpu. On Windows, close an older copy
yourself before rebuilding its executable, or use a separate Cargo target
directory. We do not automatically stop running applications.
You can also double-click **Play.cmd**. The launcher builds into `target/play`,
separate from older `target/release` builds, and then starts the game. It forwards
CLI arguments, e.g. `.\Play.ps1 --seed 42`. Close a previous copy launched from
`target/play` before rebuilding that same copy. No app is stopped automatically.

The window and inspector identify recurrent-v1/checkpoint 12. An already-running
old process will continue showing the old model until you launch the new build.
Old checkpoints are rejected, never migrated or deleted.

Defaults: 1,000 founders from `policies/recurrent-v1.json`, 16,384 body slots,
a 2048-unit square, eight directional food samples, four observed neighbors,
and sixteen recurrent state values per body. These are finite modeling and
engineering choices, not promises of intelligence or population equilibrium.

Fresh worlds use **0.06 metabolism**, **0.01 movement cost**, regeneration
**0.010**, and **motor response gain 4**. The lower-cost experiment (0.005/0.002)
made survival possible but often reached the storage ceiling; it is no longer
the default. Instead, continuous motor sensitivity is calibrated independently
of energy costs. Zero intent still stops; maximum speed remains 1.2. This is an
explicit physical parameter, not a migration rule or learned intelligence.

The original response is available with `--motor-gain 1`. Compatible old
checkpoint-12 files without that setting retain gain1, and all checkpoints
restore their own costs. New checkpoints contain the new setting and should
be opened with this build, not an older executable. Running/previously built
processes retain their old settings; rebuild/relaunch for fresh-world defaults.
Reset uses current sliders and the loaded founder bank, not the world's current
survivors. Export descendants explicitly if you want to reuse their weights.

## Controls

- Space: pause/resume. UI: step and speed. Keys 1, 2, 4, 8, 6, M select speeds.
- WASD/arrows: pan; wheel: zoom; Home: camera reset; L: cycle lens.
- Select a body for raw observations, intentions, actual consequences and state.
- UI provides explicit food/kill interventions, reset, checkpoints and history.

Save uses `recurrent-world.checkpoint`; load restores that world and pauses.
A save refuses to overwrite an existing file: preserve/rename it before saving
again. History export uses `recurrent-history.json` and replaces that export.
No automatic reset or reseeding follows extinction.

## Headless operation

```powershell
cargo test -- --test-threads=1
cargo run --release -- --headless --seed 101 --ticks 12000 --sample 1000 --output reports/my-run.json
```

Headless runs use GPU compute without creating a window. The report directory
must exist; report, founder-export and checkpoint paths must be new.
`--help` lists supported options; `--version` identifies the model without
opening a window. No Python is needed for normal simulation.

- `--bootstrap`: explicitly unprepared, mutable seed weights with standing variation.
- `--founders PATH`: another compatible recurrent-v1 bank; missing/invalid files fail.
- `--checkpoint PATH`: resume saved settings, weights, private state and tick.
  It cannot be combined with seed/founder/world-setting overrides.
- `--no-force`, `--no-signals`: disable those physical capabilities.
- `--static-landscape`: freeze geography, not weather.
- `--metabolic-cost X`, `--movement-cost X`, `--motor-gain X`: explicit body
  settings, also available as live controls. Cannot override saved checkpoints.
- `--famine-at T --restore-at U`: remove vegetation and stop regeneration at
  absolute tick T; restore the configured growth rate at U. Carried/dropped food
  remain. Restoration is not a map refill.
- `--save-checkpoint PATH`, `--export-founders PATH`: explicit artifacts.
  Export requires living descendants; export failure is recorded in the report.

The inspector can export living descendants and load a compatible bank into a
new world. Loading a bank resets personal experience; loading a checkpoint
restores it. Exporting a bank neither validates it nor changes the default bank.

Headless reports include sampled path distance versus endpoint displacement,
matched only for agents alive at consecutive observations. This distinguishes
some circling from net movement, but is not a route or intelligence score.

`training/prepare.py` uses Python's standard library, registers a finite run
budget before launching, and never selects weights from evaluation worlds:

```powershell
cargo build --release
python training/prepare.py --directory reports/new-campaign
```

## What this model does—and does not—claim

Weights change only through birth mutation. During life the controller changes
its sixteen numerical memory values, not its weights. Those values have no
assigned meanings, guaranteed storage duration, or built-in map. A finite
recurrent controller can retain information; useful memory and navigation must
still arise from its weights and experience.

The initialization is not a blank slate: mutable seed weights favor basic
feeding, local food-directed movement and reproduction. The documented
preparation produced viable descendants, not evidence that these capabilities
were discovered from nothing. We still author bodies, sensors, ecology,
initial conditions, network size and mutation.

The ecology was preserved: fertile hubs, low-yield bands and barren gaps,
harvest depletion, rain/drought and geography blending every 8,192 ticks.
Existing bands are authored terrain, not agent-built roads. Clusters or visible
tracks alone are not evidence of social organization or route planning.
All seven campaign runs reproduced; none exercised transfer, force or signaling
in ordinary operation. Wiring tests exercise those actions separately.

Read [CONTROLLER.md](CONTROLLER.md) for the exact neural contract,
[KERNEL_SPEC.md](KERNEL_SPEC.md) for costs and physical resolution, and
[DESIGN_PLAN.md](DESIGN_PLAN.md) for the cutover decision.
The old implementation remains at Git tag `pre-recurrent-cutover`.
Historical reports are retained as history, not descriptions of this runtime.
