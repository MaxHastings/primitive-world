# Return to random-origin populations

The user chose to stop relying on authored survival starting weights after the
communication audit of world-9042701 at tick 256353. Recommended fresh starts and
new training use random genomes. The V3 runtime and trainer already default to
random banks; the previously recommended optional starter was the exception.

## Evidence, not a promise

The checkpoint audit in communication-audit-9042701-tick256353/checkpoint-audit.json
found zero historical emissions, transfers and force actions. A sufficient bound
on output-score differences proves these three actions cannot beat a rival for
any hidden state in all 256 starter genomes and all 139 living survivor genomes.
Mutable genes did not guarantee accessible behaviors. This is a defect of this
initializer under highest-score selection, not proof that every possible form of
bootstrapping prevents emergence.

The random-origin campaign v3-feeding-100-20260904 provides a useful contrast.
Its benchmark-r0-0 selected all six actions and recorded 154430 emissions, but
went extinct at tick 1920. At this review it had completed 26 rounds; both round25
benchmarks still went extinct (ticks 2048 and 2336). Action variety is present;
reliable survival and useful communication have not been established.

## Keep the questions separate

- Random initialization varies inherited controllers without a feeding template.
- Existing action selection always takes the highest score. An individual is not
  guaranteed to try every action. Population-wide variety is not lifetime exploration.
- Working state changes during life; weights do not. Mutation and offline family
  selection change future populations, not the current body's weights.
- Probabilistic action sampling would be a separate model change, not an effect
  of removing the starter. No such change was made in this decision.
- Starting conditions can permit partial survival without programming what action
  succeeds. The existing adaptive curriculum and family selection remain unchanged.
- More pressure is not automatically more learning; early extinction can remove
  the opportunities needed for reproduction and selection of later competence.

## Preservation and scope

No checkpoint, exported user bank, frozen executable or active campaign was
modified or deleted. The authored initializer remains historical reproducibility
code, not a recommended route. Existing --founders and --initial-bank support
remain available for explicitly chosen evolved or user-played populations.
No new duplicate campaign was launched, no default was promoted, and main was
not updated. The active random-origin campaign continues its registered protocol.

## Ongoing feedback loop requested by the user

The user subsequently requested continued selection/mutation rounds, carrying
stronger genetic families into later worlds instead of resetting the search.
At the final process check, the random-origin campaign was running with 27 rounds
complete. Its current selection retains ORIGINAL tested candidate weights based
on descendant-family outcomes; it does not export literal endpoint survivors.

The existing Review V3 survival campaigns heartbeat was updated to prioritize
this line. After a completed, verified campaign, it may start one sequential
100-round continuation from that campaign's retained candidate bank, using fresh
registered seeds, unchanged biology and the existing selection/curriculum. Before
launching it must append provenance, parent hash, directory and command here.
It must not create parallel duplicates, extend the authored-starter line, promote
defaults, or silently change action selection. Continued search is not a promise
of improvement. No continuation has been launched yet.

The user subsequently approved stopping the authored-starter campaign. It was
stopped after 16 completed rounds, during train-r16-i2-t0. Its STOPPED_BY_USER.md
records the intentional stop; the untouched summary may still say running.
All artifacts remain preserved. The heartbeat prohibits resuming this campaign;
only the random-origin line continued at that point.

## Family-scoring campaign subsequently stopped

The user asked to inspect progress and end this campaign if it was not promising.
On 2026-09-04 at approximately 23:26 America/Chicago it was stopped after 40
completed rounds, during train-r40-i2-t2. All 18 fixed benchmark runs had gone
extinct; round40 mean extinction time was 2160 ticks, below round15's 2640.
Food collection and juvenile maturation improved, but ancestry remained depth2
and all curriculum levels remained zero. This justified an early stop under the
user's criterion, not proof of permanent failure. The detailed evidence and
preservation notice are in v3-feeding-100-20260904/STOPPED_BY_USER.md.

Both campaigns are now intentionally stopped. Their artifacts remain intact.
The Review V3 survival campaigns heartbeat is PAUSED, with automatic continuation
authorization removed. Do not resume or create successor campaigns without a
new user request. No alternative training method has been implemented/launched.

## Actual-survivor loop authorized and implemented — 2026-09-05

The user's subsequent "lets do it" authorized the actual-survivor feedback loop,
not restarting either retired campaign. The new bounded pilot is under
`v3-survivor-loop-20260905`; its [protocol](../training/SURVIVOR_LOOP_PLAN.md)
freezes four independent random-origin lines and eight transfers per line.

An optional read-only observer captures current living-body genomes every128ticks
and retains the newest nonempty sample after extinction. Both founders and actual
descendants are eligible; no tested original ancestor is substituted for a child.
Every sampled genome is carried exactly, then additional externally mutated copies
fill the next bank. Source body metadata and exact f32 genome hashes document the
cross-world chain. Normal-world physics and the argmax controller are unchanged.

Separate evaluation worlds at rounds0/4/8 never supply selection genes. The pilot
stops after completion, with no default promotion or automatic next campaign.
The old heartbeat remains paused and the two retired campaigns remain stopped.
This is explicitly survival-selected serial transfer, with rejuvenation and possible
terminal bottlenecks; it is not literal untouched evolution or proven intelligence.

The pilot subsequently completed all32 training and24 evaluation worlds. The
[results](SURVIVOR_LOOP_RESULTS.md) show increased mean observed survival but
declining births and severe bottlenecks. Frozen completed resume reproduced its
summary exactly without rerunning worlds. It is stopped and unpromoted; no
automatic continuation was launched.

## User-authorized visible extinction-only continuation — 2026-09-05

The user next requested watching an ongoing survivor loop with no premature tick
termination. Implemented `training/watch_survivors.py` and the viewer's
`--watch-output NEW_DIRECTORY` mode. Each world runs until verified extinction;
only then are the actual late-survivor genomes copied/mutated into a fresh world.
There is no round limit, tick cutoff, automatic difficulty ramp or model change.
Closing the window stops the outer loop and saves a full paused checkpoint.

Launched `v3-visible-survivors-20260905` visibly from the frozen pilot's
`round8-line0.bank.json` (best mean measured survival across the two evaluation
seeds, not proven reproductive competence). Input SHA256:
`1ac8d6b9151f67c8daa637c931369001cd47640db0868397cfebb0a90157a5c1`.
The first world seed is3866876028. Its ready marker confirms no tick limit; the
window was detected after launch. This is new interactive play, not reopening an
old campaign or claiming exact continuation of a pilot world's bodies/memories.
Runtime/source/scripts are archived in the new directory. The old heartbeat
remains paused; the visible runner itself handles extinction and transfer.

Verification: the preceding60-test Rust suite passed; two additional targeted
tests then checked no automatic cutoff through tick1000001, extinction-only
export, safe manual close/checkpoint load, and incompatible CLI rejection.
All26 Python tests passed, including close-means-stop, living-world refusal and
physical-control carryover. Clippy and formatting passed. No old saves deleted,
no other user simulation stopped, and nothing promoted/merged to main.

First visible-world handoff correction: world000001 naturally went extinct at
tick11056, saving one actual descendant. The Python launcher then failed because
Rust omits default environment_rotation=0 from serialized settings. No agent/save
data was lost; `round1.bank.json` had already been created. Fixed the absent-default
handling and added the regression case. Preserved the failed directory unchanged.
Reopened visibly as `v3-visible-survivors-20260905-continued` from that exact
round1 bank (seed sequence9054002), retaining the unchanged physical defaults.
The responsive new window was verified, with first seed183890187 and no tick cap.
Its launcher handles subsequent extinction-only transfers; closing stops it.

## Smooth, single-window continuation — 2026-09-05

The user objected to resetting speed on every app reopening. Replaced the
recommended launcher with one native `--watch-loop` viewer: extinction archives
the actual survivors, replicates/mutates their genomes and resets the Simulation
in place. Window, renderer, speed, camera, zoom and lens remain untouched. Only
world-local selections/counters/history reset. No tick/round limit or difficulty
ramp. The native variation uses a documented splitmix64 PRNG with the same .02
probability, ±.03 range, exact retained copies and balanced mutated replicas.

The old continued viewer had been closed by the user in world25 at tick1198 and
saved its checkpoint. With explicit approval "Reopen at MAX", launched the native
build from that EXACT checkpoint, not a fresh world/bank. Runtime is archived in
`v3-native-visible-runtime-20260905`; live output is `v3-native-visible-20260905`.
Initial ready marker confirms seed1990899984 and initial_tick1198. The same native
process27308/window handle2558672 was verified on worlds3,15 and17. It started
at MAX; the user subsequently selected16x, which persisted through worlds15–17.
No window was closed or process killed by the assistant during this upgrade.

Verification:63 Rust tests passed,1 opt-in test ignored;26 Python tests passed;
Clippy, formatting and whitespace checks passed. The GPU transition test verifies
two extinctions carry actual genes, preserve changed physical settings and keep
the viewer loop alive. The Python launcher now launches once, with checkpoint
support and no external generation/reopening loop. Old archived scripts remain
untouched. The UI runs until extinction, user pause/close, or an explicit error;
errors pause rather than silently replace a living world. Still unpromoted to main.
