# Inherited controller and founder preparation

The current runtime is an evolutionary ecology experiment with a finite,
explicit controller architecture. It is not an open-ended intelligence claim.
Physics, local perception and actual reproduction determine what happens.
There is no desired population, migration, cooperation or conflict reward.

## Candidate-v1

Each agent carries 128 inherited floats. The first 16 retain movement-proposal
traits and reserved entries; the remaining 112 are seven independent rows of
16 signed weights. Every feasible action is scored with its own row. Mutations
can reverse a preference and can condition it on local facts. Nearby targets
are scored individually by the agent. The world does not assign social value.

The feature order is: bias, own energy/100, inventory/8, hunger, inventory
space, local food, attainable collection weighted by hunger and inventory
space, attainable ingestion weighted by squared hunger, local competition,
candidate inventory/8, candidate proximity, recent received scalar event,
candidate's recent scalar event, candidate velocity alignment with a movement
proposal, movement proposal utility, and whether this repeats the last action.
Recent scalar events expire after 32 ticks. Their sign is a physical payload,
not a kernel classification of kindness or danger. It is deliberately ambiguous
whether a scalar came from a signal, transfer or force outcome.

The controller still has authored feature extraction and movement proposal
generation. These are architectural biases, not learned discoveries. A more
general recurrent controller is a future experiment, not a hidden claim about
these weights. The bootstrap weights express collection, ingestion, waiting and
movement competence, with a small opportunity cost for unfamiliar social acts.
Founders start with small standing variation. That bootstrap is explicitly
named `candidate-v1-bootstrap`; it is never labelled a pretrained GRU.

Fresh runs default to the bundled `policies/ancestor-v1.json` descendant bank,
prepared on seeds 11 then 22 for 30,000 ticks each. `--bootstrap` explicitly
starts from the physiological seed weights instead. The bank is compiled into
the executable, so launching from another directory cannot silently skip it.
The UI and reports expose the actual founder name. Loading a checkpoint restores
that checkpoint's controller and founders, not the fresh-run defaults.

Birth copies the parent's weights and independently mutates each entry with
10% probability by up to 0.03 in either direction. Bounds are +/-1 for the first
16 entries and +/-4 for action weights. The process has fixed nonzero error.
There is no cost-free, perfectly faithful copying trait. A child's private
place records, event history, recurrent state and travel history start empty.

Force stays enabled and retains its physical costs: up to 0.6 attacker energy,
0.3 recipient energy, and a chance of spilling up to one food onto the ground.
No direct food transfer to the attacker is implied. Transfer and emission are
also feasible and may become preferred through signed weights. An absence of
force in a run must not be confused with the force affordance being disabled.

## Place memory and travel

Agents keep up to 16 fixed, separated observation anchors. A record is refreshed
only for the same food cell. New records near an existing anchor do not slide
its coordinate along with the agent. This corrects the previous loss of return
destinations while travelling. The controller considers locally observed
directions, affordable remembered destinations and persistent exploration.

This supplies repeatable individual navigation, not guaranteed shared trails.
Unvisited food outside sensory range is unknown. Shared corridors require
evidence from individual trajectories and controls; visual clusters alone are
not proof. Signals can affect candidate scores, but no structured map is copied.

## Preparing founders through physical selection

`training/prepare.py` runs ordinary worlds longer than a founder lifespan.
At the end of each preparation world, it samples living descendants uniformly
over bodies, at their actual family abundance. It starts a fresh seed with
those inherited weights. It does not rank action histories, reward a shape or
repair an extinct population. A world with no surviving descendants fails
export explicitly. This is an offline experimental reseeding procedure;
ordinary live runs never spontaneously reseed.

Evaluation seeds never feed weights back into the bank. The script records the
executable and bank SHA-256 hashes, all run commands, long-term births, force,
ancestry, and separate no-force and famine controls. Persistence on tested seeds
is evidence for that finite regime; it cannot establish universal stability.

```powershell
cargo build --release
python training/prepare.py --directory reports/new-experiment
cargo run --release -- --founders reports/new-experiment/founders-2.json
```

`--legacy-controller` preserves the original authored action scores as a named
comparison. `--neural` is an archived shared-GRU experiment. Neither is the
candidate controller. The GRU's weights do not evolve at reproduction; only
private hidden state changes during a live run. Its default survival feedback
does not credit descendants. Its reserve feedback penalizes resource transfers,
including reproduction costs. Neither is an appropriate substitute for lineage
selection without a separately specified experiment.

## Evidence and lifecycle

Reports contain cumulative action ticks and overlapping birth-gate failures
(immaturity, energy, inventory, movement, cooldown), plus all-gates-open ticks
and requests. Force attempts and resolutions, actual energy spent and food
spilled are separate. Resource totals are not evidence of local access.
Counters are 32-bit and intended for bounded experiments; runs exceeding about
4.29 billion events in a counter require a wider-counter schema.

`generation` internally remains the slot-incarnation token used to prevent
identity reuse. `ancestry_depth` is parent depth + 1 and is observer-only. The UI
and evolutionary summaries now distinguish them. `next_birth` is an absolute
cooldown tick; the UI shows remaining ticks and the other eligibility gates.

Reproduction remains an automatic physical gate with stochastic timing. It is
not an explicit agent action or proof that an agent planned parenthood. Mature,
settled agents require at least 2 inventory and sufficient energy (75 at default),
then receive a 3/1024 chance each eligible tick, subject to cooldown and capacity.
Force can qualify as a settled action; loss of reserves, rather than an explicit
anti-conflict birth rule, reduces eligibility. Parent spends 50 energy and one
food by default; child receives 40 energy and one food. The remainder dissipates.

Checkpoint 11 captures expanded genomes, place records, ancestry and counters.
Earlier schemas are rejected rather than reinterpreted. Old checkpoint files
are not deleted. Exported founder weights exclude private memories.
