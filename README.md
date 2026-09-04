# Primitive World

A GPU simulation of local survival, reproduction, remembered places, giving, food reports, and costly force. The long-term direction is in [DESIGN_PLAN.md](DESIGN_PLAN.md). Social groups are an outcome to investigate, not a demonstrated claim.

## Run

```powershell
cargo run --release
```

Close an older running copy before rebuilding that executable on Windows. A fresh launch uses current defaults. Loading a checkpoint restores its saved settings, including regeneration.

Defaults: 1,000 initial agents, regeneration 0.01, metabolic cost 0.06, movement cost 0.01, and 8 energy per food unit. A harvest collects up to 0.025 food; an eating action consumes up to 0.1. Carrying capacity is 8 food. These are experiment settings, not guarantees of stability at every slider combination.

Reproduction requires maturity, energy, at least 2 carried food, and an elapsed cooldown. At default cost, the parent spends 50 energy and transfers 1 food; the child receives 24 energy and that food. Survivors can repopulate after shortages. Complete extinction requires an explicit reset. The 100,000-agent capacity is a storage limit, not a population target.

## Controls and observation

- Pause: Space or the UI button. Step is available in the UI.
- Speed: 1, 2, 4, 8, 6 (16x), M (maximum), or UI buttons.
- Pan: WASD or arrow keys. Zoom: wheel. Reset camera: Home.
- Cycle visualization: L. Food and action views help distinguish reserves from movement.
- Select an agent to inspect its state, local observations, action scores, place memory, and eight directed relationships.
- Choose a resource or kill intervention, then click the world to apply it.
- Save/load checkpoints, export population history, and refresh recent interactions through the UI.

The dashboard reports food per living agent, the number carrying at least 1.5 food, and the number with energy below 20. Useful ties include learned food benefit and successful guidance. Nearby means within sensory range, not necessarily within exchange distance. Birth, death, gift, report, and force counters are cumulative; population history is needed to interpret recent changes.

## Headless checks

```powershell
cargo test -- --nocapture --test-threads=1
cargo run --release -- --headless --ticks 16000 --seed 1 --regeneration 0.025 --sample 2000 --famine-at 6000 --restore-at 8000 --output reports/run.json
```

The output directory must exist. Headless mode runs GPU compute without opening a window. Reports include initial and final settings, intervention timing, reserve/energy summaries, harvest totals, relationships, and population history.

`--famine-at` removes vegetation and stops regeneration; carried and dropped food remain. `--restore-at` restores the configured regeneration rate, letting fertile regions grow food again; it does not refill the map. This is a severe controlled shock, separate from ordinary weather. `--no-social` disables concern, reciprocity, social steering, and reports; force remains enabled. `--no-force` disables force separately. `--static-landscape` freezes geography. `--shuffle-at` reassigns remembered identities among survivors as a disruptive comparison, not a matched local control.

## Model and limits

The world is a bounded 2048-unit square with a 512-squared food/fertility grid and a 256-squared spatial index. Seeded geography creates large and small rich hubs joined by lower-yield forage bands, with irregular edges and some barren gaps. Rain, drought, soil depletion from harvesting, and fractional regeneration affect food within those regions. The variation slider controls variation within regions; changing the seed and resetting changes their placement. Agents have four place memories and eight generation-aware relationship entries. Actions are wait, move, harvest, eat, give, force, and communicate. Harvesting and eating are separate actions; agents do not automatically feed while walking.

Each tick updates ecology and spatial indexing, perceptions and relationship evidence, decisions, collection, bodies and movement, disjoint pair interactions, death drops, and births. Atomic collection and disjoint interactions protect resource transfers from double spending. Birth allocation reuses dead slots with new generation identifiers. Offspring have the same behavioral weights and adult capabilities; there is currently no inherited policy evolution.

Known helpers affect movement only while locally visible. Agents compare destinations partly by whether they preserve access to someone whose help or guidance has proved useful. There is no generic flocking force or assigned leader. Reports retain their observation time and source, and senders remember recent deliveries. Guidance earns credit on encountering usable food near the reported destination. Force spends energy and can displace a victim and spill food, which must be collected afterward. Learned benefit, harm, and information reliability remain separate records.

Tests cover physical transfers, reproduction and famine recovery, action selection, a voluntarily selected gift, learned outcomes, local-only social perception, reports, force, contested gifts, checkpoint replay, and batched clocks. Validation has used NVIDIA Vulkan. Atomic ordering means population-scale runs can diverge even with the same seed; headless results are not cross-GPU determinism claims.

Four compass samples bias food-directed travel toward horizontal and vertical paths. Clusters and trails alone do not demonstrate cooperation or social groups. See [calibration results](reports/CALIBRATION.md) for evidence and remaining gaps.

The landscape now combines broad fertile regions with several scales of smooth value noise. Peaks drift, change size and richness, and sometimes fade as others emerge. Geography blends between seeded keyframes every 8,192 simulation ticks; faster weather and local harvesting act on top. Potential productivity is normalized between keyframes, while actual growth still depends on soil, weather, and unused capacity. The **Landscape fertility** lens shows geography independently of current food. **Evolving food landscape** can freeze it for comparison.

Trips persist through small preference changes and eating/harvesting pauses. A controlled two-departure scenario tests whether reports and learned companion access affect arrival and survival; see [integration results](reports/INTEGRATION.md). This demonstrates a mechanism, not a guarantee of large herds in ordinary runs. The optional `cargo test motion_diagnostic -- --ignored --nocapture` records individual movement rather than inferring competence from population totals.

Version-6 checkpoints save guidance attribution and delivery history. The loader accepts versions 3–5, initializing missing fields and leaving landscape evolution off for old saves. Reset for the current geography and defaults.
