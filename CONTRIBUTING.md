# Contributing

Keep the rules simple and the evidence honest. All modes share one agent model.
Adding an ability should expose a primitive interaction, not
prescribe a desired social behavior. Keep observers out of agent inputs and
selection. Extinction and unused abilities are allowed outcomes.

## Layout

- `src/`: Rust application, simulation scheduling, persistence, and observers.
- `shaders/`: GPU sensing, neural decisions, ecology, actions, and rendering.
- `docs/`: current play instructions and model contracts.
- `tools/`: optional read-only diagnostics and local backup utilities.
- `runs/`, `reports/`, `target/`: ignored local data/builds, never release assets.

## Checks

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test -- --skip simulation::tests::
python -m unittest discover -s tools -p "test_*.py"
```

On a machine with a compatible GPU, run the full simulation suite serially:

```sh
cargo test --release -- --test-threads=1
```

The ignored manual diagnostics require explicit inputs and are outside the default
suite. CPU-only CI skips
the `simulation::tests` module; that must not be reported as full GPU verification.
Physics and wiring tests establish integrity, not intelligence.

Use a separate target directory when an executable from the usual build folder
is already running. Never stop someone’s experiment just to replace its binary.

## Changes and reports

Include the version, seed, relevant settings, hardware/OS, and reproduction steps
with a bug report. Do not commit personal checkpoints, weight dumps, runtime
archives, or machine-specific paths. Large evidence belongs in a separately
curated artifact, not the source tree.

Write documentation, UI text, and code comments around current behavior and
contracts. Use Primitive World as the project name and describe components by
their purpose. Keep development chronology out of usage instructions and code
comments; record release changes in the changelog.

Keep model/checkpoint identity explicit. Layout changes require format validation
and an intentional version change; do not silently reinterpret saved data. Changes
to brains, physical costs, or selection protocols should be documented separately
from UI, build, and organization changes.
