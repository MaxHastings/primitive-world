# Release status

Primitive World is an experimental artificial-life sandbox. Intelligent behavior
and general adaptation are unverified capabilities.

## Build and data formats

- Application version: 0.4.0.
- Model identifier in reports and exported banks: `primitive-world`.
- Checkpoint format: 15.
- Founder-bank format: 4.

Data-format numbers identify storage contracts, independently of the application
version. Compatible saves retain their settings and genomes. The bank loader
accepts the equivalent stored model identifier for existing format-4 banks;
exports use `primitive-world`. Unsupported formats or models fail validation.

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
