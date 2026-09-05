# Play Primitive World

This guide describes the isolated physiology-v2 research branch, not the
released main build. It uses UNPREPARED founders by default and has not met the
broad adaptation goal. See CONTROLLER.md for what is authored and what can evolve.

Double-click **Play.cmd**, or run `.\Play.ps1` from this folder.
The launcher builds the current source in `target/play` before starting it.
The window/inspector should identify **build0.3.0-dev**, physiology-v2/checkpoint14.
This is a local sandbox: no account, server, Python trainer or external model
is needed to play. Rust and a supported GPU are needed for the launcher build.

## First few minutes

- Space pauses/resumes. Start slowly; raise simulation speed when you want to
  watch generations and weather changes rather than individual decisions.
- WASD/arrows move the camera, the wheel zooms, Home fits the world, L changes
  the information lens. Select an agent to inspect its actual inputs/actions.
- The inspector refreshes the selected body each frame and labels the observed
  tick (before the next frame's simulation steps). Death or slot reuse stops
  tracking and leaves an explicitly labelled snapshot; a newborn is never
  silently substituted. Initial/newborn bodies show no decision until their
  first controller update. Dead bodies retain terminal body data, but no claim
  about their last controller inputs: those GPU buffers may already be cleared.
  Selecting empty space clears the inspector.
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

**Save new checkpoint** preserves that exact world, weights, state and settings
in a uniquely named file under `reports/checkpoints`. It fills the editable
checkpoint path; **Load checkpoint** restores that path paused. Paste a previous
save's path to load it. Saving again preserves previous files, including when
paused at the same tick. A filename collision fails rather than overwriting.

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

Only v2 checkpoint14 is compatible; it retains its explicitly saved settings.
V1 checkpoints and banks are rejected without modifying them. The initial load
path is `recurrent-world.checkpoint` for existing saves; new saves use the folder
above. Check the inspector's model identity, not the filename. CLI save/load
supports explicit paths. Existing saves are never overwritten by a save operation.
Do not compare runs with different settings and call the difference learning.

This remains a finite artificial-life experiment. No scripted migration,
automatic population rescue, destination planner or cooperative-behavior reward
is running underneath it. Organized travel and anticipation remain things to
investigate, not guarantees made by the visuals.
