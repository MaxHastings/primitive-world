# Primitive World v46: decisions and evidence

The design is judged by **useful naturally closed reproductive chains per
real hour**, with a familiar wallpaper and a population commonly in the
hundreds. It is not judged by TPS, packet count, births, or ancestry depth
alone. The [execution record](fast-evolution-execution.md) defines the
integrated release and manual outcome metrics.

| Rank | Decision | Basis | Remaining risk |
| ---: | --- | --- | --- |
| 1 | Preserve v45's entire evolved population through an isolated v46 import. | The complete v45 checkpoint contains living bodies/packets, both genome banks, lifetime weights/traces, and reservoir. A converter round-tripped a copied 542-entity, 2.654B-total-tick source. | Imported agents can still face changed dynamics; the source and frozen copy remain recoverable. |
| 2 | Use BIO_DT=2 with two physical substeps and paid three-packet reproductive bursts. | BIO_DT=3 lost population and viable fusion rate in matched short replay. BIO_DT=2 with a paid burst retained 504 living and 133 births at 3,000 biological units; the same run later reached closed depth three. | Burst strategy may alter long-run selection; only a wallpaper-hour run can decide. |
| 3 | Keep one full local-plasticity write per decision; exponentiate retention across biological time. | Tripling write injection caused mass packet production and energy loss within ten macro-steps in the imported fixture. Removing it restored the old early action distribution. | Adaptation per biological unit is lower than v45; a stronger variant needs lineage evidence. |
| 4 | Restore 512² ecology stores and real 512² food probes; retain the eight-unit growth cadence. | The owner prefers v45's fine food field. Virtual 512 positions over 256 stores preserved sector coverage but could not restore within-cell information. A one-time 256→512 expansion preserved exact natural food, dropped food, and pending extraction totals; direct v45 import now carries its 512-grid ecology byte-for-byte. | Ecology now costs more GPU time; fine-cell competition and slower growth cadence still change selection and need a long wallpaper run. |
| 5 | Use moving-pair swept torus contacts and measured displacement broad phase. | Final-position contact can miss encounters with longer macro-steps. The implementation queries relative closest approach across periodic images. | Dense high-motion scenes may cost more GPU time; short runs do not bound the worst case. |
| 6 | Keep packet-owned immutable genome copies and stable existing slots for the first release. | Existing copy-by-value already preserves the genotype after producer death and slot reuse. The imported 500-entity world runs at roughly 2.8–3.2K macro-steps/s headless in short probes; no handle-pool gain has been measured. | Dense arrays and full body copies constrain the speed target; a handle pool plus reclamation is a later engineering experiment. |
| 7 | Distinct v46 model/save/wallpaper boundary and exact closed-chain counters. | Normal v46 load rejected v45; conversion, save/reload, and short closed-chain continuation worked. Cross-version singleton prevents overlap; stop/tray/startup/log/save lookup are v46-scoped. A live packaged restart exposed and fixed filename-based resume selection; retention now protects the most advanced receipt. The 512-grid format uses checkpoint magic 62 and receipt version 6. Its packaged wallpaper resumed from accumulated decision 6,456,821 at MAX. | A world rollover occurred soon after the 512-grid handoff; the manual run must verify sustained population and chain retention. |
| 8 | Exit fusion's exact-copy telemetry comparison at the first mismatch. | On a copied 531-entity v46 checkpoint, short selective GPU samples fell from about 100 to 60 µs per controller decision for `inherit_fusion`. A 1,000-decision headless replay took 0.790 s in the packaged reference and 0.750 s in the candidate, with identical population, packet, birth, and exact-copy counts. | The wall-time difference is one concurrent-wallpaper sample; it is not a sustained throughput or evolutionary-fitness result. |

## Measured versus inferred

**Measured v45 baseline:** 2,660 headless ticks/s and 2,353 wallpaper ticks/s
in an earlier 582-organism/53-packet profile on RTX 4070 SUPER. The sampled
GPU batch was about 308 µs/tick; sensing, fusion inheritance, ecology,
plasticity, and decision samples were about 66, 53, 51, 23, and 18 µs/tick.
Those samples are not additive. See
[the baseline](fast-evolution-baseline.md) and
[GPU performance record](gpu-tick-performance.md#saved-world-layer-profile).

**Measured short replay on the earlier 256-grid variant:** The exact copied v45 source retained 528 living
and produced 182 viable births over 3,000 biological units in 1.03 seconds
of headless simulation. The final v46 configuration retained 504 living
and produced 133 viable births over 3,000 biological units; at 12,000 units
it retained 520 living, recorded 612 births, 295 closed births, and maximum
closed depth three in 1.90 seconds of simulation. These are separate
deterministic trajectories from the same copied source, not statistical
estimates. They establish that a closed chain is possible and the population
is stable over this short horizon. They do not establish a wallpaper-hour
gain or strategy usefulness.

**Measured 512-grid handoff checks:** A 3,654,631-step v46 checkpoint was
expanded without changing its tick, 270 living entities, learned state, or
genome buffers. Natural food, dropped food, and pending extraction totals were
exact before and after. From that identical state, 1,000-decision headless
continuations took 0.525 s on 256² and 0.614 s on 512², ending with 262 and
221 living. This is one short divergent trajectory, not an estimate of
long-run strategy retention. The direct v45 import into the 512-grid format
also validated, with the same 2.654B-tick source hash and no food regridding.

**Inference:** BIO_DT=2 should allow earlier maturation in controller
decisions. The paid burst can preserve physical packet supply, and closed
births per wall hour may increase if the wallpaper sustains a comparable
population. Neither conclusion is guaranteed by the short replay.

**September 2026 targeted speed follow-up:** At 512² food resolution, `consume`
cost about 19–34 µs per controller decision, while sensing and fusion inheritance
cost about 119–177 and 89–116 µs respectively in short timestamp probes of a
copied live checkpoint. Sparse feeding could save only a few percent even if
the entire consumption pass disappeared. Two sensing arithmetic substitutions
showed no reliable gain and were removed. The retained fusion change skips
remaining read-only genome comparisons once exact inherited equality is
already false. It does not change genes, learning, ecology, rendering, or
the definition of exact-copy telemetry. Short candidate/reference replay
showed equal 576 living, 97 packets, 76,050 cumulative births, and 624 exact
copies at 1,000 decisions; tiny food and energy differences are consistent with
the existing unordered GPU interaction/reduction behavior. The measured
headless simulation-time difference was about 5.1% in this one paired sample.

**BIO_DT=4 and adaptation-ramp probe (headless only):** The copied 512-grid
v46 import was replayed with equal biological time. A direct BIO_DT=4 variant
kept ecological updates about eight biological units apart and allowed up to
five fully paid packets per production decision. At 3,000 biological units it
had 386 living and five viable births, versus BIO_DT=2's 418 living and 35
births. In single-world continuations, the BIO_DT=4 world became extinct at
18,896 biological units with 31 births; BIO_DT=2 lasted 51,192 units with
495 births. Over 100,000 biological units with normal reservoir rollover,
direct BIO_DT=4 completed three worlds and ended in world four with five
living; BIO_DT=2 completed one world and had 1,447 living in world two.

A separate 200,000-unit ramp gradually mixed 2/3-unit decisions, then 3/4,
with ecology scheduled by elapsed biological time. It was held at four for
another 100,000 units. The ramp recovered repeatedly after world restarts
and ended with 635 living in world eight, but had completed seven worlds; its
late four-unit worlds did not demonstrate sustained inherited-chain retention.
These are individual GPU trajectories from one imported checkpoint, not a
population-level fitness estimate. The owner chose to keep BIO_DT=2. The
packaged wallpaper, v45 source, and v46 saves were untouched by the probes.

## Explicit alternatives deferred

- **BIO_DT=3:** More biological time per decision, but lower realized
  reproductive closure and major early population loss in the imported
  fixture. Revisit only with better intra-step encounter and controller
  treatment, not as a TPS shortcut.
- **BIO_DT=4 or a 2→4 ramp:** Faster biological clock in the short replay,
  but much weaker direct reproductive closure and repeated later-world
  failures in the ramp probe. Neither is selected for the live wallpaper.
- **Threefold plasticity injection:** Immediately destabilized imported
  controllers. One observation currently makes one learned write.
- **256-grid food or shared body/signal fields:** The coarse food store loses
  within-cell information even if virtual probes preserve sector coverage.
  Retain exact 107 channels and real 512-cell food probes. The independent
  body contact grid remains 256².
- **Genome handle pool, light packet records, in-place body update, dense
  compaction:** Potentially valuable GPU work, but requires a separate
  ownership/checkpoint migration with measured benefit. Packet genomes
  currently have independent by-value ownership, so safety does not
  require a handle pool.
- **Increasing packet contact radius:** A 3.2-unit probe only modestly
  improved short BIO_DT=3 births and changed physical ecology; the release
  retains the 2.0-unit radius.

No population clamp, direct mating, authored signal meaning, identity-based
cognition, or reward oracle belongs in v46.
