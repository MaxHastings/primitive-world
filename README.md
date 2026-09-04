# Primitive World

Primitive World is a GPU-first artificial-world experiment. The v0 substrate contains only continuous 2D space, a locally varying regenerating resource field, finite-energy agents, local perception, movement, remaining, consumption, and consequences. It does not encode social institutions or interpret emergent patterns.

## Run

```text
cargo run --release
```

The default initial population is 50,000 agents, with a fixed GPU capacity of 100,000. Reproduction can fill the remaining slots. The simulation state is kept in GPU storage buffers; the CPU dispatches work and drives the window/UI. A debug build is useful for iteration, while `--release` is the intended performance run.

## Controls

- Pause / Resume: `Space` or the UI button
- Speed: `1`, `2`, `4`, `8`, `6` (16x), `M` (maximum), or UI buttons
- One tick: UI `Step`
- Pan: `WASD` or arrow keys
- Zoom: mouse wheel
- Reset camera: `Home`
- Cycle raw visualization lens: `L`
- Click in `select` mode to inspect the nearest agent
- Select `+ resource`, `- resource`, or `kill agents`, then click the world to apply a local intervention

The inspector exposes only primitive state, local perception, candidate scores, and the selected action. Direction scores compare sampled resource changes against the resource underfoot, so a nearby local gradient can affect movement without giving agents a global target or path.

## GPU architecture

Agent state is a tightly packed `AgentGpu` structure in two ping-pong storage buffers. Perception, decisions, and resource requests are separate data-oriented buffers. The resource field is a 512² fixed-point grid over a 2048-unit continuous world; this tile resolution keeps the first interactive experiment practical while preserving local spatial heterogeneity. A matching persistent fertility layer gives each cell ecological memory. Rain blooms and drought zones drift between seeded event centers, while harvesting slowly depletes soil and recovery restores it. The occupancy grid is 256². Each tick also builds a reusable GPU spatial index: occupancy counts are copied into an inclusive 16-pass binary prefix scan, per-cell atomic cursors are initialized from the resulting offsets, and living agent indices are scattered into contiguous cell ranges. For cell `c`, the list is `agent_indices[start..end]`, where `start` is zero for the first cell or `cell_offsets[c - 1]`, and `end` is `cell_offsets[c]`. Current behavior uses the occupancy counts for local crowd pressure; future exact-neighbor experiments can consume the index without changing the agent state contract.

Each simulation tick is dispatched as:

1. free-slot flagging, prefix scan, and compaction
2. resource regeneration
3. occupancy clear
4. occupancy count
5. spatial prefix scan
6. spatial cursor initialization
7. spatial agent scatter
8. local perception
9. primitive action scoring
10. fixed-point atomic resource consumption
11. ping-pong agent update and birth-candidate flagging
12. birth-candidate prefix scan, compaction, and slot assignment
13. GPU living-agent count

The resource consumer uses bounded atomic compare-and-exchange so competing agents cannot spend the same fixed-point resource units. Every living agent consumes from the resource cell under its current position, including while moving; movement still costs energy. Agents write only their own destination slot. Rendering reads the current GPU agent buffer and the render copy of the resource field directly; there is no full agent-array readback per frame.

The decision stage uses local occupancy as a crowd-pressure signal, giving risk-sensitive agents a stronger density penalty. Agents become eligible to reproduce after the maturity age when they have enough energy. A small deterministic per-agent chance gates each birth; the parent pays a reproduction cost, and the offspring inherits mutated movement, sensing, attraction, persistence, and risk traits. A GPU prefix-scan allocator pairs birth candidates with arbitrary dead slots, so births can grow the population until the fixed capacity is reached. Juveniles move more slowly and agents die from energy loss or at the maximum age.

Selection is intentionally exceptional: a click runs a GPU nearest-agent reduction, resolves one record, and maps only that small record to the CPU for the inspector. Living-agent count is sampled through a 4-byte readback roughly every 30 rendered frames.

## Reproducibility and limits

Initialization is deterministic for a seed, and each agent has an independent integer PRNG state. Runs should reproduce closely on the same backend/hardware configuration. Exact bit identity is not promised across GPU vendors/backends because floating-point execution and atomic ordering can differ.

The current instrumentation reports render FPS, simulation ticks per second, living agents, cumulative food units consumed, starvation deaths, age deaths, CPU submission time, and—when the adapter exposes timestamp queries—separate GPU simulation and world-render timings. On adapters without timestamp support, the UI explicitly reports that limitation instead of presenting CPU submission time as device execution time.

The main v0 scaling risks are resource-grid work per tick, the fixed-cost spatial scan, contention in dense resource cells, point/quad overdraw, and intentional stalls when selecting or sampling the living count. These boundaries are isolated in `src/simulation.rs` and the WGSL pass files.
