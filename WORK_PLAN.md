# Autonomous artificial-life goal — unfinished

The 0.2 release below completed a narrower delivery milestone, **not the user's
goal**. Calling the broad goal complete was premature. The active acceptance
contract is in `GOAL_CONTRACT.md`. No survival, migration or user-experience gate
in that contract has yet been demonstrated. Passing integrity tests is not
evidence that adaptation works.

Current progress: the GPU probe confirms the released bank's sensor-slot /
world-motor initialization mismatch. Six registered development worlds tested
fixed food-sensor axes: two lifetimes increased, but all six worlds went extinct
before200k. The ablation is isolated in `ClownSimulator-frame`; it is not a final
controller. See `reports/FOUNDATION_FRAME_AUDIT.md` for complete endpoints.

An optional sampled journey recorder now distinguishes departure / poor-space
crossing / new collection / ingestion / reproduction sequences. It does not
attribute them to major geography renewals yet. Next work must establish a clean
versioned sensory/motor contract and evaluate preparation under it, while finishing
relocation attribution. No final fresh holdout seeds have been consumed.

The longer journey control recorded246 depleted departures and47 poor-space
crossings, but no destination collections qualifying under the sampled definition;
it died at98,304ticks. Before adding agent complexity, retain failed departure
trajectories and energy to examine where these attempts end. See
`reports/JOURNEY_DEVELOPMENT.md`. This is not a migration or training success.

Goal: a usable, understandable artificial-life playground with a small coherent
foundation, not scripted travel or unsupported intelligence claims.

1. Restore ordinary energy costs; isolate continuous motor-response calibration.
   Zero intent must remain stopped, reverse intent must reverse motion, maximum
   body speed and per-distance costs stay unchanged. No destinations, minimum
   speed, famine alarm, hidden exploration timer or movement reward.
2. Evaluate a finite set of response gains in preparation worlds before choosing
   a working configuration. Export only actual living descendants, preserve
   failure, then compare frozen starting banks on distinct fresh worlds. Do not
   conflate the body calibration with inherited improvement.
3. Improve playable controls, founder/settings identity, capacity warnings and
   one clear launcher. Keep optional research banks distinct from the release.
4. Test CPU/GPU behavior, physical accounting, persistence, legacy checkpoint
   settings and headless build/startup. Record what passed and what remains open.
5. Commit a coherent result to main, provide a built artifact/launcher and a short
   play guide. Preserve user files and historical/failed experiments. No GUI use.

The previous eight-run comparison was interrupted at user redirection during
its first run; it is not evidence for or against preparation. Its files remain
in the separate retention research worktree, marked interrupted.

Earlier release milestone: calibration and all nine held-out comparisons completed. Gain4 is
the working physical default with original costs. The prepared bank improved
survival on one seed but failed the two-seed promotion gate; original released
weights remain. Launcher/controls/observer and28 integrity tests are complete.
See reports/PLAYABLE_RELEASE.md for evidence and unresolved research questions.
