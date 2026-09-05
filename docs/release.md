# Release status

V3 is the permanent active model. Version 0.4.0 is an experimental artificial-life
sandbox; it is not a certification of intelligent behavior or general adaptation.
Checkpoint format 15 and founder-bank format 4 remain unchanged by this cleanup.

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
- Review repository history before first public publication. Removing historical
  research data from the current tree does not remove it from old commits.
- Publish binaries only for tested platforms. Windows is the locally exercised
  platform; other builds need their own evidence.

The repository stays private until the owner explicitly chooses publication.
Updating private `main` is not a public release. Old saves and active local
research checkouts remain intact, and repository history is not rewritten.
