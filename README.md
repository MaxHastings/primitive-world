# Primitive World — primitive-v3 research build

**Experimental0.4.0-dev · checkpoint15 · untrained random founders.**
This worktree is not promoted to main. The [completion contract](GOAL_CONTRACT.md)
is unachieved. A cleaner architecture is not evidence of competent agents.

Bodies have local senses,16 private recurrent values, six action choices and
independent continuous movement. Their1760 inherited weights stay fixed during
life; offspring receive mutations, not their parent's memories. No destination
planner, authored feeding policy, migration reward or hidden population rescue.

## Play this source

From this folder, build into a separate directory so an older running executable
is left alone:

```powershell
cargo run --release --target-dir target/v3-play -- --seed 909
```

Rust and a supported GPU are required; no Python or model server is needed to
play. Check the window's model identity: primitive-v3 / checkpoint15. Space
pauses, WASD/arrows pan, the wheel zooms, Home fits the world, and clicking a body
opens its live inspector. Death/slot reuse ends tracking with a labelled snapshot.

Fresh worlds use256 deterministic random genomes with no feeding/navigation
template. Random weights are not random action sampling: a body may repeat an
ineffective action until it dies. Collecting ground food is chosen; digestion of
carried food is automatic. Walking through food does not automatically collect it.

A compatible v3 bank can be supplied with `--founders PATH`. Newer does not mean
better: no v3 bank has yet qualified for promotion. V1/v2 banks and checkpoints
are rejected, not silently converted. Your older saves and frozen executables
remain intact; the historical v2 bank-after16.json needs its v2 executable.

Use fresh random weights, without the authored survival starter:

```powershell
cargo run --release --target-dir target/v3-play -- --random-founders --seed 9042701
```

The authored starter is retired from the recommended path. Its saved weights
mathematically prevented transfer, force and emission from winning action selection
in all 256 original genomes and all 139 survivors of the inspected user save.
Mutable did not mean behaviorally accessible. Historical banks, saves and frozen
campaigns are preserved; see [the decision record](reports/RANDOM_ORIGIN_DECISION.md).
Random-origin training already uses random weights when --initial-bank is omitted.
This does not change highest-score action selection into random action sampling.

Fresh defaults:1000 bodies;16384 slots;2048-unit world; metabolism.06;
movement cost.01 per unit; regeneration.01; adult maximum speed1.2; motor gain4.
Founders start with65 energy and2 food. Births cost parent energy and create
offspring with0..40 energy, no carried food, and maturity at400ticks.

Save new checkpoint preserves the world, genes, internal states and settings to
a new file under reports/checkpoints. Load checkpoint restores the named path
paused. Export living descendants saves weights for a fresh world, not settings
or experience. Export history currently writes recurrent-history.json, replacing
the previous history export, and contains only the latest400 metric samples.
Preserve a copy before exporting again if you need earlier history.

## Headless simulation and experiments

To **watch an extinction-only survivor loop**, use a compatible starting bank and
a new directory. There is no tick or round limit; each extinction saves the actual
late survivors and starts a fresh world **inside the same window**, with their
genes plus mutated replicas. Speed, camera position, zoom, lens and physical
settings are retained. The viewer is not relaunched:

```powershell
cargo build --release --target-dir target/feeding-audit
python training/watch_survivors.py --initial-bank reports/v3-survivor-loop-20260905/round8-line0.bank.json --directory reports/my-visible-loop
```

The example bank is a local research artifact, not shipped or validated as a
self-sustaining species. The visible loop starts a fresh world from these genes,
not an exact continuation of an earlier world's bodies or memories. Pause/speed
controls work normally; this launcher starts at MAX unless `--view-speed` specifies
another speed. Closing the window **stops** the loop and saves the current
world to its `world-NNNNNN/paused.checkpoint`; it does not seed another world.
Reset and loading a different world are disabled while this mode is active.
The next world's physical controls inherit your final settings. This is
interactive play, not a controlled benchmark. Crash/failed save stops the runner
with evidence intact rather than silently restarting. Each `world-NNNNNN` folder
contains its ready marker, endpoint report, survivor bank and transfer provenance.

Resume the exact saved world with the smooth loop using:

```powershell
python training/watch_survivors.py --checkpoint PATH --directory reports/my-resumed-loop --view-speed MAX
```

Python is optional: the equivalent viewer options are `--checkpoint PATH
--watch-loop NEW_DIRECTORY --view-speed MAX` (or `--founders PATH --seed N` for
a fresh world from genes). In-place transfers are native Rust. They retain every
sampled genome exactly and make balanced mutated replicas at .02 probability per
weight and ±.03 range. The versioned native PRNG differs from the historical Python
campaign, not the mutation pressure or brain/physics rules. The old `--watch-output`
single-world exit mode remains only for archived launcher compatibility; do not use
it for smooth continuous viewing.

The active training path is [actual-survivor serial transfer](training/SURVIVOR_LOOP_PLAN.md):
random origin, capture late living bodies, carry their **current** genomes into the
next world, repeat. Descendant mutations are retained; no original-founder family
ranking. Frozen separate evaluation worlds track progress but never seed training.
This is external selection with body-state resets, not within-life weight learning.

```powershell
cargo build --release --target-dir target/feeding-audit
python training/survivor_loop.py --directory reports/my-survivor-loop
```

The default bounded pilot is four independent lines and eight transfers, with
8192-tick/extinction limits and unchanged normal-world physics. Inspect the run's
`summary.json` for separate training/evaluation results. No bank is automatically
promoted. The family-scoring trainer below is historical, not this loop.

The [first survivor-loop pilot](reports/SURVIVOR_LOOP_RESULTS.md) is complete:
mean observed evaluation survival rose from2400 to4680ticks, but births declined
and the two8192-tick survivors were lone founders. Useful survival improvement,
not a demonstrated self-sustaining species. That bounded pilot is stopped;
later interactive continuations are separate runs.

```powershell
cargo build --release --target-dir target/v3-headless
.\target\v3-headless\release\primitive_world.exe --headless --seed 808 --ticks 200000 --sample 1024 --output reports/new-world.json
```

Use a new output path/directory. The earlier
[founding-ecology experiment](training/FOUNDING_ECOLOGY_PLAN.md) compares eight
paired random-origin seeds across baseline, uniform-food and3x-regeneration
conditions. Geography is stationary; weather/regrowth/depletion still operate.
No external ranking or breeding selects genomes. Each world runs to16384ticks
or extinction and saves its actual endpoint checkpoint. Local registration and
results are under reports/v3-founding-ecology-20260904-validated; the sibling
unsuffixed folder preserves a disclosed validator preflight, not another campaign.
Python3.11+ standard library is sufficient for the runner.

The [completed24-world results](reports/FOUNDING_ECOLOGY_RESULTS.md) found no
established population in any condition. Uniform habitat improved food encounters
and collection but did not produce continuing generations. This diagnostic is
finished; its independent worlds did not carry genes between runs. The survivor
loop above tests that separate question.

To reproduce this protocol locally using the preserved runtime (new directory):

```powershell
python training/founding_ecology.py --runtime-origin reports/v3-feeding-100-20260904 --directory reports/new-founding-ecology
```

This command requires that local runtime archive; it is not a public distribution
dependency. The old family-scoring trainer remains historical research code:
inside worlds ordinary inheritance occurs at births, while that trainer adds
external ranking/mutation across worlds. It is not equivalent to selection only
through reproduction. Do not restart its intentionally stopped campaigns.

The [training protocol](training/TRAINING_PLAN.md) specifies100 selection rounds,
independent islands, adaptive bounded trials, frozen benchmarks and separate
200k-or-extinction development evaluation. Trials stop on extinction within one
at-most32-tick GPU batch. Diagnostic feeding/action counters never select genes.
No bank is automatically installed as the default.

Resume a genuinely interrupted founding suite with its frozen runner/protocol,
only after confirming no runner is active and inspecting any failure:

```powershell
python reports/new-founding-ecology/replay/training/founding_ecology.py --resume --directory reports/new-founding-ecology
```

## Current evidence

The six-round v3 pilot collected more food but died earlier than baseline on all
four comparison seeds. [The feeding audit](reports/V3_FEEDING_AUDIT.md) finds
improved juvenile collection but severe juvenile starvation and lower birth
investment. Its replay produced one grandchild, not reliable family persistence.

The [registered100-round campaign](training/FEEDING_CAMPAIGN.md) was intentionally
[stopped after40 rounds](reports/v3-feeding-100-20260904/STOPPED_BY_USER.md): feeding
improved, but every fixed benchmark still went extinct and sustained survival
stalled. The authored-starter campaign was separately stopped after16 rounds.
Both artifact sets are preserved; their raw summaries may still say running.
The follow-up is paused: do not automatically resume either campaign. Neither
completed its planned100 rounds; no final eight-seed/migration validation was run.

The trainer also accepts `--initial-bank PATH` to archive and use an explicitly
named starting pool, without added initialization noise. Independent islands,
normal mutation, fixed benchmarks and separate final comparisons remain. To
compare improvement, use each trained pool's actual starting bank as its baseline.

## Contracts and verification

- [CONTROLLER.md](CONTROLLER.md): senses, recurrence, intentions and initialization.
- [KERNEL_SPEC.md](KERNEL_SPEC.md): body, resource accounting and ecology.
- [Agent audit](reports/AGENT_AUDIT.md): retained assumptions and removed behavior rules.
- [Completion contract](GOAL_CONTRACT.md): unchanged survival, migration and play gates.

`cargo test --release` and `python -m unittest discover -s training -p test_prepare.py`
check integrity, not intelligence. GPU population trajectories are not promised
bitwise deterministic. Sampled journey evidence still lacks major-relocation
attribution required by the completion contract.

Earlier v1/v2 reports and experiment runners are historical evidence, not
compatible v3 launch instructions. The shared-GRU runtime remains retired.
This is a research build, not a public-release-ready package.
