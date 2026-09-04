# Primitive World: committed project plan

Status: core primitive pipeline, evolving geography, and experience-dependent companion travel implemented. Physical invariants and targeted behavior checks pass on Vulkan; a controlled two-departure scenario measures reports, learned guidance, association, and survival. Long-term social emergence and the full acceptance criteria below remain unproven. See README.md and reports/INTEGRATION.md for current evidence.

This document supersedes the exploratory proposals in the conversation. It describes the target, not features already demonstrated by the current build. Changes to this plan should follow a failed acceptance check or a new user requirement, rather than another abstract redesign.

## 1. The project we are building

Build a persistent, interactive social ecology in which individuals' circumstances and experiences produce different, consequential ways of surviving together across abundance, population growth, famine, migration, death, and recovery.

The experience is to watch individuals and populations, intervene in the environment, and investigate surprising outcomes through their local causes. Population explosions and severe famines are central to the experience. Relationships must change decisions and material outcomes, not merely particle colors or cluster shapes.

The first complete release includes ecology, the full lifecycle, competent individual survival, learned relationships, giving, local food information, and costly physical conflict. It does not promise that every run produces tribes, leaders, war, or collapse. Stable coexistence, separation, failed cooperation, and extinction are valid outcomes.

The project does not require symbolic institutions, human-level cognition, or open-ended evolution to succeed. Those are not prerequisites for completing this plan.

## 2. Decisions we are settling now

| Question | Decision |
| --- | --- |
| Keep reproduction? | Yes, from the foundation onward. Use single-parent reproduction with maturity, reserves, and a birth cooldown. |
| Preserve boom and famine dynamics? | Yes. Verify their resource causes and recovery pathways; preserve the experience rather than numerical defects. |
| First motive for giving? | A small, explicit social-concern preference shared by all agents. Make it adjustable to zero for comparison. |
| Why stay near someone? | Preserve access to individuals who have been useful, while accounting separately for expected harm, food competition, and travel expense. |
| Are agents different at birth? | Initial behavioral weights and adult capabilities are the same. Location, age, reserves, information, and subsequent experience differ. |
| What is learned first? | Place quality, expectations about particular individuals, and reliability of local food information. Policy weights initially remain fixed. |
| Is communication included? | Yes, as a narrow local exchange of one food-location estimate. No global shared map. |
| Is force included? | Yes, after peaceful survival and exchange pass their checks. Force costs energy and can displace and spill food. It does not award automatic loot. |
| What happens at extinction? | The world continues regenerating but agents do not spontaneously appear. The UI reports extinction and offers explicit reseeding. |
| Are group labels agent state? | No. Group/network summaries belong to the observer tools and never feed hidden membership bonuses into decisions. |

## 3. World and physical primitives

### Ecology

Keep continuous space, spatially varied renewable food, fertility, rain/drought, and changing local carrying capacity. Keep the existing GPU spatial infrastructure.

All processes use the simulation clock, independently of rendering and speed selection. The world seed controls environmental events as well as initial conditions.

Fractional regeneration must accumulate or use an unbiased quantization method. Empty ground is not itself evidence of harvesting: soil pressure must distinguish extraction from weather-driven shortage. Depleted locations must be able to recover under suitable conditions after consumers leave.

Use one coherent boundary convention for movement, sensing, and ecological distance. Start with the existing bounded world and reflecting agent movement; environmental distances respect the same bounded geometry.

Add a separate ground supply for dropped food. It is collectible through ordinary harvesting and is not clipped to vegetation capacity. Any decay is explicit and recorded as a resource sink. This prevents deaths or force from silently destroying supplies because a vegetation cell is full.

### Body and lifecycle

Physical state includes position, velocity, energy, carried food, age, adult movement/sensing capability, and next eligible birth time. Inventory and energy have finite capacities. Juveniles have reduced movement capability until maturity.

Birth requires maturity, adequate energy, adequate food, and an elapsed cooldown. A modest per-tick chance among eligible adults avoids synchronized births. A single parent funds the child's initial food and energy and pays an additional dissipative reproduction cost. Birth must leave the parent a viable reserve. Start without mating requirements or inherited personality mutation.

Offspring appear nearby with empty learned memories. Parent and child do not receive an automatic loyalty relationship or copied place knowledge. Ordinary exposure, giving, and later information exchange establish their relationship.

Agents die through depleted energy or a seeded individual age limit. Age-limit variation prevents identical thresholds from dominating cohort mortality. Death releases carried food; remaining bodily energy leaves the modeled usable-resource budget. Generation-aware identities prevent recycled slots from inheriting another individual's relationships.

Recovery comes from surviving agents and recovering resources. No hidden population floor, compulsory population correction, or automatic immigration is added. The fixed GPU capacity is an implementation ceiling and must be reported when reached.

### Resource accounting

Track ground food, carried food, and energy converted into a common food-equivalent unit for accounting checks. Harvesting and giving conserve food. Eating converts it into energy at a documented rate. Birth conserves transferred reserves while dissipating its explicit cost. Living, movement, communication, and force spend energy. Environmental growth supplies food; recorded decay and environmental destruction remove it.

Changing an energy-conversion parameter requires recalibrating the lifecycle and endurance budget. Do not treat the partial overhaul's changed constants as established tuning.

## 4. Perception, memory, and motivations

### Local observation

Agents observe nearby ground food, individuals, movement, carried supplies, and completed visible interactions. Coarse signs of exhaustion may be exposed as physical observations. They cannot inspect another individual's exact private energy, intentions, destination, relationship table, or map.

Use bounded attention with fair sampling. Known individuals still require a valid, local observation before their current position influences decisions. Remembered positions become stale estimates rather than remote tracking.

### Bounded memories

Start with four place entries and eight directed individual entries per agent. These are computational budgets, not prescribed group sizes. Test sensitivity to those budgets before interpreting group structure.

Place entries contain location, estimated food, observation age, evidence/confidence, and source identity when communicated. Observing depletion updates a remembered rich location. Unvisited estimates become less certain and can allow for possible regeneration rather than treating a formerly empty location as permanently worthless.

Individual entries contain identity, familiarity, expected benefit, expected harm, separate supporting evidence, and last observation. Peaceful exposure increases familiarity; actual consequences update benefit or harm. These remain separate inputs through decision scoring, so a useful but dangerous individual is distinguishable from an irrelevant one.

Use bounded prediction-error updates and age evidence over time. Do not implement expectations as permanent accumulating reward totals. Incidental encounters cannot evict every important relationship, but stale relationships cannot monopolize memory forever. Consequential encounters can acquire memory immediately. Direct experience outweighs unsupported reports.

A small temporary outcome record supports attribution: action, counterpart identity, time, and observed result. This is limited bookkeeping, not an unbounded autobiographical memory.

### Motivations

Use common preferences for survival, reserve preparation, safety, access to useful individuals, and modest concern for familiar individuals. Social concern seeds helping; expected future benefit can reinforce it. Both are explicit, independently adjustable terms.

Concern depends on observable need and is constrained by the giver's own reserve forecast. A recipient with little food is not automatically in the same condition as one visibly exhausted. No decision rule may assume exact access to another individual's survival clock.

Do not guarantee generosity or attachment. Preserving a relationship competes with other opportunities. Separation alone is not evidence of hostility. Failed assistance is not equivalent to deliberate harm.

## 5. Actions and decision model

The complete initial vocabulary is move, harvest, eat, give, communicate a food estimate, apply force, and wait. Reproduction is a resource-gated lifecycle process. Approaching, following, exploring, yielding, defending, and migration are purposes or observed sequences of these actions.

Use a bounded set of candidates drawn from visible resources, remembered places, visible individuals, and exploratory destinations. Candidate evaluation estimates near-term energy/food consequences over a consistent horizon, including travel time, expected intake, destination crowding, danger, and access to particular individuals. Expose utility components separately in the inspector.

Movement targets persist briefly. Reconsider on arrival, expiry, newly observed danger, critical hunger, loss of a social target, or a substantial change in opportunity. Commitment never prevents immediate eating needed for survival. Compare remaining at the current location with the actual projected consequences of moving; do not impose identical movement penalties and call them directional avoidance.

Social access has a preferred interaction range and diminishing returns. It does not increase indefinitely with proximity or crowd size. Account for observed resource competition and learned harm; avoid redundant attraction or alignment forces that create groups independently of experience.

Exploration introduces modest, seeded variation among feasible choices and sustains destinations long enough to make progress. It does not repeatedly select impossible actions or replace every intention with white noise.

### Giving and useful information

Giving transfers an affordable bounded quantity to a nearby individual with capacity. Only completed transfers create evidence of receiving help.

Communication spends a small action/energy cost and shares one known food-location estimate with a nearby individual at a bounded rate. Reports retain observation age and source; relaying a report does not make it fresh or independently confirmed. Recipients can act on reports but do not receive guaranteed accurate destinations.

When an agent follows an individual or acts on their information, retain enough temporary attribution to compare the observed result with the expectation. Update navigation/information reliability separately from generosity when choosing guides. Credit is limited to observed outcomes; do not assume that a successful arrival proves the guide caused it.

### Costly force

Force is local and contestable. Both participants pay energy; success can displace a target and spill a bounded amount of carried food into the collectible ground supply. Ordinary collection determines who obtains spilled food. No persistent injury state is needed for the initial release.

Generate force candidates for obtaining access to food and interrupting a visible harmful act against a valued individual. Use the same physical action for both. Observers react only to events they witnessed; they do not know the full history or automatically identify a morally correct side.

Expected retaliation must be grounded in observed responses, not hidden access to everyone's social network. Opposition to harm is a behavior to measure, not a promised emergent legal system.

## 6. Implementation order and acceptance gates

### Milestone 0 — Restore a trustworthy executable

- Complete the paused integration without overwriting unrelated existing work.
- Reconcile Rust/WGSL layouts, inspector fields, statistics, shared shader definitions, and build scripts.
- Disable unfinished social/force features until their milestone is ready.
- Establish headless GPU initialization and stepping, seed-based scenarios, and small state readbacks.
- Correct per-tick parameter advancement inside speed batches.

Gate: Rust compilation, all shader/pipeline validation, rendering initialization, reset, selection, and short GPU stepping pass. One-at-a-time and batched ticks advance clocks consistently. No invalid numeric state or generation/slot corruption.

### Milestone 1 — Ecology, bodies, and population cycles

- Correct fractional growth, recovery, resource accounting, death drops, maturity, birth cooldown, and reserve-funded offspring.
- Complete move/harvest/eat/wait and bounded food storage.
- Establish coherent units and calibrate endurance, travel, maturation, and reproduction together.
- Retain age and starvation mortality, births, weather, and fertility from the beginning.

Gate: abundance supports growth; controlled shortage causes understandable deaths; restored conditions allow surviving viable adults and subsequent generations to recover. Juveniles can survive and mature in adequate conditions. Low fractional growth accumulates. Extinction remains explicit. Accounting closes within documented numerical tolerances.

### Milestone 2 — Competent remembered foraging

- Add bounded place memory, stale-information handling, travel forecasts, exploration, and interruptible destination commitment.
- Use local candidate-specific competition estimates.

Gate: agents stop, stock food, eat before depletion, leave exhausted locations, return to remembered opportunities, revise depleted memories, and abandon infeasible journeys. Compare with a memory-disabled version under controlled resource shocks. No claim of social behavior at this gate.

### Milestone 3 — Relationships and material help

- Add fair local encounters, familiarity, learned benefit/harm, evidence-sensitive forgetting, and memory replacement.
- Enable giving, social concern, and movement that preserves access to observed useful individuals.
- Integrate newborn encounters and support through the same local mechanisms.

Gate: transfers conserve food under contention; completed help changes the recipient's expectations; incidental encounters do not erase important history; stale ties allow new relationships. Measure association and survival after depletion with normal, disabled, and shuffled relationship histories. Demonstrate both affordable giving and withholding under own-reserve pressure in controlled cases.

### Milestone 4 — Information and movement cascades

- Enable bounded local food reports, provenance/staleness, and outcome attribution for following and report use.
- Expose active destinations and sources in the inspector.

Gate: an agent can benefit from information it did not observe directly; stale reports can fail; failed information is revised; repeated relay cannot manufacture freshness/confidence. Compare continued association and travel outcomes after patch depletion. Investigate guide dependence and splits without assigning leader or group roles.

### Milestone 5 — Conflict within the same ecology

- Add force costs, displacement, spilled food, witnessed harm, and intervention candidates.
- Resolve contention without duplicate transfers, overlapping ownership of state writes, or arbitrary permanent priority.

Gate: force is costly and can fail; spilled food is accounted for; harm updates only relevant observed beliefs; an affordable intervention can occur in a constructed case. Peaceful escape, giving, and non-intervention remain available choices. Test abundance and famine with force enabled and disabled. Do not tune every famine into conflict.

### Milestone 6 — Complete the observable long-running world

- Add population/resource histories, relationship inspection, event trails, scenario save/checkpoint support, and comparison runs.
- Run multi-generation scenarios through repeated abundance and famine.
- Profile sparse and dense populations at 1,000, 10,000, and up to the 100,000-agent ceiling. Display device performance and capacity limits honestly.
- Document which social outcomes were observed, under what conditions, and which remain hypotheses.

Gate: the complete release supports inspection of a causal chain from environmental pressure through individual actions to a population outcome. At least some measured differences depend on actual relationship history, not solely common destinations, initial reserves, slot order, or the population ceiling. Low-population recovery, migration, deaths of connected individuals, and newcomer integration remain functional across repeated cycles.

## 7. Scenario suite and observer tools

Maintain a compact set of repeatable scenarios:

1. Abundance, reproduction, and maturation.
2. Local depletion with a reachable alternative patch.
3. Famine followed by restoration with surviving adults and juveniles.
4. Established relationships versus shuffled targets with identical physical state.
5. Unequal reserves with giving enabled/disabled and concern set to zero.
6. Accurate versus stale food reports.
7. A food contest with and without a nearby informed companion.
8. Death of a connected individual followed by population turnover.
9. Dense competition and simultaneous transfers/contests.
10. Complete extinction and explicit reseeding.

Use multiple fixed seeds for behavioral comparisons. Event-level numerical checks and aggregate statistical comparisons serve different purposes; GPU atomic ordering may prevent bit-identical trajectories. Do not assert behavioral success from a single attractive run.

Report population, births, juvenile survival, deaths by cause, vegetation food, dropped food, carried food, regeneration, intake, dissipation, transfers, force attempts, migration outcomes, and relevant relationship summaries. Selected-agent views show memory, evidence, candidate consequences, chosen target, and completed interactions. Observer logs do not grant additional knowledge to agents.

## 8. Long-term extension after the complete release

The next planned research stage is bounded behavioral adaptation: compare the fixed policy with agents learning a few context-sensitive action preferences from experienced outcomes, such as reserve thresholds for giving, reliance on social information, and persistence in exploration. Keep physical rules, observations, and actions unchanged so the comparison is interpretable.

Evaluate this extension across generations and environmental changes before introducing inherited behavioral variation. Reputation transmission, persistent injury, additional resources, constructed structures, and symbolic institutions are outside the committed first release. Add them only when the completed world exposes a concrete missing capability worth investigating.

## 9. Working rule

Build in the milestone order. Preserve the full ecological lifecycle in the main world; simplify it only in explicitly labeled tests. At each gate, repair concrete causal or implementation failures, then continue. Do not restart the architecture because a desired social label has not appeared.

The current next step is observation of the integrated release: distinguish useful migration and recovery from shared food attraction, pacing, and repeated boom-and-famine cycles. Add mechanisms only in response to a concrete missing capability; the remaining acceptance gates are still criteria to test, not completed claims.
