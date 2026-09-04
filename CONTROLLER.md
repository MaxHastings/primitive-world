# recurrent-v1 controller contract

Implemented in `src/model.rs`, `shaders/perceive.wgsl`,
`shaders/decide.wgsl` and the physical execution shaders.
One decision per simulated tick, identical in GUI, headless and preparation.
There is no other production controller.

## What chooses what

The controller receives 64 scalar measurements and its previous 16 state values.
It computes the next 16 state values and 16 output values. All 1,568 weights
belong to this body. No observer metric or another body's weights are inputs.

For each state unit: `h' = tanh(W_input*x + W_state*h + bias)`.
Outputs are a linear projection of h' plus output biases.
State persists until updated by the next decision, death/reset or checkpoint
restore. It is 16 float32 values (64 bytes), not sixteen labeled memories.
There is no separately authored write/forget/rank rule. The network determines
retention through recurrence. It can also immediately overwrite everything.
Useful long-term memory is possible in principle, not verified by the current
population results.

Weights stay fixed within a lifetime. Changing state is not within-life weight
training. There is no gradient descent, online reward, replay buffer, shared
policy update, external database, episodic place list or automatic map.

## Inputs (zero-based indices)

All inputs are clamped to [-8,8] after normalization.

| Index | Measurement |
| --- | --- |
| 0 | Own energy / 100 |
| 1 | Own inventory / 8 |
| 2 | Food underfoot |
| 3 | Age / 10,000 |
| 4–5 | Previous actual voluntary velocity / 1.2 |
| 6–9 | Previous actual food collected, ingested, energy spent, food received |
| 10–11 | Previous actual displacement / 1.2, including force displacement |
| 12 | Event received on the previous tick, otherwise zero |
| 13 | Remaining reproductive recovery ticks / 240 |
| 14 | Previous requested body action index / 6 |
| 15 | Current sensor orientation / pi |
| 16–39 | Eight triples: food, actual offset x / sensory radius, actual offset y / radius |
| 40–63 | Four neighbor sextets: offset x/radius, offset y/radius, velocity x/1.2, velocity y/1.2, inventory/8, previous-tick event |

Food is vegetation plus dropped supplies (dropped component capped at 8 for
sensing), in food units. Sensors query grid cells at points, not area maxima.
Four samples are at radius/6 and four at radius; their cardinal directions
rotate with the body's attention. Boundary-clamped samples report their actual
offsets. At default radius 24, these distances are 4 and 24 world units.

Neighbor sampling rotates its starting cell pseudorandomly, examines at most
two candidates per spatial cell and keeps at most four within range. It does
not rank wealth or behavior. It is bounded sampling, not a uniformly random
sample of all neighbors. Dense cells can hide relevant bodies.

An absent neighbor contributes six zeros. A coincident, motionless, empty body
with no event can therefore be numerically indistinguishable from absence;
target resolution still masks absent slots. No persistent neighbor identity
is provided to the network. Slot/incarnation identifiers are carried separately
to validate its selected target, not supplied as cognitive features.

Offsets and movement use world-aligned axes with a compass convention, but no
absolute position or destination is an input. No global food, patch ID, map,
population, lineage, ancestry, route score, reputation, hunger utility or
automatic navigation gradient is supplied.

Events are limited physical feedback, not language understanding: positive
received transfer, negative contact-force feedback, or the emitted scalar
payload. Signals and feedback can overlap numerically; there is no truth,
trust or social-value label. A receiver observes only the last accepted event.
Observers can inspect the event type and participants separately.

## Outputs

| Output | Resolution |
| --- | --- |
| 0–6 | Logits for none, collect, ingest, transfer, force, emit, reproduce |
| 7–8 | tanh(gain * motor logits), vector length capped at one; physical body scales it |
| 9 | tanh angular change, at most 0.25 radians/tick |
| 10 | Sigmoid amount in [0,1], with pre-sigmoid clamp [-20,20] |
| 11 | tanh signal payload in [-1,1] |
| 12–15 | Target logits over the four actually observed bodies |

Largest action logit wins; exact ties favor the earlier index. Target selection
uses the same convention among present bodies. Locomotion and one body action
can occur together. The world does not replace impossible intentions with an
available action. Reproduction with inadequate reserves, transfer without a
nearby receiver, or ingest with no inventory can simply accomplish nothing.

Attention affects the next observation, not one already collected this tick.
Motor response gain is a declared per-world actuator calibration (fresh default
4, historical1), not an inherited gene, forced movement or new observation.
The controller can still stop exactly or choose arbitrarily small movement.
Maximum body speed and energy per actual distance do not change with the gain.
The amount output controls collection rate, ingestion rate, transfer quantity,
potential force spill, and offspring energy investment. It does not adjust
force's fixed collision cost or displacement.

Nonfinite internal calculations are flagged. That tick gets zero locomotion,
no body action and cleared recurrent state; metabolism still applies. Finite
but ineffective output is not corrected. Fault counts must be reported.

## Genome and heredity

Storage order: 16 rows of 81 values (64 input weights, 16 recurrent weights,
bias), then 16 output rows of 17 values (16 state weights, bias).

At an actual birth, each weight independently has an approximately 2% hashed
mutation chance. A selected weight receives a uniform-style hashed perturbation
in [-0.03,0.03]; resulting weights are clipped to [-4,4]. There is no reward-
dependent mutation, action-dependent copying or mutation-rate trait. Child
recurrent state and personal event history start at zero. Body capacities are
fixed; offspring inherit the parent's speed/sensory capacity, not learned state.
Maximum lifespan is freshly drawn from 9,000–11,000 ticks.

## Initialization is authored and explicit

Unprepared bootstrap relays normalized energy, inventory, underfoot food,
four near samples and age into eight state units. Before standing noise:
collect logit is 3*underfoot-state - inventory-state + 0.2;
ingest is -3*energy-state + inventory-state + 1.7;
reproduce is 3*energy-state + 2*inventory-state - 2.1.
None has bias -0.1; transfer/force/emit have bias -0.3.
Opposed near-food state pairs weakly steer x/y; amount bias is 3.
Each bootstrap weight receives initial noise in approximately [-0.01,0.01].
Other channels begin near zero. These are mutable starting dispositions, not
a runtime fallback. They do not establish what a random network would learn.

The bundled bank was exported from preparation seed 22 at tick 12,000 after a
preceding preparation world on seed 11. It has 128 living-descendant genomes.
Fresh bodies cycle through the bank without additional initialization noise,
start with 65 energy and 2 inventory, random locations, age 0–300, empty state.
Their initial reserves are declared world initialization, not resources invented
by subsequent births. `--bootstrap` intentionally bypasses this bank;
a bad requested bank fails rather than silently using bootstrap.

Preparation samples up to 128 living descendants by stable hashed lineage order,
at actual body abundance—not best routes, longest lives, most births or preferred
actions. Each preparation world starts fresh. The inherited bank is copied into
a new environment; personal memories never migrate between worlds.
See [the validation record](reports/RECURRENT_VALIDATION.md) for exact provenance.

## Open questions, not delivered abilities

Sixteen-state ungated recurrence may forget, saturate or encode poor estimates.
Argmax actions, finite sensing, weak initial movement and sparse mutation impose
strong limitations. The current bank is viable in the recorded finite tests,
but organized travel, memory usefulness, communication, cooperation and
open-ended adaptation remain unproven. Improving these would be a new,
explicit model decision—not a hidden scorer added underneath this controller.
