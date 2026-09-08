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
- [x] 9. Fresh 16-unit masked architecture: every organism inherits a 1–16 active-unit topology, a 2,612-value controller, and bounded local-plasticity traits. No fixed eight-unit baseline or legacy loading path remains.
- [x] 10. Gated recurrent private memory and bounded local plasticity remain; founders and newborns reset recurrent, trace, and learned-weight state. `brain.rs`, `decide.wgsl`, `inherit_genomes.wgsl`, `plasticity.wgsl`, `simulation.rs`.
- [x] 11. Near/far regional senses, nearest sector neighbors/targets, signals and physical actions remain connected. `perceive.wgsl`, `decide.wgsl`, `interactions.wgsl`.
- [x] 12. Feeding/digestion, movement, local ecology, physical costs and paid reproduction remain. No rescue behavior, balance retuning or founder/newborn endowment equalization.
- [x] 13. GPU scans, active-body dispatch, independent playback, parallel terrain generation, reset uploads and streamed birth inheritance preserved.
- [x] 14. No per-tick CPU population readbacks for outer selection. Two bounded founding snapshots, sparse proposals at transitions and full biological ticks at every speed.
- [x] 15. UI distinguishes ticks/s from FPS. `docs/performance.md` records current-model throughput, measurement conditions, and population limits.
- [x] 16. One current model/checkpoint/receipt format; no conversion or legacy execution. Obsolete CLI aliases, individual-selection machinery and optional capability-experiment runner removed. Current read-only observers remain.
- [x] 17. Current/candidate groups, paired baseline, progress RNG, complete physical/memory state, outcomes/history/provenance persist. Validation precedes GPU writes. Saves append complete snapshots; ordinary retention preserves each experiment's newest valid snapshot.
- [x] 18. New Game and default headless both use seed-specific founders and population evolution. `--single-world` explicitly selects bounded diagnostics; no obsolete opt-in evolution alias.
- [x] 19. Latest 64 completed worlds retain duration, births, highest ancestry generation, feeding, seed and population provenance. Individual identity counts are labelled as identities, not genome diversity.
- [x] 20. UI/help/docs explain current population, candidate, comparison and natural completion. Pairing provides matching conditions and never kills a living world on a timer.
- [x] 21. Source changes are in the ordinary checkout used by `Play.cmd`; the launcher builds its separate play target.
- [x] 22. Run the release GPU suite and the ignored per-pass throughput diagnostic on the current model; distinguish implementation integrity from ecological success.
- [x] 23. Review startup, GPU ordering, inheritance, promotion, persistence, observers, telemetry and current docs; record known platform and measurement limits below.
- [x] 24. Matched pairs share an ecology orientation; successive comparisons rotate orientation, preserve uniform terminal-descendant anchors, and report local food-gradient alignment without feeding it back into behavior or selection.

## Current verification record (September 8, 2026)

- The release GPU suite passed after the inheritance shader cleanup: 102 tests, 6 ignored manual diagnostics. This includes cooperative-versus-serial decisions with masked controllers and learned weights.
- CPU/GPU inheritance parity covers 512 seeds across capacities and both topology changes. Separate GPU tests verify paid learning, first-tick founder learning, fault cleanup, newborn state reset, and checkpoint replay with nonzero learned weights.
- Python helper tests: 12 passed. Formatting, strict Clippy, and release compilation passed.
- The full per-pass profiler ran on the RTX 4070 SUPER. The 16-unit cooperative implementation measured 1,194–1,250 ticks/s at 1,000 starting bodies; see [performance](performance.md) for conditions and larger-population limits.
- Release-only startup wiring, telemetry/timestamp separation, complete trait inheritance, and sparse descendant readback were reviewed and fixed.
- The normal launcher uses `target/play/release/primitive_world.exe`. The startup
  entry must point to that executable; obsolete target directories are not a
  supported launch path.
- Current model: `primitive-v26-masked-plastic-16`; checkpoint 39; founder bank 15; receipt 4. Old files remain untouched and are rejected.
- These tests establish wiring, replay and measured throughput. They do not establish intelligence or improved ecological survival. Windows wallpaper and multi-monitor behavior still need their own visual checks before public distribution.
