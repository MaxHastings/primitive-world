# Candidate engineering checks

Version 0.9.1; model `primitive-v44-open-investment`; checkpoint 58; founder bank 21.

The release decision and scientific unknowns are in [the north star](north-star.md).
Historical evidence files retain their original model identities and scope.

- Full serial release GPU regression suite: 163 passed, 0 failed, 25 intentionally ignored.
- Strict Clippy, formatting and 12 Python tool tests checked.
- Paid newborn surplus has exact checkpoint/save-load and continuation coverage.
- Regression coverage includes natural world transitions, bounded save retention,
  ecology continuity, resource accounting and observer neutrality.
- `Play.ps1 --version` builds and runs the local 0.9.1 executable.
- Wallpaper attached at 3440x1440. Save-and-stop, resume and another save-and-stop
  preserved the same experiment and advanced its tick from 2,604 to 3,073.
- The regular viewer loaded that wallpaper save, rendered the world and paused
  through its UI. It was closed after the smoke check. A subsequent label-only
  rebuild replaces the misleading phrase "Permanent juvenile dependence".
- Wallpaper framebuffer inspection remains unverified: the computer-use API did
  not expose the embedded desktop surface. Multi-day operation is unverified.

No 20–50 million tick experiment was run for this handoff. The owner will launch
that experiment. Unassisted matured-descendant reproduction remains unproven;
reservoir-born adult founders do not establish it.

See [candidate engineering evidence](evidence/open-investment-engineering.json).
