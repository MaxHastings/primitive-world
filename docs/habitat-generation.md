Current timing: the recovery model uses one-million-tick terrain keyframes.
The generator and spatial-quality findings below describe the preserved baseline.

# Correlated habitat generation

Only persistent geography generation changes. Biology, sensing, actions, costs,
gathering, soil, weather, depletion, seasons, and resource-update formulas are
unchanged. No model, checkpoint, founder-bank, or experiment version is changed.
Existing saved experiments remain compatible. Loading retains saved bodies,
food, and soil; with evolving landscape enabled, the existing terrain refresh
rebuilds geography using the new generator on the next simulation step. With
landscape evolution disabled, saved geography remains in place. This is a change
to future environmental history, not a reset or a migration of saved experiments.

The field mixes periodic smooth value noise at 4, 11, and 23 lattice cells per
world with weights 0.55, 0.28, and 0.07. A 0.10 ridge component forms narrow smooth
bands around the random 8-cell noise field's contours. The existing 7-cell,
0.045-amplitude domain warp is retained with periodic sampling. Quintic
interpolation makes noise C2 continuous, including across the torus boundaries;
a quintic zero shoulder below 0.43 creates genuinely barren space. There are no
patch centers, authored routes, endpoint searches, or connectivity guarantees in
the resulting field. Each seed/epoch determines one canonical map; the existing
quarter-turn permutation and one-million-tick keyframe interpolation remain unchanged.

Each new map is scaled to the old generator's mean for that exact seed/epoch.
The frozen legacy generator is retained to calculate only this scalar budget;
its locations do not influence the new map. This incurs an additional noise-grid
pass at generation time but avoids replacing the old economics with a sampled
average constant. Habitat contrast still blends with that mean. Existing code
normalizes productivity to approximately one. Initial total food and summed
geographic capacity therefore remain approximately unchanged; integer food
rounding still applies. Realized production can differ because spatial weather,
local saturation, depletion, and accessibility interact with the new topology.
No statistics are calibrated against evolved agent performance.

Run the CPU-only comparison with:

```text
cargo test simulation::habitat_tests -- --nocapture
```

The fixed cases are (seed, epoch) = (1, 0), (42, 3), (91, 30), (3137, 101).
Diagnostics print old/new mean habitat, normalized productivity, exact-zero
barren fraction, fraction below 0.01, axial toroidal autocorrelation at 1, 4, 16,
64, and 128 cells, seam neighbor RMS, initial food totals, and four-neighbor
component counts/largest sizes at thresholds zero, 0.01, and the map mean.
Tests verify seed and epoch sensitivity, repeatability, mean preservation,
finite nonnegative fields, barren space, short-range correlation, contrast,
rotation permutations, and continuous boundary values and slopes. Component
sizes are reported rather than prescribing connected routes or requiring a
particular number of components.

Observed comparison (rounded):

| Seed/epoch | Mean, both | Barren old/new | Correlation at 16 cells old/new | Seam RMS old/new |
| --- | --- | --- | --- | --- |
| 1/0 | 0.04842479 | 0.648 / 0.414 | 0.664 / 0.813 | 0.087194 / 0.000765 |
| 42/3 | 0.03556675 | 0.614 / 0.295 | 0.668 / 0.754 | 0.025557 / 0.001643 |
| 91/30 | 0.02988621 | 0.559 / 0.423 | 0.622 / 0.800 | 0.241228 / 0.000951 |
| 3137/101 | 0.03143683 | 0.786 / 0.643 | 0.137 / 0.849 | 0.209585 / 0.001357 |

All mean differences are below 1e-7; normalized productivity is within 0.0001
of one. Initial integer food differs by less than 0.7% in these cases. Opposite
edge cell centers are one cell apart, so their RMS difference need not be zero;
exact seam coordinates are tested separately. At habitat threshold 0.01 the new
maps have 8, 8, 4, and 11 components respectively, including small islands and
large regions. These are diagnostic observations, not acceptance targets.
