# Cumulative preparation: first completed development checkpoint

Partial campaign result, not promotion or final validation. The registered
16-world chain continues unchanged; this document describes only the completed
four-world preparation checkpoint and its three evaluation worlds.

Prepared262,144 ticks through seeds11,22,303,404, each65,536 ticks. Each world
loaded the prior world's sampled living-descendant bank; no test descendants
returned to training. All four exported128 genomes, with clean sampled
population accounting and no invalids or95%-capacity samples. Frozen body,
executable, mutation and initialization match the original unprepared baseline.

| Evaluation seed | Original baseline end / living | Budget4 end / living | Baseline complete journeys | Budget4 complete journeys |
| --- | --- | --- | --- | --- |
|808|98,304 /0 (both repeats)|98,304 /0|0|0|
|909|200,000 /1,181|200,000 /1,260|0|2|
|1001|96,256 /0|97,280 /0|0|0|

Survival remains1/3 distinct seeds: no additional surviving world. A1,024-tick
extinction-sample difference on1001 is not evidence of a reliable gain. Baseline
replication at the end of the full chain is still required by its development
plan. No invalids, accounting discrepancies or cap exposure in these evaluations.

## Inspecting the two complete sampled sequences

These are raw-record inspections, not just aggregate counters. Both came from
budget4/seed909, and both involved bodies born during the evaluation world.

| Lineage | Born | Depleted departure | Destination collection | Later ingestion | Birth interval |
| --- | --- | --- | --- | --- | --- |
|23052|24,227|27,712|29,184|29,216|29,280–29,312|
|23988|27,908|31,776|33,984|34,016|34,240–34,272|

The recorded origins had fallen from peak footprint vegetation0.0800 to0.01933,
and0.07633 to0.01667, respectively. Their poor-corridor records show sampled
zero collection/ingestion and net crossing>=48 units. Both later collected at
a separate location>=96 units from the original collection position. Birth
counters subsequently rose3→4 and1→2 near the recorded destinations. The second
body also reproduced once earlier along its longer post-departure history; that
earlier birth was not substituted for the required destination-side birth.

These are meaningful sampled behavior records, but not proof of foresight,
offspring survival or a reliable inheritance advantage. Both occur in the same
early portion of the world, not across three separate major renewals. Missing
intervening ticks and footprint-based patch definitions remain disclosed limits.

## Post-hoc reset-state choice assay

An offline float64 surrogate of the declared controller inspected identical
counterfactual inputs with hidden state/neighbor/previous-feedback values zero.
This is not guaranteed bitwise GPU replay, a complete life, fitness or a new
selection signal. No result affects campaign continuation or founder choice.

At energy100, inventory0.5 and uniform food0.02, the baseline requested
reproduction in123/128 genomes despite stock below1; budget4 did so in29/128.
Collect choices rose5→99. At energy10, empty inventory and food0.2, both banks
chose collect in128/128 cases. At energy50 in empty space, predicted mean adult
speed changed0.0641→0.0764 against a maximum1.2. No near-tied action margins
under1e-5 occurred in these reported conditions.

The inherited parameters are changing behavior; that alone does not prove
learning to survive. Longer matched tests, later budgets and independent final
validation are still necessary. Parameter drift is not a loss curve.

## Artifacts

Research directory: reports/cumulative-preparation-20260904. All command lines,
reports, banks, logs and evaluation JSONL remain local and preserved.
Bank-after4 SHA256:
56e1ed75e9216d88bbcc926e294a3224498e4b59f6515501bc605eec74bd1da4.
Seed909 report SHA256:
717af7bea55a3f0d21a3ba4238550078d939b1d17891ce3f17338859e1d7ebb6.
Seed909 journey JSONL SHA256:
9e246536df57039ce6cc8c9f54b16544ebfa08c7c191451879f52faa7f8edc09.
Choice assay: choice-assay-budget0-4.json; reproducible with
experiments/inspect_bank_choices.py. Four offline integrity/assay tests pass;
they are not substitutes for behavioral evidence.
