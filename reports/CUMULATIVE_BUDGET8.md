# Cumulative preparation: eight-world development checkpoint

This checkpoint completed after524,288 cumulative training ticks. It is a partial
result within the registered16-world campaign, not promotion or final validation.
The executable, body, world settings, mutation and initialization are unchanged.

| Preparation worlds | Surviving development seeds | Complete sampled journeys |
| --- | --- | --- |
|0 (original baseline)|1/3|0|
|4|1/3|3|
|8|2/3|10|

Each preparation snapshot was evaluated on808,909,1001 for200k ticks or extinction.
The baseline's extra808 repeat also went extinct with zero journeys. Additional
baseline repeats are still due at the end of the campaign; this is one prepared
evaluation per seed/budget, not a repeatable-effect or significance claim.

| Budget8 seed | End tick | Living | Complete sampled journeys |
| --- | --- | --- | --- |
|808|98,304|0|0|
|909|200,000|1,553|3|
|1001|200,000|178|7|

All three budget8 evaluations passed the runner's per-sample population accounting
and loaded-genome/physical-setting checks, with no invalid outputs or95%-capacity
samples. These checks do not establish the full goal.

## What the evidence supports

The previously failing1001 environment now has178 living descendants at200k.
Its seven complete sampled journeys occur in two time clusters: departures
25,568–28,000 followed by destination-side births26,560–30,592; and departures
102,112–104,096 followed by births103,552–105,568. These are not three separate
renewal-attributed events and cannot satisfy the final relocation requirement.
The other surviving world,909, has three complete sampled journeys. Seed808
still dies at98,304 despite the extra preparation.

The result warrants completing cumulative preparation before concluding that
the controller needs another redesign. It does not prove that more preparation
must keep helping, that survival is caused by these particular journeys, or that
the agents have acquired foresight, social organization or general intelligence.

A post-hoc reset-state float64 controller assay also changed across budgets:
at energy100/inventory0.5/uniform food0.02, unstocked reproduce requests were
123/128 at baseline,29/128 after4 worlds, and0/128 after8. All128 budget8 genomes
chose collection in that condition and in the hungry empty-inventory/food0.2
condition. Predicted mean adult speed at energy50/inventory0/empty space changed
0.0641→0.0764→0.1385. This is a surrogate input assay, not a complete life or
guaranteed bitwise GPU replay; it never enters selection or campaign continuation.

## Provenance

Frozen bank-after8 SHA256:
47661fda0c639adc62af5e7c28fade7bfa3c16ed9edd1bc9ae55a6d68a444704.

All raw reports, trajectories, commands, logs, banks and registration are retained
in reports/cumulative-preparation-20260904 in the physiology research worktree.
The source registration fixes budgets, orders, selection rules and failure rules.
No evaluation descendants return to training. No final fresh holdout seeds
have been used, and the broad GOAL_CONTRACT remains unfinished.

Evaluation report / trajectory SHA256 values:

- Seed808: 3fc7ebf901bed8339be505c484489bef3d8bf7095b1a3d3a3617bca10da55158 / d660c3fb7e2a1d6cd91c7180d214c013c5e4f0a3c11491d8e24ff42e2ff263bd
- Seed909: 556659e97fba6bd1267cb45aac87f90b0ca57662cde297afd68b8c375fd7f96a / 6318e59203744d03a723f6cf43cca40e594c845ca4a3e9d06dea0cbe4825e96a
- Seed1001: c07784051b1765d5fb9ec7a539b59b180c1da23cc4d43a31a5675eb53cdb4aaf / 9f2a5411092cdc8b93560083f6642978d49e10ecb16e7cd8bc970e3d8c2db70a
