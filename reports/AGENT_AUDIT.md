# Agent and training audit — September 4, 2026

Scope: actual physiology-v2 implementation at aa805d3, followed by the authorized
primitive-v3 clean cutover. Historical trained banks do not validate this new model.
This is a model audit plus implementation work, not a claim that every emergent
behavior or every concurrency edge case has been exhaustively proven.

## Decision rule

Keep a rule when it supplies a coherent finite body, local information, consistent
resource consequences, or a necessary compute bound. Remove a rule that chooses
an interpretation or outcome the controller could choose itself. An engineering
bound is still authored; calling it physics does not make its exact value sacred.
Autonomy means choosing intentions, not selecting whether conservation applies.

## Capability-by-capability decisions

| Area | Audit finding | Decision |
| --- | --- | --- |
| Initial policy | Hand-coded gathering, energy/inventory reproductive trigger, local food steering, depressed social logits | Removed; exchangeable random initialization, honestly unprepared |
| Sensing | Eight compass food points, four sampled neighbors; no global destinations | Keep local budget/geometry; disclose sampling limitations |
| Crowding | Cell occupancy calculated but absent from cognitive inputs | Expose raw local count; no crowding trend, departure alarm or reward |
| Presence | Empty neighbor and some actual bodies could look identical; zero event indistinguishable from silence | Explicit body and signal presence bits |
| Previous action | Numeric action index invented an ordering between unrelated acts | One-hot encoding |
| Movement | Per-axis tanh saturates differently along axes and diagonals | Radial saturation; keep finite speed and distance cost |
| Memory | 16 recurrent values; no authored importance/forgetting list | Keep; no map, bigger brain or scripted persistence added |
| Action budget | One body action plus concurrent movement; shared amount/target channels | Keep compact interface; no impossible-action fallback |
| Gathering | Chosen, inventory-limited, finite throughput, dropped food first | Keep; commodity priority/quantization are implementation choices |
| Digestion | Automatic conversion of carried food to energy | Keep as body physiology; no free resource or navigation help |
| Transfer | Controller-owned local quantity of existing stock | Keep; no imposed generosity, consent utility or gift reward |
| Force | Energy-based success lottery, fixed recipient damage, food spill, fixed push-away, east fallback at overlap | Remove combat package; independently chosen paid displacement vector |
| Signal | Mixed transfer/force/message scalar; one recipient and four-tick cooldown; pair claims could block contact | Local scalar emission through visible source; no physical pair claim, event valence or semantic labels |
| Reproduction | Chosen and energy-paid, plus mandatory one food unit competing with digestion | Keep chosen construction/investment cost; remove extra stock requirement and child food transfer |
| Development / aging | Maturity, recovery, juvenile speed, finite lifespan | Retain explicit body assumptions; not declared universal necessities |
| Birth allocation | Lower slots preferred at capacity | Rotate parent priority; retain bounded capacity without a population floor |
| Heredity | Fixed mutation at actual birth, empty child state, no adult weight learning | Keep; parents transmit weights, not memories |
| Ecology | Authored patches, connecting low-yield regions, relocations and weather | Keep environment; never call its bands evolved roads |
| Preparation | Endpoint descendant sampling plus fresh-world resets; limited direction variety and one population chain | Independent lines, explicit ramp, varied longer worlds, separate evaluation |
| Evaluation | Survival and interesting-looking activity can be confused with general adaptation | Keep held-out comparison and failures; no ability-use quota or auto-promotion |
| Observation / saving | Diagnostics must not alter decisions or overwrite prior artifacts | Keep isolation and new-file saves; distinguish current vs historical schemas |

## Reproduction finding: requests were not absent

An exploratory random-founder uniform-distribution run (seed240901) made195,791
reproduction requests but zero eligible births. Inventory failure was recorded
194,220 times; gate failures overlap and are not independent causal percentages.
The extra food requirement competed with automatic digestion. This was an
authored restriction, not proof that random weights universally avoided birth.

After removing that requirement, the same seed produced687 births but became
extinct by the3,072-tick sample. There was no free food: offspring energy and
construction were paid by the parent. This change permits reproduction; it does
not prove sustainable feeding/reproduction or establish an optimal birth rate.

Those early smoke binaries had stale physiology-v2 model text in their report
identity while executing in-development v3 mechanics. They are exploratory
artifacts, not compatible v2 evidence or registered v3 results. The identity was
corrected before the16-line registered founding experiment. The interface and
genome dimensions now come from Rust constants when assembling GPU shaders.

## Training philosophy

Difficulty alone does not create learning. Reproduction supplies opportunities
for mutation; survival determines which descendants remain. Too little pressure
may admit fragile habits; complete extinction prevents further within-world
inheritance. Selecting the last starving individual is not the same as selecting
a viable family. Hence: independent populations, a transparent initial ramp,
then varied lean/recovery/crowded worlds rather than endless escalation.

The current [protocol](../training/TRAINING_PLAN.md) is a finite starting design.
It does not claim optimal durations or proven generalization. The separate
founding-only run asks whether random programs can establish repeated generations.
If it fails, report that; do not label it a completed200k training campaign.

## Things deliberately NOT added

No migration reward, communicated food coordinate, trust/honesty label, global
map, target chooser, forced exploration, rescue, population floor, skill-use
quota, artificial mating strategy, or online survival bonus. No promise that
every capability will evolve a use. An unused ability may be irrelevant to the
environment, difficult to discover, or simply not advantageous; observation and
causal intervention are how we distinguish those explanations.

The broad completion contract remains unachieved. Public release requires a clear
runnable build and credible behavior evidence, not just this architectural cleanup.
