# Research status — September 4, 2026

Main is still recurrent-v1, build0.2.0, checkpoint12, with the released v1 bank.
The broad GOAL_CONTRACT remains active and unachieved. A playable build and
passing wiring/physics tests are not proof of evolved adaptation or user fun.

## Completed evidence

The v1 audits reproduced a mismatch between rotating food sensors and the
authored compass-following prior, and a hungry empty-inventory eating choice.
Observed unsuccessful crossings often went straight but slowly, with limited
reserves. See [the departure audit](DEPARTURE_ATTRITION.md) and
[the sensor-frame audit](FOUNDATION_FRAME_AUDIT.md). These findings
do not establish that more memory or a scripted migration policy is necessary.

An isolated alternative body/controller lives on branch `codex/physiology-v2`,
source65e18b2; documentation/results and training registration at a14a678.
It fixes compass probes, removes attention and the ingest action, automatically
digests carried food at a bounded rate, and starts with a frozen UNPREPARED v2
bank. Gathering and reproduction remain chosen. Original energy/movement costs,
maximum speed, mutation and ecology remain unchanged. This changes the task,
not just a numerical implementation detail; v1 banks/checkpoints are incompatible.

Its registered initial development batch completed:

| Seed | Repeat | End tick | Living | Complete sampled migration/feeding/birth sequences |
| --- | --- | --- | --- | --- |
|808|1|98,304|0|0|
|909|1|200,000|1,181|0|
|1001|1|96,256|0|0|
|808|2|98,304|0|0|

No invalid outputs, population accounting discrepancies or95%-capacity samples.
These are three distinct development seeds, not four independent environments.
No causal improvement over v1 or prepared-versus-unprepared gain was demonstrated.
The alternative has NOT been promoted to main.

The frozen research executable SHA256 is
7a2729ddbd68ccdad1a94d67b10e80ae2a93ce779044059bbb27c55aa6ccc4e5;
the unprepared bank SHA256 is
34c32e136ed80d34845ce9a7cf298ccf7f848eb67ee6e67115c002b3f8750b65.
Full results and raw evidence remain in the separate physiology worktree under
reports/physiology-development-20260904; the research branch versions its summary
and registration. The main launch path and user artifacts are unchanged.

## Cumulative preparation is a separate experiment

A200k-tick world is not200k weight updates. Weights remain fixed within a life;
offspring inherit small mutations. In the surviving v2 world, maximum terminal
living ancestry was67 parent-child links. An extinct report's zero terminal
ancestry is not a count of all generations that occurred.

Independent worlds starting from the same unprepared bank do not accumulate
cross-world training. The registered next campaign, in the research branch's
experiments/CUMULATIVE_PREPARATION_PLAN.md, carries endpoint descendant banks
through up to16 training worlds of65,536 ticks. Candidate snapshots after4,8,16
episodes are tested on separate development environments without returning test
descendants to training. The body/executable/baseline remain fixed. A failed
training population is recorded, not secretly restored from an older bank.

Completed development checkpoint results now show survival1/3 at budget0,
1/3 after4 preparation worlds,2/3 after8,and3/3 after16. Complete sampled journeys total
0,3,10,22 respectively. One of the22 budget16 records belongs to a founder, not
a descendant; some precede the first major relocation. Neither aggregate
journey count nor survival can pass the relocation-evidence gate. This is preliminary development evidence, not final
validation or proof of a repeatable/generalizable gain. Reports are recorded on
the research branch in `reports/CUMULATIVE_BUDGET4.md`, `CUMULATIVE_BUDGET8.md`
and `CUMULATIVE_BUDGET16.md`.
All16 training worlds have since completed (1,048,576 preparation ticks); the
budget16 comparisons completed, while end-of-campaign baseline repeats are still in progress.
Do not equate a newer exported bank with a better tested bank.

The user reports strong leftward movement followed by collapse in play. A
read-only CPU reset-state probe finds a condition-dependent leftward bias in
the latest bank (especially when carrying food), but has not established the
cause of the live extinction. GPU and reversed-cue checks are queued after the
frozen campaign. Keep this limitation visible alongside the successful runs;
do not prescribe rightward movement or adjust weights to hide it.

An [independent foundation review](INDEPENDENT_FOUNDATION_REVIEW.md) recommends
finishing this experiment before another architecture change, fixing stale
inspection/persistence usability, and closing relocation-evidence gaps.
Final fresh eight-seed validation has not begun.
No amount of parameter drift, elapsed effort or one successful seed can replace
the broad survival, migration, integrity and user-acceptance requirements.
