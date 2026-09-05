# Play Primitive World

> This is the historical v1 play guide. On the isolated physiology-v2 research
> branch, the executable is0.3.0-dev/checkpoint14 with UNPREPARED founders.
> See experiments/PHYSIOLOGY_V2_PLAN.md and CONTROLLER.md. It is not a promoted
> replacement for the main build, and v1 save/bank files remain incompatible.

Double-click **Play.cmd**, or run `.\Play.ps1` from this folder.
The launcher builds the current source in `target/play` before starting it.
The window/inspector should identify **build 0.2.0**, recurrent-v1/checkpoint12.
This is a local sandbox: no account, server, Python trainer or external model
is needed to play. Rust and a supported GPU are needed for the launcher build.

## First few minutes

- Space pauses/resumes. Start slowly; raise simulation speed when you want to
  watch generations and weather changes rather than individual decisions.
- WASD/arrows move the camera, the wheel zooms, Home fits the world, L changes
  the information lens. Select an agent to inspect its actual inputs/actions.
- Watch energy, inventory and reproduction together. Lots of agents or motion
  alone is not evidence of intelligence. Population collapses are not hidden.
- Change the seed under Physical settings, then click Reset / new world to
  start a different environment with the loaded founder bank.
- You can add/remove food or kill agents with the explicit intervention tools.
  Those are your experiments, not hidden assistance from the simulation.

## What starts fresh, and what persists

Fresh worlds load the named founder weights and create fresh bodies with empty
private state. Births copy parent weights with small mutations; an adult's
weights do not train during its lifetime. Its recurrent state does change.

**Save checkpoint** preserves that exact world, weights, state and settings.
Loading restores it paused. Save refuses to overwrite an existing checkpoint.

Under **Founder banks**, export living descendants to preserve a sample of the
current gene pool. The output path appears in the status message. Loading that
bank into a new world clears bodies' experience but carries the weights forward.
Bank files do not carry the world's costs or motor gain; those stay at the
current settings. Use matching settings when comparing brains.
Exporting is not a claim that the bank is better, and never changes defaults
silently. A world with no living descendants cannot export a bank.

## Physical controls

Working defaults: metabolism0.06, movement cost0.01, regeneration0.01, motor
response gain4. These are explicit model assumptions, not natural constants.
Gain changes how strongly small continuous motor signals actuate the body. It
does not force travel, select a destination, impose a minimum speed or increase
the maximum speed. Zero motor intent still means no voluntary movement.

Costs/sensitivity controls affect the live world. Initial-body count and seed
take effect on reset. Reset uses current settings and the loaded founder bank,
not the present world's evolved survivors. Export them first if needed.

Old compatible checkpoints keep their original settings, including gain1 when
the old file has no gain field. Use this build to open newly created files.
Do not compare runs with different settings and call the difference learning.

This remains a finite artificial-life experiment. No scripted migration,
automatic population rescue, destination planner or cooperative-behavior reward
is running underneath it. Organized travel and anticipation remain things to
investigate, not guarantees made by the visuals.
