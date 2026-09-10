# Sensor-valid reachability and bootstrap audit â€” 2026-09-10

Historical audit: model-specific evidence and earlier proposals follow. The current
release decision is defined in [the north star](north-star.md).

Production biology, ecology, sensing, transfer targeting, reproduction, costs,
mutation and reservoir rules remain frozen after the gathering-rounding repair.
This pass adds test-only controllers and observers. No controller is installed
in ordinary founders or production saves.

## What has been demonstrated

The selected closed-loop control starts with two ordinary adults at 35 energy
and zero food, with no resource or reserve injections. A real in-world offspring
receives food, matures, gathers as an adult, manufactures packets and contributes
to two further births. The stronger observer records both fusion parents.

This is a witness that the available information and actions can compose. The
hand-written controller is externally evaluated, with command memory and a
one-unit initial body; it is not an evolved genome, a proof that one neural unit
can implement the program, or a proof of indefinite population persistence.
Mutated inherited body/packet traits and their real costs remain active.

## Adult sensor tests

The food-steering policy accepts exactly 16 actual GPU food-region input values
plus own energy, inventory and age. It gets no exact cells, position, heading,
actual angular velocity, global clock, identity or food-map lookup. Spin is
estimated from its own previous torque commands. The first policy uses a food
direction vector with forward persistence; the second chooses a food sector
with a forward preference. Decisions are held for four ordinary ticks.

Both were declared before testing at the known seed-91 corridor location. Both
adults in each test built surplus, funded packets and survived 4,000 ticks.
Neither policy produced a birth. The predeclared tie rule selected policy 0.

That fixed policy was then tested on seeds 101â€“108, with 16 standard-constructor
positions and headings per world, ordinary reserves, and a 4,000-tick horizon.
All **128/128** controlled founders built more than .1 carried food, produced
packets and survived the horizon. They produced 2,764 packets and no births.
These are sparse controlled populations, not the ordinary 4,096-founder density.
They establish sensor-valid foraging over unselected placements, not random
behavioral success. Sampled inventory peaks are lower bounds because observations
occur every four ticks. Packet production is measured from the physical records.

Raw evidence: `reports/sensor-audit-01/sensor-adults.json`.

## Natural-harvest care

Care controls add anonymous body occupancy and proximity pressure from the same
regional inputs. Juveniles follow occupancy; adults use food steering and local
cohesion. Transfer is the ordinary nearest-organism action. Policies are updated
every tick. For the care-only comparison, initialization places an actual age-zero
newborn obtained from normal size-16 packet fusion: 21.746 energy and zero food.
Adults start with 35 energy and zero food and must gather all transferred food.
No stores are edited after initialization.

The first test policy required donor energy above 50. Newborns died before donors
reached that threshold. Its transfer-enabled and disabled trials were identical.
This localized a policy gate, not a biological obstruction. A follow-up changed
only this test-policy threshold to 25; production physics remained unchanged.

| Adults | Transfer | Newborn outcome | Received food | Delivery ticks |
| --- | --- | --- | ---: | ---: |
| 1 | Off | Died at age 361 | 0 | 0 |
| 1 | On | Died at age 543 | 1.366 | 59 |
| 2 | Off | Died at age 359 | 0 | 0 |
| 2 | On | Matured at age 1800; both adults also alive | 9.926 | 430 |

In the successful two-adult trial, they gathered 16.171 and 18.045 food. The
juvenile gathered 1.455 food. Its per-tick opportunity chain was:

| Juvenile observation | Ticks |
| --- | ---: |
| At least one adult within transfer range | 1736 |
| Nearby adult had positive pre-interaction carried food | 1299 |
| Such an adult selected transfer | 512 |
| Juvenile was the reconstructed nearest target | 430 |
| Juvenile actually received food | 430 |

There were **84 donor offers diverted to another nearer organism**, but **zero
full-nearest-recipient blocks**. Offer counts can exceed unique child-tick counts
because multiple donors may act on the same tick. Inventory is reconstructed
from real digestion and collection; positions are post-movement and there is no
force in these controls. The CPU nearest reconstruction does not reproduce the
GPU priority tie-break, so exact-distance ties remain a stated limitation.
Actual received food is measured directly, not inferred from targeting.

Raw comparisons: `reports/sensor-audit-01/sensor-care.json` (threshold 50) and
`reports/sensor-audit-02/sensor-care.json` (threshold 25).

## Reproductive coordination and closed loop

With only the two adults initialized, the care policy could fund packets but
did not close the life cycle. Directing packet placement toward the sensed body
region still produced zero births. Over 7,000 ticks it made 12 packets; compatible
packets coexisted after fusion processing for only 17 ticks and never came closer
than 34.506 units. At the first deposit, the other adult had nearly 100 energy
but .005 food, below this policy's .3-food production gate. The adults moved on
before their separately triggered packets could meet.

A single follow-up policy change reacts to an increase in anonymous regional
occupancy: a mature adult above 40 energy may produce a packet, subject to its
existing 200-tick command cooldown. It is not told that the new occupant is a
packet or a partner. The earlier surplus-triggered production and body-directed
placement remain. This uses existing information and actuators, not new signaling
semantics or a production reproduction rule.

The 7,000-tick selected test then achieved the following chain. Times below are
completed, one-based ticks; internal `birth_tick` fields are one lower.

| Event | Tick / quantity |
| --- | --- |
| Founders 1 and 2 produce the first compatible packet pair | 2852 / 2853 |
| Offspring 16387 born from those two founders | 2854 |
| Juvenile receives repeated food | 487 deliveries, about 11.438 food |
| Offspring matures | 4654 |
| First adult gathering by that offspring | 4655 |
| Subsequent births involving that offspring | 5267 and 5835 |
| Offspring's total adult gathering by tick 7000 | 20.560 food |
| Offspring's funded packets | 2 |

Both later offspring died (ages 840 and 475). Thus the demonstrated maximum
closed-life-cycle depth is **1**, while maximum genealogical depth is **2**.
This establishes one complete descendant loop, not indefinite persistence.
The first one-parent trace conservatively counted one descendant-associated
birth; the dual-parent observer verifies that both later births included 16387.
No signaling was used or required by this controller.

Raw spatial/timing diagnostic: `reports/sensor-audit-04/sensor-care.json`.
Successful original witness: `reports/sensor-audit-05/sensor-care.json`.
Successful dual-parent observer replay: `reports/sensor-audit-06/sensor-care.json`.
The final witness includes 15 produced packets, 3 births, 1 maturation, 1 adult
descendant forager, 2 descendant packets and 2 births involving that descendant.

## Amended random-policy bootstrap protocol

The original declaration was seeds 2001â€“2100, 4,096 ordinary random founders per
independent world, and extinction or 10,000 ticks. That run began; by the time it
was stopped, seeds 2001â€“2011 had completed. Their partial reports are preserved
under `reports/sensor-random-100`, including an intentionally incomplete empty
summary file from the interrupted run. They are not an additional sample.

At the user's request, the protocol was amended to **natural extinction or
20,000 ticks**, before starting the replacement run. All the same 100 seeds
are replayed under that rule with the richer observer. No seed is replaced,
selected or dropped based on outcomes. The replacement cohort has its own output
directory, `reports/sensor-random-100-20k`. A living population or viable packet
at tick 20,000 is recorded as **censored**, never as an extinction or failed loop.
The status at tick 10,000 is retained if a world reaches it. Empty-world stopping
is detected in batches of at most 64 ticks.

Each world records the complete funnel: packet manufacture; simultaneous
different-producer packets; births; juvenile delivery; maturation; individual
adult-descendant gathering and packet production; dual-parent births; maximum
closed-life-cycle depth. Global packet coexistence does not itself imply local
contact or enough combined energy for a viable fusion. Per-incarnation event
records link both contributing packets to their actual producers and check that
a closed-link producer matured and gathered before manufacturing its contributed
packet. Mere birth depth is reported separately.

Signals are observed through emissions, nonzero aggregate sensory exposure,
actions selected during exposure, and actual packet manufacture within 16 ticks
of actor exposure. These are associations, not causal signal effects or success
conditions. The prototype raw observer also contains a recent-signal transfer
field estimated from inventory debits. That estimate can include terminal food
drops, so the audited summary excludes it rather than treating it as actual
transfers. Juvenile delivery counts use actual received food and are unaffected.
Food sums in this GPU observer round each event to millifood;
the earlier detailed CPU traces retain floating-point transfer amounts.

Interpretation is fixed: this cohort measures how often random initial policies
enter the viable basin at ordinary density. It does **not** test whether successive
mutation and hereditary-reservoir restarts can approach a solution. No outcome of
this cohort automatically authorizes changes to the frozen substrate.

## Validation and next experiment

The completed 100-world baseline produced **217,684 packets, 413 births in 97
worlds, and 318 juvenile delivery ticks across 36 worlds**. Forty-six offspring
received food, totaling 1.787 food. All 413 offspring died; the longest recorded
juvenile life was 727 ticks. There were **zero maturations, adult-descendant
foragers, descendant packets, descendant-parent births, or closed loops**.
All 100 worlds naturally emptied before tick 10,000; none was censored at 20,000.
The longer horizon therefore did not change survival classification in this
cohort. Global different-producer packet coexistence totaled 324,332 world-ticks.
These results establish a difficult random-policy bootstrap, not evolutionary
impossibility. No production parameters were changed in response.

Signals were common: 49,469,654 emissions and 56,689,301 aggregate exposure
agent-ticks. Transfer was selected during exposure 12,148,069 times; reproduction
10,507,496 times; 74,751 packets were manufactured within 16 ticks of exposure.
These associations do not demonstrate useful communication.

The exclusive raw files and generated summary are in
`reports/sensor-random-100-20k`; `tools/summarize_sensor_bootstrap.py` verifies the
exact seed set and reconstructs the summary. The run's provenance file records
the executable and observer hashes, and copies preserve the protocol source.
Curated numeric evidence is retained separately for the
[100-world baseline](evidence/sensor-bootstrap-100.json) and
[controlled closed loop](evidence/sensor-closed-loop.json), without genome dumps
or personal saves.
The later observer report renames its uncertain transfer-inventory proxy to
explicitly mention possible death drops; the GPU counter itself is unchanged.

The input-isolation test verifies that changing undeclared sensory channels does
not change the controller's output. The observer binds all production buffers
read-only and writes only its own scratch/counters/events. A controlled fixture
matches physical/cognitive state, resources, ecology and ordinary counters with
and without the observer, excluding concurrently allocated diagnostic lineage
labels. Birth and packet counters are cross-checked. The successful closed loop
also passes with the observer enabled and an explicit depth assertion.

Dense GPU comparisons exposed atomic lineage-label ordering and one small
floating-point mismatch; repeated paired probes subsequently matched. Do not
promise bitwise identical dense trajectories across GPU scheduling changes.
Per-world incarnation identities and both parents are recorded within each run.
This pass makes no production scheduling or numerical changes to address that
separate reproducibility question.

After bootstrap measurement, the next scientific experiment is consecutive
natural extinction/restart worlds with the ordinary blind hereditary reservoir
carried forward. It should report progression across restarts separately from
long-run population persistence. Reservoir admission, mutation and climate rules
remain frozen; its duration and stopping rules should be declared before running.

The evolutionary acceptance run is **1,000,000 cumulative simulation ticks in one
unassisted hereditary chain**, starting with seed 3001 and 4,096 ordinary random
founders. There is no world-count target and no per-world censoring horizon.
It uses the production `advance_world` transition and its unchanged blind
reservoir. No seed is selected from the bootstrap outcomes. Only natural
extinction triggers a restart; reaching the cumulative horizon saves any living
world without forcing it to end. A positive closure does not shorten the run.

This supersedes the initial 100-world batch declaration while that process was
already running. That executable saves after its 100th natural extinction. Its
exact checkpoint and verified world journal carry the same chain into the
remaining ticks; no founder reconstruction, new initial seed, reservoir reset or
outcome selection occurs at the process boundary. The initial process also had
a 200,000-tick per-world censor that stopped the process rather than ending a
world; the amended runner removes it. Original source and protocol are preserved.

Per-world reports retain the same read-only funnel and both-parent evidence;
the journal is flushed after every world. Record the first fed juvenile, first
maturation, first matured descendant reproduction, increasing closed-loop depth,
and the distribution of world durations. These are observations, not rewards or
selection criteria. After evolved closure is demonstrated, complete engineering
save/restart/desktop checks, freeze, review, merge/push to main, and prepare the
wallpaper release. An absent closure leaves that acceptance gate unmet and does
not authorize biological tuning.
