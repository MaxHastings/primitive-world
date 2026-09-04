# Primitive World: clean cutover plan

Status: **implemented and validated**, 2026-09-04. The delivered model is
recurrent-v1 / checkpoint 12, with one bundled descendant bank prepared on
seeds 11 and 22. All three held-out worlds reproduced across generations.
See reports/RECURRENT_VALIDATION.md for the bounded evidence and limitations.
CONTROLLER.md describes the active implementation; the original model remains
recoverable from Git tag pre-recurrent-cutover. The plan below records the
cutover decision, not an outstanding migration or a second runtime.

## Decision

Replace the agent model as one coherent system. Do not gradually patch the old
controller, maintain two permanent runtimes, or require a separate behavioral
experiment to authorize every design decision. Implement the new architecture,
verify its integrity, evaluate it as a whole, then make one deliberate cutover.

**We provide capabilities and consequences. The controller determines how to
use them.** This is a finite evolutionary model, not a promise of open-ended
intelligence, organized routes, cooperation, or population equilibrium.

The original plan-only commit did not implement the model. The subsequent
execution delivered this cutover without migrating old checkpoints or deleting
the user's existing artifacts. Founder competence is finite evidence, not a
claim of organized travel, social balance or open-ended intelligence.

## Ownership contract

| Owner | Responsibility | Must not do |
| --- | --- | --- |
| World | Geometry, resources, weather, possession and physical resolution | Score cleverness, exploration or social desirability |
| Body | Finite senses, reserves, movement and bounded action capabilities | Supply a preferred destination, feeding strategy or settled-surplus strategy |
| Controller | Interpret observations, update private state and produce complete intentions | Read observer metrics, remote world buffers or other agents' private state |
| Heredity | Copy and mutate controller weights at actual births | Copy personal experience or rescue unsuccessful lineages |
| Observer | Record histories, causes, costs, lineage and experiments | Feed labels, route scores or population targets back into decisions |

Each mechanism needs a resource, information, body-capability or engineering-
budget justification independent of the pattern we hope to see. Numeric bounds
remain authored modeling choices; document their meaning rather than calling
them laws of nature.

## Target model: decisions made upfront

### One inherited recurrent controller

- One compact, fixed-size recurrent network per agent. Its weights are the
  inherited genome; each body has its own private recurrent state.
- Inputs: current local measurements, body state, previous intention and
  measured action consequences. Outputs: next intention and next internal state.
  Normalize units, but do not supply desirability features such as movement
  utility or hunger-weighted harvest value.
- The recurrent update controls information retention. Remove automatic
  food-location anchors, food-priority replacement and semantic memory slots.
  Internal state is finite numerical working memory, not unlimited storage or
  guaranteed reliable long-term spatial memory.
- Weights remain fixed during life; internal state changes during life and
  weights mutate between generations. Do not call this within-life weight
  training. Plasticity, evolving topology and external memory are outside this
  cutover, not prerequisites that can keep expanding its scope.
- One documented decision cadence across runtime, preparation and checkpoints.
  No archived trainer-specific timing or hidden overrides.

### Bounded information and controllable attention

- Expose reserves, body age/development as applicable, actual local displacement,
  local food samples, nearby bodies and local events.
- Pair every spatial observation with its actual body-relative coordinate.
  Use a finite near/far sampling pattern within sensory range, with orientation
  controlled by the agent. Geometry and sample count are explicit sensor-budget
  choices, not an automatic search for the best food cell.
- Neighbor selection is bounded and value-neutral. Do not expose people because
  the world has classified them as rich, helpful, dangerous or desirable.
- No absolute world coordinates, remote food locations, patch identities,
  navigation gradients, reputation scores or population statistics. Actual
  displacement supports internal spatial tracking without supplying a map.
- Actual collection, expenditure, displacement and received events are feedback
  measurements, not reward points or labels for good and bad decisions.

### Complete intentions and physical execution

- Directly output bounded movement direction and effort. Remove destination
  preselection, random-goal generation, exploration bonuses, travel-affordability
  advice, commitment timers and forced continuation.
- Locomotion is a separate effort channel from one body action per decision:
  none, collect, ingest, transfer, apply force, emit, or reproduce. Concurrent
  locomotion and a body action is an explicit body capability, not a hidden
  special ban on reproducing while moving.
- The controller selects the body action, observed target where needed, bounded
  amount and signal payload. The world applies physical limits and resolves
  attempts; it does not substitute a more sensible action when an attempt fails.
- Specify one resolution order. Shared reserve accounting must prevent motion,
  transfer and reproduction from spending the same resources twice. Validate
  target incarnation and locality at resolution, not just at observation.
- Retain transfer, contact force and signals as physical capabilities, not
  generosity, aggression or language objectives. Document displacement, spill,
  cost and signal-range mechanics. No automatic structured map copying.

### Deliberate reproduction and actual inheritance

- Reproduction is requested by the controller, not automatically triggered by
  a programmed surplus condition or an eligibility lottery.
- Constrain it by actual parent reserves, offspring endowment, reproductive
  capacity/development and any justified recovery time. Remove the old surplus
  threshold, 3/1024 request lottery and special non-movement condition.
- Transfer reserves explicitly, including stated dissipation. No invented
  matter, double spending, free reproduction or preferred population size.
- Copy weights with documented nonzero bounded mutation. Clear the child's
  recurrent state and personal event history. Lineage IDs are bookkeeping only.
- Body, memory and network capacities stay fixed in this release. Evolving
  morphology or computing capacity is not necessary to complete the cutover.

## Remove from active code

- Candidate-v1 scoring, handcrafted desirability features and movement proposals.
- Place-memory retention, guide attribution, food reports and commitment rules.
- Automatic birth eligibility/lottery and settled-behavior gating.
- Dormant trust, helper, danger, companion and relationship machinery/UI.
- Legacy-controller and archived shared-GRU paths, flags, buffers and bridges.
  This is a new inherited controller, not activation of the old neural experiment.
- Old founder banks as active defaults; their weights have incompatible meaning.
- Tests requiring the replacement to imitate old prescribed actions or routes.

No compatibility shim, production controller switchboard, old-controller
fallback, rescue birth, automatic reseeding, population floor, migration reward,
cooperation reward, or ecology adjustment to make the display look successful.

## Preserve useful engineering

Adapt GPU execution, spatial indexing, resource accounting, identity safety,
headless operation, rendering, observers and checkpoint plumbing where useful.
Preserve existing ecology initially unless an independently identified physical
or correctness issue requires a documented change. This is not an unrelated
renderer rewrite or a simultaneous effort to manufacture a favorable landscape.

Keep integrity tests for conservation, concurrent resolution, locality, invalid
outputs, birth/death identity reuse, checkpoint replay and batched clocks.
Adapt their interfaces; do not keep obsolete architecture merely to satisfy an
old test.

Audit lifespan, rates, recovery and force costs. Record what each represents.
A parameter cannot justify itself solely because it suppresses an undesirable
population curve, conflict level or travel pattern.

## Fresh founders, no hidden competence

Prepare founders for the new architecture; never relabel the old bank. Use a
reproducible initialization with standing variation. Any initial physiological
bias must be explicit, confined to mutable initial weights and recorded in
provenance. No scripted feeding/navigation fallback or distillation of the old
controller into the new one in this cutover.

Preparation uses actual births and living descendants. Sample descendants at
actual family abundance, not by routes, cooperation, novelty or desired
population. Record seeds, conditions, initialization, mutation settings, run
limits, failures, model hash and bank hash. Evaluation worlds never feed weights
back into the bank.

Register a finite preparation/evaluation run budget before execution. Do not
keep trying seeds until an attractive result appears. If all candidates die
before producing descendants, preparation failed: there is no reproductive
selection to carry forward. Investigate the new model or declared initialization;
do not restore old behavior or rescue the live population. An unprepared bank
must never be labeled trained founders.

## One implementation effort, one cutover

These are delivery responsibilities, not staged releases or individual
hypotheses requiring proof before implementation:

1. Preserve the last pre-rewrite revision with a clearly named Git tag. Build
   the replacement on one implementation branch. Keep historical artifacts
   recoverable; never automatically delete checkpoints or local reports.
2. Replace controller, state, intention, reproduction and heredity contracts
   together, including Rust/WGSL layouts, buffer ownership and serialization.
3. Update UI, CLI, observers and preparation together. Show actual observations,
   intentions, resolved outcomes and internal-state values, not invented motives
   or old fields relabeled as new cognition.
4. Remove old active code and rewrite authoritative documentation in the same
   delivery. History preserves old behavior; production flags do not.
5. Verify integrity, complete bounded whole-model preparation/evaluation, record
   results, then merge the complete replacement to main as one cutover.

All automated work and evaluation are headless. Do not control, close, restart
or rebuild in place the user's running application. Internal commits and module
ordering are fine; intermediate usable releases, piecemeal promotion and separate
ablations for every architectural choice are not required.

## Release gates

### Architectural completeness

Exactly one active controller and model contract. No hidden old policy bypass,
memory manager, desirability scorer or founder fallback. Assign an explicit new
model ID and checkpoint schema. Reject incompatible checkpoints with a useful
message; leave existing files untouched.

### Implementation integrity

Formatting, compilation, GPU/layout validation and applicable integrity tests
pass. Invalid outputs cannot corrupt the world. Check that valid output changes
can affect movement, actions, amounts, signals and reproduction. That verifies
wiring; it does not demand that evolved agents exhibit prescribed behavior.

Validate release startup, reset, save/load, parameters and render/UI integration
through available headless tests. Visual interactions not exercised headlessly
are explicitly unverified, not claimed as a manual GUI pass. This plan requires
no computer control.

### Honest whole-model evidence

Run registered, bounded preparation and separate unseen-seed evaluations.
Report births, descendant depth, deaths, reserves, energy/matter accounting,
numerical stability and runtime/memory cost. A prepared founder release must
demonstrate actual multigeneration reproduction; it need not survive every seed
or reproduce the previous population trajectory.

Architecture completion and founder competence are separate statuses. A clean
but sterile model can be committed as an explicitly unprepared research model;
it is not a completed prepared release. Do not hide failure or indefinitely
delay architectural cleanup in pursuit of attractive behavior.

Routes, cooperation, diversity and equilibrium are not release gates. Study
them afterward as questions about the completed model, not required pictures.

## Documentation and end state

- README: one normal launch command, active model/founder identity and truthful
  initialization/checkpoint guidance.
- KERNEL_SPEC.md: the implemented world/body/controller/heredity boundary.
- CONTROLLER.md: exact observations, recurrence, intentions, memory limits,
  initialization and inheritance; no claims of learning that are not implemented.
- This plan: mark the architectural cutover completed only when delivered;
  record founder-preparation status separately if unresolved.
- Reports: preserve historical evidence and add one new-model validation record
  with limitations and reproducible commands. Remove old scripts/imports/flags
  from active tooling or make their archived status unmistakable.

Success is one understandable model whose decisions belong to its controller,
with physical accounting we can trust and results we can describe honestly.
It is not the old simulator with another intelligence layer bolted onto it.
