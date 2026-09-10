# Evolutionary accessibility: interpretation of the frozen chain

Historical audit: model-specific evidence and earlier proposals follow. The current
release decision is defined in [the north star](north-star.md).

The sensor-valid controller establishes a compositional behavioral witness. It
does not establish a path of small heritable improvements to that witness. The
unassisted 1M cumulative-tick chain measures one such search history without
changing production mechanisms or selecting favorable restart seeds.

## Completed 1M result

The chain completed exactly **1,000,000 simulation ticks**, with 109 natural
extinctions and world 110 saved alive at tick 2,080. It began at seed 3001 with
ordinary 4,096 random founders. The existing process boundary after world 100
was crossed through its exact checkpoint; the loader verified world, seed, tick
and completed progress against the original journal before the normal restart.
No world was forced to end. The final living world is censored, not extinct.

| Observation | Result |
| --- | ---: |
| Packets produced | 342,314 |
| Births | 1,115 |
| Juveniles receiving any food | 147 |
| Juvenile delivery ticks | 1,097 |
| Total juvenile food received | 6.148 |
| Juveniles matured | 0 |
| Matured descendants reproducing | 0 |
| Maximum closed-loop depth | 0 |

All 1,115 offspring were dead at the final observation. The first feeding occurred
at world 1, tick 15; the longest offspring life was 403 ticks. Natural-world
durations ranged from 8,775 to 9,347 ticks, with median 9,144. First/last-quarter
medians were 9,160/9,172 ticks, with no substantial extension. Mean births per
completed world rose from 5.52 to 17.33 across those quarters. This is a
descriptive comparison within one chain, not a replicated causal estimate.

Reservoir snapshots counted 1,112 changed record slots across sampling intervals;
distinct genomes rose from 2,580 to 3,064. Changed slots underestimate intervening
replacement. Turnover and increased births did not establish successful juvenile
development. See [curated evidence](evidence/unassisted-chain-1m.json), the original
`reports/unassisted-reservoir-001` journal, and
`reports/unassisted-reservoir-1m-continuation` with the final checkpoint.

## What the reservoir actually preserves

`shaders/update_reservoir.wgsl` admits an organism only on its birth tick, with
zero lived ticks and positive ancestry depth. It copies the newborn's inherited
genome and cognitive traits. Admission does not inspect received food, lifetime,
maturation, subsequent packets, or offspring. Later death does not delete or
discount that record. Colliding births resolve a whole-record replacement claim;
the replacement RNG advances by the number of birth attempts.

`Simulation::advance_world` in `src/evolution.rs` requires natural extinction,
then uses the normal rollover path to sample unchanged pool records uniformly
with replacement. These records receive ordinary founder bodies, including adult
starting age, 35 energy and zero food. Learned weights, recurrent state and traces
are not inherited. Completed-world observations never enter these draws.

Consequently a genotype from a juvenile that died before maturity can return in
an adult founder body. Birth can therefore carry heredity across extinctions
without successful development in the preceding world. This is a deliberate
cross-world continuity assumption in the current model, not a numerical accident.
The existing reservoir tests verify whole-record copying and that altering world
observations does not alter founder sampling.

A direct checkpoint comparison confirms actual re-entry. A fresh tick-zero
seed-3001 checkpoint reconstructed all 4,096 original inherited founder records.
At the end of the 1M chain, **848 of the 4,096 reservoir entries** were absent
from that original set. Of the 77 living adult founders at world 110, tick 2,080,
**13 also carried inherited records absent from the original founder set**.
Comparison uses exact genome and inherited-trait bytes; private learned state is
excluded. All 1,115 offspring in that chain had died without maturation.
These observations demonstrate hereditary re-entry without successful development,
not just the source-code possibility. They do not isolate the causal effect of
an individual care mutation. See [re-entry evidence](evidence/reservoir-reentry-1m.json)
and the read-only `tools/audit_reservoir_reentry.py` comparison.

## What follows, and what remains unproven

Care can help physically before it closes a life cycle. In the isolated
sensor-valid one-adult control, enabling generic transfer extended juvenile life
from 361 to 543 ticks with 1.366 received food. That did not produce maturation.
The separate two-adult control delivered 9.926 food and reached maturity. These
are different care scenarios, not a measured linear dose-response curve.

Under the present reservoir rule, a longer-lived juvenile does not directly earn
more representation. It may get more opportunities to receive food, mature and
eventually reproduce; only additional births create additional admission
attempts. Adult behavior can also affect its own survival and packet production.
Whether a small increase in proximity or transfer has a net hereditary benefit
depends on those physical consequences and who inherits the resulting genomes.
The source code alone cannot establish that gradient.

A full-reproductive-continuation admission gate is not an automatic remedy.
Restart sampling copies records unchanged; mutation occurs during reproduction.
With zero qualifying continuations, such a gate would admit no newly produced
genomes and the cross-world pool would remain fixed. It could discard the
partial variation whose accumulation is being investigated. This consequence
follows from the current scheduling and sampling rules; no gate was implemented.

## Controlled partial-care dose response

After the unchanged chain finished, the existing two-adult natural-foraging care
fixture was repeated at seed 91. Adults started at 35 energy and zero food; the
same actual newborn from ordinary packet fusion was placed at initialization.
The same sensor-limited policy ran with an external diagnostic gate that stopped
transfer after a declared number of actual juvenile delivery ticks. This gate
was confined to the experiment and was never part of an evolved controller or
the hereditary chain. Multiple donors can contribute within one delivery tick;
food quantities, not just event counts, are recorded.

| Delivery-tick cap | Food received | Age reached | Nearby-adult ticks | Outcome |
| --- | ---: | ---: | ---: | --- |
| 0 | 0 | 359 | 295 | Died |
| 1 | 0.025 | 362 | 298 | Died |
| 5 | 0.125 | 375 | 311 | Died |
| 20 | 0.458 | 420 | 356 | Died |
| 50 | 1.117 | 507 | 443 | Died |
| 100 | 2.170 | 646 | 582 | Died |
| 200 | 4.486 | 954 | 890 | Died |
| 400 | 9.205 | 1,695 | 1,631 | Died |
| Unlimited | 9.926 in 430 delivery ticks | 1,800 | 1,736 | Matured |

Partial care had a smooth, substantial physical payoff in this scene. Added
lifetime also added contact and available-food opportunities: the latter rose
from 206 ticks without delivery to 1,219 at cap 400 and 1,299 with unrestricted
care. There were zero full-nearest-recipient blocks throughout these trials;
other-recipient diversions remain recorded separately. This does not establish
that targeting is harmless in every arrangement.

The energy ledger closes to within 0.00042 energy in each trial:
initial energy + 8 times actual food ingested - recorded expenditure - final
energy. There is no evidence here of an accidental numerical loss swallowing
small feeding doses. One selected geometry and policy do not establish the
frequency or genetic accessibility of this response across ordinary populations.
Physical payoff passes this controlled test; hereditary payoff remains open.
See [dose and maintenance evidence](evidence/juvenile-care-dose.json).

## Juvenile maintenance accounting

The production body-update shader charges **0.05 basal energy per tick at every
age**, plus **0.00025 per expressed neural unit**. Matched GPU body-update probes
at ages 0, 450, 900, 1,350 and 1,800 confirmed identical debits: 0.05025 with one
unit and 0.054 with sixteen. Separate lifetime-learning, gathering, turning,
movement and interaction charges add to these amounts in full runs. The probe
isolates the body pass; an initial attempt using an entire tick correctly exposed
additional memory-trace writing costs, which are not basal maintenance.

Over 1,800 ticks the basal term alone is 90 energy. Development already changes
reserve/inventory capacity, movement and gathering, but not that basal term.
The simulator does not define a conserved body-mass variable from which a unique
maintenance law follows. Development-scaled upkeep is therefore a possible
physical-model choice, not a demonstrated numerical correction. No maintenance,
gathering, transfer, ecology or reservoir parameter was changed.

In particular, zero maturation during a finite chain does not distinguish a very
rare accessible path from a broad region with little incremental reproductive
payoff. Birth admission may weaken selection for development, but the current
evidence does not isolate it as the cause. Requiring maturation for admission
would change the selection assumptions; it is not automatically a correction.

The remaining useful measurements are isolated, declared perturbations of
heritable partial policies: actual inherited descendants per small change in
proximity/transfer behavior, including the donor's foregone reproduction and
relatedness of recipients. The dose-response and added contact opportunities
are now measured, but are not a substitute for that hereditary comparison.
Any such controlled experiment must remain separate from the uninterrupted
hereditary chain. No ecology, dependence, signaling, cognition, mutation or
reservoir change is authorized by this audit.

The final gate remains an unassisted lineage that matures and reproduces.
It has not passed. Prolonged engineering release acceptance, model freeze,
merge/push to main and delivery as a validated release remain pending that gate.

## Declared A/B continuation

Both arms load the identical final 1M checkpoint, including live bodies, brain
state, environment, RNG streams and reservoir. Each receives another 1M cumulative
ticks across unlimited natural transitions. A retains newborn admission.

The user clarified B while A was running: an actual successful birth makes its
mature hereditary producers eligible, including founder incarnations. One producer
is chosen blindly per birth, preserving A's replacement attempt count, collision
rule and pool RNG increment. B captures that packet's immutable genome and traits
before fusion; it never stores the newborn recombinant. Packets remain valid after
their producer dies. Manufacture alone and failed fusion admit nothing. A novel
recombinant cannot enter until it develops, produces a packet and contributes to
a successful birth. Frequency changes among already eligible founders remain possible.

The earlier guarded no-admission B excluded founder readmission and is superseded.
Its source/protocol are preserved; it is not the requested mature-parent comparison.
A finishes unchanged. The full B runs separately from the original shared fork,
with its own output directory, B-mature-parent. Neither physical parameters nor
controllers change. This single fork tests consequences of removing the bypass,
not whether the already demonstrated bypass exists. It also inherits A's historical
pool; it is not a fresh-founder replication of B.

B remains test-only. Its checkpoint requires the B runner on resume; the production
app would otherwise use A. It is not a wallpaper release. The final evolved gate
has not passed. Protocol and per-world evidence are in reports/reservoir-ab-1m.

## Completed A continuation

A completed exactly 1M additional ticks, with 109 natural extinctions and a live
final world at tick 4,096. This extends the same original A lineage to 2M total.
The additional segment produced 8,146 births; all 8,146 offspring died. Of those,
607 received food, through 3,801 delivery ticks totaling 21.897 food. No offspring
matured or reproduced. Maximum offspring death age was 515 ticks.

Mean births per full world rose from 24.74 in the first quarter to 146.56 in the
last. Median natural world duration was 9,163.5 ticks; maximum was 9,294. Distinct
pool genomes rose from 3,064 to 4,033, with 7,940 changed-record slots summed across
samples. These are descriptive observations of one chain, not a replicated claim
that birth admission alone caused this trajectory. They show substantial
hereditary turnover without any completed offspring development.

The completion file's old B scope text is a shared-writer labeling error; A's
arm label, exact horizon, journal and checkpoint establish what ran. A completed
and saved before the obsolete B process was stopped. Full mature-parent B runs
from the original fork and is not a continuation of the superseded B phase.
