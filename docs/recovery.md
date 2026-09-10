# Preservation contract: climate and permanent juvenile care

The old Primitive World is the baseline for presentation, controls, world scale,
sensing, brain richness and interaction capabilities. Preserving it is a
requirement. This recovery makes only two intended biological changes:

1. Remove world-age assistance. Juvenile dependence is permanent physiology;
   ordinary food transfer can support maturation. No care policy is installed.
2. Replace opening ecology assistance with ongoing environmental variation:
   smooth climate variation and local refuges over millions of ticks, independent
   of population success. Long abundant stretches and shorter substantial harsh
   stretches are a calibration target, not authored modes or a proven result.

## Implementation

The recovery uses committed baseline `921b297`, not the material-world rewrite.
Its full GPU simulation, 512x512 food grid, default 2048-unit square world,
wallpaper-sized rectangular habitat, renderer, compact wallpaper overlay,
camera, brush controls, inspection, behavioral lenses, population history and
save library are restored. So are 16-region area sensing, packet occupancy,
gated recurrent memory, per-unit plasticity/traces, inherited mutation controls,
module inheritance, packet velocity/pushing and directional impulses/recoil.

Newborn gathering is always `0.01 + 0.99*x^6`, with x the clamped fraction of
the organism's 1,800-tick development interval. Storage grows with body age.
Packet upkeep is always .02 times size^(2/3), and fusion range is always two
units. The old different-producer pairing rule, inherited packet sizes, lifespan
assumptions and physiology remain. None are silently replaced with the rewrite's
material construction, thermal repair or identity-independent merging rules.

Climate has no abundant/drought modes or fixed season. Global rainfall and
temperature combine smooth seeded random-like components at unrelated correlation
scales: rainfall at 173,003 / 1,100,009 / 4,700,021 ticks, temperature at
281,003 / 1,700,029 / 6,100,033 ticks. Regional weather varies around 47,003 ticks;
local weather around 997. C2 temporal interpolation avoids keyframe jumps.

New worlds condition only the starting global keyframes: moisture is sampled
from .8-.95 and temperature from .45-.55. The same smooth interpolation leads to
ordinary unconstrained seeded keyframes thereafter. Local/regional weather is
unconditioned. There is no grace-period timer, resource guarantee or population
feedback. This starting state is serialized as `flourishing_start`; older v42
saves missing the field retain the previous unconditioned climate. Physical
stocks and juvenile physiology are unchanged by this setting.

A 256-seed scan at 10,000-tick intervals found the first departure from rainfall
>=1 and global temperature .25-.75 at a minimum of 660,000 ticks and median of
2,080,000 ticks. Some seeds remained within that proxy through the 8-million-tick
observation limit. This measures favorable forcing, not vegetation abundance or
successful reproduction; those require separate biological observations.

Persistent seeded elevation, retention and permeability affect local water
storage, evaporation and drainage. Each cell stores water, mineral and detritus.
Vegetation growth consumes mineral; recession returns it to detritus;
moisture/temperature-dependent decomposition returns mineral. Mineral weathering
is an external input; rainfall is an external water input; evaporation, drainage
and overflow leave the local water store. Harvested vegetation leaves this
soil subsystem through the existing food/organism system. This is a simplified
open system, not a claim of full watershed transport or closed-world nutrient
conservation. The old dropped-food and body physiology rules are retained.

Habitable coverage responds continuously to stored water, available mineral and
temperature. Favorable conditions support sparse food between the old rich patches;
dry conditions contract cover. Refuges arise from substrate and water history.
The between-patch contribution blends into the original geography rather than
clamping it to a flat minimum, preserving patch edges in favorable conditions.
Initial physical stocks are water .7, mineral 3 and detritus .3 per cell, with an
established vegetation snapshot. These are initial conditions, never a changing
juvenile subsidy. No climate variable reads population, diversity, care or success.

The subsequent [life-cycle validation](juvenile-validation.md) distinguishes a
successful finite-reserve controlled policy from the failed ten-world random
cohort. Favorable climate is implemented; naturally discovered continuity is not
yet demonstrated.

The original terrain generator remains, with keyframes slowed to one million
ticks. Static-landscape mode fixes geography without freezing weather. The
correlation scales are declared assumptions, not a schedule for evolution.

## Preservation and saves

The complete pre-recovery working tree, patch, original baseline archive and
previous packaged executable were preserved outside the repository in a dated
`ClownSimulator-before-recovery-*` sibling directory. The inactive material-world
source was removed from the active tree only after matching archived file hashes.
The detailed [historical audit](recovery-audit.md) describes that archived state.

Current model: `primitive-v42-climate-care`; checkpoint 58; founder layout 21.
Old model saves are preserved but not silently loaded under different biological
laws. Fresh experiments use the restored save library and snapshot retention.
Material-world cohort numbers do not describe this recovered model.

## Separate acceptance requirements

Presentation: inspect the actual wallpaper framebuffer at desktop size; preserve
the normal world palette, fine patch edges and bright agents; retain the compact
overlay, lenses, inspection, menus, painting and camera behavior. Source parity
with the original UI is required except for removal of obsolete ramp controls.

Biology: exercise permanent juvenile dependence and transfer-supported maturation
at ticks 0, 100,000 and 3,000,000. Verify constant packet physics and checkpoint
continuation. Sample climate across 24 million ticks, including smooth late-run
boundaries. Exercise the actual GPU food shader under abundance and drought,
checking water/mineral limits, spatial variation, dry cells and recovery.

Run the restored Rust regression suite, strict Clippy and formatting, plus Python
tool tests. GPU fixtures that formerly assumed opening packet range/upkeep must
use the permanent values while retaining their original physics assertions.
Scarce-food allocation fixtures must isolate weather recession from collection.

These checks establish physical reachability and implementation behavior. They do
not establish evolved care, long-run population persistence, or performance at
every density. Multi-million-tick full-population runs are a separate experiment;
sampling the climate function over that horizon is not such a run.

## Presentation capture

Validation on 2026-09-10: the release Rust suite passed 149 tests, with 10 manual
diagnostic/performance tests ignored. All 12 Python tool tests, strict Clippy,
formatting and diff whitespace checks passed. The packaged executable matched
the release binary by SHA-256 and produced an inspected 3440x1440 wallpaper
framebuffer with the old compact overlay, agents and visible patch boundaries.
Desktop menus and painting were not exhaustively exercised by hand. The visual
test used isolated saves and was stopped through the normal save-and-close path.

Set `PRIMITIVE_CAPTURE_FRAME` to a new absolute `.ppm` path before launching the
viewer or wallpaper to capture its third rendered frame, including the real HUD.
This is an observer-only diagnostic and never overwrites an existing image.
`PRIMITIVE_WORLD_SAVES` can isolate visual-test saves from ordinary experiments.
No capture work is performed when the environment variable is absent.
