# Long-horizon performance plan

Status: implementation plan only. No simulation changes or benchmark results are implied.

## Objective and scope

Reduce the cost of the existing experiment without narrowing its biological capabilities. Preserve the running experiment and its inherited history. Support reliable operation across at least 10,000,000,000 cumulative experiment ticks under the existing multi-world policy.

This plan supersedes the conversational proposal for a fixed eight-unit brain, no plasticity, reduced senses, and stationary food. Those changes are not implementation tasks.

The deliverable is a measured, validated optimization release, or an evidence report explaining why the tested candidates were rejected. No speedup or absence of future biological limits is guaranteed.

## 1. Non-negotiable preservation contract

Keep unchanged:

- 107 inputs, all 16 directional/radial samples, physical channels, normalizations, range, signal aggregation and timing.
- 1-16 expressed gated recurrent units, all latent inherited parameters, active masks, topology mutation, recurrent equations, traces, plasticity, retention, update cadence, reset behavior and cognitive costs.
- All 14 outputs, current categorical primary-action selection, continuous motor mappings, directional force, recoil and packet placement. Do not introduce simultaneous primary actions.
- All movement, gathering, digestion, transfer, signal, upkeep, maturation, storage, lifespan, packet-size, decay, fusion and energy-accounting rules.
- Both pre-movement and post-movement spatial snapshots and existing tick ordering.
- Full ecology: grid resolution, terrain, climate scales, temporal interpolation, water/mineral/detritus/fertility dynamics, dropped food and per-tick updates.
- Mutation, module inheritance, random draws, producer compatibility, contact arbitration, capacity admission and depth-biased hereditary-pool retention.
- Existing model identity, checkpoint compatibility, settings and provenance. Preserve historical settings carried by saves rather than silently applying fresh-world defaults.
- All viewer controls and inspection capabilities. Optional diagnostic work may be gated only when its absence cannot affect subsequent biology, rollover, saving or displayed requested data.

Use current checked-in implementation and regression fixtures as the reference. Documentation contains historical disagreements; record discrepancies rather than changing biology to match old prose.

Preserving today's capacity does not establish that 16 units, aggregate senses or current population limits are sufficient forever. Capacity expansion is outside this release and must be a separately versioned design decision, not an optimization shortcut.

## 2. Work isolation and baseline capture

Implementation starts on `codex/long-horizon-performance`, in an isolated worktree from the recorded main commit. Preserve any unrelated user changes. Use a separate Cargo target directory and explicit output directories under ignored `reports/long-horizon-performance/`.

Do not stop, restart, replace the binary of, inject input into, or overwrite saves from the user's active experiment. Do not run Build.cmd, installer/startup commands or default resume launchers as part of development.

Record baseline commit, model and format versions, adapter, driver, OS, resolution, presentation rate, settings, build commands and whether another simulation is running. Use only a completed checkpoint copied into the report directory; hash the source before/after copying, reject a changing source, and load the copy to verify integrity. Subsequent tests load only immutable copied fixtures with explicit paths, never a moving 'latest save'. If no valid save is available, proceed with synthetic fixtures and mark saved-world validation pending.

Baseline outputs: `baseline.json`, `preservation-matrix.md`, and a manifest mapping every protected subsystem to source files and existing tests. Keep personal checkpoints and dumps untracked.

## 3. Measurement harness and acceptance rules

Extend existing diagnostics in src/performance_tests.rs instead of building a second simulation harness. Existing starting points include profile_retained_inner_loops, profile_neural_input_layout_and_capacity, profile_tick_optimizations and profile_saved_experiment_streaming. Add explicit fixture-path selection before using a diagnostic that currently auto-selects a save.

Benchmark workloads:

1. Default fresh worlds, seeds 42 and 1337, initial body counts 32, 1,000, 4,096 and 8,192.
2. The immutable current-experiment checkpoint, when available.
3. Controlled dense-contact and packet-heavy fixtures, including storage-capacity pressure.
4. Controlled 1-, 8- and 16-expressed-unit fixtures with nonzero traces/learned deltas; include maximum plasticity/retention bounds and inactive latent genes.

Controlled fixtures are test inputs, never founders or online interventions in the real experiment.

For each candidate, restore the same fixture for every run; warm up for 32 ticks; measure 1,024 ticks in batches of 32. Run five paired repetitions, alternating reference/candidate order. Record per-pair synchronized ticks/s, elapsed time and population/packet counts. GPU pass timestamps diagnose costs but are not acceptance evidence on their own.

Run one GPU benchmark at a time. Do not launch a second wallpaper for benchmarking while the user's wallpaper is running. Shared-GPU measurements are preliminary and labeled as such. Clean viewer measurements require an already available idle window; do not interrupt the experiment to obtain one. Missing clean measurements remain an explicit release gate, not an invitation to change biological rules.

Candidate acceptance, after correctness passes:

- At least 5% median paired throughput improvement on the declared target workload, positive in at least four of five pairs.
- No greater than 3% median regression on any representative workload.
- If run-to-run noise can account for the result, repeat once with 4,096 measured ticks. If still inconclusive, reject the candidate.
- Record peak allocated GPU memory and any CPU, save-latency or responsiveness tradeoff. No out-of-memory or lost-capability regression is accepted.
- A memory-only candidate requires at least 10% lower allocated GPU memory, no greater than 3% throughput regression, and full compatibility checks.

Declare the target workload before measuring; do not select it afterward to rescue a result. Keep each independent optimization as a separate commit. Revert rejected prototype code; retain its concise evidence record. Do not add percentages from different benchmarks.

## 4. Correctness gate before performance gate

Keep a reference path accessible to tests for each changed algorithm or layout. Compare logical state after 1, 32 and 1,024 ticks, not just population totals. Cover organisms, packets, inherited genomes/traits, masks, recurrent state, traces, learned weights, reserves, food, terrain/ecology, RNG/provenance, hereditary pool, pending birth state and counters that affect progression.

Require byte equality in controlled deterministic fixtures and unchanged arithmetic/reduction order in affected neural and ecological computations. Reuse existing narrowly scoped tolerances only for already documented nondeterministic neighbor reductions; do not widen tolerances or accept changed births/contact choices to pass a candidate. For crowded fixtures, establish reference-versus-reference variability first. If a causal difference cannot be separated from that variability, the candidate is not eligible for release.

Include wrap boundaries, empty worlds, dense contacts, full storage, juvenile starvation/maturation, simultaneous birth/death, pushed packets, invalid-neural-state handling, climate/terrain epoch boundaries and newborn cognitive resets.

Observer-disabled/enabled and viewer/headless configurations must produce the same biological state under controlled fixtures. Save/load continuation must preserve all logical state and match uninterrupted continuation under the same determinism conditions.

No test asserts that communication, care or intelligence must evolve. Correctness tests establish mechanics and preservation, not ten-billion-tick behavioral equivalence.

## 5. Ordered implementation work packages

Execute A through F in order. Within each package, audit first, prototype only eligible candidates, validate, measure and retain/reject before moving on. Existing failed prototypes in docs/gpu-tick-performance.md are not repeated unless a new measurement identifies a materially different bottleneck; document that difference first.

### A. Remove unnecessary observation and host work

Inspect src/playback.rs, src/observability.rs, src/inspection.rs, src/renderer.rs and observer scheduling in src/simulation.rs.

Create a reader/writer list for every readback and observer pass. Gate only passes with no consumer in the current mode; preserve requested inspector data, required summaries, overflow checks and all counters used by world transitions. Remove duplicate readbacks and redundant copies only when the dependency map proves them redundant. Existing active observers continue at their current cadence.

Measure normal wallpaper behavior at unchanged presentation settings when an idle benchmark window is available. Separately document the existing optional --view-fps setting; do not silently lower the user's rendering rate. Output: dependency map, retained changes and paired results.

### B. Reduce neural and plasticity data traffic

Inspect shaders/decide_parallel.wgsl, shaders/plasticity.wgsl, shaders/common.wgsl, src/model.rs and buffer scheduling in src/simulation.rs.

List each Decision and Perception field's producers and consumers. Test reducing duplicate intermediate storage only where plasticity, cost accounting and inspection still receive exactly the original values. Investigate eliminating redundant loads/stores within the existing tick, preserving all connection updates and accumulation order.

Do not assume a zero plasticity coefficient makes learned state or trace work inert: retention, previous learned deltas, resets and energy charges can still matter. A skipped operation needs an exact precondition and a test covering nonzero prior state. Do not omit inactive hereditary genes, because future topology mutation can express them.

No topology reduction, precision reduction, learning-frequency reduction or added approximation. Output: field-consumer map and individually measured candidates.

### C. Optimize ecology execution without replacing ecology

Inspect shaders/resource_update.wgsl, src/climate.rs and src/environment.rs.

Identify uniform, per-region and per-cell expressions and their update dependencies. Test hoisting or reuse only when it returns the same values at every tick, including seed/orientation/epoch changes. Use existing hoisted-weather and climate-boundary tests as the reference.

Keep all cells and all updates. No lazy replacement for the nonlinear ecology, skipped growth ticks, lower-resolution grid, constant producers or changed weather interpolation. Static/epoch caches require measured benefit after accounting for refresh and memory costs; an earlier large cache regressed and is not a default task.

Output: dependency classification, exact-state comparisons and retained/rejected result.

### D. Optimize scheduling and storage access

Inspect src/simulation.rs, spatial kernels and action/birth scheduling. Use measurements from A-C to select at most two new scheduling/storage candidates per pass through this package.

Candidates must identify the exact dispatch, copy, barrier or access pattern being replaced, why its dependency remains satisfied, and why earlier rejected scheduling prototypes do not answer the same question. No global-workgroup synchronization assumption, contact-order change, missing inactive state or slot-identity change is acceptable.

Do not target a predetermined kernel count. Keep existing batch/latency policy unless a separately measured host-only change passes responsiveness and rollover tests. CPU world creation stays in place for this release. CUDA, fp16 and packed biological state are excluded.

### E. Audit the ten-billion-tick operational boundary

Inspect src/model.rs, src/playback.rs, src/experiments.rs, src/session.rs, checkpoint code and shader counters. Inventory widths and overflow handling for cumulative ticks, per-world ticks, world number, ancestry, identity, RNG inputs, signals, event sequences and resource/action accounting.

Current code uses MAX_WORLD_TICKS = u32::MAX - 1,000,001 and a u64 cumulative tick total. Accounting can trigger an existing world rollover. Preserve that policy and distinguish accounting rollover from natural extinction in all reports. This plan supports ten billion cumulative ticks across worlds, not a promise of a single ten-billion-tick world.

Add synthetic boundary fixtures immediately below/at/above supported rollover and accounting thresholds. Test batched stepping, checkpoint/resume and total ticks crossing 2^32 and 10^10 without actually waiting billions of ticks. Verify no double counting, lost hereditary state, incorrect extinction attribution or repeated rollover.

Fix display-only/reporting overflow defects within this package. If an existing defect requires changing causal RNG, identity, rollover or biological rules, record it as a separate model-change issue and block the affected compatibility claim; do not silently fold it into an optimization.

Output: long-horizon audit with field-by-field limits, boundary results and unresolved constraints, including the existing finite brain and population ceilings.

### F. Integrate and validate the retained changes

Compare the combined candidate with the original baseline, not just the previous candidate. Run required checks once after integration:

```text
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --release -- --test-threads=1
python -m unittest discover -s tools -p "test_*.py"
```

Use a separate CARGO_TARGET_DIR for all Cargo commands. Run diagnostics individually with --ignored --nocapture --test-threads=1; do not launch all ignored tests.

Repeat the acceptance benchmark matrix for the combined release. On an available idle GPU window, run three fresh seeds (42, 1337, 9001) for 100,000 ticks each and a copied saved-world continuation for 1,000,000 ticks. Restore/save/reload within these runs and test forced boundary fixtures separately. These are operational soaks, not proof of future behavior. Do not use differences in natural births as a performance-quality score.

Required viewer check: explicit copied save, unchanged resolution/presentation settings, isolated output paths, rendering and inspection working, bounded readback, save/resume working and no new UI stalls. When the active experiment prevents this check, mark it pending and deliver the reviewable build/evidence without installing it.

## 6. Completion, rejection and deployment

The implementation effort is complete when every work package has a result, accepted code passes checks, the combined performance result is recorded, and limits are documented. An empty set of accepted optimizations is a valid engineering outcome; do not shrink biology to manufacture a gain.

Release eligibility additionally requires the copied-current-save continuation, clean viewer validation and all required gates. Missing access or an idle benchmark window is reported as pending, never as passed.

Deliver:

- Focused source/test commits for accepted changes.
- Updated docs/performance.md and docs/gpu-tick-performance.md with measured results and reproduction steps.
- `reports/long-horizon-performance/` manifests, comparison results and long-horizon audit (personal data remains untracked).
- A separately built executable, rollback instructions and a compatibility statement.

Do not deploy over or restart the active experiment as part of this plan. Installation is a separate user-directed step after the result is concrete and reviewable. Preserve the old executable and completed checkpoint for rollback; do not overwrite either.

## 7. Explicitly deferred research

Smaller/larger brains, sparse or different plasticity, different sensory aggregation, identifiable neighbors, simultaneous primary actions, ecological replacement or lower update cadence, alternative selection, numeric approximation and CUDA are separate proposals. None is authorized by this optimization plan.

A future biological proposal must name the capability being changed, expected computational benefit, lost information or dynamics, compatibility consequences and comparisons needed. Short runs can establish reachability or expose failure; they cannot prove that long-term evolutionary potential is unchanged.
