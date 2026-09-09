# World and body rules

[direction.md](direction.md) is the design contract. These constants describe the
implemented world, not inevitable laws of life. Model identity is
`primitive-v35-body-frame-contact`, checkpoint format 55, founder-bank format 20.
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
4. Digest, pay gathering effort, turn, damp velocity, apply thrust and integrate.
   Pay body/cognitive upkeep, age, check death, emit, and request reproduction.
5. Apply paid local plasticity; rebuild post-movement contact indexing.
6. Choose contact proposals from an immutable snapshot, then resolve disjoint pairs.
7. Allocate affordable births; copy/mutate inherited records; clear newborn learning.
   Successful births replace random whole records in the hereditary pool.
8. Release dead inventory once and count living bodies.

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

## Reproduction and retained lifecycle assumptions

For B=50 and investment a=sigmoid(output 8), child energy is 0.8*B*a and
construction dissipation is 0.2*B. The parent pays their sum; no food is created or
required as a separate prerequisite. Actual parental affordability and survival
are rechecked after contact. Placement is a parent-controlled body-relative vector
of length at most two, wrapped onto the torus. Child heading is a uniform offset
from parent heading; velocity, age, signals, feedback and learned state reset.

The following are deliberately retained physiology, not turnover controls:

| Mechanism | Decision and reason |
| --- | --- |
| Maturity at age 400 | Retain a fixed organ-development time before reproduction; energy alone does not complete development instantly. |
| Reproduction cooldown 240 ticks | Retain a fixed reproductive-effector recovery time; reserve availability does not remove the tissue-recovery constraint. |
| Juvenile speed factor 0.6 to 1 through age 400 | Retain gradual motor development, coupled to the same maturity interval. |
| Maximum age sampled uniformly from 9,000-11,000 ticks | Retain a coarse finite tissue-maintenance horizon; this is an acknowledged hard-aging approximation, not an engineering capacity rule. |

These are modeling assumptions, not empirically derived necessities. Costs and
resource access remain the primary affordability constraints. The current model
has no evolving development, tissue health or repair physiology. Do not tune
these timers to obtain faster turnover or more interesting behavior. Reopen them
only under the evidence criteria in the direction/freeze contract.

## Accounting and ecology

Vegetation and dropped stock use integer milli-food; body inventory uses float32.
Digestion converts food into reserve energy. Thrust, cognition, gathering, emission
and construction are explicit reserve sinks. Death rounds remaining food to the
nearest milli-food (at most 0.0005 food residual per death) and discards stored
energy. Per-step float32 regression comparisons allow 0.002 energy/food units;
long-run budgets must include accumulated quantization bounds. Crowded sample
reductions may differ with unordered spatial scatter; the regression allows
1e-6 for those float32 values while requiring integer body counts to match exactly. Rounded cumulative
telemetry is an observation, not an exact ledger. Population counts must balance
births and all death causes. There is no kinetic-energy conservation claim.

Seeded correlated geography, periodic weather, soil recovery, depletion and seasonal
production operate at full fixed strength from tick zero. Habitat contrast blends
the geography with its mean, preserving mean habitat but not guaranteeing equal
carrying capacity. No parameter depends on population performance. There is no
metabolism ramp, environmental curriculum, inherited age floor or online rescue.
Fresh random founder probes demonstrate reproductive reachability at stationary
upkeep; they do not establish intelligence, adaptation or permanent survival.

## Persistence, observation and engineering limits

There are 16,384 body slots and a separate fixed 4,096-record hereditary pool.
Exhausting body/identity/tick/accounting capacity censors and stops the engine; it
cannot count as extinction or seed a successor. Counters must not silently wrap
and then guide reset behavior. Finite budgets and ceilings are engineering limits.

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
