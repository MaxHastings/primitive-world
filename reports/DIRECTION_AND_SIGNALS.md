# Directional bias, signal usage and direct state retention

Post-hoc development diagnostics, not final evaluation or a new controller.
The user observed persistent leftward movement and collapse in play, then asked
whether existing communication and memory are used. No new language system,
movement rules, rewards or weights were added. Reports below concern frozen
unprepared, budget8 and budget16 founder banks, not every later live descendant.

## Direction: confirmed on the actual GPU controller

The first-decision CPU assay and GPU shader agree within2.6e-7 adult motor units
over all120 tested bank/condition combinations. Each bank has128 genomes; cues
are synthetic mirrored food measurements with no neighbors and empty state.
State/body conditions are explicit in the raw reports.

At energy50/inventory2, the latest bank has127/128 initial leftward intentions
in bare space and119/128 leftward with weak0.02 food readings on right-side
near/far probes. With inventory0, the bare-space split is70 left/58 right.
The starting unprepared bank is approximately balanced in bare space.

The GPU sequence probe holds energy50,inventory2,age500 and position fixed.
It carries private state, previous action and movement feedback, presents one
direction for64 updates, then reverses it for64. The final16-update means are:

| Bank | Weak right cue: left/right brains | Strong right cue: left/right brains |
| --- | --- | --- |
|unprepared|22/106|0/128|
|budget8|120/8|0/128|
|budget16|126/2|0/128|

Both cue orders give those counts. Thus this is not an absolute inability to
turn right: strong food evidence reverses every tested brain. Weak cues can be
outweighed by an inherited drift, reinforced by controller/body feedback in this
probe. It remains unproven that this explains the user's live extinction or
that a particular training-world direction caused the inherited tendency.
Synthetic held-state cues are not full ecological situations or navigation tests.

Diagnostic source: `f26eb65`, test `simulation::tests::directional_bank_gpu_probe`.
Actual food sensing and update accounting are covered separately by the release
tests; the probe deliberately injects perceptions to isolate decision behavior.
`experiments/check_direction_gpu.py` verifies first-decision agreement and
summarizes measured reversals without asserting a preferred direction.

## Communication: delivery is used, usefulness is unproven

Completed budget16 development worlds record:

| Seed | Emit choices | Delivered signals |
| --- | --- | --- |
|808|11,366|10,882|
|909|11,171|10,936|
|1001|7,784|7,550|

These29,368 delivered signals prove use of the existing channel, not useful
information exchange. An EMIT targets one sampled nearby body, carries a
controller-chosen scalar in[-1,1], costs up to0.02energy and has a4-tick sender
cooldown. The receiver sees the event on the next decision; public neighbor
event observations can also expose it. No words, food coordinates, truth labels,
trust score or automatically interpreted direction are attached. Transfer and
force feedback share the same scalar event channel. Last accepted event wins.

## Direct pulse sensitivity and retention

`experiments/inspect_signal_memory.py` uses the documented float64 recurrent
equations, comparing an event-input12 pulse(+/-0.5 at update1) with no pulse.
Inputs thereafter are identical and fixed: no neighbors, bare food probes,
adult age500; three energy/inventory conditions. Private state alone is carried.
Motor/action feedback is held fixed to isolate recurrence, so this excludes
memory maintained through changed movement, position or subsequent interactions.
These signal-pulse results have not themselves been GPU-parity checked.

For budget16 at energy50/inventory2 and pulse+0.5:

- Update1: mean motor-vector change0.07275;27/128 different action choices.
- Update2: mean motor change0.01791;4 different actions.
- Update4: mean motor change0.0002423;0 different actions.
- Update8: mean motor change1.49e-7;0 different actions.
- Update16: mean motor change8.69e-14;0 different actions.

There is sensitivity to the event input, but no durable private pulse memory in
this tested context. Even the unprepared bank responds to event pulses; mere
sensitivity can arise from random initial connections and is not evidence of a
learned convention. These findings neither establish language/deception nor
prove the architecture cannot support more persistent or embodied memory.

The conservative next communication question is causal utility in actual
contexts: does changing/silencing received payloads change behavior and outcomes,
with sender cost/contact opportunities held matched? Whole-world signal disabling
also changes costs and competition, so it would not isolate information alone.
Do not assign semantics, reward communication or expand the controller merely
because the channel exists. Migration evidence remains the primary missing gate.

## Artifacts and checks

All raw diagnostics reside in `reports/cumulative-preparation-20260904`:

- `direction-gpu-budget0.json`, `direction-gpu-budget8.json`, `direction-gpu-budget16.json`.
- Checked direction summary SHA256: `3cf278bd7d4a4c60f53c19892c907267d0ba42ca1cf700349bac358a7fb8ec84`.
- Pulse report SHA256: `a4cb7204bd1e27b66d901fd021baf654dde3b724e677b0a5959feef676ab0f32`.

Three direction-helper and three pulse-helper CPU tests passed. Tests check
layout, symmetry of synthetic cues, zero/disconnected networks, motor bounds,
and known retention/forgetting networks; they do not force evolved brains to
use a particular behavior. The broad goal remains active and unachieved.
