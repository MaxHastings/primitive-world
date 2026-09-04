# Primitive World

One GPU artificial-life model: **recurrent-v1**, checkpoint **12**.
Agents inherit neural weights, keep private recurrent state, and directly request
movement, attention, interactions and reproduction. There is no destination
scorer, place-memory manager, automatic birth lottery or alternate controller.

The clean cutover is implemented. The bundled bank contains actual descendants
prepared on seeds 11 and 22. Held-out results and limitations are in
[the validation record](reports/RECURRENT_VALIDATION.md).

## Run

```powershell
cargo run --release
```

Requires Rust and a GPU supported by wgpu. On Windows, close an older copy
yourself before rebuilding its executable, or use a separate Cargo target
directory. We do not automatically stop running applications.

The window and inspector identify recurrent-v1/checkpoint 12. An already-running
old process will continue showing the old model until you launch the new build.
Old checkpoints are rejected, never migrated or deleted.

Defaults: 1,000 founders from `policies/recurrent-v1.json`, 16,384 body slots,
a 2048-unit square, eight directional food samples, four observed neighbors,
and sixteen recurrent state values per body. These are finite modeling and
engineering choices, not promises of intelligence or population equilibrium.

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
- `--famine-at T --restore-at U`: remove vegetation and stop regeneration at
  absolute tick T; restore the configured growth rate at U. Carried/dropped food
  remain. Restoration is not a map refill.
- `--save-checkpoint PATH`, `--export-founders PATH`: explicit artifacts.
  Export requires living descendants; export failure is recorded in the report.

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
