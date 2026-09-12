# Profiling a saved experiment

Use a completed checkpoint and its matching JSON receipt, copied into an isolated
directory. Record source/build hashes, adapter, driver, workload and competing GPU
work. Run one simulation benchmark at a time in release mode. Stop an active
experiment only when its owner authorizes it; `--stop-wallpaper` uses the normal
save-before-close path and leaves the application open if saving fails.

The ignored diagnostics use the actual simulation tick encoder and explicit
fixture paths. They never choose the latest save or write into the source library.
Set a separate Cargo target directory and a new output filename for every probe:

```powershell
$env:CARGO_TARGET_DIR = 'target/review-build'
$env:PRIMITIVE_PROFILE_RECEIPT = 'C:/absolute/copied-fixture/save.json'
$env:PRIMITIVE_PROFILE_OUTPUT = 'C:/absolute/report/layers.json'
cargo test --release --locked profile_checkpoint_layers -- --ignored --nocapture --test-threads=1

$env:PRIMITIVE_PROFILE_OUTPUT = 'C:/absolute/report/selective.json'
cargo test --release --locked profile_checkpoint_selective -- --ignored --nocapture --test-threads=1

$env:PRIMITIVE_PROFILE_OUTPUT = 'C:/absolute/report/observers-v2.json'
cargo test --release --locked profile_checkpoint_observers -- --ignored --nocapture --test-threads=1
```

The layer diagnostic measures uninstrumented synchronized batches, batch GPU
timestamps, telemetry, one-millisecond polling, periodic metrics, detailed
observation and single-tick submissions. It also saves a separate checkpoint.
GPU timing probes require timestamp features, including timestamps inside compute
passes. Every timed dispatch occurrence receives its own query pair. Full tracing
can significantly perturb execution: use selective kernel captures and always
compare changes using normal production throughput. Kernel intervals from separate
captures are not additive exclusive time shares.

Observer diagnostics separately time metrics, evolution, hereditary-pool search,
sample serialization, checkpoint load/upload, durable experiment saves and each
checkpoint buffer readback. They write isolated save samples next to the report.
Queued uploads are explicitly submitted and completed before timing observers.

## Actual wallpaper measurements

The viewer accepts an opt-in `PRIMITIVE_PROFILE_VIEWER` environment variable naming
a new JSONL output. Set `PRIMITIVE_WORLD_SAVES` to a separate directory containing
a copied experiment directory, then launch the separately built executable:

```powershell
$env:PRIMITIVE_WORLD_SAVES = 'C:/absolute/report/viewer-run'
$env:PRIMITIVE_PROFILE_VIEWER = 'C:/absolute/report/viewer-run/viewer.jsonl'
$exe = 'C:/absolute/project/target/review-build/release/primitive_world.exe'
$process = Start-Process -FilePath $exe -ArgumentList '--wallpaper','--resume','--view-speed','MAX' -WindowStyle Hidden -PassThru
# Collect the intended measurement window, then use the normal close path.
& $exe --stop-wallpaper
$process.WaitForExit()
```

The log records completed ticks, elapsed seconds, FPS, population, dimensions,
batch GPU time, CPU encoding/submission, CPU rendering/presentation, surface
acquisition, and asynchronous world/UI render-pass GPU samples. Render samples
skip frames while a previous sample is pending. CPU rendering includes surface
acquisition, and batch wall time includes encoding and GPU completion; do not add
overlapping metrics. Missing GPU timestamp support is identified in each row.

Discard startup windows. Reload the same immutable fixture for every repetition;
do not continue one benchmark's evolved output into another. Keep resolution,
requested speed and refresh policy fixed unless deliberately testing presentation.
Remove profiling/save-root environment overrides before resuming the original.

Windows GUI-subsystem executables may return immediately when invoked with `&`.
For serial headless experiments use `Start-Process -PassThru`, then
`WaitForExit()` and check `ExitCode` before launching the next run. Measure both
reported simulation wall time and complete process duration.

Personal checkpoints and raw reports remain under ignored `reports/`. Publish
only aggregate measurements with their workload, methodology and limitations.
See the [saved-world measurements](gpu-tick-performance.md#saved-world-layer-profile)
for an example, including a prototype rejected after throughput comparisons.
