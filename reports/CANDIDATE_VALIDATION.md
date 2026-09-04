# Candidate-v1 validation — 2026-09-04

## Root cause and scope

The old authored scorer assigned force an immediate opportunity value against
nearby food carriers. Force actually spends attacker and recipient energy and
may spill food; it does not directly transfer food to the attacker. Reproduction
requires both energy and inventory. Therefore the scorer could repeatedly choose
an action that depleted the very reserves required for descendants. This was
not evidence that evolution had discovered a long-term advantage of conflict.

The archived GRU was opt-in, shared rather than inherited, and unvalidated for
multigeneration persistence. Its survival reward does not credit descendants;
its reserve reward charges reproduction as a loss. Short unsuccessful training
does not establish that recurrent learning cannot work. That research question
remains open, separately from the active evolutionary controller.

This revision replaces action desirability with independent signed, inherited
candidate weights. It retains explicit physiological bootstrap biases and
authored movement proposals. It fixes drifting place anchors and expands private
place memory from four to sixteen entries. It does not claim that feature
extraction, navigation, or basic foraging were learned from scratch.

## Protocol

All population experiments below are headless, start with 1,000 bodies, and use
ordinary default ecology and costs. Force is enabled except in named controls.
Preparation samples living descendants by stable hash order, without ranking
action histories. Each new world starts fresh private memories. Evaluation never
exports its weights into the founder bank. These are individual observations,
not confidence intervals; GPU atomic ordering permits divergence even within a
seed.

Preparation: bootstrap seed 11 for 30,000 ticks, then its descendant bank on seed
22 for 30,000 ticks. The resulting 128-genome bank is bundled in
`policies/ancestor-v1.json`:

`ED9FFC63ECBA0B2CF69EA5E0F67D86E0415D42E22B5D520F6C90A96946255C6D`

Preparation and held-out/control batch executable SHA-256:

`315D8F95A8CC19AC2A8FF45E58DE5AD2824485AF067B40B38097A363B2D3DC8F`

That batch predates the final default-bank embedding and archived-GRU timing
fix. Its candidate physics, scoring, inheritance and memory match the final
candidate implementation. A separate final-release default-bank run is recorded
below. Local raw JSON reports are intentionally ignored by Git; the bundled bank
and this methodology/results record are durable repository artifacts.

## Results at 30,000 ticks

| Run | Seed | Living | Cumulative births | Resolved force | Maximum ancestry depth |
| --- | ---: | ---: | ---: | ---: | ---: |
| Bootstrap preparation | 11 | 1,809 | 8,516 | 4,933 | 14 |
| Descendant preparation | 22 | 1,809 | 8,379 | 22,094 | 14 |
| Frozen-bank held-out | 101 | 1,851 | 9,453 | 21,407 | 15 |
| Frozen-bank held-out | 202 | 1,509 | 7,391 | 15,176 | 15 |
| Frozen-bank held-out | 303 | 1,010 | 7,800 | 14,956 | 13 |
| No-force control | 101 | 1,884 | 9,710 | 0 | 14 |
| Famine stress | 303 | 908 | 7,315 | 13,939 | 13 |

All three ordinary held-out worlds reproduced beyond founder lifespans
(9,000–11,000 ticks) while force remained reachable and was actually used.
No rescue births, immigration, population floor, cooperation reward, shared map,
or forced migration was introduced. The no-force comparison does not show that
conflict is harmless; it shows that the earlier near-total reproductive collapse
was not reproduced by this controller on this tested seed.

The famine treatment removed ground food globally at tick 16,000, suspended
regeneration for 500 ticks, then restored growing conditions without refilling
food or reseeding bodies. Population was 1,859 before intervention, 1,680 at
restoration, 573 at tick 25,000 and 908 at tick 30,000. Recovery was not immediate,
and this is not evidence of survival through arbitrarily long famine.

For context, prior local reports from the old scorer on seed 1 at 30,000 ticks
record 29 living / 191 births / 52,954 force, versus 1,727 living / 8,257 births
with force disabled. The archived GRU report records extinction and zero births.
Those are historical diagnostics, not a paired estimate of this revision's
effect: seed, scorer, memory, initialization and mutation differ.

Raw files: `candidate-stage1.json`, and `candidate-validation/prepare-0-seed22.json`,
`evaluate-seed101.json`, `evaluate-seed202.json`, `evaluate-seed303.json`,
`control-no-force.json`. The batch also writes `summary.json` with bank and
executable hashes.

To repeat the protocol on a new output directory (exact populations may differ):

```powershell
cargo build --release
python training/prepare.py --directory reports/repeat-candidate --prepare-seeds 11 22 --eval-seeds 101 202 303 --prepare-ticks 30000 --eval-ticks 30000
```

On held-out seed 101, 3,198,655 eligible agent-ticks produced 9,454 birth requests,
of which 9,453 completed. The request frequency is approximately 0.00296,
consistent with the configured 3/1024 per eligible tick. Low births should now
be diagnosed through eligibility and actual costs, not inferred from global food
abundance or from a slot-incarnation number.

## Default-bank durability probe

The fourth unseen seed, 404, ran for **60,000 ticks** without `--founders` or
`--bootstrap`, verifying the bundled default. It ended with **1,576 living,
14,688 births, 33,168 resolved force interactions, 3,887 transfers, and maximum
ancestry depth 29** (mean living depth 17.54). Force stayed enabled. Population
was 631 at tick 25,000, 880 at 30,000, 769 at 50,000, then 1,576 at 60,000;
the criterion was persistence and real reproduction, not a flat population.

```powershell
.\target\release\primitive_world.exe --headless --seed 404 --ticks 60000 --sample 5000 --output reports/candidate-validation/final-default-seed404.json
```

Probe executable SHA-256:
`F7970D08B60CE1E42E69C7448891F156A3FCA4C828EC6890E0D288504B105246`.
The subsequent packaging build only adjusts GUI reset bookkeeping and adds
terminal-trace test assertions; candidate simulation behavior is unchanged.

Packaged executable SHA-256:
`290715D93BD9AF4B7B6D345D3F7E6181BC5601D209509FFFCBB6A0D238714FBA`.
Its separate seed-505, 1,000-tick headless smoke run finished with 1,052 living,
52 births and 17 force interactions. No application window was opened for it.

## Contract checks

Final headless suite: 21 passed, 0 failed, 1 intentionally ignored optional
trajectory diagnostic. `cargo fmt --check`, included-test-file formatting,
`cargo check --all-targets`, Python compilation, and release compilation pass.

The headless suite covers CPU/GPU GRU numerical parity, physical conservation,
interaction contention, locality, birth inheritance, actual ancestry, checkpoint
replay, and batched clocks. Candidate-specific regressions check that signed
weights can choose or reject force, raw events can affect decisions, observer
identity/counters cannot affect decisions, founders load explicitly, and place
records do not drift into a moving agent's position.

A controlled navigation regression gives one agent a private remembered food
coordinate 300 units away, clears ground food, and verifies arrival within a
24-unit sensory radius in at most 320 ticks. This proves usable navigation across
empty space to known coordinates. It does **not** prove discovery of unknown
patches, repeated inter-patch commuting, or emergent shared travel corridors.

The archived neural bridge captures a decision before deaths/reused slots can
overwrite it. Newborn GRU agents wait for the next global eight-tick boundary,
preventing unrecorded recurrent steps between training samples. Off-boundary
bridge frames are rejected. Sampled slot occupancy is no longer called lineage
survival; whole-world births and ancestry are reported separately.

A two-update PPO transport smoke test (64 sampled slots, 16 steps per update)
completed with maximum Python/GPU recurrent error `1.79e-7`. Both input and
recurrent parameters changed. This checks numerical/training plumbing, not
learned survival competence. Its experimental actor was not promoted to defaults.

## Boundaries and remaining research

- This is finite controller evolution, not open-ended intelligence or a claim
  of universal ecological stability. Prepared weights have not been shown to
  outperform the bootstrap in a matched multiseed comparison.
- Reproduction is still an automatic physical eligibility gate, not an explicit
  decision to have offspring. Its energy, inventory, maturity, movement and
  cooldown failures are now measured separately; failures overlap.
- Longer-run strategy drift, genuinely recurrent inherited controllers, causal
  signal use, and repeated route formation remain research questions.
- Checkpoint schema 11 rejects older schemas without deleting old files.
- Counters are 32-bit; measurements require bounded runs to avoid overflow.
- GPU/render pipeline tests are headless. A full interactive GUI checklist was
  not completed; the user explicitly requested no computer control. The existing
  user window was left alone.
