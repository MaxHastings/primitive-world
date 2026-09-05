# Live inspection and persistence changes

Status: implemented and headlessly verified on the research branch. Not yet
promoted to main or included in the frozen cumulative-preparation executable.
Current playable source is version0.3.1-dev so it is distinguishable from the
frozen0.3.0-dev campaign build. The model/checkpoint remain physiology-v2/14.

The previous GUI retained a click-time snapshot and highlighted only a storage
slot. Inspection now reads the same slot/incarnation/lineage/birth-tick identity
before each frame's simulation batch, labels the snapshot tick, and refreshes
the displayed body and controller data. Death retains labelled terminal body
data; slot reuse ends tracking without substituting a newborn. Readback errors
retain a labelled old snapshot and retry rather than diagnosing death. Highlight
checks incarnation in the render shader, including reuse during the next batch.

Initial/newborn bodies do not display a previous slot occupant's decision. The
independent reviewer also identified that dead-slot decision/perception buffers
can be cleared by subsequent ticks. Dead bodies now suppress those traces; a
batched-death regression was added. Controller inputs are labelled as preceding
the last step's movement, and actual feedback as the last body update, not a
whole-frame total.

Save creates a timestamped checkpoint in `reports/checkpoints` and fills an
editable load path. Repeated saves, including at the same paused tick, preserve
earlier files. The underlying writer still uses create_new, so even a filename
collision fails rather than overwriting. Load retains existing paused-restore
behavior. The initial load path still finds `recurrent-world.checkpoint`.

## Verification completed

- Three CPU inspection-state tests passed: live/dead refresh, read failure and
  retry/identity disappearance, initial/newborn trace validity.
- One CPU snapshot-path test passed for distinct filenames at the same tick.
- `cargo clippy --release --all-targets --target-dir target/inspector-fix -- -D warnings`
  passed. The separate build directory leaves running user executables untouched.
- Passed headless GPU checks: selected-body refresh versus actual buffers,
  read-only physical/genome/resource/ground/accounting state, dead/changed identity
  and invalid slots, batched death with cleared traces, and render pipeline/camera
  layout validation, plus existing checkpoint refusal/roundtrip tests.
- Full release suite:46 passed,0 failed,1 explicitly ignored research probe;
 21.02seconds. Source commit `f26eb65`; separate test executable SHA256
 `057525d5142842a4f1bee089f6cec8c397f7f4568a3335355d2d49da50a9b9c5`.
- Rebuilt as0.3.1-dev and reran the full suite:46 passed,0 failed,1 ignored,
 22.31seconds. `--version` reports the expected0.3.1-dev/physiology-v2/checkpoint14.
- No GUI/computer control performed. Automated checks cannot establish the user's
  experience or visual usability; the broad goal remains active.

The GPU directional probe added alongside these tests is a separate explicitly
ignored diagnostic requiring a bank and new report path. Ordinary tests do not
silently depend on local research banks. It tests actual decision-shader outputs
under synthetic cues, not ecological survival. It subsequently passed separately
for budgets0,8,16, after the cumulative campaign ended. CPU/GPU motor agreement
was within2.6e-7 across the first-decision cases. See `DIRECTION_AND_SIGNALS.md`.
