# Release status

Primitive World is an experimental artificial-life sandbox. Intelligent behavior
and general adaptation are unverified capabilities.

## Build and data formats

- Application version: 0.7.1.
- Candidate archive: `life-reservoir-v3`; selector weights format 4; training format 2; game receipts format 2.
- Model identifier in reports and exported banks: `primitive-v6-variable-brain`.
- Checkpoint format: 20 (factual lifetime records and explicit founder slots); older formats need their matching executable.
- Founder-bank format: 7.
- GPU genome storage: about 105.4 MiB in one storage binding, plus world/render buffers.

Data-format numbers identify storage contracts, independently of the application
version. V6's sparse graph, larger state capacity, world mutation law, and structural
costs change the genome and storage contracts. V5 and earlier banks/checkpoints
are rejected explicitly; they are not converted.

Start a fresh experiment for V6. Old experiment files remain untouched and can
be used with their matching older executable; there is no genome conversion.
The prior V5 source is preserved at the `primitive-v5-pre-cutover` branch.
Food integration and exact nearest-sector selection retain their existing costs.
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
