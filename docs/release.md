# Release status

Primitive World is an experimental artificial-life sandbox. Intelligent behavior
and general adaptation are unverified capabilities.

## Build and data formats

- Application version: 0.6.0.
- Model identifier in reports and exported banks: `primitive-v5`.
- Checkpoint format: 17.
- Founder-bank format: 6.
- GPU genome storage: about 166 MiB in one storage binding, plus world/render buffers.

Data-format numbers identify storage contracts, independently of the application
version. V5's sensory field, sector targets, and gated memory change the genome
and storage layouts, so V4 and earlier
checkpoints and founder banks are intentionally rejected. Unsupported formats or
models fail validation.

Start a fresh experiment for V5. Old experiment files remain untouched and can
be used with their matching older executable; there is no genome conversion.
Food integration and exact nearest-sector selection cost more than sparse probes.
Dense coincident populations have quadratic neighbor-scan work; performance
measurements must state population and spatial arrangement.

## Before publishing a GitHub release

- Confirm the owner’s license choice and include that license in source/archives.
- Run the checks in [CONTRIBUTING.md](../CONTRIBUTING.md), including the full GPU
  suite and checkpoint compatibility on a supported machine.
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
