# Primitive World

A GPU simulation of local survival, reproduction, remembered places, material exchange, local signals, and costly physical interaction. The active boundary is defined in [KERNEL_SPEC.md](KERNEL_SPEC.md), with the research direction in [DESIGN_PLAN.md](DESIGN_PLAN.md). Social labels are observer interpretations, not agent objectives or world laws.

## Run

```powershell
cargo run --release
```

Close an older running copy before rebuilding that executable on Windows. A fresh launch uses current defaults. Loading a checkpoint restores its saved settings, including regeneration.

Defaults: 1,000 initial agents, regeneration 0.01, metabolic cost 0.06, movement cost 0.01, and 8 energy per food unit. A harvest collects up to 0.025 food; an eating action consumes up to 0.1. Carrying capacity is 8 food. These are experiment settings, not guarantees of stability at every slider combination.

Reproduction requires maturity, energy, at least 2 carried food, and an elapsed cooldown. It is checked on a settled harvest/eat/wait tick rather than during movement, so arriving at a newly discovered patch does not create a synchronized birth pulse. At default cost, the parent spends 50 energy and transfers 1 food; the child receives 24 energy and that food. Survivors can repopulate after shortages. Complete extinction requires an explicit reset. The 100,000-agent capacity is a storage limit, not a population target.

## Controls and observation

- Pause: Space or the UI button. Step is available in the UI.
- Speed: 1, 2, 4, 8, 6 (16x), M (maximum), or UI buttons.
- Pan: WASD or arrow keys. Zoom: wheel. Reset camera: Home.
- Cycle visualization: L. Food and action views help distinguish reserves from movement.
- Select an agent to inspect its state, local observations, action scores, private place memory, raw nearby bodies, and lineage ancestry.
- Choose a resource or kill intervention, then click the world to apply it.
- Save/load checkpoints, export population history, capture an observer-only evolution snapshot, and refresh lineage-aware recent interactions through the UI.

The dashboard reports food per living agent, reserves, hunger, births, deaths, local signals, and lineage-aware observer data. None of these measurements are visible to the controller or used as a reproductive objective.

## Headless checks

```powershell
cargo test -- --nocapture --test-threads=1
cargo run --release -- --headless --ticks 16000 --seed 1 --regeneration 0.025 --sample 2000 --famine-at 6000 --restore-at 8000 --output reports/run.json
```

The output directory must exist. Headless mode runs GPU compute without opening a window. Reports include initial and final settings, intervention timing, reserve/energy summaries, physical interaction totals, and population history.

`--famine-at` removes vegetation and stops regeneration; carried and dropped food remain. `--restore-at` restores the configured regeneration rate, letting fertile regions grow food again; it does not refill the map. This is a severe controlled shock, separate from ordinary weather. `--no-signals` disables the local signal affordance; `--no-force` disables the force affordance separately. `--static-landscape` freezes geography.

The default controller is an inherited local controller. Founders use a viable ancestor; offspring inherit eight controller traits with small bounded mutations, including a copying-fidelity trait with a nonzero error floor. Reproduction is therefore the selection boundary for behavior, while private memories remain individual. The optional `--neural` flag loads the historical GRU experiment from `policies/forager-v3.json`; it is retained for comparison, not used by default. See [NEURAL_POLICY.md](NEURAL_POLICY.md) for the archived GRU contract.

## Model and limits

The world is a bounded 2048-unit square with a 512-squared food/fertility grid and a 256-squared spatial index. Seeded geography creates large and small rich hubs joined by lower-yield forage bands, with irregular edges and some barren gaps. Rain, drought, soil depletion from harvesting, and fractional regeneration affect food within those regions. The variation slider controls variation within regions; changing the seed and resetting changes their placement. Agents have four private place memories and observe a bounded set of raw nearby bodies. The action intent vocabulary is wait, move, collect, ingest, transfer, apply force, and emit; semantic labels belong to the observer, not the kernel. Collection and ingestion are separate actions; agents do not automatically feed while walking.

Each tick updates ecology and spatial indexing, local physical perception, agent-owned intents, collection, bodies and movement, disjoint pair interactions, signal events, death drops, and births. Atomic collection and disjoint interactions protect resource transfers from double spending. Birth allocation reuses dead slots with new generation identifiers and creates a new lineage identity linked to the parent. Offspring inherit adult capabilities and a boundedly mutated controller genome; private memories are cleared.

Nearby bodies expose position, velocity, carried matter, and recent local events. The active kernel does not identify helpers, enemies, leaders, groups, or trustworthy reports. Transfer and force spend or move matter through physical resolution; emit creates a bounded local event without copying a structured map. Any social interpretation belongs to the observer.

Tests cover physical transfers, reproduction and famine recovery, action selection, local sensing, signal locality, generic force, matter conservation, checkpoint replay, lineage capture, and batched clocks. Validation has used NVIDIA Vulkan. Atomic ordering means population-scale runs can diverge even with the same seed; headless results are not cross-GPU determinism claims.

Four compass samples bias food-directed travel toward horizontal and vertical paths. Clusters and trails alone do not demonstrate cooperation or social groups. This directional bias is a known substrate choice, not an emergence claim.

The landscape now combines broad fertile regions with several scales of smooth value noise. Peaks drift, change size and richness, and sometimes fade as others emerge. Geography blends between seeded keyframes every 8,192 simulation ticks; faster weather and local harvesting act on top. Potential productivity is normalized between keyframes, while actual growth still depends on soil, weather, and unused capacity. The **Landscape fertility** lens shows geography independently of current food. **Evolving food landscape** can freeze it for comparison.

Trips persist through small preference changes and eating/collection pauses. The observer records movement and physical encounters without treating clusters or correlated movement as proof of a social group. The optional `cargo test motion_diagnostic -- --ignored --nocapture` records individual movement rather than inferring competence from population totals.

Version-10 checkpoints save the controller genome, lineage identity, current neural weights, private memories, and decision traces. Checkpoints are intentionally single-schema: an incompatible file is rejected instead of being silently rewritten into a different world.
