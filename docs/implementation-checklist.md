# Primitive World implementation evidence

Current model: `primitive-v39-shorter-lifespans`; checkpoint 57
(`PRIMWORLD057`); founder bank 21. Earlier biological formats are rejected.

## Implemented contract

- Reproduction requires local, passive packets from different producers. Fusion
  radius eases from 6 to 2 world units; packet upkeep eases from .002 to .02
  times size^(2/3) per tick over the same 100,000-tick opening ramp. Resource
  depletion limits viability; fusion combines resources minus construction loss
  and recombines both genomes. Packet size is heritable and mutable.
- Contact transfer, pushing, sensing, private memory and optional generic signals
  remain. Contact mating and fixed parental roles are removed.
- New worlds start with 4,096 agents by default.
- Body metabolism defaults to .05 per tick and is adjustable live; lifespans are 9,000–11,000 ticks.
- New worlds begin with food across the whole map. A temporary habitat and
  productivity floor fades smoothly to zero by tick 100,000. Existing rich
  patches retain normal capacity and growth; the quantity multiplier is removed.
- Ecology starts at 10% speed and smoothly reaches normal speed by tick 100,000.
  Its integrated clock keeps terrain and weather continuous. Soil changes share
  the speed ramp. Agent movement and cognition retain their normal speed.
- Checkpoints preserve the opening phase. New worlds restart it.
- Full entity storage skips unpaid manufacturing requests and allows in-place
  fusion. Capacity does not pause playback. Accounting horizons roll over;
  transient save/readback failures keep playback running.

## Verification

The release suite covers packet manufacturing and resource conservation,
different-producer fusion, blind recombination, mutation parity, full entity
storage, identity-horizon rollover, and checkpoint replay. It also checks smooth
weather boundaries, gradual vegetation loss, initial coverage, and reproduction
ramp endpoints at ticks 0, 50,000, 100,000 and 200,000.

Required checks are `cargo fmt --check`, strict all-target Clippy, the CPU-focused
test command and Python unit tests in CONTRIBUTING.md, plus the serial release
GPU suite. A fresh current-model smoke run confirmed 4,096 agents and .05
metabolism. Wallpaper size does not scale the requested starting population.

These establish implementation behavior. They do not establish evolved social
coordination, anisogamy or multiday ecological persistence. The earlier matched
seed-42 packet encounter comparison used 4,096 founders and permanent packet
assistance; it is not evidence for persistence under the current settings.
