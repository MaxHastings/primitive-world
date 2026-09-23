# Fast evolution: ranked design decisions

This is the decision record for a new `primitive-v46-fast-evolution` model. The
ranking is **expected evolutionary gain per engineering effort**, not the order
in which source files must be edited. The aim is more naturally closed
reproductive transitions per wall-clock hour in an interactive world, not a
large TPS number obtained from sparse or trivial organisms.

The measurements below are from v45 and identify expensive work. They do not
measure v46 speedups. A saved 582-organism, 53-packet world ran at 2,660
synchronized headless ticks/s and 2,353 wallpaper ticks/s. Its GPU batch took
about 308 µs/tick; independently sampled kernels took about 66 µs sensing, 53
µs fusion inheritance, 51 µs ecology, 23 µs plasticity, and 18 µs decisions.
Those kernel samples overlap other work and are **not additive savings**. See
[the reference profile](gpu-tick-performance.md#saved-world-layer-profile).

With `BIO_DT=3`, **5× effective biological throughput** requires about 1.67×
the old raw step rate at a comparable population; **10×** requires about 3.33×.
At the historical wallpaper rate of 2,353 ticks/s, those illustrative thresholds
are roughly 3,922 and 7,843 v46 macro-steps/s. Actual comparisons must use
matched workloads and include births, maturations, and successful descendant
reproduction; multiplying a declining-population microbenchmark by three is
not evidence of evolutionary improvement.

## Commit to these changes

| Rank | Change | Why it is high value | Cost or biological risk | Decision |
| ---: | --- | --- | --- | --- |
| 1 | `BIO_DT=3` with one held decision per macro-step | Directly triples biological time per step if raw step rate holds; maturity moves from ~1,800 to ~600 decisions without removing childhood | Poor scaling can distort energy, learning, or contacts; swept toroidal motion/contact is required | **Build** as the defining v46 change |
| 2 | 256×256 ecology, integrated every four macro-steps | Replaces 262,144 per-tick cells and a measured ~51 µs kernel with one-quarter as many cells at one-quarter the cadence; preserves a 2,048-ish world | Food cell size doubles; each cell represents four times the area, so capacity, stock, extraction, and growth need area-aware accounting | **Build** with food, capacity/fertility, and climate pressure |
| 3 | Fixed-cost local sensing from shared fields | Existing sensing is the largest sampled kernel (~66 µs), and cost currently rises with the number of nearby food cells and bodies | Fields blur individual events; the sixteen directional/radial samples must still distinguish food, bodies, approach, proximity, and signed signals | **Build** fields once, sample sixteen times per organism |
| 4 | Separate compact packets and dense live organisms | Avoid cognitive/body work and storage traffic for packets and thousands of empty slots; 500–1,000 organisms currently share 16,384 heavyweight slots | Largest implementation change; stable slot assumptions in inspection, birth, and save must become 64-bit identity references | **Build** despite cost because it enables durable scaling |
| 5 | Immutable genotype handles in packets and reservoir | Packet manufacture should not copy a 2,494-float genome; inheritance was ~53 µs in the saved-world kernel sample and can be worse in packet-heavy worlds | Pool ownership, reclamation, saves, and recombination must be correct after producer death | **Build**; materialize a new genotype only at a viable birth |
| 6 | Remove temporary perception/decision buffers where useful | Field sensing can feed the brain directly and avoid storing every input for a later pass | Over-fusing can increase register pressure and regress performance; retain only values needed for learning, physics, and inspection | **Build opportunistically**, with the new data layout |
| 7 | Simplify lifetime plasticity | Input plasticity is the largest learned matrix, but the sampled plasticity pass is only ~23 µs | Removing an entire learning route is a meaningful scientific change, and other redesigns already shrink the input vector | **Defer**; keep input/recurrent/gate/output learning in base v46 |
| 8 | Decouple rendering from simulation and make observation cheap | Wallpaper can render at normal display cadence without retaining every diagnostic record or synchronizing each macro-step | Inspection must remain responsive and show the selected stable identity | **Build** as part of wallpaper integration |
| 9 | Track reproductive transitions and opportunity metrics | Prevents optimization toward empty, disconnected, or merely prolific worlds | Some counters and periodic sampling have overhead; keep detailed readback outside hot passes | **Build** minimal persistent counters and sampled reports |

Ranks 2–6 are not independent multipliers. Ecology and sensing both touch food
storage; packet handles and dense packet storage interact. Measure the integrated
build against v45, not a sum of hypothetical per-layer gains. The source
implementation may do rank 4 before rank 3 or 5 to establish the new storage
contract. Keep full plasticity in base v46; the smaller input count reduces its
input matrix without removing that learning capability.

## Keep because they create evolutionary opportunity

These are design constraints rather than speed candidates:

| Capability | Decision and reason |
| --- | --- |
| 1–16 expressed recurrent units and latent inactive hereditary weights | **Keep.** The evolved snapshot happened to use mostly four units; that does not prove a four- or eight-unit ceiling can reach future strategies. |
| Local memory and unsupervised learning | **Keep.** Hidden state, input/recurrent/gate/output fast weights, traces, and inherited learning traits remain. No reward or success label is given to a brain. |
| Childhood near 1,800 biological units and lifespan near 9,000–11,000 | **Keep.** The step count changes, not the duration of development or the need to survive it. |
| Energy budget, foraging, digestion, depletion, climate, scarcity, and old age | **Keep.** Automatic abundance would make reproduction cheap and erase selection pressure. |
| Physical, paid, decaying packets and spatial fusion | **Keep.** A direct `mate()` action would remove an important open-ended coordination problem. |
| Signed local signals, anonymous transfer, and physical force | **Keep.** They permit cooperation, exploitation, and conventions without authored meanings or kin targeting. |
| Roughly 500–1,000 ordinary living organisms and original logical world scale | **Keep as calibration goals.** Interactions must remain common; no population clamp is added. |
| Modular recombination, mutation, topology change, and hereditary reservoir | **Keep the opportunities.** The reservoir can hold immutable genotype references. Its selection rule and provenance must be explicit. |

## Set aside for the first v46 release

| Idea | Why it is tempting | Why we are not choosing it now |
| --- | --- | --- |
| `BIO_DT=2` or 4 | Two may protect contacts; four may compress time further | Three is a useful first balance. Compare two/three/four only after the integrated model has a working life cycle. `BIO_DT=10` risks missing too many decisions and encounters. |
| Ecology cadence 2 or 8 | Could improve responsiveness or save more GPU work | Four is the starting point. Choose another cadence only if depletion/recovery or timing data justifies it. |
| Eight-unit maximum brain | Smaller neural buffers and less worst-case work | Long-term search headroom may be lost, while the evolved saved population already expressed mostly four units. Treat eight as a separate biological variant. |
| Recurrent/output-only, low-rank, or per-unit plasticity | Could shrink learned state, especially the input matrix | Learning is central to the experiment; the sampled 23 µs pass does not justify another mandatory biological change in the first cut. Revisit if cognition becomes the measured bottleneck. |
| Smaller physical world or 100-organism target | Higher encounter frequency or superficially faster steps | Changes spatial selection and may hide loss of social opportunity; benchmark separately, never use population collapse to claim speed. |
| Remove packets, childhood, metabolic pressure, or force births | Would raise births/second quickly | Makes successful reproduction easier by rule rather than increasing computational evolutionary search. |
| Directed food, mate, child, or signal semantics | Could improve apparent behavior immediately | Authors the strategy that evolution is supposed to discover. |
| Full v45 trajectory parity | Gives a strong exact-regression target | V46 deliberately changes biology; test capability, conservation, save safety, and a closed life cycle instead. |
| Reproduce every ecological intermediate or all 107 inputs | Avoids any information change | Retains large hidden costs that agents cannot directly exploit; keep useful gradients and senses instead. |
| Fuse every GPU kernel | Fewer dispatches | Register pressure and repeated work can outweigh dispatch savings. Fuse sensory sampling and cognition only where it reduces actual memory traffic. |
| Lower wallpaper FPS, disable drawing, or turn off inspection | Easy benchmark gain | Wallpaper use is a requirement. Rendering can be decoupled, with measured overhead reported. |
| CPU rewrite, CUDA-only engine, or half-precision everywhere | May offer specialized speed or simpler code | Replaces a functioning cross-platform GPU path or risks numerical/capability changes before the high-value model changes are exhausted. |

## Contracts that make the five bets clean

1. **BIO_DT is a units contract, not a speed label.** The simulation stores both
   macro-step count and biological time. Every continuous rate declares its
   units; retention/damping use `rate.powf(BIO_DT)`. Packet expiry, climate
   epochs, observation windows, and statistics use biological time where they
   represent biological duration. Swept contact tests use both endpoints and
   toroidal wrapping. A 3× larger represented time interval does not by itself
   guarantee 3× more successful evolutionary transitions.
2. **Ecology conserves world-scale opportunity.** Quartering the number of cells
   must not quarter total food mass or productivity. Each v46 cell represents
   four v45 cell areas. Give stock, carrying capacity, regrowth, extraction, and
   dropped-food accounting explicit units per area or per cell. Check total
   resource budget and local depletion/recovery, then tune population through
   ecology rather than a population controller.
3. **Fields are senses, contacts remain physical.** A field sample can blur
   individual organisms, but actual transfer, force, and packet fusion use a
   spatial contact structure with swept trajectories. Keep signed signals local
   and verify emission can be detected by another agent even after field
   aggregation. No field reveals identity or global position.
4. **Dense means live-scaled work.** Avoid a full heavyweight copy or sort of
   every entity at every step just to achieve contiguous indices. Compact at
   safe boundaries or use an equivalent dense active representation. Stable
   64-bit identity, not GPU index, connects inspection, lineage, events, and
   saves. The genotype pool retains every handle owned by organisms, packets,
   or reservoir records until a safe reclamation boundary.
5. **Closed depth is distinct from birth depth.** A newborn gets ancestry
   metadata, but a successful reproductive transition is counted only when a
   naturally born organism matured and contributed heredity to a viable later
   birth. Persist both transition count and maximum such depth across saves and
   world rollover, with provenance for any founder/reservoir restart.

## Minimum evidence for a usable build

This is a short mechanical check, not a months-long test program: release build,
fresh headless and wallpaper launch, v46 save/resume, v45-save rejection, stable
inspection, a controlled closed reproductive loop using only legal senses/actions,
and one sustained population/throughput sample. The user's subsequent wallpaper
run determines whether open evolution produces deep descendant chains. Report
that outcome as observed; do not infer it from packet or birth counts alone.

If the integrated build is slower than expected, inspect the new layer costs and
population before choosing another simplification. The next 2× should target
the measured bottleneck, not automatically lower neural capacity or organism
count.
