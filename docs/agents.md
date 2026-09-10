# Agents

The controller owns intentions. The world owns consequences. The design contract
is [direction.md](direction.md); all execution modes use the same biology.

## Controller and inherited state

There are 107 physical inputs, 16 potential gated recurrent units, and 14 outputs:
2,494 inherited float32 parameters. A heritable mask expresses 1-16 units. Every
unit has the same connectivity and update equations; inactive units have zero
state, readout, learning, write cost, and upkeep effect. Latent inherited weights
remain available to blind topology mutation.

Candidate activity is tanh(input projection + previous-state projection + bias).
A gate is clamp(candidate projection + bias, 0, 1). Next state is
`(1-gate)*previous + gate*candidate`; zero retains exactly and one replaces.
Outputs are linear projections of next state. Candidate means an intermediate
neural activation, not an evolutionary contestant.

Each active unit has an inherited signed local-plasticity rate. Local pre/post
activity, inherited trace retention, and inherited learned-weight retention update
bounded fast connection deltas. There is no reward, optimizer, outcome label, or
mandatory learning. Birth clears recurrent state, traces, and learned deltas.
Only inherited weights, the active mask, plasticity/retention traits and the three
mutation controls enter hereditary storage. Speed and sensory radius copy at birth
and are fixed by world physiology, not evolved morphology.

Body upkeep is 0.05 energy/tick. Each active unit adds 0.00025; actual absolute
recurrent/trace/learned-state changes cost 0.0001 energy per unit of change.
Capacity and learning are optional and paid. Genome copying has no extra upkeep.

## Inputs (zero-based)

| Inputs | Physical measurement |
| --- | --- |
| 0-3 | Energy/100, inventory/8, food underfoot, age/10,000 |
| 4-5 | Actual velocity in the body frame /1.2 |
| 6-7 | Net own energy/100 and inventory/8 changes since last tick; zero at birth |
| 8-9 | Last integrated displacement in the current body frame /1.2 |
| 10 | Nearby body count /16 |
| 11-106 | Sixteen repeated samples, six channels each |

Each sample contains mean food density, body count/16, mean relative velocity x/y
in the body frame /1.2, mean signed signal activity, and mean proximity pressure.
Pressure is `max(0, 1-distance/radius)`. Channels are finite and clamped to [-8,8].
There are no unused input slots, target identities, nearest-body records, signal
self-history, success inputs, other bodies' inventories, absolute coordinates,
absolute heading, lineage, map, or global population inputs.

Samples partition the local disk into eight body-relative angular wedges and two
radial bands (inside/outside radius/2). Default radius is 24. Every sample uses the
same six channels, without nearest-neighbor selection. Food integrates wrapped
4-unit grid-cell centers within the disk; bodies contribute according to their
actual wrapped positions. Empty channels read zero. This is finite-resolution
area sampling, not point vision or identity tracking. The square resource lattice
has quarter-turn and grid-translation symmetries; arbitrary subcell rotations or
translations can change sampled food through raster aliasing. Continuous body
geometry uses the same body-frame transformation at every bearing.

Signal activity averages other bodies' preceding-tick scalar emissions in each
sample; silent bodies contribute zero. Signed cancellation and zero emissions are
indistinguishable from silence in this aggregate measurement. Transmission has no
built-in vocabulary, receiver identity, delivery guarantee, or authored meaning.
Inspector metadata is not visible to the controller.

## Outputs (zero-based)

| Outputs | Capability |
| --- | --- |
| 0, 2-5 | Primary logits: none, transfer, contact impulse, emit, manufacture packet |
| 1 | Independent gathering effort, clamp to [0,1] |
| 6 | Turn effort, tanh, torque up to 0.0375 radians/tick² |
| 7 | Signed forward thrust effort, tanh(output * motor gain) |
| 8 | Transfer amount or fraction of reserves available for packet manufacture, sigmoid [0,1] |
| 9 | Signal scalar, tanh [-1,1] |
| 10-11 | Body-relative contact impulse, radial tanh, magnitude at most 3 |
| 12-13 | Body-relative packet placement, radial tanh, distance at most 2 |

The largest enabled primary logit wins; exact ties use the earlier output index.
This is an explicit categorical-effector convention, not a ranking of organisms.
Movement and gathering can accompany any primary action. Transfer, impulse,
emission, and reproduction share one primary effector. Impossible intentions are
not replaced with useful ones. There are no controller target slots.

Turn output applies torque: `angular_velocity = 0.85*angular_velocity +
0.0375*turn_effort`, then heading integrates angular velocity and wraps. Applied
turn effort costs `0.02*abs(turn_effort)` energy; effort scales down when energy
is insufficient. Angular coasting incurs no effort charge. Founders and newborns
start with zero angular velocity. Heading updates first, then
damped velocity becomes `0.85*velocity + thrust`, then
position wraps after integration. Maximum adult voluntary thrust is 0.18, giving
1.2 cruising speed under sustained straight effort without contacts. Contact can
change speed independently. Thrust cost is `length(thrust)/0.15 * 0.01`; drifting
velocity damps without another thrust charge. Force and placement vectors use the
updated heading at the physical boundary. All bodies have equal unit inertial
mass; no mass field exists because it never varies.

Nonfinite output is flagged, clears recurrent state and actuator intent, and does
not bypass digestion, upkeep, or death. Numerical normalization floors are 0.0001.

## Initialization and mutation

Fresh founders use seed-specific random genomes: sensory weights uniform in
[-0.1,0.1], other parameters in [-0.35,0.35], and an additional 0.5 gate bias.
Active capacity is uniform from 1-16; unit locations are randomized. Heading is
uniform relative to the world's rotation. There is no scripted founding policy.

Parameter-mutation rate, parameter-mutation step and topology-mutation rate are
independent inherited multipliers bounded to [0.25,4], initially log-uniform.
Parameter variation occurs with probability 0.25*rate; one expressed parameter,
one active plasticity rate and both retention traits receive bounded 0.03*step
perturbations. Each mutation control drifts independently during that event.
Topology retirement and activation independently occur with probability
0.01*topology-rate when eligible. Activation duplicates a random expressed unit
with bounded jitter and splits outgoing weights. Mutation is blind; exact copies
are permitted. See [evolution.md](evolution.md) for continuity and provenance.

## Reproductive packets

Action 5 requests manufacture of one packet. After physical upkeep, learning and
body interactions, the organism must be mature (age 1,800) and its spending budget
`energy * sigmoid(output 8)` must cover its inherited `packet_size`. Manufacturing
debits exactly that size in energy. It neither needs a partner present nor causes
a birth. Output 12/13 places the packet within two body-relative world units of
the producer. Output 8 limits spending; it does not determine packet size.

Packet size is a heritable scalar bounded to [1,48], initially uniform. It is
blindly inherited from either packet at fusion and, during the existing parameter
mutation event, multiplied by a uniform [0.9,1.1] factor and clamped to its bounds.
Smaller packets permit more manufacturing from the same energy; larger packets
carry more offspring provisioning. At most one packet is manufactured per body
per tick, an explicit throughput ceiling. There is no reproductive cooldown.

Packets carry their own immutable genome and trait snapshots. They do not think,
gather, signal, learn, or propel themselves. Ordinary organism contact force can
push them, with the same paid impulse and opposite recoil as any other target.
Packets integrate velocity with world wrapping, then damp it by 0.98 each tick;
force does not damage them or change their genome or reserves. Transfer remains
organism-only, and only compatible packet pairs can fuse. Packets
spend `packet_upkeep * packet_size^(2/3)` energy per tick. The upkeep
coefficient smoothly rises from .002 to .02 over world ticks 0–100,000. They expire at zero energy, without a separate lifetime timer. A fresh
packet has an initial viability scale of approximately 500–1,817 ticks, falling
to 50–182 at normal upkeep. Existing packets pay the current world-age rate. This is a
modeling choice, not a measured biological constant or guaranteed social outcome.

The fusion radius smoothly falls from six wrapped units to two over world ticks
0–100,000 (four at tick 50,000). Two packets within that radius fuse if they
came from different producers.
Producer provenance is a physical compatibility check, never a controller input;
there are no kinship classes, mating types, sex labels, preferences or mate scores.
Nearest contact and a tick-varying unique arbitration key resolve contention;
each packet participates at most once. Packets released this tick are eligible
starting next tick. Producers need not still be alive or release simultaneously.

Fusion consumes both packets. Remaining energies sum, and a fixed default
`fusion_loss = 10` is dissipated constructing the offspring. If that exhausts the
reserves, fusion fails and leaves no organism. Neither producer pays again.
The offspring appears at the wrapped midpoint with zero velocity, age, signals,
memory, traces and learned deltas. Its heading is uniformly randomized.

For each of sixteen neural modules, a fair draw chooses either packet's expression
bit, input/recurrent/gate rows, biases, readout weights and plasticity coefficient
together, including latent units. Empty expression is blindly redrawn. Output
biases and global hereditary traits segregate independently; mutation follows.
Genome donation never depends on packet size or contribution.

Signals retain their existing local physical range (default 24 units), much
longer than fusion range. There is no signal requirement, predefined meaning,
cooperation reward or scripted courtship. Transfer, pushing, local sensing and
private memory remain available. Packets contribute unsigned physical occupancy
to the existing generic samples, without a semantic packet channel.

Organisms and packets share 16,384 entity slots. When manufacture exceeds free
storage, a tick-varying cyclic allocation order admits the requests that fit;
others are counted and skipped without charge. The game never pauses for this
limit. Fusion reuses a consumed packet's slot and can occur with no free slots.
This admission rule is an acknowledged gameplay limit, not a biological result.

Body observers exclude packets. `packets_produced` records manufacturing, not
successful offspring. An offspring's family/parent annotation describes one
producer branch; it is not a complete two-parent pedigree. Extinction waits until
both organisms and viable packets are gone. Successful offspring alone enter
the hereditary reservoir.

## Juvenile physiology

Default maturity is 1,800 ticks. With `x = clamp(age / maturity, 0, 1)`,
gathering yield is multiplied by `floor + (1-floor)*x^6`. The world-age
opening ramp sets `floor = 1 - 0.99*smoothstep(0, 100000, world_tick)`: 100%
at tick zero, 50.5% at tick 50,000, and 1% from tick 100,000 onward. This uses
the same saved world-age schedule as food and packet assistance, even with a
static landscape. It never observes transfers or reproductive success. Existing
bodies experience the current conditions; newborns do not restart the ramp.
Maturity stays at 1,800 ticks, with no moving maturity threshold.
Gathering effort still pays
its ordinary cost. The resource ledger rounds down to millifood, so very small
requests can yield zero. Movement, cognition, signalling and automatic digestion
retain their ordinary mechanisms; there is no feeding quota or care action.

Energy storage grows linearly from 48 to 100 and food inventory from 1 to 8.
Digestion retains its ordinary throughput and conversion efficiency but stops at
the current energy capacity. Gathering and generic transfers respect inventory
capacity. Fusion retains remaining packet energy after construction loss up to
the newborn's 48-energy capacity; excess is dissipated during construction.
Larger packets improve birth reserves until that physical saturation, while
smaller packets remain cheaper to manufacture. Packet upkeep still rewards larger
packets with a longer viability window. No packet size buys juvenile independence.

At default upkeep, a stationary body needs at least 90 energy to mature, before
cognition or effort. Maximum birth reserves are 48; one full newborn inventory
adds only 8. Early in the ramp, gathering can support independent juvenile
survival. After assistance ends, a GPU regression supplies unlimited ground food and maximal
gathering effort: the early deficit still kills that unprovisioned juvenile.
A separate controlled fixture survives through ordinary repeated food transfers.
These are physical feasibility checks, not behaviors installed in founders.
Custom metabolism, conversion or maturity settings can change these bounds.

The initial population consists of mature bodies with 35 energy and unchanged
seed-specific random controllers. This supplies the bootstrap generation for a
world without pre-existing provisioners. Every actual offspring starts at age zero.

Family report schema 3 adds `juvenile_transfers_received` (successful receiving
ticks; interaction pairs are disjoint) and `juvenile_received_milli`. Existing
`juvenile_starvation_deaths`, `matured_descendants`, `births`, maximum depth and
`births_to_descendant_parents` distinguish mortality, maturation and reproductive
continuity. These counters cover descendants, including terminal ticks, and are
observer-only: no controller can read them. The model ID is now
`primitive-v41-juvenile-opening-ramp`; old model checkpoints are rejected.
