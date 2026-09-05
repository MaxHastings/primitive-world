# Cumulative inherited preparation — registered development learning curve

## Question and fixed model

Does more cross-world inherited preparation improve200k survival and sampled
migration under the SAME physiology-v2 body, compared with its frozen unprepared
bank? This is a development experiment, not final eight-seed validation. Failure
does not prove this controller can never evolve useful behavior; success does
not prove general intelligence or satisfy the broad goal.

Use the completed physiology-development-20260904 frozen executable and bank:

- exe7a2729ddbd68ccdad1a94d67b10e80ae2a93ce779044059bbb27c55aa6ccc4e5
- baseline34c32e136ed80d34845ce9a7cf298ccf7f848eb67ee6e67115c002b3f8750b65

No body, controller architecture, mutation, sensing, ecology, action masks or
initialization change. Each world starts with1000 bodies at seeded random
positions,65 energy,2 carried food,age0–300 and empty hidden state. Weights cycle
through its assigned bank without additional noise. Costs0.06/0.01, motor gain4,
regeneration0.01, evolving geography, force and signals enabled. No intervention.

## Preparation chain and authored selection

Run16 sequential training episodes, each65,536 ticks or extinction, using seeds:
11,22,303,404,505,606,707,1,1101,1102,1103,1104,1105,1106,1107,1108.
All are training/development seeds from registration onward, never final holdout.
Each complete65,536-tick episode includes two completed major geography renewal
transitions, plus intervening drift/weather. The full preparation budget is
1,048,576 simulated ticks, if the chain survives every episode.

Start episode1 from the frozen unprepared bank. At each endpoint, the existing
export samples up to128 living descendant bodies by deterministic lineage-hash
order. This samples at body abundance: larger surviving families are represented
more often. Pass ONLY that exported bank into the next fresh world. Do not pool
old elites, rank routes, add random genes, restore a failed population or retain
a previous bank after extinction. If no living descendants can be exported,
stop the chain as a recorded preparation failure; do not pretend its budget was
completed or evaluate the previous bank under a larger budget label.

The reset and sampling are authored offline selection rules. New worlds supply
initial reserves; this is not a continuous natural ecosystem or hidden live-world
rescue. Preparation favors descendants alive at a fixed endpoint and can select
shorter-term strategies or spatial luck. That is WHY separate evaluation is
required. More elapsed preparation does not guarantee a monotonically better bank.

Ordinary birth mutation remains independent~2% per weight, perturbation[-0.03,
0.03], clipping[-4,4]. There is no within-life weight update, gradient loss,
teacher or movement reward. Histories/state are not carried across worlds.

## Development evaluation and checkpoint budgets

Preserve every exported bank and report. After episodes4,8 and16, run frozen-bank
evaluations on already-used development seeds808,909,1001, each200,000 ticks or
extinction. Orders: budget4:909,1001,808; budget8:1001,808,909;
budget16:808,909,1001. Evaluation never exports or supplies genes back to training.
Training episode5 uses the episode4 bank, not any evaluation descendants.

Budget0 baseline is the four completed registered development runs (808 repeated).
After the chain and budget16 evaluation, repeat baseline808,909,1001 with the
same executable and same settings. Preserve original and repeated baselines;
do not replace unfavorable outcomes. Together these expose baseline variability.
No checkpoint is selected or promoted mid-campaign based on evaluation results.

Metrics every1024 ticks. Evaluation journey samples every32, exactly the existing
schema2 definitions/thresholds. Training does not collect journey evidence or use
observer scores. Every world records accounting, action/birth gates, energy and
terminal living ancestry/weight statistics. Record bank mean/variance/unique
genomes: parameter movement is NOT a learning metric. Ancestry depth resets on
fresh initialization; do not sum unrelated per-world maxima and call that an
actual lineage's total generations. Report cumulative ticks and per-world depths.

Report survival count, capped survival times, population/cap exposure and complete
journeys at all reached budgets, including regressions. Existing journey evidence
is sampled and not yet attributed to major renewal cycles; this experiment
cannot pass the final migration gate. Development seeds are already observed
and reused, so cannot supply an unbiased final generalization estimate.

## Failure and integrity rules

Freeze executable, baseline, runner and plan hashes before episode1. Verify loaded
float32 genomes, physical parameters and no intervention for every report.
Population accounting must balance at every sample; numerical/observer invalids
are failures, not retry invitations. Preserve logs, command lines, source-bank
hashes, all outputs, failed results and cumulative budget actually consumed.
An extinct preparation episode stops the chain; a dead evaluation does not.
No arbitrary success gate promotes this model or bank. GOAL_CONTRACT is unchanged.
