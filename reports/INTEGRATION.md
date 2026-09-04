# Integrated social ecology release

This pass connects moving food geography, purposeful trips, local reports, and learned access to useful companions. It removes redundant generic attraction/alignment and obsolete migration scripts. The seven actions remain wait, move, harvest, eat, give, force, and communicate; reproduction remains a reserve-funded lifecycle process.

## Environment

Broad fertile regions combine with multiple scales of coherent value noise. Seeded peaks drift, change size/richness, and fade as replacements emerge. Smooth interpolation spans 8,192 simulation ticks; regional renewal spans three such intervals. Faster weather and extraction-driven soil pressure remain separate. Regional seasonal phases now span the full cycle. Normalized potential productivity avoids making every geography transition an arbitrary increase in supply, but does not guarantee constant actual food production.

The Landscape fertility lens shows potential habitat independently of current vegetation. Evolution can be frozen. Headless famine restoration now restores growth conditions without painting food across barren ground.

## Social mechanism

Senders remember recent report deliveries; reports retain original observations and provenance. Guidance credit requires encountering usable food near the reported location. Route refinement preserves attribution. Learned generosity, successful guidance, and harm affect access to locally visible individuals. All travel candidates apply the same social access and avoidance terms. No group ID, assigned leader, or remote companion tracking was added.

The controlled migration fixture uses three familiar agents, one informed about a replacement patch. A second departure standardizes physical state and removes new reports, retaining or erasing learned experience. In the passing comparison, all three arrive and survive with learned access; only the informed agent survives when reports were absent, experience is erased, or companion access is disabled. See [raw comparison](social-migration.json). This is evidence for a mechanism in a constructed scenario, not population-wide herd behavior.

## Integration evidence and remaining work

- [Seed 1, 32,768 ticks](integrated-social-seed1.json): population reaches 8,308 at the end, with continuing reproduction, sharing, conflict, and landscape transitions.
- [Seed 2 famine and recovery](integrated-famine-seed2.json): 5,787 agents before famine at tick 6,000; 831 at the sampled low at tick 10,240; 8,043 by tick 16,384. Regeneration resumes at tick 8,000 without a global refill. These integration runs precede the final unification of social travel scoring.
- GPU tests cover transfers, births, local observations, learned outcomes, migration controls, landscape boundaries, clocks, and checkpoint compatibility. Version 6 loads versions 3–5 with missing fields initialized.
- The [population movement diagnostic](integrated-motion-before.json) found residual repeated reversals in some agents. A short-horizon tracking adjustment broke the controlled migration case and was reverted for this release. Social pursuit needs further work; this is a known behavioral limitation, not a solved pacing claim.

Reproduction still uses maturity, reserves, cooldown, and a per-tick eligibility chance. It does not forecast sustained income or add dependent offspring. Boom/famine cycles remain possible. Food sensing still uses four compass samples; behavioral weights are fixed. The release integrates the mechanisms and exposes their outcomes without claiming the long-term acceptance plan is complete.
