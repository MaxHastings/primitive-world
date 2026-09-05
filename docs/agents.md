# Agents

The controller owns intentions. The world owns consequences. All bodies use the
same implementation in interactive and headless worlds.

## Computation and memory

Fresh genomes contain four recurrent units and 144 explicit connections.
Each has eight randomly selected sensory connections, four recurrent inputs,
four candidate-to-gate inputs, and connections to all 20 outputs. There are no
semantic node roles. Inheritance can change the graph within 1–64 units and
0–512 connections; the 108 senses and 20 actuator outputs stay fixed.

Candidates use tanh(sensory projection + previous-state projection + bias).
Gates use a linear projection of candidate values plus bias, clamped to [0,1].
Each next state is `(1-gate)*previous + gate*candidate`. Zero retains exactly,
one replaces, intermediate values blend. Outputs are linear projections of the
next state. Sparse edges explicitly encode each projection; missing edges do
no work. Gates have no mandatory positive bias or forgetting floor.

Structure and parameters stay fixed during life. Child state starts empty.
World mutation rules can duplicate/delete a unit, insert/delete a connection,
and perturb actual parameters at reproduction. Duplication copies incoming
structure and splits outgoing weights, including self loops and gates, so it
preserves the old computation before subsequent mutation. Deletion removes
incident edges. There is no online optimizer or intelligence reward.

Upkeep is 0.001 energy per unit and 0.00002 per connection per tick by default.
Every encoded connection costs upkeep even at zero weight. A disconnected unit
still exists and costs energy. Reproduction additionally costs 0.001 per logical
encoded value: `2 + 2*units + 20 + 3*connections`. Unused GPU allocation is
zero padding, never inherited dormant material and never charged.

## Inputs (zero-based)

| Inputs | Measurement |
| --- | --- |
| 0–3 | Energy/100, inventory/8, food underfoot, age/10,000 |
| 4–5 | Previous voluntary velocity/1.2 |
| 6–9 | Previous collected food, digested food, spent energy, received food |
| 10–11 | Previous actual displacement/1.2, including contact displacement |
| 12 | All other bodies within sensory radius, divided by 16 |
| 13 | Remaining reproductive recovery ticks/240 |
| 14–19 | One-hot previous requested action: none, collect, transfer, force, emit, reproduce |
| 20–51 | Sixteen regions, each: mean food, body count/16 |
| 52–107 | Eight sector neighbors, each: offset x/y, voluntary velocity x/y, signal, body-present, signal-present |

Neighbor offsets use sensory radius; velocities use 1.2. Others' inventories
are not observable. Inspector identities are not cognitive inputs.
Inputs are bounded to [-8,8]. Food sensing combines vegetation and dropped food;
dropped stock is capped at eight food units for sensing, not possession.

Eight fixed compass sectors run clockwise E, SE, S, SW, W, NW, N, NE, centered
45 degrees apart. Regions 0–7 cover distance <= radius/2 (12 units by default);
8–15 cover the remainder out to radius (24). Food is the arithmetic mean over
every 4×4 food-cell center in each region. This is coarse grid-resolution coverage,
not exact continuous vision: cells straddling region/range boundaries are assigned
by their centers. Empty regions read zero; off-world cells are not counted or
wrapped. Underfoot food is also measured directly.

Every living other body within radius contributes to exactly one regional count.
There is no per-cell candidate cap. The nearest body in each angular sector is
individually observable and targetable; an exact-distance tie selects the lower
storage slot. Coincident bodies are assigned to E. Sampling has no tick/RNG shuffle.
Targets can still switch at sector boundaries or when nearest distances cross;
this is not identity tracking. Counts are measurements, not crowding trends or
advice to leave. The brain must infer trends using its own state and feedback.

A neighbor signal is that neighbor's own scalar emission on the preceding tick.
Presence distinguishes zero from silence. No signal says food, help, harm, lie,
truth, or direction unless controllers develop such an interpretation.
Signals contain only the sender's chosen scalar. They are visible only through
the nearest body in each sector; transmission does not guarantee reception.
No sender identity is fed to cognition. There is no persistent reputation,
relationship list, map, patch ID, absolute position, destination, lineage or
global population input.

## Outputs (zero-based)

| Outputs | Capability |
| --- | --- |
| 0–5 | Action logits: none, collect, transfer, force, emit, reproduce |
| 6–7 | Voluntary movement vector |
| 8 | Collection/transfer amount or offspring energy investment, sigmoid [0,1] |
| 9 | Emitted scalar, tanh [-1,1] |
| 10–17 | Target logits over the eight sector neighbors |
| 18–19 | Contact displacement vector |

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

## Initialization and inheritance

Default founders are 256 reproducible random genomes, cycled across fresh bodies.
There is no food-seeking template, reproductive threshold template, favored
action bias, or suppression of social actions. Input weights start uniform in
[-.25,.25], recurrent weights/state biases and gate weights/biases in [-.35,.35],
and output weights/biases in [-.5,.5]. These exchangeable numerical scales are
authored assumptions. The seed is 0x184a2321 with the LCG in src/brain.rs.

--random-founders instead gives each initial body an independently generated
genome using the environment seed. --founders loads an explicitly named bank
without additional initialization noise. Invalid banks fail, never fall back.

Birth inheritance and [between-world evolution](evolution.md) are distinct.
The native survivor loop carries a rolling archive of up to 64 bodies' genomes,
including descendant mutations, into the next world. It does not rank original
founders by family scores. Each sampled genome is retained exactly, with balanced
replicas filling the next bank using the current world mutation settings. Body state and memories reset; genes do not.

The bundled bank contains untrained random graphs. Random does not mean competent.
