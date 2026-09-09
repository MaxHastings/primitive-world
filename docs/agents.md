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

Body upkeep is 0.015 energy/tick. Each active unit adds 0.0005; actual absolute
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
| 0, 2-5 | Primary logits: none, transfer, contact impulse, emit, reproduce |
| 1 | Independent gathering effort, clamp to [0,1] |
| 6 | Turn effort, tanh, up to 0.25 radians/tick |
| 7 | Signed forward thrust effort, tanh(output * motor gain) |
| 8 | Transfer amount or offspring investment, sigmoid [0,1] |
| 9 | Signal scalar, tanh [-1,1] |
| 10-11 | Body-relative contact impulse, radial tanh, magnitude at most 3 |
| 12-13 | Body-relative offspring placement, radial tanh, distance at most 2 |

The largest enabled primary logit wins; exact ties use the earlier output index.
This is an explicit categorical-effector convention, not a ranking of organisms.
Movement and gathering can accompany any primary action. Transfer, impulse,
emission, and reproduction share one primary effector. Impossible intentions are
not replaced with useful ones. There are no controller target slots.

Heading updates first. Damped velocity becomes `0.85*velocity + thrust`, then
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
