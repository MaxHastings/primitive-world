# World and body rules

[direction.md](direction.md) is the design contract. These constants describe the
implemented world, not inevitable laws of life. Model identity is
`primitive-v39-shorter-lifespans`, checkpoint format 57, founder-bank format 21.
The release/freeze evidence is tracked in [implementation-checklist.md](implementation-checklist.md).

## Geometry and tick order

The default habitat is a 2048-unit square torus with 512x512 resource cells and
256x256 occupancy cells. Rectangular habitats are supported. Wrapped shortest
displacement governs sensing, contact, ecology, interventions and UI picking.
Exact half-world ties choose the negative displacement. Resource-grid aliasing
limits exact rotation symmetry to square-grid quarter turns; bodies and their
headings use continuous relative geometry.

1. Reserve slots dead at tick start; update ecology and pre-action spatial indexing.
2. Sample the same pre-action world; evaluate controllers and read-only observers.
3. Gather at pre-movement positions using proportional sharing within each food cell.
4. Organisms digest inventory carried into the tick, add newly gathered food to
   inventory, pay gathering effort, turn, damp velocity, apply thrust and
   integrate; pay body/cognitive upkeep, age, check death and emit. Packets consume
   their reserves for viability without running a controller.
5. Apply paid local plasticity; rebuild post-movement contact indexing.
6. Choose contact proposals from an immutable snapshot, then resolve disjoint pairs.
7. Manufacture affordable packets into free slots; fuse contacting packets in place,
   recombine/mutate inherited records and clear newborn learning. Only successful
   organism births replace whole records in the hereditary pool.
8. Release dead inventory once; count organisms and viable packets.

Contact impulses change velocity for the next integration, rather than teleporting
recipients. Newborns act on the following tick. This ordering is explicit discrete
time physiology, not simultaneous continuous physics.

## Resources and physical costs

| Mechanism | Default rule |
| --- | --- |
| Energy/inventory capacity | 100 energy / 8 food |
| Founder provision | 35 energy, zero food, age zero |
| Body upkeep | Stationary 0.05 energy/tick from tick zero |
| Cognitive upkeep/writes | 0.00025 per expressed unit; 0.0001 per absolute state change |
| Gathering | clamp(output 1,0,1), up to 0.025 food/tick |
| Gathering effort cost | 0.005 * effort, including unsuccessful effort |
| Digestion | At most 0.1 carried food/tick, 8 energy/food, limited by energy headroom |
| Thrust | At most 0.18 adult units/tick of velocity change; cost length(thrust)/0.15 * 0.01 |
| Damping | Retain 0.85 of prior velocity each integration |
| Signal | 0.01 activation + 0.02 * absolute payload energy |
| Contact range | Wrapped center distance at most 6 |
| Contact impulse | At most 3; actor pays 0.1 * squared impulse magnitude |

Gathering is requested, never automatic. Dropped food is allocated before growing
food. All requests sharing a food cell receive the floor of their proportional
share, using exact integer arithmetic. Unallocated milli-food remains in that
cell; it is not awarded by thread timing or body-slot order. The maximum rounding
remainder is less than one milli-food per requester per stock type. Gathering
charges effort independently of action amount and available stock.

Newly gathered food becomes digestible on the following tick. It remains carried
material through this tick's contact phase, so an organism can transfer its intake
without first filling its own energy reserves. The same delay applies to every
organism and action. Untransferred food is retained for next tick's digestion;
death releases it normally. Fresh intake cannot rescue an organism that exhausts
its reserves this tick. This is an explicit assimilation delay, not a cooperation
reward or a conditional change to metabolism.

Transfer moves existing inventory to the nearest available physical contact,
limited by sender stock and receiver capacity. Contact ties and conflicting pair
proposals use a tick-varying bijective hash of intrinsic lineage IDs, independent
of storage slots. Pairs are exclusive; this local matching can leave opportunities
unused. There is no target input/output, kin preference, utility score or reciprocity.

Force adds impulse to the recipient velocity and subtracts the same impulse from
the actor. Equal fixed inertial masses make momentum change exactly zero up to
float32 rounding. Remaining actor energy bounds impulse by sqrt(energy/0.1).
The energy charge is an actuator-effort law. Velocity is dissipative transport
state, not an additional conserved energy currency; there is no claim of closed
Newtonian kinetic/thermal energy. Damping transfers momentum to an implicit
substrate. No collision packing, injury, food spill, loot, or recipient reserve
penalty is modeled. Both beneficial and harmful displacements are possible.

## Reproductive packets and social opportunity

Organisms manufacture local, stationary reproductive packets using their energy.
Packet size is inherited and evolves; production timing and bounded local release
position remain controller-owned. Two packets from different producers fuse within
a radius that smoothly shrinks from six to two wrapped units by tick 100,000. Offspring receive the combined remaining reserves minus ten
energy of construction loss. No body mating or cooldown path remains.

Packet maintenance costs `(.002 + .018 * smoothstep) * size^(2/3)` per tick,
using the same world-age smoothstep as the ecology ramp. Resource depletion ends
viability; there is no arbitrary expiry timer. The opening 500–1,817 tick viability scale falls to 50–182 at normal upkeep; it leaves room for asynchronous coordination without indefinite
broadcast storage. These constants are declared assumptions, not a claim that
coordination or differentiated packet-size strategies must evolve.

Signals are generic, optional and longer-range than fusion. Gathering, transfer,
pushing, local sensing and memory retain their previous physical meanings.
See [the exact packet contract](agents.md#reproductive-packets).

Organism maturity at age 400, juvenile motor development, and uniformly sampled
maximum age of 9,000-11,000 ticks remain coarse physiological assumptions. There is no
packet aging deadline and no prescribed reproductive role.

## Accounting and ecology

Vegetation and dropped stock use integer milli-food; body inventory uses float32.
Digestion converts food into reserve energy. Thrust, cognition, gathering, emission
and construction are explicit reserve sinks. Death rounds remaining food to the
nearest milli-food (at most 0.0005 food residual per death) and discards stored
energy. Per-step float32 regression comparisons allow 0.002 energy/food units;
long-run budgets must include accumulated quantization bounds. Crowded sample
reductions may differ with unordered spatial scatter; the regression allows
1e-6 for those float32 values while requiring integer body counts to match exactly. Rounded cumulative
telemetry is an observation, not an exact ledger. Body counts balance births and body deaths; packet counts separately track
manufacturing, fusion and resource depletion. There is no kinetic-energy conservation claim.

Seeded correlated geography, periodic weather, soil recovery, depletion and seasonal
production evolve on the gradually accelerating environmental clock. Habitat contrast blends
the geography with its mean, preserving mean habitat but not guaranteeing equal
carrying capacity. No parameter depends on population performance. There is no
metabolism ramp, inherited age floor or population-triggered rescue.
New worlds start with 4,096 agents by default. Opening ground cover supplies food across the whole map, including normally
barren cells. With t=clamp(world_tick/100000,0,1), the temporary habitat and
productivity floor is 0.5*(1-t*t*(3-2*t)). Initial food uses the same floor.
It reaches zero smoothly by tick 100,000. Existing rich patches retain normal
capacity and growth; there is no food-quantity multiplier. Climate
and geography start at 10% speed and smoothly accelerate to 100% by tick 100,000.
Soil changes use the same speed factor. The environmental clock integrates this
rate (55,000 environmental ticks have elapsed at world tick 100,000); it never
jumps forward when the opening allowance ends. Rain/drought strength, weather
positions and local growth variation interpolate smoothly between event endpoints.
Food capacity is a target: excess vegetation recedes at 1% of the excess per
environmental tick, scaled by ecology speed. Fractional losses are carried across
ticks. Even disappearing patches fade; dropped food is separate. At tick 100,000
the targets are normal, while existing vegetation continues responding gradually. Agent speed is unchanged; no population or behavior controls the
allowance. Every new world restarts it; checkpoint continuation does not.
The packet model's social outcomes and long-run persistence remain open.

## Persistence, observation and engineering limits

There are 16,384 shared entity slots and a separate 4,096-record hereditary pool.
Excess packet-manufacturing requests are counted and skipped without charge;
fusion reuses a packet slot even at full capacity. The game keeps running.
Accounting/tick horizons roll over to another world from the hereditary pool,
without recording a natural extinction. This is an explicit gameplay policy.

Current checkpoints preserve physiology, settings, genomes, learning, resources,
hereditary pool/RNG streams, bounded history and assisted provenance. Derived
indices/terrain rebuild on load. Validation precedes live writes; incompatible
formats and removed biological fields are rejected without rewriting old files.
Save/export refuses existing destinations. Manually supplied founder banks mark
an externally chosen experiment as assisted; that flag remains sticky through
extinction, history eviction, checkpointing and exports.

Observers cannot supply inputs, change weights, retain preferred genomes or
choose resets. Their identity metadata, search-health histograms and interpretation
remain outside biology. Reports must distinguish finite-resolution observations
from hypotheses about communication, cooperation, planning or intelligence.
