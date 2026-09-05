# Agents — V3

The controller owns intentions. The world owns consequences. All bodies use the
same implementation in play, headless worlds, preparation, and evaluation.

## Computation and memory

76 local measurements plus 16 previous recurrent state values produce 16 next
state values and 16 outputs. There are 1,760 inherited weights. Each state unit
uses tanh(input projection + recurrent projection + bias); outputs are linear
projections of that state. The 16 float32 state values occupy 64 bytes. They are
not 16 labelled memories, a map, or a list of places. Retention is controller-owned.

Weights do not update during life. Only recurrent state changes during decisions.
At birth, each weight has approximately 2% mutation probability, adding a hashed
perturbation in [-.03,.03], then clipping to [-4,4]. Child state starts empty.
No loss function, online reward, gradient optimizer, or reward-dependent mutation
exists. Useful long-term memory and communication have not been demonstrated in v3.

## Inputs (zero-based)

| Inputs | Measurement |
| --- | --- |
| 0–3 | Energy/100, inventory/8, food underfoot, age/10,000 |
| 4–5 | Previous voluntary velocity/1.2 |
| 6–9 | Previous collected food, digested food, spent energy, received food |
| 10–11 | Previous actual displacement/1.2, including contact displacement |
| 12 | Other bodies in the same 8×8 spatial cell, divided by 16 |
| 13 | Remaining reproductive recovery ticks/240 |
| 14–19 | One-hot previous requested action: none, collect, transfer, force, emit, reproduce |
| 20–43 | Eight food probes: food, actual offset x/radius, actual offset y/radius |
| 44–75 | Four neighbors: offset x/y, voluntary velocity x/y, inventory, signal, body-present, signal-present |

Neighbor offsets use sensory radius; velocities use 1.2; inventory uses 8.
Inputs are bounded to [-8,8]. Food sensing combines vegetation and dropped food;
dropped stock is capped at eight food units for sensing, not possession.

Food probes sample cardinal directions at radius/6 and radius (4 and 24 units by
default), using world-aligned axes and actual boundary-clipped offsets.
They are point samples, not vision of every cell in a radius.
Bounded neighbor sampling rotates its starting cell, checks at most two bodies
per cell and accepts at most four in range. This is not uniform sampling.
Density input is a raw cell count, not a computed crowding trend or advice to leave.
The brain must infer trends, if useful, using its own state and feedback.

A neighbor signal is that neighbor's own scalar emission on the preceding tick.
Presence distinguishes zero from silence. No signal says food, help, harm, lie,
truth, or direction unless controllers develop such an interpretation.
Messages no longer contain received transfer/force feedback. They are visible
only through locally sampled bodies; transmission does not guarantee reception.
No sender identity is fed to cognition, but the currently visible source's offset
is available. There is no persistent reputation, relationship list, map, patch ID,
absolute position, destination, lineage or global population input.

## Outputs (zero-based)

| Outputs | Capability |
| --- | --- |
| 0–5 | Action logits: none, collect, transfer, force, emit, reproduce |
| 6–7 | Voluntary movement vector |
| 8 | Collection/transfer amount or offspring energy investment, sigmoid [0,1] |
| 9 | Emitted scalar, tanh [-1,1] |
| 10–13 | Target logits over the four visible neighbors |
| 14–15 | Contact displacement vector, independent of voluntary movement |

Largest action logit wins; ties favor the earlier slot. Movement accompanies one
body action. Target choice applies to transfer and force, not local emissions.
The shared amount/target outputs are a compact actuator interface, not a rule
about when to help, attack, reproduce or migrate. Impossible finite intentions
are not replaced with sensible ones.

Movement applies radial tanh saturation at gain 4, scaled by maximum body speed.
Force uses radial tanh saturation at gain 1, scaled to at most three units.
There is no preferred compass direction, minimum movement, or minimum force.
Very small vectors use a numerical normalization floor of .0001.

A nonfinite decision is flagged, gets no movement/action, and clears state.
It is not a viability rescue: metabolism and digestion continue.

## Initialization

Default founders are 256 reproducible random genomes, cycled across fresh bodies.
There is no food-seeking template, reproductive threshold template, favored
action bias, or suppression of social actions. Input weights start uniform in
[-.25,.25], recurrent weights/state biases in [-.35,.35], and output weights/biases
in [-.5,.5]. These exchangeable numerical scales are still authored assumptions.
The seed is 0x184a2321 with the documented LCG in src/model.rs.

--random-founders instead gives each initial body an independently generated
genome using the environment seed. --founders loads an explicitly named bank
without additional initialization noise. Invalid banks fail, never fall back.

Birth inheritance and [between-world evolution](evolution.md) are distinct.
The native survivor loop carries actual late-surviving bodies' current genomes,
including descendant mutations, into the next world. It does not rank original
founders by family scores. Each sampled genome is retained exactly, with balanced
mutated replicas filling the next bank. Body state and memories reset; genes do not.

No prepared bank or authored survival starter is silently bundled. Random does
not mean competent. The retired models and trainers remain in Git history, not
as alternate execution paths in this release.
