# Primitive World — physiology-v2 research branch

**Experimental build0.3.1-dev · checkpoint14 · unprepared default founders.**
This branch is not promoted. Main remains recurrent-v1/build0.2.0/checkpoint12.
The broader [completion contract](GOAL_CONTRACT.md) is active and unachieved.

The model gives bodies local measurements,16 recurrent state values,6 action
choices and simultaneous continuous movement. Their1518 neural weights change
only through birth mutation. Gathering and reproduction are chosen; digestion of
carried food is automatic and bounded. There is no destination planner, scripted
migration, automatic population rescue or reward for desired-looking behavior.

## Run or inspect

See [the play guide](PLAY_GUIDE.md) for controls and persistence. To launch this
experimental source yourself, use `.\Play.ps1` or double-click Play.cmd. It
builds into target/play and does not stop an older running application.
Rust and a supported GPU are required; no Python trainer or server is needed.

For headless operation:

```powershell
cargo build --release
.\target\release\primitive_world.exe --version
.\target\release\primitive_world.exe --headless --seed 808 --ticks 200000 --sample 1024 --output reports/my-v2-world.json
```

Use a new output filename. Inspect `--help` for options. `--founders PATH`
loads a compatible bank; `--export-founders PATH` samples living descendants
at the end; `--save-checkpoint PATH` preserves a whole world. Existing saves
and founder exports are not overwritten. V1 banks/checkpoints are rejected,
never silently converted. Do not load experimental banks into main.

Fresh defaults:1000 bodies,16,384 available slots,2048-unit world, metabolic
cost0.06, movement cost0.01, regeneration0.01, maximum adult speed1.2, motor gain4.
No extra initialization noise is added to a loaded bank. The default frozen bank
contains128 unprepared variants of disclosed authored starting dispositions.
It is not blank random weights and is not the old prepared v1 bank.
`--bootstrap` instead generates standing variation from the environment seed.

## What the research has established

[The initial v2 development batch](reports/PHYSIOLOGY_DEVELOPMENT.md) completed
four runs across three distinct seeds. One seed survived200k; none recorded a
complete sampled depleted-area/crossing/feeding/reproduction sequence. Those
outcomes do not prove that the body is sufficient, that more training cannot
help, or that changing the body improved intelligence. Main remains unchanged.

[The registered cumulative-preparation experiment](experiments/CUMULATIVE_PREPARATION_PLAN.md)
holds that executable and body fixed. It carries descendant weights through
training worlds and evaluates separate snapshots. Evaluation descendants never
feed training. A training extinction stops the chain without restoring an older
bank. This is a development learning curve, not final eight-seed validation.

The [completed campaign](reports/CUMULATIVE_BUDGET16.md) found survival1/3,1/3,
2/3,3/3 across preparation budgets0,4,8,16; final baseline repeats again survived
1/3. The16-world bank has1,048,576 preparation ticks. Its [directional bias and
signal/memory limitations](reports/DIRECTION_AND_SIGNALS.md) remain visible;
survival on three development seeds does not establish the broad goal.

To watch that bank with the live-inspector/save fixes from this source:

```powershell
cargo run --release --target-dir target/play -- --founders .\reports\cumulative-preparation-20260904\bank-after16.json --seed 808
```

The bank is a locally preserved research artifact, not a promoted default.
The exact frozen campaign executable remains in that report folder as world.exe
(build0.3.0-dev); build0.3.1-dev changes inspection/persistence, not its body rules.

The preserved local frozen artifacts are under reports/physiology-development-20260904.
The cumulative runner requires that exact artifact set and writes a fresh output
directory; it does not rebuild or change the executable. Rebuilding with another
toolchain may change its hash and requires a newly registered experiment.

## Contracts and limits

- [CONTROLLER.md](CONTROLLER.md): actual inputs, recurrence, outputs, authored
  initialization, mutation and founder selection.
- [KERNEL_SPEC.md](KERNEL_SPEC.md): physical costs, tick order, ecology, conservation
  and replay limits.
- [Model-change contract](experiments/PHYSIOLOGY_V2_PLAN.md): why this differs from v1.
- [GOAL_CONTRACT.md](GOAL_CONTRACT.md): unchanged acceptance requirements.

`cargo test --release` checks controller/physics/layout/persistence fixtures;
`python -m unittest discover -s experiments -p test_cumulative_preparation.py`
checks experiment integrity and an offline choice assay. Neither proves evolved
adaptation. Sampled journey evidence has explicit gaps and does not yet establish
attribution across major geography renewals. Parallel population runs are not
promised bitwise deterministic.

Historical reports, v1 policies and earlier experiment runners are retained for
provenance, not compatible v2 instructions. The pre-recurrent implementation
remains at Git tag pre-recurrent-cutover. No personal artifacts were removed.
