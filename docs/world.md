# World and body rules

Simple, local capabilities; consequences independent of intended meaning.
The numeric constants below are declared modeling choices, not discovered laws.

## Body and substrate

| Mechanism | Rule |
| --- | --- |
| Space | Bounded 2048-unit square; no wrapping |
| Food / spatial cells | 512² vegetation cells / 256² neighbor cells |
| Capacity | 16,384 GPU body slots, not a target population |
| Reserves | Up to 100 energy and 8 carried food |
| Upkeep / movement | .06 energy/tick; .01 energy per actual voluntary distance |
| Movement | Adult maximum 1.2 units/tick; juvenile speed .6–1 of adult through age 400 |
| Collection | Requested, at most .025 food × amount/tick, limited by stock/capacity |
| Digestion | Automatic, at most .1 carried food/tick; 8 energy/food, energy-headroom limited |
| Local contact | Transfer and force require a currently valid target within 6 units |
| Signal | One scalar emission per chosen emit action, .02 energy; no target/cooldown |
| Reproduction | Chosen, age at least 400; 240-tick recovery; paid energy investment |
| Aging | Death at a freshly sampled maximum age of 9,000–11,000 ticks |
| Sensing / state | Radius 24, eight food points, up to four bodies, 16 recurrent values |

Digestion does not harvest for the agent. Finite throughput and reserves create
tradeoffs. Development, recovery, aging, sensory geometry and their exact values
remain explicit body assumptions; they must not be advertised as inevitable
first principles. No solid-body packing, collision damage, momentum, mating,
kin recognition, health meter, or inherited body-shape evolution is simulated.

## Tick order

1. Reserve only slots already dead at tick start for births.
2. Update ecology, rebuild spatial indexing, sense the same pre-action world.
3. Evaluate each agent's recurrent controller.
4. Collect at pre-movement positions.
5. Digest, move, pay upkeep, update age/state; emit if chosen and affordable;
   determine death and eligible reproduction requests.
6. Resolve disjoint local transfer/force pairs at post-movement positions.
7. Allocate births, rechecking actual parental energy and survival.
8. Release dead bodies' remaining inventory once; count living bodies.

A tick is a discretization, not simultaneous continuous physics. Collection uses
atomic stock subtraction with bounded retries: contention may lose an opportunity
but must not duplicate food. Dropped supplies are picked up before vegetation.
Boundaries clip motion; motion cost follows actual displacement.

## Interactions without prescribed social meaning

Transfer moves up to the chosen amount of existing inventory into a nearby body,
limited by sender stock and receiver capacity. There is no obligation, recipient
utility score, kin preference, or automatic sharing.

Force is a kinematic contact actuator: a chosen vector displaces a nearby body up
to three units. The actor pays .2 energy per actual displaced unit. Affordable
distance is bounded by its remaining energy; world edges can shorten it. No
success roll, recipient energy tax, automatic food spill, loot or eastward fallback
exists. There is no recoil or momentum, consistently with kinematic locomotion.
The explicit contact cost is a drag calibration, not a penalty for aggression.
Displacing a body can help or hinder it through where it ends up. This model does
not directly model injuries, and should not call displacement itself damage.

Physical pairs use rotating priority and exclusive ownership to avoid concurrent
writes. This bounded matching can underutilize contact opportunities; it is not
optimal matching. Disabled, stale and out-of-range requests cannot claim a pair.
Signals do not participate in this arbitration and cannot provide a contact shield.

Emit pays a full .02 energy and makes a controller-chosen scalar observable on the
next tick through local neighbor sensing. It works without a target. Zero is a
valid payload, distinguishable from silence. There is no broadcast of someone
else's received events, truth tag, built-in vocabulary, receiver energy penalty
or enforced response. An emission counter is not a count of recipients or useful
communications.

## Reproduction and material accounting

With reproductive cost B=50 and controller amount a:
child energy = .8 × B × a; construction dissipation = .2 × B.
The parent pays both from its current energy. No extra inventory prerequisite or
mandatory food transfer exists; the child starts with zero inventory. Thus birth
does not create food, nor require stockpiling while automatic digestion consumes
the same stock. Parents may exhaust themselves; the world does not prevent it.

Children spawn two units from the parent in a hashed direction, boundary-clipped,
with age/state/signals cleared. Only the next tick can act on them. Weights copy
with ordinary mutation; speed and sensory capacity copy without mutation.
Free slots are allocated with a tick-rotated parent priority so low storage slots
do not always win at capacity. Unallocated requests do not spend reserves.
Resource provision to fresh founders (65 energy, 2 food, age 0–300) is explicit
initialization, not the rule for later births.

Vegetation/drop stock uses thousandths; body inventory uses float32. Death drops
rounded remaining inventory. Accounting has quantization residuals; cumulative
summary rounding is not exact long-run energy closure. Age death discards stored
energy, not food. Costs and birth construction are sinks. Population accounting
must balance births against starvation, aging and contact-actor exhaustion.

## Ecology and environment controls

Seeded hubs, irregular low-yield regions, barren gaps, weather, seasonal growth,
soil recovery and harvest depletion create the ecological environment. Resource
geography interpolates keyframes every 8,192 ticks; hub reseeding occurs every
third keyframe. First major transition is 16,384–24,576, then repeats every 24,576
ticks. These are gradual transitions, not instantaneous food teleports.

Habitat contrast in [0,1] mixes geography with its spatial mean: zero is uniform
distribution, one the complete patch/gap field. Total mean habitat is preserved,
but distribution changes attainable food and therefore difficulty. Mean-preserving
does not imply equal carrying capacity or guarantee easy founding.

Environment rotation applies quarter-turns to initial positions and the entire
habitat/weather history. It is never a brain input. These two controls change the
environment, not the body or weights. Normal play uses contrast 1; no progressive
difficulty escalator or population rescue occurs inside it. Optional manual
interventions are user experiments; record them when comparing outcomes.

## Persistence, observation, and limits

Checkpoints use format 16; founder banks use format 5. The primitive-v4 model
does not load V3 files because its brain has two additional outputs.
Unsupported formats are rejected without rewriting the file.
Checkpoints preserve settings, bodies, genomes, food, soil, event counters and
controller traces. Derived indexing/terrain is rebuilt after load. Loading
validates before mutating the world. Save/export refuses existing destinations.
Local inspector identity tracking does not modify behavior.

Physics/controller wiring checks are not evidence of learned intelligence.
Headless worlds check for extinction after each GPU batch (at most 32 ticks),
independent of report frequency. Initially empty worlds run zero ticks. The final
report/optional journey footer is flushed at the early stop, with an explicit
extinction or tick-limit termination reason. The reported stop tick is detection
time, not an exact death tick (at most 31 extra ticks inside a submitted batch).
Observers never feed controller inputs or select desired behavior. The sampled
journey observer has known between-sample and resource-relocation attribution
gaps; it is not proof of general adaptation. Population-wide replay is
not guaranteed bitwise deterministic because atomic contention can vary.
Long/high-population runs can overflow u32 counters; bounded protocols and
accounting checks must expose this rather than silently accept it.
