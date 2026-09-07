# Agents

The controller owns intentions. The world owns consequences. All bodies use the
same implementation in interactive and headless worlds.

## Computation and memory

All brains contain eight fixed recurrent units, 108 senses, and 20 outputs.
Dense sensory-to-candidate, previous-state-to-candidate, candidate-to-gate, and
state-to-output matrices plus biases total 1,188 inherited float32 parameters.
Eight units keep memory and gates compact without assigning semantic node roles.
This size is an implementation choice, not a measured evolutionary optimum.

Candidates use tanh(sensory projection + previous-state projection + bias).
Gates use a linear projection of candidate values plus bias, clamped to [0,1].
Each next state is `(1-gate)*previous + gate*candidate`. Zero retains exactly,
one replaces, and intermediate values blend. Outputs project the next state.
Fresh gate biases center on .5; inherited parameters may close gates completely.
No mandatory forgetting floor is imposed.

Structure and parameters stay fixed during life. Birth and cross-world founding
reset recurrent state. Ordinary parameter mutation applies at paid reproduction
and to next-world variants. The shared mutation law is not a brain output.
There is no node/edge mutation, online optimizer, or authored action reward.

Body and brain upkeep begin at .01 energy/tick and rise linearly to .06 by world
tick 50,000. Construction is covered by the fixed .2 times reproduction-cost
overhead. There is no separate connection upkeep or genome-length copying charge.

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

Default founders each receive a seed-specific random genome. Sensory weights start
uniformly in [-.1,.1]; other weights and biases start in [-.35,.35]. Gate biases
receive an additional .5, giving [.15,.85]. These are numerical initial conditions,
not authored food-seeking or reproduction policies. Random does not mean competent.

`--founders` imports an explicitly named current-format bank without initialization
noise. It repeats across founding positions; invalid data fails without fallback.

Paid births copy the parent's fixed genome and then apply the shared continuous
mutation law: each inherited genome samples its own log-uniform temperature from
1/8 to 8, scaling the .02 per-parameter chance and .03 step size. Every newborn
therefore differs from its parent while retaining fresh body state and zero memory.
Between worlds, [population selection](evolution.md) compares complete founding
groups. A sparse uniform sample of terminal biological descendants can supply
inherited sources for the next candidate, but each source is varied and only the
complete candidate group's world duration decides what carries forward.
