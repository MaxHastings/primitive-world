# Design principles

Primitive World asks how much adaptive behavior can emerge from general physical,
cognitive, and evolutionary mechanisms. The organisms begin with random brains;
the simulator supplies opportunities and consequences, not a strategy to follow.

This page distinguishes that design intent from the choices actually made in
`primitive-v45-depth-retention`. For exact mechanics, read [agents](agents.md),
[world rules](world.md), and [evolution](evolution.md).

## Define possibilities, not behavioral goals

Movement can support exploration, orbiting, approach, escape, or inactivity.
Food transfer can provision another organism without a built-in definition of
help. A signed local signal may acquire a use, remain noise, or go unused.

The simulator does not train a controller to maximize food, lifespan, intelligence,
cooperation, or a behavioral score. There is no authored courtship or care policy.
A successful experiment need not produce human-like or steadily increasing
complexity. Extinction and simple strategies are valid outcomes.

The search space must nevertheless permit complete life cycles. Physical budgets,
sensing, actions and hereditary variation determine what is reachable. Controlled
test brains can check physical reachability; they do not become random founders
or online guidance. A mechanism being available does not demonstrate that evolution
will discover it.

## What the simulator authors

All simulated physics involves design choices. This model specifies:

- Energy, food, developmental costs and lifespans.
- Local physical sensing, movement, contact and action arbitration.
- Packet reproduction, inheritance and blind mutation.
- Gated recurrent memory, lifetime plasticity and their energy costs.
- Spatial ecology, climate variation and initial conditions.
- Finite entity storage, hereditary retention and world rollover.

Two choices deserve explicit treatment rather than being hidden behind a claim
of complete neutrality:

**Depth-biased hereditary retention.** Each newborn challenges two random pool
entries and targets the one with lower stored ancestry depth. This statistically
protects deeper within-world reproduction. It is an authored selection preference;
founder sampling remains uniform and mutation does not consult behavior. The
[evolution protocol](evolution.md#the-rolling-pool) defines the exact rule.

**Favorable climate initialization.** Fresh default worlds begin with wet/mild
initial climate keyframes. Subsequent variation follows the ordinary seeded
process, independent of population success. This is an initial-condition choice,
not proof that every world begins with a particular duration of abundance. Older
saves can preserve different initialization settings.

## Memory, learning and inheritance

Recurrent state carries information through time. Local plasticity changes bounded
connection deltas during an organism's lifetime, using local activity and inherited
rates/retention. Neither receives an external reward explaining which behavior
was successful.

Offspring inherit weights and traits that enable memory and learning. They do not
inherit acquired recurrent state, traces or learned connection deltas. Evolutionary
change between generations and adaptation within a lifetime are distinct.

Expressed capacity and state changes have physical costs. Evolution may use or
reduce learning and memory; providing the machinery is not a claim that useful
learning has been demonstrated. A currently unused capability is not, by itself,
evidence that it should be removed.

## Expose physical information

Controllers receive body condition, motion, local food, occupancy, proximity and
aggregate signals. They do not receive neighbor identities, kinship labels, global
maps, behavior scores or explanations such as “you were helped.” Inspector
metadata is not a controller input.

These interfaces have real limits. Aggregate measurements can make different
situations indistinguishable; finite brain capacity limits available state and
computation. The model offers opportunities for complex behavior, not a guarantee
of unbounded intelligence or evolutionary potential.

## Keep observation separate from selection

Research tools can measure survival, ancestry, movement, learning, signals and
resource use. Their reports do not supply controller inputs, adjust mutation,
select founders or change hereditary retention. The depth metadata used by the
pool is explicitly part of selection, not a supposedly passive observer.

Describe observations at the level the evidence supports. Packet production is
not successful reproduction. Signal emission is not demonstrated communication.
A food transfer does not alone establish parental care. Population movement can
reflect births and deaths rather than travel by the same individuals.

Manual food painting, physical-setting changes and founder imports are
interventions. Preserve their provenance when interpreting a run. See the
[observation guide](observing.md) for tools and measurement limits.

## Treat engineering limits honestly

Organisms and packets share 16,384 slots. Unfulfilled manufacturing requests are
counted and skipped without charge; fusion can reuse a packet slot. Contact
arbitration uses a tick-varying permutation of storage slots. These choices can
affect outcomes and are not biological discoveries.

Cumulative telemetry, world time, and entity identities use 64-bit storage.
Crossing a former 32-bit boundary continues the same living world, including
reproduction. Autosave failures are reported and
retried; they do not intentionally end the live experiment.

Performance changes should distinguish cheaper execution from different biology.
Reducing sensory detail, learning frequency or ecology cadence changes the model;
it is not equivalent to reducing rendering or redundant buffer traffic. Current
presentation, controls, sensing, cognition and interactions are preservation
requirements unless a separate change explicitly revises them.

## Review a proposed change

1. Which capability, physical rule or implementation cost does it change?
2. Does it expose a measurement or supply an interpretation?
3. Could several incompatible strategies use it?
4. Does it add a preference based on behavioral outcomes?
5. What information, dynamics or possible behaviors does it remove?
6. Does it alter costs, timing, random draws, selection or save compatibility?
7. What do the tests establish, and what remains an experimental question?

Prefer mechanisms that organisms can combine for different purposes. Record
exceptions and limits explicitly; do not turn a design aspiration into a claim
about observed intelligence.

## Model stability

After a model is frozen, unfamiliar or uninteresting behavior alone is not a
reason to retune it. Reopen biology for a demonstrated reachability problem,
correctness defect, unintended bias or fundamental restriction on composition.
Keep new experiments distinguishable from continuations of existing ones.

Completion means the mechanisms, accounting, persistence and operation are
verified and their remaining assumptions documented. It does not require
communication, cooperation or any other favored outcome. Historical release
criteria and evidence are recorded in the [north star](north-star.md) and
[implementation checklist](implementation-checklist.md); unperformed checks must
remain unperformed rather than being presented as passes.
