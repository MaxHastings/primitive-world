# Departure attrition diagnostic (development, not final validation)

Registered before running schema2 observation. Use already-consumed development
seed808, released recurrent-v1 bank,1000 bodies, costs0.06/0.01, gain4,
regeneration0.01, original rotating food sensors, evolving geography,200k cap or
extinction. Metrics every1024 ticks, journey evidence every32. No export, rescue,
parameter change or retries. Prior observer runs are preserved; this is a new
instrumented run, not replacement of their outcomes.

Retain all qualifying departures that end or reach the horizon, including sampled
waypoints, energy, inventory, age, speed, path counter, attention and body action.
Distinguish observed dead bodies from reused/missing identities and right-censored
living tracks. A global scan at departure finds the nearest qualifying vegetation
cell/footprint outside96 units of origin. This is an observer-only geometric
comparison; nothing is supplied to controllers or genotype selection.

Compare the departure-time distance with `min((energy+8*inventory)*speed /
(metabolism+movement_cost*speed), remaining_age*speed)` at both maximum body speed
and observed subsequent path speed. The former is an optimistic no-new-food
straight-line budget, not a guarantee of finding/persisting food. The latter is
an illustrative constant-speed scenario, not a proof of impossibility. Report
path/net efficiency, observed feeding, and reproduction after departure. Preserve
all records and distinguish defined sampled attempts from the whole population.

Decision: use these observations to prioritize physical reach versus direction /
sensing / preparation limitations. No survival or migration acceptance gate can
be passed by this diagnostic. Do not tune thresholds to turn a failed attempt
into a successful journey.
