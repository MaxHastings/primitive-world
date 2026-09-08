# Long-run testing

Start a **New Game** for model `primitive-v29-composable-reservoir`; old and
intermediate saves are incompatible. Existing saves are preserved.

## Desktop experiment

```powershell
.\Play.cmd --wallpaper --seed 42 --view-speed 16x
```

The launcher builds current source. For a regular window, omit `--wallpaper`.
Start around 16x and adjust to comfortable GPU load. The viewer autosaves changed
state every five minutes; Save, Main menu and close also save. Resume with
`.\Play.cmd --wallpaper --resume`. If an older wallpaper is running, quit through
its tray menu first so it can save before rebuilding its executable.

For an unassisted run, avoid adding food with desktop clicks. Manual additions
are permitted but mark the experiment as assisted. Watch population and births
over time, ancestry, invalid-output counts, throughput, save errors, and repeated
extinction patterns. Do not judge progress by intelligence labels or population
size alone. Desktop input routing, monitor changes, sleep/wake and multi-day
stability require this real desktop test; headless checks cannot establish them.

## Resumable unattended experiment

```powershell
.\tools\Soak.ps1 -Hours 12 -Seed 42
```

This builds a release executable and copies it into a unique ignored
`reports/soak-*` directory. It runs in 50,000-tick chunks, validates each checkpoint
by loading it, and atomically advances `session.json`. It retains the newest two
validated checkpoints, plus reports/logs. Each checkpoint is roughly 430 MiB;
allow several GiB of free space. Incomplete interrupted attempts are preserved
for diagnosis and can use additional disk space.

The displayed directory contains the exact executable and its SHA-256 digest.
Resume using the same binary even after source changes:

```powershell
.\tools\Soak.ps1 -ResumeDirectory 'reports/soak-YOUR-RUN' -Hours 12
```

`Hours` is the duration of this invocation, checked between chunks; it may overrun
by one chunk and checkpoint validation. Ctrl+C may lose the unfinished chunk;
the previous completed receipt remains resumable. `-ChunkTicks 10000` gives more
frequent recovery points at greater save overhead. Output samples every 4,096 ticks.
A directory lock prevents two runners from advancing the same experiment.

Engine body/identity/tick capacity stops are explicit and censored. The runner
saves that boundary and stops rather than treating it as extinction or reseeding.
A failed executable, checkpoint validation, or disk write stops the runner and
preserves its last completed receipt. Inspect the attempt logs before resuming.
A backup `session.json.bak` retains the preceding receipt; the two most recent
checkpoints support manual recovery if the newest receipt is damaged.

The headless test uses the same physical rules and transitions as the viewer,
but does not test rendering, tray behavior or Explorer hosting. Reports and
checkpoints are local artifacts; do not commit them.

## Reachability evidence and interpretation

The regression came from the **uncommitted** transition away from the committed
`codex/masked-learning-16` bootstrap: full .06 upkeep and maximum environmental
strength were applied immediately. The committed branch was not used as the
rapid-extinction baseline.

Development probes used 1,000 fresh random founders, seeds 7, 42 and 123, an
RTX 4070 SUPER / NVIDIA 591.86 / Vulkan on Windows, and no interventions:

| Candidate rules | Seed 7 | Seed 42 | Seed 123 |
| --- | --- | --- | --- |
| Unfinished rules, upkeep .06, contrast 1 | Extinct at detection tick 3,056 | Extinct at 2,128 | Extinct at 2,512 |
| Upkeep .015 alone, contrast 1 | 1 alive at 12,000 | Extinct at 10,416 | Extinct at 10,800 |
| Upkeep .015, composable gathering, contrast .5 | 2,991 alive / 7,549 births at 20,000 | 1,450 alive / 3,231 births at 20,000 | 1,448 alive / 3,523 births at 20,000 |

These are exploratory development probes, not controlled estimates of each
change's independent causal effect or the exact final-default configuration.
Contrast redistributes the existing mean habitat rather than increasing it. The
final model uses its documented .01→.06 body-upkeep ramp while gathering remains
a signed controller request that can run alongside reproduction and movement.
No founder policy, reward, ranking or online rescue was added.

All three populations persisted past the maximum founder lifespan (11,000 ticks).
That demonstrates reachable multigeneration life cycles for these seeds, not
intelligence, general adaptation or indefinite persistence. Extinction remains
valid. Look for a mix of viable and failing lineages and strategies rather than
optimizing the experiment toward a particular visible behavior.
