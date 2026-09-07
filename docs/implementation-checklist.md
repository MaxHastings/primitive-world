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
- [x] 8. Simple mutation and periodic random exploration; no similarity clustering, novelty score or extra diversity-management system.
- [x] 9. Fixed eight-unit/1,188-parameter architecture. World settings control mutation; all weights/biases/gate parameters can be inherited. No within-life weight updates.
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
- [x] 22. No historical-version investigation; no tests, benchmarks or simulation experiments after the user's correction. The earlier pilot had already finished. Only edits, compilation, formatting and static inspection are used for this implementation.
- [x] 23. Final source review covered startup/configuration, proposal/comparison, GPU update order, completion, paused/budgeted continuation, save/load, current receipts, UI/CLI/docs and tools. Static checks below passed. Runtime outcomes remain explicitly unverified.

## Verification record

- `cargo check --all-targets`: passed. Final `cargo clippy --all-targets -- -D warnings` also compiled all current targets successfully.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release --locked`, and `git diff --check`: passed in this worktree.
- All 25 current WGSL modules, including generated active-body variants, parsed and statically validated with the project Naga library. This performed no GPU execution.
- All six Python sources parsed with Python AST; no Python module or test was executed. Static Markdown link scan: 12 files, zero unresolved local links.
- Release executables built in the ordinary checkout at `target/release/primitive_world.exe` and `target/play/release/primitive_world.exe`; both are 0.9.0 builds from the delivered source.
- No current-revision Rust/Python tests, GPU execution, simulations, benchmarks or manual viewer session performed. Earlier test totals do not validate this implementation.
- Actual longer-lived worlds, final performance and runtime save/resume behavior remain unverified under the no-tests/no-experiments constraint.
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
