# Release status

Primitive World is an experimental artificial-life sandbox. Intelligent behavior
and general adaptation are unverified capabilities.

## Build and data formats

- Application version: 0.9.0.
- Model: `primitive-v10-live-winner-search`.
- Checkpoint format: 24, including founding populations, paired outcomes/history, search RNG, and candidate descendant carryover counts.
- Founder-bank format: 9; game receipts: 4.
- Genome allocation: 74.25 MiB for 16,384 bodies, plus world/render buffers.

These identities are separate from application version. Noncurrent files remain
untouched and are rejected; no compatibility execution or conversion is provided.
Regional sensing and gated memory are preserved. Dense coincident populations
still have quadratic neighbor work. See [performance](performance.md) for measurements.

The population-search integration is checked by formatting, compilation and static
review only. No new tests, simulation experiments, playback session or performance
benchmark was run after the user requested code-only verification.

## Before publishing a GitHub release

- Confirm the owner’s license choice and include that license in source/archives.
- Run the checks in [CONTRIBUTING.md](../CONTRIBUTING.md), including the full GPU
  suite and current-format checkpoint loading on a supported machine.
- Verify play, checkpoint loading, smooth extinction transitions, and autosaves
  on the exact release executable. Code tests do not replace a visual release check.
- Include an authentic screenshot or short recording from the release build,
  labeled with relevant settings; do not advertise unverified language/planning.
- Package only the executable, current docs, notices, and any explicitly curated
  optional assets. Never upload the local `reports/` or `runs/` directories wholesale.
- Review the complete repository and release assets for data intended to stay private.
- Publish binaries only for tested platforms. Windows is the locally exercised
  platform; other builds need their own evidence.

Public distribution and licensing require the owner’s explicit decision.
