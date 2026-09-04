# Headless behavioral diagnostics

These are observer experiments, not founder preparation or policy training.
They do not award fitness for a route, reseed an extinct agent, or change the
ordinary application's defaults. The active project is **Primitive World**.

## Two-patch travel

```powershell
cargo build --release
python experiments/travel.py --directory reports/my-travel-experiment
```

Use a new directory for every batch. The runner writes an incremental
`summary.json`, exact commands, executable SHA-256, and per-case JSON traces.
It refuses an existing directory. Individual runs refuse to overwrite reports.

Defaults: patch separations 120 and 300 world units, regeneration 0.002 and
0.02, seed/founder pairs 7/0 and 19/1, both initial-knowledge conditions, and
memory present/erased: **32 conditions, not 32 independent seed samples**.
Runs end at 3,000 ticks or the original individual's death.

Each trial uses the actual candidate-v1 GPU controller and physics, with:

- one mature body, energy 65, inventory 2, adult speed 1.2;
- two circular patches, default radius 16 and 0.3 initial food per food cell;
- patch habitat/productivity 1, initial soil fertility 0.65, barren gaps;
- normal collection, ingestion, metabolism, movement costs and weather;
- static geography, no other bodies, and births suppressed by cooldown.

This is deliberately a custom diagnostic landscape, **not** a claim to reproduce
ordinary-world population fitness. Productivity is not globally normalized to
the tiny fertile area. Patch labels and observer measurements never enter the
controller.

### Conditions

- `discovery`: the body starts in patch A, both patches contain food, and place
  memory is empty. Patch B must be found through actual local sensing.
- `known-target`: patch A starts empty (but can regrow), and a fresh private
  memory of B is supplied. This tests navigation to known coordinates; it is
  not evidence of spontaneous discovery or learned environmental knowledge.
- `--erase-place-memory`: erase only place records before every decision.
  Existing destination commitment, genome and RNG remain. Thus this is not an
  ablation of all temporal state.

Single-case example:

```powershell
.\target\release\primitive_world.exe --headless --travel-diagnostic --travel-mode known-target --travel-distance 300 --travel-food 1 --regeneration 0.02 --seed 7 --ticks 1600 --output reports/known-target.json
```

Other diagnostic options: `--travel-radius`, `--travel-genome` (bundled bank
index), and `--bootstrap` (explicit bootstrap weights without standing noise).
The batch runner accepts `--distances`, `--regenerations`, `--seeds`, `--modes`,
`--food`, `--radius`, `--ticks`, and `--exe`. See `python experiments/travel.py --help`.
Unrelated simulation options are rejected by the diagnostic rather than silently
changing or being ignored by the trial.

### Matched sensing experiment

`--travel-sensing baseline|near|sweep` (batch runner: `--sensing`) changes
only the geometry of the four remote food/crowd samples. Underfoot sensing is
unchanged. Default `baseline` is the ordinary controller's four cardinal points
at radius 24. `near` puts those four points at radius/6 (4 units). `sweep` cycles
through radii 4, 12, 24 at cardinal directions, then the same radii at diagonal
directions, repeating every six ticks. This is sparse scanning, not a filled
sensor disk; it adds neither sample reads per tick nor random draws.

Movement candidates and private memories use the actual sampled coordinates.
The authored destination scores, goal commitment, genomes, costs, resource
production and reproduction rules are unchanged. Sweep trades temporal
coverage for instantaneous coverage; its channels are sample slots, not fixed
compass labels. These modes are restricted to the headless diagnostic CLI and
are not promoted to GUI/default sensing or the archived neural controller.
They use reserved flag bits 8/16 in the existing GPU experiment flag word;
storage layouts and checkpoint version are unchanged.

Matched example (use a separate new directory for each sensing mode):

```powershell
python experiments/travel.py --directory reports/my-sensing-baseline --sensing baseline --distances 300 --regenerations 0.002 0.02 --seeds 7 19 31 43 --modes discovery --ticks 3000
```

Repeat with `--sensing near` and `--sensing sweep`. These are 16 conditions
per mode but only four seed/genome pairs, not 16 independent samples. Compare
each condition directly across modes; do not compare only the best trajectories.

### Metric definitions

- `observed_patch_tick`: first positive food sample belonging to each patch;
  does not count the supplied initial memory as an observation.
- `entered_patch_tick` and `visits`: geometric body arrival. A visit is not a
  claim that the agent collected food. Consecutive visits to the same patch
  collapse; an A–B transition and A–B–A return are distinct.
- `food_collected_per_patch`: actual collection requests resolved by the GPU.
- `trips`: exits through arrival, death, or end-of-run censoring, including path,
  displacement, reserve change, collection and destination changes. Excursions
  returning to the same patch are retained but are not inter-patch transitions.
- `goal_changes`: executed movement destinations changing by more than 2 units.
  These include legitimate new journeys, not just mid-trip abandonment.
- `trace`: every 16 ticks plus goal changes, patch crossings and death. Readbacks
  and metric accumulation run every tick; intermediate movements are not lost.
- Final patch food, regenerated food, and dropped food expose inaccessible
  surplus. Food-cell membership uses cell centers; visits use body coordinates,
  so boundary labels have food-grid resolution rather than exact shape identity.
- Reserves are energy plus inventory times conversion efficiency. Nominal
  energy-budget residuals include floating-point error and the final death-tick
  energy clamp. The additional death-drop-inclusive residual accounts for food
  released on death. These diagnostics are not rewards.

Findings from the first batch and controls are recorded in
[TRAVEL_DIAGNOSTIC.md](../reports/TRAVEL_DIAGNOSTIC.md).
The matched sensor-geometry results and destination-score characterization are
in [SENSING_DIAGNOSTIC.md](../reports/SENSING_DIAGNOSTIC.md).
