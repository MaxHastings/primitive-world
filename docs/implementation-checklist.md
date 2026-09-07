# Acceptance checklist

This checklist covers the final accepted requirements, including corrections to
earlier plans. Checked means implemented and inspected in current source; it does
not mean observed adaptive success. Verification and delivery remain explicit.

- [x] 1. Natural **survival duration** selects founding populations. `src/evolution.rs` compares terminal occupied ticks; births/feeding never enter the comparator.
- [x] 2. Progress is cumulative: current founding genomes survive failed candidates and are mutated into later proposals.
- [x] 3. Simple current/candidate pair, same seed and reset body/environment conditions, immediate strict outliving acceptance, ties keep current population.
- [x] 4. Eight-individual lifetime selection and its shader/passes/state are removed. No individual endurance ranking constructs candidates.
- [x] 5. Inheritance is explicit: saved **founding groups** carry forward, with a sparse uniform sample of terminal descendants eligible only through a challenger that outlives its matched incumbent. `docs/evolution.md` explains the longevity-only comparison.
- [x] 6. Living worlds continue indefinitely; a living challenger is promoted immediately after outliving its incumbent, while pause, budget, save and tick-capacity stops cannot create an extinction score. Natural completion is idempotent; exact duration excludes empty trailing batch ticks.
- [x] 7. No learned selector, secondary scoring network, training-round engine, broad candidate pool or authored behavioral reward components. Only one population comparator exists.
- [x] 8. One continuous individual mutation law for inherited challenger founders and births, with uniform terminal-descendant anchors and a small direction-agnostic random-immigrant stream; no similarity clustering, novelty score or directional selector.
- [x] 9. Fixed eight-unit/1,188-parameter architecture. All weights/biases/gate parameters can be inherited and vary under the shared law. No within-life weight updates.
- [x] 10. Gated recurrent private memory remains; fresh founders and newborns reset memory/body state. `brain.rs`, `decide.wgsl`, `apply_births.wgsl`, `simulation.rs`.
- [x] 11. Near/far regional senses, nearest sector neighbors/targets, signals and physical actions remain connected. `perceive.wgsl`, `decide.wgsl`, `interactions.wgsl`.
- [x] 12. Feeding/digestion, movement, local ecology, physical costs and paid reproduction remain. No rescue behavior, balance retuning or founder/newborn endowment equalization.
- [x] 13. GPU scans, active-body dispatch, independent playback, parallel terrain generation, reset uploads and streamed birth inheritance preserved.
- [x] 14. No per-tick CPU population readbacks for outer selection. Two bounded founding snapshots, sparse proposals at transitions and full biological ticks at every speed.
- [x] 15. UI distinguishes ticks/s from FPS. `docs/performance.md` labels earlier 0.8.0 measurements as earlier evidence, not final-revision results.
- [x] 16. One current model/checkpoint/receipt format; no conversion or legacy execution. Obsolete CLI aliases, individual-selection machinery and optional capability-experiment runner removed. Current read-only observers remain.
- [x] 17. Current/candidate groups, paired baseline, progress RNG, complete physical/memory state, outcomes/history/provenance persist. Validation precedes GPU writes. User run/save data is untouched; saves append without deletion.
- [x] 18. New Game and default headless both use seed-specific founders and population evolution. `--single-world` explicitly selects bounded diagnostics; no obsolete opt-in evolution alias.
- [x] 19. Latest 64 completed worlds retain duration, births, highest ancestry generation, feeding, seed and population provenance. Individual identity counts are labelled as identities, not genome diversity.
- [x] 20. UI/help/docs explain current population, candidate, comparison and natural completion. Pairing provides matching conditions and never kills a living world on a timer.
- [x] 21. Delivered to the ordinary checkout and built both release targets there. `Play.cmd` invokes `Play.ps1`, which builds/launches `target/play/release/primitive_world.exe`. The exact play target and standard `target/release/primitive_world.exe` both built successfully; launcher syntax/path were inspected without execution.
- [x] 22. Historical reports were compared against current controlled runs; the Vulkan GPU test suite and headless evolution stress tests were executed after the user's correction.
- [x] 23. Final source review covered startup/configuration, proposal/comparison, GPU update order, completion, paused/budgeted continuation, save/load, current receipts, UI/CLI/docs and tools. Runtime outcomes are recorded below, including known remaining extinction-prone attractors.
- [x] 24. Matched pairs share an ecology orientation; successive comparisons rotate orientation, preserve uniform terminal-descendant anchors, and report local food-gradient alignment without feeding it back into behavior or selection.

## Verification record

- `cargo check --all-targets`: passed. Final `cargo clippy --all-targets -- -D warnings` also compiled all current targets successfully.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release --locked`, and `git diff --check`: passed in this worktree.
- All 25 current WGSL modules, including generated active-body variants, parsed and statically validated with the project Naga library. This performed no GPU execution.
- All six Python sources parsed with Python AST; no Python module or test was executed. Static Markdown link scan: 12 files, zero unresolved local links.
- Release executables built in the ordinary checkout at `target/release/primitive_world.exe` and `target/play/release/primitive_world.exe`; both are 0.9.0 builds from the delivered source.
- Current-revision Rust tests passed under Vulkan: 91 passed, 3 ignored. Python backup-helper tests passed: 9 passed.
- Controlled headless runs covered v20 seed 45 and v21 random-immigrant runs on seeds 42 and 45, including a million-tick seed-42 stress run. These validate reduced directional lock-in, not universal extinction resistance.
- Source delivery and both executable builds were verified before committing this change. Recovery copies were retained locally; generated binaries, reports and run/save data are excluded from source control.

## Ordinary-checkout delivery evidence

- Destination: the ordinary project checkout used by `Play.cmd`.
- Source and destination started at the same current commit with a clean destination; the failed app handoff left no stash. Source modifications were intact.
- A checked binary Git patch transferred tracked changes/deletions. Its post-transfer SHA-256 matched exactly. All four new source/documentation files matched SHA-256; all 17 obsolete tracked files were absent.
- No build caches, reports, save/run data or Git directories were copied. No user data or unknown stashes were deleted.
- Destination `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, standard locked release build, locked `--target-dir target/play` release build, and `git diff --check` passed.
- `Play.ps1` was parsed without execution. `Play.cmd` forwards arguments to it; its executable path resolves to the successfully built play target.
- Ordinary executable: `target/play/release/primitive_world.exe` in the ordinary checkout.
- Standard executable: `target/release/primitive_world.exe` in the ordinary checkout.
- No simulation, viewer session, tests or benchmark was executed for delivery. All runtime/performance limitations above remain.
