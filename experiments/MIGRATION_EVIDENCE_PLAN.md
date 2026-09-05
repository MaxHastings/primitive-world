# Migration evidence upgrade — implementation plan, not final registration

The cumulative preparation campaign continues with its frozen schema2 observer.
Do not rewrite that experiment's definitions or rerun a failed observation as if
it never happened. This plan concerns the observer required BEFORE final eight-
seed evaluation. It changes measurement, not controller inputs, physics, genetic
selection or training reward. Final registration still requires a tested frozen
implementation, exact outcome definitions and unused seeds.

## Gaps to close

1. **Between-sample activity.** A32-tick sample exposes only the last tick's
   collection/ingestion/displacement. A sampled poor-space corridor can contain
   unobserved feeding, rich space or a shove. Add optional observer-owned GPU
   accumulation across every tick, keyed to slot/incarnation/lineage/birth tick.
   Record actual collection, ingestion, force displacement and per-tick habitat
   observations, with interval bounds. Reset on identity replacement; never
   attribute previous occupants' counters to a newborn. Preserve unknown/lost
   intervals rather than filling them with invented zeros. The CPU journey
   classifier must consume these interval facts, not endpoint feedback alone.
2. **Feeding and births in the right place/order.** Keep concrete feeding
   coordinates/ticks and actual birth evidence. A lifetime counter proves a birth
   occurred, not where it occurred between samples. Record event identity, tick,
   position and parent/child lineage for qualifying births. Do not infer a birth
   location from two nearby endpoints if the intervening path is unknown.
   Collection and digestion occurring in the same tick are a real ordered
   physiological sequence; do not require an artificial delay to count them.
3. **Distinct resource patches.** Keep source depletion, poor-space crossing and
   destination feeding as separate requirements. Source/destination distance
   alone does not establish distinct patches. Use a documented observer-only
   spatial patch classification derived from actual vegetation, and preserve
   its evidence at source/departure/arrival. Operational patch identity must
   handle depletion, drift, splits and merging without inventing persistent
   geographical knowledge in the agents. Dropped inventory is actual food but
   not by itself proof of a new vegetation source.
4. **Major-relocation coverage.** Register the real geography timeline, not a
   guessed50k period: epoch length8192; major hub change every third epoch; first
   major blend16,384–24,576, then40,960–49,152, and so on. Attach explicit
   old/new habitat evidence and event times to each qualifying journey. A
   journey must not be counted against multiple relocations. Report each
   relocation separately, including zero evidence, and distinguish temporal
   association from proof that the relocation caused the decision to depart.
   No same-individual lifespan requirement; no new same-family-across-all-cycles
   criterion beyond the user's contract. Require actual descendant identities.
5. **Force-assisted travel is not automatically invalid travel.** The current
   development observer excludes observed shoves during a poor corridor. That
   is a classifier choice, not a rule in the completion contract. The successor
   must retain and label voluntary versus externally displaced movement so
   successful organically force-assisted sequences are not silently discarded.
   This is not evidence that cooperation exists: a shove followed by survival
   is insufficient to establish mutual benefit, intent or causal assistance.

## Implementation and verification boundaries

- Observer buffers are optional and separate. They must never be bound as inputs
  to perception, decision, movement, interaction, reproduction or founder export.
  Global patch/geography knowledge stays entirely in observation/reporting.
- Bound event storage and detect overflow. Silent truncation cannot pass an
  integrity check. Preserve failures and partial evidence. Prefer conservative
  unknown classifications to fabricated route continuity.
- Test identity reuse, newborn/dead slots, same-tick feeding, intervening feeding
  and shoves, births away from destination, dropped food versus vegetation,
  source depletion, patch drift/split/merge and relocation-boundary attribution.
- Demonstrate observer write isolation with headless buffer/accounting checks.
  Observer synchronization can alter nondeterministic resource competition;
  do not promise identical whole-population histories merely because buffers
  are read-only. Both comparison arms must use the same observation protocol.
- Validate on development seeds first. This is instrumentation development, not
  permission to choose permissive thresholds until six worlds pass. Freeze
  final definitions and artifacts before drawing fresh evaluation seeds.

The next design task is a bounded per-tick observation/event representation and
an explicit operational patch/relocation definition. Those details are not yet
implemented or registered here; this file is not evidence of completed migration.
