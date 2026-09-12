# Long-run testing

Start a **New Game** for model `primitive-v42-climate-care`; old and
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

Entity capacity skips unallocated packet requests while simulation continues.
The viewer and headless runner continue the same world across former 32-bit
accounting, time, and identity boundaries using 64-bit storage. Only natural
extinction automatically seeds another world.
A failed executable, checkpoint validation, or disk write stops the runner and
preserves its last completed receipt. Inspect the attempt logs before resuming.
A backup `session.json.bak` retains the preceding receipt; the two most recent
checkpoints support manual recovery if the newest receipt is damaged.

The headless test uses the same physical rules and transitions as the viewer,
but does not test rendering, tray behavior or Explorer hosting. Reports and
checkpoints are local artifacts; do not commit them.

## Packet-model validation

The packet model is a fresh experiment. Historical contact-reproduction survival
and throughput results are not evidence for its ecology. See the current
[validation ledger](implementation-checklist.md) for checks actually performed.

Track organisms and packets separately, including packet energy, inherited size,
failed fusions and storage-rejected production. None of these observers guides
reproduction, mutation or hereditary retention.
