# Primitive v46 fast evolution: implementation decision

Status: proposed implementation cut, written before simulation changes.

The [ranked decision record](fast-evolution-decisions.md) explains the expected
payoff, risks, retained capabilities, and options deferred from the first cut.

## Decision

Build a distinct `primitive-v46-fast-evolution` model in the existing wallpaper
application. The old v45 code and experiment remain available on the original
branch and in existing saves. V46 must reject v45 checkpoints, use a new save
format, and place automatic wallpaper saves in a model-specific directory so a
normal launch cannot consume or overwrite the active v45 experiment.

The single integrated cut is deliberately ambitious. It changes time, ecology,
sensing, organism/packet storage, inheritance, and learning together. We will
make implementation commits by subsystem for review and recovery, but will not
run a prolonged parity program or treat every intermediate commit as a model
ready for the user's wallpaper. The first user-facing run is the integrated v46.

## Physics and ecology

- One decision and one macro-step represent **three biological time units**.
  Biological ages, maturity near 1,800, lifespan near 9,000–11,000, metabolism,
  upkeep, gathering, digestion, packet decay, and resource rates use biological
  time. Damping and retention use exponentiation by `BIO_DT`, rather than a
  linear multiplier. Output actions are held over a macro-step.
- Integrate movement over the held action and use swept, toroidal contact tests
  for transfer, force, organism encounters, and packet fusion. End-position-only
  tests are insufficient at BIO_DT=3.
- Use a **256×256 food grid** at the same logical world size. Keep food,
  fertility/carrying capacity, and one optional productivity state. Climate
  changes growth/capacity independently of population success. Agents deplete
  food continuously; the ecology integrates accumulated extraction and three
  biological units per macro-step **once every four macro-steps**. Render the
  coarse grid smoothly.

## Sensing and cognition

- Build shared coarse body, motion, and signed-signal fields once per step.
  Agents sample sixteen body-relative positions (eight directions, two radii),
  each with food, proximity, relative motion, and signed signal. Add private
  energy, inventory, development, velocity, and recent physical-change inputs.
  No IDs, coordinates, lineage, map, or authored signal meaning enter cognition.
- Keep **1–16 expressed recurrent units**, latent inactive hereditary weights,
  topology mutation, the existing general-purpose action set, hidden memory,
  and locally learned recurrent/gate/output connections. Remove lifetime-plastic
  input-to-hidden deltas and their traces/storage. Scale trace and learned-weight
  retention for the macro-step. No reward or scripted behavioral policy is added.

## Entities and inheritance

- Keep organisms and packets in separate dense live arrays. Organisms own body
  and cognitive state; packets own physical state, ancestry, and a genotype
  handle. Both have persistent 64-bit identities independent of storage index.
- Store inherited parameters once in an immutable genotype pool. Packet creation
  copies a handle, not a genome. At a physically successful fusion, recombine and
  mutate the two referenced genomes, allocate one child genotype, and give it to
  the child. Pool reclamation must retain handles used by live organisms, live
  packets, and the hereditary reservoir.
- Reproduction remains energy-funded, spatial, decaying, and compatible-packet
  based. Transfer, force, and signed local signaling remain anonymous physical
  actions. Childhood, energy death, and old age remain selection pressures.

## Wallpaper and experiment use

- The normal viewer and wallpaper modes run v46 with pause, save/resume,
  inspection by stable identity, organisms, packets, smooth terrain/food, and a
  HUD showing macro-steps/s and biological units/s. Rendering may skip simulation
  steps but should retain normal display cadence.
- Fresh v46 worlds should be tuned for an ordinary living population in the
  hundreds to roughly a thousand, without a hard population controller.
- A v45 receipt/checkpoint never loads as v46. Development uses isolated output
  paths and does not touch the running v45 wallpaper or its save library.

## Evidence before handing over the app

Use a short, focused release-build check: compile, launch headless and wallpaper,
save/reload a v46 world, reject a v45 save, and verify a controlled natural birth
can mature and contribute to another birth. Record one representative population
and macro-step/biological-time throughput measurement. This is a mechanical
readiness check, not an attempt to predict what open evolution will discover.
The user's longer wallpaper run is the scientific trial. We will report its
evolutionary throughput only from observations actually produced by that run.

The target is at least 5× effective biological-time throughput, with roughly
10× as the intended outcome. It is a target, not a pre-existing measurement.
If the integrated cut misses it, the recorded layer timings and population will
identify the next bottleneck without redefining success as raw TPS or a tiny
population.
