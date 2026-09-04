# Playable-world release 0.2.0

This is a historical release milestone, **not completion of the user's project
goal**. The active, unmet acceptance criteria are in `../GOAL_CONTRACT.md`.

2026-09-04. Runnable launcher, explicit physical defaults, founder controls and
read-only travel diagnostics. No scripted migration, compulsory movement,
population rescue, semantic memory, new sensor or travel reward was added.

## What changed

- Restored metabolism0.06 and movement cost0.01; regeneration remains0.01.
- Added explicit motor response gain: `tanh(gain * motor_logit)` per axis,
  followed by the existing vector-length cap and body speed scaling. Fresh
  default4, historical1. Zero remains zero; arbitrarily small/reversed motion
  remains possible; maximum speed is still1.2 and actual distance is charged.
- This is a tested actuator calibration, not an engine speed-cap bug fix or
  evidence that the brain learned. It changes an authored body parameter.
- Old checkpoint12 files missing the gain field restore gain1. Saved costs and
  explicit gains are restored, not overwritten with current defaults. New files
  include the field and require this build; old parsers may reject that field.
- Added CLI physical overrides, visible costs/gain/founder identity and slot
  warning, explicit export/load of descendant banks, and clearer reset labels.
- Added Play.cmd/Play.ps1 and build0.2.0 identity. Launcher builds in target/play;
  no user application is stopped. All work/testing was headless.
- Added sampled path-versus-endpoint diagnostics outside the simulation. They
  do not enter the controller, physics, reproduction or genotype selection.

## Calibration before evaluation

[Registered calibration](motor-calibration-20260904/plan.json),
[all outcomes](motor-calibration-20260904/summary.json),
[runner](../experiments/motor_calibration.py).

Same released bank, seed1, original costs,1000 founders,131072 ticks per gain:

| Gain | Final living | Births | Peak living |
|---|---:|---:|---:|
|4 |1,042 |37,302 |1,503 |
|8 |239 |66,952 |1,926 |
|16 |2,172 |101,172 |2,236 |

All persisted and had zero reported invalid outputs. The registered rule chose
the **smallest passing gain,4**, before evaluation, not the largest population.
Total393216 ticks. The gain4 descendant bank is a sample of128 living genomes,
not a ranked collection of clever agents. This preparation lasted131072 ticks;
it is NOT the earlier low-cost200k bank, which remains a separate experiment.

## Separate body calibration from inherited improvement

[Evaluation plan](../experiments/PLAYABLE_VALIDATION_PLAN.md),
[registration/hashes](playable-validation-20260904/registration.json),
[complete results](playable-validation-20260904/summary.json),
[runner](../experiments/playable_validation.py).

Nine runs on three unseen seeds, each capped at200000 ticks, original costs and
fresh bodies/state. Same seed and initial body-generation rules; only the
declared gain/bank differs. Ordinary birth mutation continues in every arm.

| Seed | Original bank, gain1 | Original bank, gain4 | Prepared bank, gain4 |
|---|---:|---:|---:|
|808 |Extinct72,704 |Extinct98,304 |Extinct98,304 |
|909 |Extinct48,128 |Extinct49,152 |556 alive at200,000 |
|1001 |Extinct48,128 |Extinct73,728 |Extinct74,752 |

Extinction times are sample-resolution observations (interval1024), not exact
death ticks. Nine completed runs used763200 of1800000 maximum ticks. Combined
with calibration:12 behavioral runs,1156416 actual ticks. No retries or rescue.
All numerical fault counts and invalid travel-observer counts were zero.
Population accounting balances in every run. No evaluation bank was exported.

Interpretation:

- **Body effect:** calibration extended observed persistence on all three
  seeds and increased net progress, but did not sustain the original bank to
  200k on any evaluation seed. It is not a complete navigation solution.
- **Inheritance effect:** the prepared bank survived one seed where the same
  body with original weights died, tied on another and lasted one sample longer
  on the third. This is promising, limited evidence, not general intelligence.
- The registered promotion gate required at least two additional surviving
  seeds and no survival-time regression. Only one additional seed survived.
  **No bank promotion:** `policies/recurrent-v1.json` is unchanged.
- All evaluation peaks were2065 bodies or fewer, with no samples above95% of
  capacity and no eligible-unresolved birth gap. These runs were not stabilized
  by the16,384-slot ceiling, unlike the earlier low-cost experiment.
- Three seeds, one repeat each and nondeterministic GPU allocation do not
  establish statistical significance. Keep this as a pilot, including failures.

## Movement evidence and limits

Mean endpoint progress per tracked agent-tick (world units):

| Seed | Original/gain1 | Original/gain4 | Prepared/gain4 |
|---|---:|---:|---:|
|808 |0.0304 |0.0610 |0.0805 |
|909 |0.0339 |0.0667 |0.0956 |
|1001 |0.0328 |0.0651 |0.0956 |

This is not merely an animation-speed comparison: matched living agents moved
farther between observations. However, path distance also increased, and only
about23–30% of sampled path length appeared as endpoint displacement. Circling
and inefficient searching remain. Sampling excludes agents that die before the
next sample and cannot reconstruct intervening routes, food arrivals, foresight
or intent. Force pushes can affect endpoints without increasing the voluntary
path counter. These are neutral measurements, not fitness rewards.

The earlier orientation probe still identifies a weakness in the released
bank's response to rotated sensors; it is not a proven coordinate-storage bug.
We did not hide it with a world-supplied food direction. Direct recurrent
retention and reliable long-horizon adaptation remain open research questions.

## Integrity and use

- 28 release tests passed: existing physics/controller/checkpoint tests plus
  continuous motor bounds, legacy settings and six synthetic observer tests.
  CLI tests verify physical override validation and checkpoint conflicts.
- Release clippy with warnings denied and formatting checks passed.
- Play.ps1 built successfully and forwarded --help without opening a window.
- Headless fresh default:1000 alive/zero invalids at32 ticks, saved and resumed
  to48 ticks with gain4 and original costs. Older checkpoint loaded with gain1.
  Final build0.2.0 additionally passed64-tick startup and --version via Play.cmd.
  A manual rejection check initially expected exit2; this runtime correctly uses
  exit1 for headless configuration errors (exit2 is for CLI parsing errors).
- Renderer construction is covered by headless tests. Native GUI interactions
  were not manually exercised; the user explicitly requested no computer control.

Use [the play guide](../PLAY_GUIDE.md). The optional local experimental bank is
`reports/motor-calibration-20260904/gain-4-descendants.json`; it can be loaded via
the inspector or --founders. Seed909 is a demonstrated example, not a promise
that it will always replay identically. It is deliberately not the default.

The previously launched eight-run low-cost comparison was interrupted at user
redirection during its first run, last observed tick105472. It has no completed
comparative result and is not counted above. Its artifacts remain in the
separate retention worktree, marked interrupted. Personal .pt files are intact.

Compact plans/summaries are versioned; large raw histories, binaries/checkpoints
and generated experimental banks remain local ignored artifacts. The released
bank and source are versioned. No failed run or previous artifact was deleted.

To run an independent reproduction after building with Play.ps1 --help:

```powershell
python experiments/motor_calibration.py --exe target/play/release/primitive_world.exe --directory reports/my-calibration
python experiments/playable_validation.py --exe target/play/release/primitive_world.exe --calibration reports/my-calibration --directory reports/my-validation
```

Use new directories; existing results are never overwritten. These commands
copy/freeze the executable for each campaign. The second requires a successful
calibration and unchanged released starting bank. Population trajectories and
exported genomes can differ across GPU runs; recorded hashes identify this
campaign, not a promise of bitwise reproduction on another run or build.
