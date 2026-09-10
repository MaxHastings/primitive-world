# Depth-retention candidate checks

Application 0.9.2; model `primitive-v45-depth-retention`; checkpoint layout 58;
founder bank 21. Prior model files are preserved and rejected unchanged.

The new GPU regression compares 512 simultaneous births against a CPU reference
using mixed stored depths, including ties and replacement collisions. It verifies
whole genomes and traits, newly stored child depth, and no repeat admission.
The checkpoint/world-reset regression stores nonzero depth, saves and reloads it,
and verifies exact continuation with unchanged pool metadata and zero-depth
founder bodies. This metadata is not supplied to brains.

Strict Clippy, formatting and 12 Python tests passed. The full serial release GPU
suite result is recorded in [engineering evidence](evidence/depth-retention-engineering.json).
`Play.ps1 --version` built and ran 0.9.2. The launch supports explicit 32x selection;
the application logs its selected playback target for operational verification.

The owner authorized a fresh seed 3001 wallpaper experiment at 32x after commit and
push to main. Physical reproduction, development, ecology and mutation are unchanged.
Improvement in sustained generations under this selection rule is unverified.
No claim of long-term stability follows from the regression tests.
