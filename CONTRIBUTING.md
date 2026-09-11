# Contributing

Primitive World is a Rust/wgpu artificial-life simulator for Windows. Contributions
can improve the viewer, documentation, performance, diagnostics or simulation
integrity. Start with the [README](README.md) to run it and the
[design principles](docs/direction.md) to understand its biological choices.

## Find the relevant code

| Area | Entry points |
| --- | --- |
| Launch and viewer | `src/main.rs`, `src/ui.rs`, `src/controls.rs`, `src/renderer.rs` |
| Desktop integration | `src/windows_platform.rs`, `Build.ps1`, `Play.ps1` |
| Simulation and defaults | `src/simulation.rs`, `src/model.rs` |
| Agent and ecology kernels | `shaders/` |
| Heredity and continuity | `src/brain.rs`, `src/evolution.rs`, `src/founders.rs` |
| Persistence and playback | `src/session.rs`, `src/experiments.rs`, `src/playback.rs` |
| Headless reports | `src/headless.rs` and the observer modules |
| Offline analysis and backups | `tools/` |
| Public references | `docs/` |

`runs/`, `reports/` and `target/` hold ignored local outputs. Do not commit personal
checkpoints, genome dumps, backup archives or machine-specific paths.

## Build without interrupting an experiment

The manifest requires Rust 1.93 or newer; CI checks Rust 1.93.1. Python tool checks
use Python 3.11. The Windows build also needs the native linker/build tools used
by the Rust toolchain.

If the normal application is running, use a separate target directory. For
example, in a development PowerShell session:

```powershell
$env:CARGO_TARGET_DIR = 'target/contributor-checks'
cargo build --release --locked
```

Do not stop an experiment or run the packaging script just to replace its binary.
Use explicit copied saves and separate output paths for experimental continuations.
A second simulation can still contend for the same GPU even with separate files.

## Verification

Run the same source checks as [CI](.github/workflows/check.yml):

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked -- --skip simulation::tests::
python -m unittest discover -s tools -p "test_*.py"
```

To reproduce CI's compiler exactly, install Rust 1.93.1 and insert `+1.93.1` after
`cargo` in these commands. Use checks appropriate to the change: documentation
edits need link/command review and the documentation tests, not a GPU soak.

On a compatible GPU, simulation changes additionally require the full suite,
run serially:

```powershell
cargo test --release --locked -- --test-threads=1
```

CPU-only CI skips `simulation::tests`; it is not full GPU verification. Ignored
manual diagnostics are outside the default suite: select the relevant named
probe and supply its required inputs rather than running all ignored tests.
Physics and wiring tests establish integrity, not intelligence.

## Performance changes

Consult [existing measurements](docs/gpu-tick-performance.md) before repeating an
optimization prototype. Compare the same build settings, seeds or copied saves,
presentation settings and GPU workload. Report actual throughput, population,
packet activity and measurement duration. Distinguish headless throughput from
visible wallpaper performance, and note competing GPU work.

Preserve physical timing, learning, sensory coverage, costs and random behavior
when claiming an execution-only optimization. Test the changed state against its
reference; faster results from skipped biology are a different model comparison.

## Biology, saves and evidence

All execution modes share the same model. A new ability should expose a general
physical or cognitive mechanism rather than prescribe a social behavior. Keep
observers outside controller inputs and selection. The existing depth-biased
hereditary retention is an explicit selection rule; do not describe it as unbiased.

Changes to brains, physical costs, timing or selection need a clear description
of their biological consequences. Layout/model changes require deliberate format
validation and versioning. Never silently reinterpret a saved experiment. Existing
save-specific settings and provenance must survive compatible continuation.

## Submit a useful change

Describe the concrete problem, resulting behavior and relevant validation. Include
build/model version, seed or fixture, settings, hardware/OS and reproduction steps
for simulation bugs or performance claims. Explain unresolved limitations without
claiming a test or long run that was not performed.

Update the public page that owns the changed behavior: [agents](docs/agents.md),
[world](docs/world.md), [evolution](docs/evolution.md), [observation](docs/observing.md),
or [play](docs/play.md). Keep the README focused on getting started and explaining
the project. Use Primitive World as the project name. Put release history in the
changelog rather than leaving obsolete behavior mixed into current instructions.
