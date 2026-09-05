# Kernel contract — physiology-v2 (experimental)

The world supplies capabilities and consequences. The controller decides how
to use them. Every numeric choice below is an authored model or engineering
bound, not a discovered law.

## Ownership

- World: space, ecology, possession, physical costs, interaction arbitration.
- Body: finite reserves, sensors, motion and action limits.
- Controller: recurrent state, movement, action, amount, target, payload.
- Heredity: actual birth allocation, copied/mutated weights, cleared child state.
- Observer: counts, traces, lineage, plots, exports. No behavioral feedback loop.

There is no authored destination choice, memory importance ranking, exploration
bonus, helper/enemy classification, structured food report, automatic surplus
birth, action lottery, population floor, live reseeding or policy fallback.

## Physical parameters and reasons

| Mechanism | Default and reason |
| --- | --- |
| Space | Bounded square, 2048 units; finite geometry, no wrapping |
| Vegetation / spatial grids | 512² cells / 256² cells; food and neighbor-compute resolutions |
| Body slots | 16,384; finite GPU storage budget, not a target population |
| Energy / inventory capacity | 100 energy / 8 food; finite reserves |
| Basal metabolism | 0.06 energy/tick; maintaining a living body costs energy |
| Voluntary movement | Adult maximum 1.2 units/tick, 0.01 energy/actual unit; movement expenditure |
| Motor response | tanh(4 * motor logit), vector length capped at one; continuous actuator sensitivity, zero stays zero |
| Development | Speed scales from 60% to 100% through maturity at age 400 |
| Collection | Up to 0.025 food * amount per tick, subject to stock/capacity; finite throughput |
| Digestion | Automatic, up to 0.1 carried food per tick, 8 energy/food, inventory/energy headroom bound |
| Contact radius | 6 units; transfer and force require physical proximity |
| Signal range | Minimum of sender/receiver sensory radii; 0.02 energy, four-tick recovery |
| Reproduction | Maturity 400, recovery 240 ticks; finite development/reproductive throughput |
| Lifespan | 9,000–11,000 ticks; declared age-limited bodies |
| Sensor budget | Eight food points, up to four bodies, default radius 24 |
| Memory / genome | 16 float32 state values / 1,518 weights; finite computation |

These rates set timescales, not a guaranteed ecological equilibrium. Age,
maturity, recovery and force costs remain modeling assumptions to question.
They are not justified by wanting fewer fights or a prettier birth curve.
The low-cost diagnostic (0.005/0.002) is retained as experimental history, not
the default: it often hit storage capacity. Original costs are restored and
motor sensitivity is calibrated separately, not as a minimum speed or food
gradient. Only checkpoint14 is accepted on this branch; motor gain is explicit
in every saved settings object. Compatible saves retain their own physical settings.
Numerical settings are validated; safety bounds include regeneration 0..1,
costs 0..100, conversion 0.000001..1000, sensory range 4..48, maturity 0..11000
and recovery up to 1,000,000 ticks. Extreme legal settings need not sustain life.

No solid-body collision/volume packing is simulated. Bodies can overlap.
Movement is kinematic, not momentum-conserving mechanics.

## Tick order and conservation

1. Identify slots already dead at tick start; only these can accept newborns.
2. Update ecology and rebuild occupancy/spatial indexing.
3. Sense locally; compute all controller intentions from the same pre-action world.
4. Collect at pre-movement positions using atomic stock subtraction.
5. Apply collected food, automatic digestion, voluntary movement, metabolism, age and
   recurrent state. Record reproduction requests meeting pre-interaction limits.
6. Propose and resolve disjoint local interaction pairs at post-movement positions.
7. Allocate births and recheck actual parent reserves after interactions.
8. Drop inventory of bodies now dead, once. Count alive bodies.

Food on the ground is quantized in thousandths. Atomic collection takes dropped
food first, then vegetation, with at most sixteen compare/exchange attempts per
source; contention can leave a request partially/unfilled, never duplicated.
Inventory uses float32; ingestion converts existing inventory into energy.
Movement is bounded by available energy and world edges and charges actual
distance. Basal metabolism then consumes remaining energy up to its cost.
Deaths occur at zero energy or maximum age.

No action substitutes for another. Sharing one tick with movement does not make
collection/digestion/reproduction free; all draw from current reserves in the
order above. Statistics are observations, not rewards.

## Interaction semantics

A tick-hashed rotating priority arbitrates pairs. An accepted pair owns both
body records, so one body cannot participate in multiple resolved pairs in the
same tick. This can underutilize possible interactions; it is not an optimal
matching algorithm. Target incarnation and range are revalidated. Disabled,
stale or out-of-range requests cannot claim a pair.

- Transfer: subtract up to the controller's amount from the sender's inventory,
  add it to the receiver, respecting both capacities.
- Force: sender spends up to 0.6 energy, receiver up to 0.3. Success probability
  is sender energy / combined energy before this collision cost. On success,
  spill up to amount from the receiver onto the ground and displace it 3 units
  away, clipped at boundaries. The sender does not directly receive that food.
  It can later collect nearby dropped stock like any body.
- Emit: deliver one bounded scalar to the targeted receiver, charge the sender
  up to 0.02 energy, and apply a four-tick sending recovery. No map is copied.

Force feedback is -max(0.3, spilled food); transfer feedback is received amount;
signal feedback is its payload. These channels are coarse physical event
encodings, not judgments about good or bad partners. Neighbor sensing can
observe another body's previous-tick event. There is no reputation system.

Death releases remaining inventory with rounding to thousandths; force spill
uses the same rounding. Thus matter accounting has a bounded quantization
residual, not perfect real-number conservation. Collision dissipation, basal/
motion/signal costs, reproductive construction and unused energy at age death
are sinks. No conflict penalty is fed into an objective function.

## Reproduction and inheritance

Let B be reproductive cost (default 50) and a the amount output:
offspring energy = 0.8*B*a; construction dissipation = 0.2*B.
The parent pays their sum and transfers exactly one existing food unit.
It must be mature, recovered and possess that inventory and energy.
Explicit reproduce must win the action logits. Movement does not disqualify it.

Offspring spawn two units from the parent in a hashed direction, boundary
clipped, with fresh age/state/event history and mutated inherited weights.
They do not act until the next tick. The parent's recovery is then set.
Exhausting a parent's energy can kill it; that is not silently prevented.

Free-slot and birth-request scans allocate a bounded number of births in slot
order. At capacity, requests without slots do not spend resources. A request
invalidated by an interaction is not reassigned within that tick.
Slot incarnation advances on reuse; ancestry depth is separate. New lineage
IDs are bookkeeping, not inputs to controller decisions. Mutation is fixed
and nonzero as specified in [CONTROLLER.md](CONTROLLER.md).

## Ecology retained across the cutover

Initial geography includes seeded rich hubs, lower-yield connecting bands,
multi-scale irregularity and barren gaps. Weather, seasonal variation, fertility
recovery and harvest depletion modulate growth. Food capacity can contract,
recorded as weather loss. Fractional regeneration accumulates rather than being
discarded. Food painted into barren space remains collectible without regrowing.
Terrain blends normalized-productivity keyframes every 8,192 ticks. Hub-center
seeding changes every third keyframe. The transition into keyframe3 occurs over
ticks16,384–24,576; subsequent major-transition intervals repeat every24,576
ticks. Thus a nominal renewal boundary is the end of a gradual transition, not
an instantaneous food teleport. Smaller drift and weather continue between them.

The regeneration default is 0.01. `--static-landscape` freezes geography only.
The existing ecology is authored substrate; its bands cannot be counted as
emergent roads. V2 retains v1 ecology unchanged; no adaptation gate has passed.

## State, accounting and replay limits

Checkpoint 14 saves settings, bodies/state, cold genomes, food, soil, ground
accounting, event counters/ring and observation/decision traces. Loading reads
and validates the entire file before touching live buffers. Derived spatial
indexing/terrain buffers are rebuilt. Old schemas are rejected, not rewritten.
Save/export refuse existing destinations. A filesystem failure can leave a
partial *new* file; it cannot truncate an older save.

Observers record cumulative births/deaths/actions, overlapping request failures,
current reserves, production/weather loss and resolved force expenditure.
Some cumulative counters are u32; sufficiently long/high-population runs can
overflow them. Summary reserve rounding is 0.001 per body. Reported ingestion
also rounds each event; do not claim exact long-run energy closure from these
counters. Per-action conservation is tested directly on body values.

Genome storage alone is 99,483,648 bytes (94.875 MiB); body buffers, observations,
decisions, ecology, staging and rendering add overhead. Checkpoints exceed
100 MiB. These bounds deliberately replace the old 100,000-body capacity.

Atomic allocation/collection can diverge between population runs even at the
same seed. Isolated deterministic fixtures verify batching and checkpoint
continuation; cross-GPU or population-wide bitwise determinism is not promised.
GUI render pipeline construction is tested headlessly; manual GUI interactions
were not exercised for this experimental branch.

## Extension rule

Can a mechanism justify itself without naming the desired emergent behavior?
If not, keep it outside the physical/controller contract or reject it.
Initial biases, sensing geometry and finite network architecture must remain
explicit even when their resulting behavior is visually appealing.
