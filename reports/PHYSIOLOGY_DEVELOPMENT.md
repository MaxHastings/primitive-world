# Physiology-v2 development results — not promoted

All four registered runs completed. Main remains recurrent-v1; this is an
unprepared alternative body/controller, not evidence of successful pretraining.

| Development seed | Repeat | End tick | Living | Births | Peak population | Poor corridors | Destination collection / ingestion | Complete journeys |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
|808|1|98,304|0|17,019|2,221|96|3 / 3|0|
|909|1|200,000|1,181|37,604|2,054|205|1 / 1|0|
|1001|1|96,256|0|13,376|1,919|42|0 / 0|0|
|808|2|98,304|0|17,042|2,219|92|0 / 0|0|

All sampled population accounting balanced; no invalid outputs, observer faults,
track truncations or95%-capacity samples. Executed492,864 ticks. The808
replicates share an extinction sample but not identical histories or feeding
events. These runs do not establish population-wide deterministic replay.

The surviving909 world has maximum living ancestry depth67 at200k. This is the
deepest surviving parent-child chain within that world, not67 controlled
optimization generations. The report field `max_ancestry` is terminal living
ancestry; zero after extinction does NOT mean no generations occurred.

In808 repeat1, the96 ended poor-crossing attempts had median departure energy
42.65, no inventory, path speed0.1065, path/net efficiency0.9942, and nearest
qualifying food distance128.88. All96 nearest distances were inside optimistic
maximum-speed energy range;77 were beyond the illustrative observed-speed
range. These are sampled selected attempts and changing destinations, not proof
that every body could find food, nor a causal attribution to digestion.

An additional unresolved controller issue is visible: of5,044,351 post-movement
reproduction requests in808 repeat1,5,020,465 failed the inventory gate; only
17,019 births occurred. Failure reasons can overlap. This is evidence to inspect
the authored starting policy and its evolutionary change, not permission to
mask intentions, gift reserves or add automatic reproduction.

## Interpretation and next question

Fixed sensors and automatic digestion pass direct capability/conservation tests
but do not deliver reliable migration by themselves. V1-to-v2 comparisons mix
body changes and different starting banks, so cannot measure an intelligence
gain. V2 has not been promoted. The next experiment must hold this executable
and body fixed and compare accumulating inherited preparation against this exact
unprepared bank. A longer fresh world is not cumulative cross-world training.

## Reproducibility

Source65e18b290bce58b9326142ef87e2d98723d363f8; frozen artifacts under
`reports/physiology-development-20260904/` in the physiology research worktree.
Full reports, JSONL trajectories, commands, logs, initial bank and registration
are preserved locally. The registration and summary are versioned separately.

- Executable SHA256:7a2729ddbd68ccdad1a94d67b10e80ae2a93ce779044059bbb27c55aa6ccc4e5
- Unprepared bank SHA256:34c32e136ed80d34845ce9a7cf298ccf7f848eb67ee6e67115c002b3f8750b65
-39 release tests passed; release Clippy with warnings denied and formatting
  passed before freezing. Tests do not establish ecological adaptation.

No final fresh holdout seeds were consumed. The broad GOAL_CONTRACT remains
unfinished; research uncertainty and this partial result are not completion.
