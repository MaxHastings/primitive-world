# Agents

The controller owns intentions. The world owns consequences. All bodies use the
same implementation in interactive and headless worlds.

## Computation and memory

Each organism has a 16-unit potential gated-recurrent substrate, 108 senses,
and 20 outputs. A heritable 32-bit active mask expresses from one to 16 units;
inactive units have no state, gates, readout, write, or energy effect. The dense
full controller has 2,612 inherited float32 values, stored in GPU banks without
reducing the population cap. No capacity is privileged.

Candidates use tanh(sensory projection + previous-state projection + bias).
Gates use a linear projection of candidate values plus bias, clamped to [0,1].
Each next state is `(1-gate)*previous + gate*candidate`. Zero retains exactly,
one replaces, and intermediate values blend. Outputs project the next state.
Fresh gate biases center on .5; inherited parameters may close gates completely.
No mandatory forgetting floor is imposed.

Birth resets recurrent, trace, and learned-weight state. It mutates expressed
weights under a fixed per-birth budget and independently may activate or retire
a unit. Activation clones a live unit's connections with a bounded perturbation;
inactive values remain inherited but latent. Each active unit also has an
inherited signed local-plasticity rate, with inherited trace and fast-weight
retention. During life, only local pre/post activity, trace, and gate modulation
update fast connection deltas; those deltas are bounded, never inherited, and
pay write energy. There is no optimizer, semantic slot, curriculum, reward, or
authored action incentive.

Each organism also carries one bounded mutation-scale trait (0.25–4). It
multiplies its fixed, capacity-independent birth-mutation budget and can itself
drift slightly at birth. This lets lineages evolve conservative or exploratory
inheritance without adding a cognitive input, action, reward, or lifecycle
state.

Energy is body upkeep plus active-unit upkeep times active capacity, plus the
actual absolute change in recurrent, trace, and learned-weight state times the
write-energy constant, plus existing action costs. Genome copying itself is not
a separate running cost.

## Inputs (zero-based)

| Inputs | Measurement |
| --- | --- |
| 0–3 | Energy/100, inventory/8, food underfoot, age/10,000 |
| 4–5 | Previous voluntary velocity/1.2 |
| 6–7 | Net change in own energy/100 and inventory/8 since the previous tick; zero at birth |
| 8–9 | Previous actual displacement/1.2, including contact displacement |
| 10–11 | Own last emitted scalar and time since emission/1,000; zero before first emission |
| 12 | Nearby body count/16 |
| 13–19 | Reserved, zero |
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
| 0–5 | Primary-action logits: none, collect, transfer, force, emit, reproduce; output 1 also drives independent gathering |
| 6–7 | Voluntary movement vector |
| 8 | Collection/transfer amount or offspring energy investment, sigmoid [0,1] |
| 9 | Emitted scalar, tanh [-1,1] |
| 10–17 | Target logits over the eight sector neighbors |
| 18–19 | Contact displacement vector |

Largest action logit wins; ties favor the earlier slot. Movement and independently requested gathering can accompany the primary
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

Paid births copy the parent's inherited controller and traits, then apply the
shared mutation law described in [evolution](evolution.md). Eligible topology
activation and retirement each have a 1% chance. Expressed parameters receive a
probability .25 times inherited mutation scale of a bounded expressed-weight
perturbation. The same event perturbs one active plasticity rate and both retention
traits. Exact copies are allowed. Between worlds, unchanged records are sampled
from the blind hereditary pool; there is no comparison or ranking.

Gathering also has an independent actuator: clamp(output 1, 0, 1) scales the
requested collection amount. It can run alongside the primary selected action,
including reproduction. A nonpositive output disables gathering even on food.
No hunger threshold or automatic collection policy is supplied.
