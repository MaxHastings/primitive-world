# Juvenile physiology validation

Model: `primitive-v41-juvenile-opening-ramp`. Default maturity: 1,800 ticks.

Newborn gathering eases from 100% to 1% over the existing 100,000-world-tick
smooth schedule. The GPU survival fixture runs both at tick zero and after
tick 100,000: independent gathering succeeds early and fails after assistance
ends. Repeated generic provisioning succeeds in both phases.

After assistance ends, the GPU feasibility fixture starts juveniles with maximum retained birth energy
(48). Reserves alone, reserves plus a full newborn inventory, and maximum
gathering effort with repeatedly replenished ground food all starve before
maturity. A separate juvenile starting with 12 energy reaches maturity through
ordinary transfers from a stationary gathering adult. This fixture establishes
physical reachability; its test controllers are not installed in the world.

Additional GPU checks cover gathering at six ages, adult gathering throughput,
transfer inventory limits, capped large-packet fusion, and starvation on the last
growth tick. Death on that tick is juvenile starvation, not successful maturation.
Additional opening-ramp checks cover the midpoint, clamped endpoint, adult
gathering independence from world age, and checkpoint replay across tick 100,000.

## Previous fixed-physiology evidence (v40)

Three fixed seeds used 4,096 random mature founders, default settings and a
12,000-tick ceiling, stopping at extinction. No founder genomes were selected or
edited, and no social parameters were tuned from these results.

| Seed | Extinction tick | Births | Juvenile starvation | Matured offspring | Juvenile transfers | Births from descendants |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 42 | 9,928 | 12 | 12 | 0 | 0 | 0 |
| 91 | 9,640 | 17 | 17 | 0 | 0 | 0 |
| 3137 | 10,344 | 14 | 14 | 0 | 0 | 0 |

These runs demonstrate first-generation reproduction, **not reproductive
continuity**. None achieved offspring maturation or descendant reproduction.
They do not prove that random populations can never achieve continuity. The
controlled provisioning result also does not demonstrate an evolved solution.
The default dependent life cycle is physically reachable, but bootstrapping an
evolving population through it remains unresolved.

## Opening-ramp evidence (v41)

Reproduce the current model with `cargo build --release`, then
`python tools/juvenile_probe.py --output reports/juvenile-probe.json`.
The diagnostic retains raw family reports, reports continuity explicitly, and
does not select founders or feed observations into the simulation. The v40 table
above records the earlier fixed-physiology comparison, not this command's model.

The same fixed seeds, population and 12,000-tick ceiling were rerun without
controller changes or selection. These observations cover only the early ramp,
not survival through the entire 100,000-tick transition.

| Seed | Final tick | Births | Juvenile starvation | Matured offspring | Juvenile transfers | Births from descendants |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| 42 | 10,120 | 10 | 10 | 0 | 0 | 0 |
| 91 | 12,000 | 14 | 12 | 2 | 1 | 0 |
| 3137 | 10,376 | 11 | 10 | 1 | 0 | 0 |

Seeds 42 and 3137 became extinct; seed 91 reached the observation ceiling.
The single transfer delivered 0.599 food units. Three offspring matured, but
none produced another generation within this window. This is evidence of early
maturation, not sustained reproductive continuity or evolved care. Physiology
was not adjusted to make a social metric positive.

Validation: all 144 release tests passed (8 manual tests ignored), including
GPU phase checks, independent ramp toggles and checkpoint replay. Formatting,
Clippy with warnings denied, and all 12 Python tests pass. The release executable
builds successfully.
