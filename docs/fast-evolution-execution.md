# Primitive World v46: integrated release and handoff

Status: v46 began running from the imported 2.654B-tick v45 source at MAX.
The owner subsequently ran `BIO_DT=3` and then selected `BIO_DT=4` for a
long adaptation trial on the continuing v46 wallpaper at MAX. The first integrated release below records
the earlier `BIO_DT=2` architecture and evidence.
The owner then chose to restore the 512² food field. The 512-grid wallpaper
is running from an explicitly expanded v46 checkpoint with its population,
learned state, and accumulated progress preserved. The architecture below supersedes the original
BIO_DT=3 proposal. The objective is
more **useful, naturally closed hereditary chains per real hour** while keeping
hundreds of interactive agents and the familiar wallpaper. Macro-step rate,
birth count, and ordinary ancestry depth alone are insufficient.

## First integrated release

1. **Biological clock.** One controller decision spans two biological units
   (`BIO_DT=2`). Age, digestion, metabolism, gathering, motion, packet upkeep,
   cognitive retention, and ecology use biological units. Energy and inventory
   feedback is divided by the interval length before it enters the unchanged
   107-input controller. A decision is held across two physical substeps.
   Transfer, force, and signaling still require an actual selected action.
   Physical packet production can make a burst of at most three packets,
   each paying its full inherited energy cost and using a distinct free slot.
   No direct birth or population controller is introduced.
2. **Ecology and perception.** Natural food, dropped food, soil, ground, and
   ecological pools use 512×512 cells, matching v45's spatial resolution and
   local food probes. The independent body contact grid stays 256×256.
   Harvest and depletion occur each macro-step; growth, weather, and substrate
   update every fourth macro-step over eight biological units. Per-cell
   capacity and growth rates again use the 512-cell area. Exact local
   body/signal scans and all 107 input indices remain.
3. **Contacts, cognition, and heredity.** Bodies and packets move in bounded
   substeps. Contact broad phase expands by measured maximum net displacement;
   narrow phase uses same-time closest approach of start-to-end trajectories
   across torus images. Packet paths are straight within one macro-step, while
   organism paths can curve and are approximated by one chord. The
   [reproductive-cadence probe](bio-contact-cadence-diagnostic.md) did not
   identify force-contact detection as the main loss at larger steps. Full
   recurrent memory and all local plasticity routes
   remain. Trace and learned-weight retention span two biological units, while
   one observed decision makes one learning write. Tripling that write caused
   rapid imported-controller failure in a matched probe. Each packet owns a
   full immutable copy of its inherited genome and traits. The producer can
   die or its slot can be reused without changing the packet. A GPU handle
   pool and lightweight packet storage are deferred because they increase
   ownership and checkpoint risk without measured first-cut gain.
4. **Closed-chain observation.** A v46 physical fusion creates a natural-born
   organism. A natural-born organism that reaches maturity is counted once.
   A later viable fusion counts each natural-born packet donor as a genetic
   parent contribution, counts the birth once if either donor qualifies, and
   advances maximum consecutive closed-parent depth. Imported v45 entities
   are marked as source generation, so their existing ancestry never counts
   as a v46 closed transition. Counters persist through save and reload.
   Genetic contribution events can repeat for one parent; distinct contributor
   identity is a later telemetry addition.
5. **Wallpaper and saves.** V46 has its own model ID, checkpoint magic 62,
   receipt version 6, default save root, log, tray class/title, stop lookup,
   and optional startup registry value. The cross-version wallpaper singleton
   remains shared so two desktop children cannot overlap. Resume preserves
   the checkpoint's logical habitat, full population, and geography when the
   monitor size differs; only the camera adapts. Shapes, palette, lenses,
   controls, and compact HUD remain, with biological throughput shown in
   details and profiling.
6. **One-time evolved-population import.** The explicit converter reads a
   complete copied v45 receipt/checkpoint and never writes to the source.
   It carries every living organism and packet, identity, age, reserves,
   hidden state, inherited genome/traits, fast weights, traces, reservoir,
   and logical habitat. It carries the original 512-grid ecology byte-for-byte,
   resets only tick-relative scratch and v46 progress, preserves climate phase, and
   writes provenance including source cumulative ticks and SHA-256. A
   complete frozen v45 source copy is stored beside the new v46 experiment
   for recovery. Normal v46 load rejects v45 checkpoints.
   A separate one-time converter expands the 256-grid v46 continuation to
   512 cells. It copies bodies, genomes, learning, reservoir, counters, and
   clock unchanged; splits food, dropped food, and pending extraction with
   exact integer totals; duplicates intensive fields; and quarters the
   extensive mineral and detritus pools. It freezes the source checkpoint,
   records provenance, and validates the expanded checkpoint before publishing
   a receipt. Earlier 256-grid receipts require this explicit converter.

## Evidence that changed the proposal

The copied 2,654,150,367-total-tick v45 checkpoint held 542 living entities.
The v45 executable, run read-only against that copy, retained 528 living and
made 182 viable births over 3,000 old biological units. The initial BIO_DT=3
build fell to 240 living and 14 births over the same biological duration.
A threefold learning injection caused hundreds of costly packet attempts
within the first ten macro-steps. Returning to one learning write restored
early actions. Virtual 512-grid sensing removed large input-sector gaps.
Paid multi-packet production recovered packet supply. BIO_DT=2 retained
substantially more of the population than BIO_DT=3. The first 256-grid build
was an intermediate variant; the owner chose 512-grid storage to retain the
food information that virtual probes could not reconstruct.

In a short headless replay of the copied source, the selected BIO_DT=2,
three-paid-packet build had 504 living, 133 viable births, and 28 natural
maturations after 3,000 biological units. At 12,000 biological units it had
520 living, 612 viable births, 424 natural maturations, 295 births with a
natural-born genetic parent, and maximum closed depth three. The measured
simulation portion took 1.90 seconds on the local RTX 4070 SUPER. These are
short **256-grid variant** observations, not a wallpaper-hour outcome or a measured
fivefold effective-evolution gain.

An earlier live 256-grid wallpaper sample showed about 5,900 biological units
per wall second; this is not a controlled comparison to v45's older
2,353-tick/s wallpaper sample. From one identical 3,654,631-step v46 checkpoint,
short 1,000-decision headless continuations took 0.525 s at 256² and 0.614 s
at 512², ending with 262 and 221 living respectively. This is a single
trajectory comparison, not a long-run fitness estimate. Exact natural food,
dropped food, and pending extraction totals survived the 256→512 expansion.

## Mechanical readiness and handoff

- Release build; short fresh and imported GPU runs; normal v46 rejection of
  v45; import total-food equality; v46 save/reload with live packets and
  closed counters.
- Confirm paused and running wallpaper frames, native display cadence,
  palette, food/terrain lenses, organism/packet presentation, paint, pause,
  tray save/quit, and resize/camera behavior. Inspect the captured 512-grid
  wallpaper frame after handoff.
- Stop the running 256-grid wallpaper only after the new build and converter
  pass. Select its newest complete save, expand it to an isolated 512-grid
  experiment, check exact food totals and headless load, then launch the
  packaged 512-grid wallpaper at MAX. Confirm the resumed tick and population
  in the log. Retain the v45 source, v46 256-grid source, original checkout,
  and frozen archives.

The final handoff expanded the latest stopped 256-grid receipt at accumulated
decision 6,456,821 (world 2, local tick 334,574, 495 living). Natural food
3,653,676 milli-units, dropped food 1,983,257 milli-units, and pending
extraction 17,333 milli-units matched exactly before and after. The packaged
512-grid wallpaper resumed from that checkpoint at MAX, with a captured frame
showing the familiar palette, agents, terrain, and HUD. Early live readings
were around 125–132 FPS and 6,000 biological units/s. The resumed world later
turned over to world 3; its population remained in the hundreds in a short
observation. Sustained chain retention still requires the manual run below.

The first packaged restart exposed a resume-order bug: the legacy library
scan sorted receipt filenames, causing `save-import.json` to outrank every
timestamped save. The scan now chooses the valid receipt with the greatest
accumulated experiment progress, breaking ties by save time. A regression
test covers both the import filename and a newer stale fork. Save retention
also protects the most advanced checkpoint when a stale fork creates newer
receipts. The 425,403-step v46 continuation was loaded successfully; the
prior 256-grid wallpaper later resumed from its 899,111-step continuation
at MAX. The app binary and login command live under the
isolated `PrimitiveWorldV46` app root; v45 source saves remain untouched.

## Targeted speed follow-up after the 512-grid handoff

A copied 531-entity live checkpoint showed that per-cell feeding was a small
part of the step: about 19–34 µs per controller decision, versus about
119–177 µs for sensing and 89–116 µs for fusion inheritance in short GPU
timestamp probes. Sparse consumption was therefore deferred. Two local
sensing arithmetic candidates were removed after no reliable speed gain.
The retained `inherit_fusion` change exits its read-only exact-copy genome
comparison at the first mismatch. It leaves the genome, mutation draws,
birth rules, 512-grid ecology, perception, and presentation unchanged.

Short paired checkpoint replay measured 0.790 s of simulation/sync time
for the prior packaged executable and 0.750 s for the candidate over 1,000
decisions. Both ended with 576 living, 97 packets, 76,050 cumulative births,
and 624 exact-copy births. This single sample suggests about a 5% speed gain;
it does not measure long-run evolutionary value. Focused fusion and
checkpoint replay tests passed. The prior packaged executable was backed up
inside the isolated v46 app folder. The wallpaper saved to world 7,
tick 51,463, accumulated decision 11,324,783, with 550 living entities;
the new build resumed exactly that receipt at MAX. Its window reported
about 133–137 FPS and 5,942–6,040 biological units/s in brief live checks.
The existing v46 login command still points to the packaged executable.
The broader `fusion` test filter still has three v46 time-step expectation
failures; running the same tests with the prior full-scan shader produced the
same three failures. The two focused fusion/checkpoint continuation tests
passed with both shader versions.

The full local release test suite was also run before the main-branch handoff:
130 passed, 45 failed, and 35 were ignored. Many failures assert v45's
one-unit physiology, one-packet production, or per-tick ecology values; some
exact GPU-state comparisons and behavioral fixtures still need separate v46
review. This is an unresolved GPU-test migration, not a clean full-suite pass.
The repository's automated CPU check excludes `simulation::tests::` and the
GPU-only `simulation::funnel_audit::` module. The earlier filter accidentally
ran a funnel GPU test on GitHub's unsupported HLSL path. The corrected CPU
check and the Python tool tests passed locally.

## Longer manual wallpaper run

Record wall time, macro-steps/s, biological units/s, FPS, live organisms and
packets (including low quantiles), natural food/dropped food and depletion,
GPU/CPU layer costs, save stalls, capacity blocks, births, natural
maturations, genetic parent contributions, closed births, maximum closed
depth, and world/reservoir restarts. Report closed transitions per wall hour
alongside survival and population. Inspect strategy persistence across
descendants; a high birth count alone is not success.

## Later experiments

Profile dense body copies and genotype duplication under the observed v46
population before replacing by-value packet ownership with a generation-
checked immutable handle pool and lightweight packet records. Explore a
stable in-place organism update and live worklists only with checkpoint and
reuse tests. Compare BIO_DT=3, stronger plasticity, and coarser shared
body/signal fields as separately labeled biological variants. Keep the
selected v46 wallpaper as the reference while those variants are tested.
