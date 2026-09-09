# Long-run testing

Start a **New Game** for model `primitive-v35-body-frame-contact`; old and
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
validated checkpoints and at most 256 diagnostic files. Each checkpoint is roughly 410 MiB;
allow several GiB of free space. Incomplete interrupted attempts remain for diagnosis. An 8 GiB run-directory
budget stops further chunks before unlimited failed-attempt accumulation; one
chunk may exceed the checked budget. A child-process timeout defaults to 1,800
seconds, adjustable with `-ChunkTimeoutSeconds`; failures preserve the last
validated receipt. The runner reports sampled peak process working set.

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

## Current reachability evidence

The stationary v35 model was tested on Windows with an RTX 4070 SUPER, NVIDIA
591.86 and Vulkan, using 1,000 seed-specific random founders, default contrast 1,
stationary body upkeep 0.015, no imported policy and no interventions. The final
20,000-tick probes include proportional gathering and the body-frame/contact
migration. Seeds were fixed at 7, 42 and 123; no further viability tuning followed.

| Seed | Living at 20,000 | Births | Invalid outputs |
| --- | ---: | ---: | ---: |
| 7 | 2,766 | 23,632 | 0 |
| 42 | 1,694 | 10,330 | 0 |
| 123 | 3,514 | 19,473 | 0 |

All exceed the maximum founding lifespan of 11,000 ticks, so these are descendant
populations. This demonstrates nonzero reachable reproductive life cycles and
supports removing the metabolism ramp. It does not establish a general success
probability, intelligence, adaptation, or indefinite persistence. Extinction is
allowed. Do not retune toward more attractive trajectories.

Local raw artifacts are `reports/final-v35-seed{7,42,123}.json`; they include actual
settings and hardware. The small checked-in evidence ledger is in
[implementation-checklist.md](implementation-checklist.md). Reports and checkpoints
remain ignored local artifacts.

## Interpreting long runs

Rolling headless reports include read-only reservoir genome diversity, mutation
control/capacity histograms, current-world founder-family representation, changed
pool records between samples, exact-copy births and topology events. They retain
at most 4,096 recent report samples per invocation. A replaced slot may contain an
identical record, and multiple replacements can occur between samples; observed
changed slots undercount turnover. Successful births count replacement attempts.

Observe freezing, explosion or collapse without adding rewards, ranking, novelty
bonuses or escape mutations. Engineering stops are censored outcomes. Finite
snapshot/report histories and process working-set samples help diagnose operation,
but a short headless run cannot prove multi-day desktop or sleep/wake stability.
Do not mark those checks complete without evidence from the target desktop.
