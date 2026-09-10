# Recovery audit and plan

Historical audit of the archived material-world overhaul. The active model has
since been recovered from the old game baseline; see [current recovery](recovery.md).

Audit date: 2026-09-10. Baseline: committed `921b297` (old model
`primitive-v41-juvenile-opening-ramp`). Current: uncommitted working tree,
`material-world-1`, including the subsequent rendering readability pass.

This is a code/document comparison, not a controlled behavioral comparison.
Historical documentation sometimes has stale model names; executable source is
the authority for mechanisms. Reasons below distinguish documented design choices
from assessment. The original request and rationale for every rewrite decision
are not available in this audit.

## Assessment

The overhaul replaced the simulation, controller, observer tooling, persistence
and viewer, rather than replacing ecology alone. It adds a substantially stronger
material model but does not preserve product or behavioral feature parity.
The old implementation remains in Git. Recovery can reuse it as a specification
and source reference; components cannot all be pasted into the new data layout.

Do not equate deleted line count with lost functionality: the new implementation
files are currently untracked, so ordinary `git diff --stat` excludes their content.

## Inventory

| Area | Old committed implementation | Current implementation | Assessment |
| --- | --- | --- | --- |
| Execution | GPU compute simulation | Deterministic CPU simulation; GPU display | Useful for reproducibility and accounting; high-population performance parity unproven |
| World scale | 512x512 resource cells; default square extent 2048 | Default 64x64 cells; square extent 256 | Eightfold reduction per axis, 64-fold in area/cell count; major change in screen detail and spatial ecology |
| Physical cell size | 4 units in default square world | 4 units | Not an eightfold increase in physical cell size; each cell occupies more of the fitted viewport |
| Geometry | Toroidal, configurable rectangular dimensions | Toroidal square, isotropic camera | Wrapping survives; rectangular world configurability lost |
| Habitat | Correlated, warped multiscale fields, ridges and barren shoulders | Persistent elevation, retention, permeability and roughness fields | Persistent geography survives in a different form; old vegetation topology does not |
| Initial vegetation | Habitat-shaped, with temporary opening cover | Identical 0.35 A + 0.35 B in every cell | Uniform establishment loses immediate patch identity |
| Environmental history | Terrain keyframes, weather, seasons, depletion and soil feedback | Persistent substrate, multiscale climate, hydrology, producers and recycling | Real replacement; old ecology was not merely static decoration |
| Production | Universal food, productivity normalization and capacity-driven regrowth | Two typed producers compete for water, mineral and space | Stronger resource differentiation and material constraints |
| Recovery | Regrowth and soil history | Propagule transport, decomposition and nutrient recycling | New recovery mechanisms retained; should be made legible |
| Physiology | Predominantly energy budget, fixed body/motion traits | Typed food, energy, assimilated material, body nutrient, damage; inherited diet, thermal tolerance, locomotion and repair | Substantial new physical tradeoffs |
| Mortality | Energy failure plus lifespan ceiling | Energy failure or terminal damage; paid repair | Deliberate removal of arbitrary age expiration |
| Juveniles | 1800-tick age development, growing capacities, opening gathering assistance | Nominal 1000 ticks plus paid body construction; fixed weak early gathering | Development retained and physically changed, not removed |
| Reproduction | Different-producer packet pair required; fixed fusion loss | A funded packet can hatch alone; identity-independent material merging with multiple source contributions | Deliberately broadens reproductive possibilities |
| Packet investment | Heritable packet size plus controller spending allowance | Controller allocates energy and nutrient | Different mechanism; old size trait not retained |
| Packet physics | Velocity, damping, wrapped motion and pushability | Position but no velocity; force contacts only organisms | Lost interaction capability; not required by material conservation |
| Force | Two-dimensional body-relative impulse with equal opposite recoil | Forward impulse to another organism, without matching actor recoil | Significant simplification, not equivalent retained force physics |
| Contacts | Nearest eligible contact with contention arbitration and disjoint pairs | Hash-prioritized nearby organism processing | Changes physical interaction scheduling and possible same-tick interactions |
| Signals | Local scalar emissions in regional aggregate sensing | Local scalar signals with distance weighting and decay | Retained but dynamics and aggregation changed; not proof of evolved communication |
| Environmental sensing | Area integration in 16 body-relative regions | One environmental sample per region at radii 4/12 | More resource channels, less spatial integration |
| Social sensing | Body/packet occupancy, relative motion, signal and proximity pressure within radius 24 | Organism density, body-frame velocity, signal within radius 16 | Packet occupancy and pressure lost; sampling geometry and scaling changed |
| Private inputs | Underfoot food, body-frame motion, displacement and physical deltas | Material, typed inventories, damage, development, scalar speed and angular speed, other state | Richer physiology but not a superset of old motion/sensory feedback |
| Brain | 16 potential gated recurrent units | 16 potential plain tanh recurrent units | Recurrence remains; learned retention gates removed |
| Plasticity | Per-unit signed rates, activity traces, separate retention traits, learned readout effects | Global signed rate and retention, fast sensory-input weights | Learning remains but mechanism is simpler and different |
| Mutation | Inherited mutation rate/step/topology controls; unit duplication/retirement | Fixed mutation probabilities/steps; expression-bit toggle | Evolutionary degrees of freedom removed |
| Recombination | Neural modules inherited together | Individual weight/trait choices weighted by contributed nutrient | Material weighting added; neural module preservation lost |
| Cross-world heredity | Blind reservoir of 4096 genomes | Blind reservoir of 256 genomes | Core mechanism retained with much smaller storage |
| Rendering | Green continuous food field, bright bodies, smooth juvenile sizing, packet size variation | New field lenses; recent pass restores green contrast/bright bodies; two-stage body sizing and fixed packet size remain | Partial visual recovery only |
| Lenses | Resource/density/energy/speed/age/intake/inventory/action/habitat views | Producers/water/mineral/detritus/temperature/roughness/diet | Ecological views gained, most behavioral views lost |
| Inspector | Senses, actions, intake/spending/transfer feedback, gates and memory | Physical summaries, age/generation/traits/unit count/parents | Much less ability to understand an individual |
| History and experiments | Population/performance history; family, journey, survivor, travel and transfer tools | Basic metrics, world outcomes, seed sweeps, life-stage funnel | Useful new reachability reports; substantial observer loss |
| Painting | Preview, radius/density controls, smooth falloff and spaced strokes | Fixed-radius/fixed-amount producer-A intervention | Basic capability retained, interaction polish lost |
| Playback | Keyboard controls, wider speed/FPS/budget settings and MAX | Pause/step, 1/4/16/64x, drag/zoom/reset | Reduced playback controls |
| Desktop integration | Wallpaper host, tray, startup and protected desktop input | Much of Windows platform implementation retained | Not a complete desktop-feature deletion |
| Saves | Snapshot library, paired receipt/checkpoint, retention/pruning and backup utility | Validated full-state JSON, temporary-file write, one-minute autosave, file picker | New exact continuation; old library/management features absent |
| Compatibility | Old model-specific saves | Fresh model-specific directory and format; rejects old state | Defensible for incompatible biology/brains; old data not migrated or deleted by this design |
| Verification | Extensive GPU, sensing, behavior, persistence and performance tests | Smaller suite centered on material conservation, deterministic replay and developmental reachability | Useful new tests do not establish restoration of old behavior or experience |

## What has evidence, and what does not

Current validation documentation reports a 100-seed, 4000-tick cohort. Only
24/100 worlds reached offspring maturation and 16/100 reached a birth with a
mature non-founder contributor. The controlled-policy test demonstrates that
provisioning, maturation and further reproduction are physically reachable.
Neither result establishes long-run self-sustaining evolution or evolved care.
These cohort numbers are recorded results, not rerun by this audit.

The latest local suite completed 32 tests with one explicitly interactive test
ignored. Fresh and tick-1000 framebuffer captures show readable agents and
depleted clearings, but the evolved field still exposes large, block-shaped
depletion footprints. Interpolation alone cannot supply missing spatial state.
The recent default-view interpolation also blends neighboring food values at
boundaries; it is a display estimate, not an exact per-pixel inventory map.

## Justification

Documented choices worth preserving: material conservation, separate energy and
nutrient budgets, typed producers, persistent substrate, physical repair and
development, identity-independent merging, no world-age assistance and no age
expiration ceiling. These serve a coherent physical-model objective.

CPU execution and a smaller baseline are engineering tradeoffs, not demonstrated
requirements of the ecology. Their benefits must be measured against performance,
spatial scale and experience.

There is no material-model requirement to remove packet pushing, full directional
forces, recurrent gates, rich observers, brush controls, save browsing, or readable
vegetation edges. The available documentation does not establish a separate
necessity for these losses. This is not evidence about the original author's
intent; it is a finding about what the current implementation requires.

## Recovery sequence

1. Preserve a reproducible baseline. Keep the old commit and existing saves;
   archive the complete new working tree, including untracked files. Record build,
   model, settings and captures for both. Create a feature-parity checklist.
   No broad checkout/reset of the mixed working tree.
2. Restore observing and visual readability first. Recover normal-view palette,
   smoothly paid developmental sizing, informative packet sizing, useful agent
   lenses, action/sense/intake inspector, population history and brush controls.
   Keep diagnostic overlays optional. Show actual resource loss and actual contact
   events; never invent feeding activity for appearances.
3. Design spatial scale explicitly. Benchmark several grid/world sizes with matched
   declared densities and budgets. Decide whether to enlarge the world or separate
   ecological scales. Do not simply change `side`: it currently changes extent,
   available material and organism density as well as display granularity.
   Establish physically justified heterogeneous initial vegetation from substrate
   and climate, or a declared ecological establishment period. Track material in
   biomass/mineral/detritus when changing initialization; version causal changes.
   Check patch edges, depletion, regrowth, wrap continuity and camera scales.
4. Recover generic interaction freedom. Reinstate packet velocity/pushing and
   two-dimensional paid impulses with appropriate recoil. Revisit packet occupancy,
   regional sensing, motion feedback and contact scheduling. Verify conservation,
   body-frame behavior, costs and checkpoint continuation.
5. Audit cognition separately. Compare gates, traces, per-unit plasticity, modular
   inheritance and inherited mutation controls against the simplified controller.
   Restore justified degrees of freedom one mechanism at a time. Rebaseline the
   full life-cycle funnel after each causal change; do not compare incompatible
   genomes as if they were interchangeable.
6. Recover durable play and scale. Restore save-library browsing, multiple recovery
   snapshots, backup tools, journey/family records and playback controls. Benchmark
   larger populations and long sessions; optimize demonstrated hotspots before
   deciding which computation should return to the GPU.

Completion means a coherent visible habitat, readable individual lives and feeding
consequences, restored generic interaction freedom, explainable ecology, robust
resumable sessions, and measured performance. Scientific correctness and gameplay
readability need separate acceptance checks. Passing one does not imply the other.

## Source map

Historical files at `921b297`: `src/model.rs`, `src/brain.rs`, `src/ui.rs`,
`src/ui_details.rs`, `shaders/render_world.wgsl`, `shaders/render_agents.wgsl`,
`shaders/interactions.wgsl`, `shaders/resource_update.wgsl`, `docs/agents.md`,
`docs/habitat-generation.md`, `docs/play.md`, `docs/observing.md`.

Current files: `src/ecology.rs`, `src/engine.rs`, `src/controller.rs`,
`src/desktop.rs`, `src/resource_view.rs`, `src/checkpoint.rs`, `src/cli.rs`,
`shaders/material_view.wgsl`, `docs/world.md`, `docs/validation.md`, `CHANGELOG.md`.
