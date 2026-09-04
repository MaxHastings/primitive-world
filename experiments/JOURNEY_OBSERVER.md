# Sampled journey evidence, schema1

This optional headless instrument observes physical state. It does not feed
anything into controllers, change weights, reward behavior or classify agents
for export. It is a development instrument, not yet sufficient to pass the final
goal's major-relocation criterion.

Example from the repository root (use new output paths):

```powershell
cargo run --release -- --headless --seed 808 --ticks 200000 --sample 1024 --journeys reports/journeys-808.jsonl --journey-sample 32 --output reports/journeys-808.json
```

JSONL starts with a definition header, then complete journey records with sampled
waypoints, then a summary footer. The ordinary report also contains summary
counts. Files are created exclusively; existing files are not overwritten.
Stage counts expose where sampled sequences stop. These count track milestones,
not unique bodies or unbiased transition probabilities; re-anchoring can count
the same body again. Maximum poor-space net distance covers the active poor
segments until they first cross the48-unit threshold, not lifetime travel.
An interrupted file without a footer is not a completed observation campaign.

## Exact development definition

The observer reads the same living body's lineage/incarnation/birth tick across
samples. Its fixed food footprint is the mean vegetation at the center plus
eight ring points at radius24. No dropped supplies enter that footprint; actual
collection feedback can nevertheless include dropped supplies.

1. Actual collection establishes an origin in a footprint with food >=0.04.
2. Food at that origin subsequently falls to <=25% of its observed peak and
   <=0.02; the body is at least48 units away.
3. Consecutive sampled footprints <=0.01, with zero last-tick collection and no
   last-tick forced displacement, span at least48 net units.
4. Actual collection occurs in another >=0.04 footprint at least96 units from
   the origin. Continuous foraging without depletion/crossing does not qualify.
5. A later sample records ingestion, then a later sample records an actual birth
   counter increase. Sampled positions stay within48 of destination during that
   collection/ingestion/birth sequence. Birth time is an interval, not guessed.

Tracks reset on missing/dead identity, changed birth tick, decreasing birth
counters, completion, or512 retained points. Continuous foraging before departure
can re-anchor the origin. Memory is bounded by live tracks, not total run length.
Records are emitted once and streamed to disk. No failed track is called a
successful journey; truncation and lost-track counts are reported.

## What this cannot establish

Sampling32 ticks sees only the most recent tick's ingestion/collection feedback.
It misses some births/deaths, feeding and path details. Consecutive poor samples
are not a proof that every intervening tick was food-poor or force-free. A
footprint is a local spatial definition, not a global connected food-region ID.
Collection then ingestion does not identify individual food molecules. A birth
does not establish offspring survival. An event is not evidence of foresight.

Most importantly, these records do **not yet attribute departure to a major
geography relocation**. That requires matching origin/destination evidence with
the evolving terrain and its renewal windows. Do not count ordinary local
depletion episodes as three major relocations to pass the goal. Freeze final
definitions before final validation; these development thresholds are explicit,
but not yet the final acceptance protocol.

Default execution does not read these extra buffers. Enabling observation adds
CPU/GPU readback overhead and may split tick batches at sample boundaries. The
integrity test compares an isolated body's complete GPU state with and without
readback/splitting. Parallel resource competition can still vary between world
runs; the observer does not promise bitwise ecosystem reproducibility.
