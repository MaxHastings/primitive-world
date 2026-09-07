# Changelog

## 0.9.0 — founding-population selection

- Compare complete founding populations by completed world duration on matched seeds; keep the current population on ties or failed candidates.
- Preserve founding combinations with sparse parameter mutation and occasional random exploration. Later descendants affect outcomes but are not directly archived.
- Remove individual-lifetime retention and obsolete selection/capability paths. Keep fixed gated brains, regional senses, ecology and GPU/playback optimizations.
- Align default desktop/headless evolution; bounded diagnostics require `--single-world`.
- Persist current/candidate founding groups, paired state and 64-world history in checkpoint 22 / receipt 4, model `primitive-v8-population-search`. No migration or compatibility execution.
- Verification for this change is compilation, formatting, lint and static shader/source review only; no new runtime tests or performance experiments.

## 0.8.0

- Fixed eight-unit gated recurrent brains with inherited weight mutation; retain all regional senses and sector targets.
- Replace learned founder selection and round/pool training with explicit longest-lifetime selection and mutated next-world founders.
- Include retained parents and restart RNG in checkpoint 21; reject incompatible saves explicitly.
- Fold brain upkeep and copying into metabolism and fixed birth overhead.
- Keep independent GPU playback and active-slot scans; reduce reset uploads and duplicate terrain computation.


## Round-based lifetime-record selection

- Add bounded, resumable training rounds: matched populations learn across repeated
  batches before their candidate pool changes. Fresh runs collect an uncredited
  initial world's lifetime archive.
- Refresh pools through equal-world sampling independent of duration or behavior;
  expire old records without renewing them when selected. Preserve source mappings
  between factual measurements and opaque genomes.
- Save partial worlds, incoming sampling state, and batch/round transitions. Resume
  preserves exact choices and credit; unfinished worlds receive no reward.
- Add configurable rounds, batches, compositions, seeds, and retention, plus frozen
  and uniform controls. Learned improvement remains unestablished.
- Use checkpoint format 20, selector weights format 4, training and game receipts
  format 2, and viewer snapshots and retained pools format 1.

## Genome-only population selection

- Replace biological transfer scores/quotas with genome-only pool attention and
  conditional founder sampling, retaining uniform exploration.
- Capture founders before ticking and newborns on their birth tick; preserve a
  behavior-independent 256-individual reservoir after death.
- Instantiate exact selected copy counts through explicit founder slots. Mutation
  remains at ordinary in-world births; external founding copies are unmutated.
- Train only on exact natural world duration, with optional 1/2/4-world credit,
  persisted optimizer/PRNG/actions, and no reward for pauses or interventions.
- Add headless training, uniform/individual baselines, frozen matched-pool
  evaluation, and explicit censoring. Learned improvement remains unestablished.
- Advance checkpoints to 19 and evolution/selector state to 2. Old formats are
  rejected without rewriting files; genome/bank formats and physics are unchanged.


## 0.7.1

- Replace late-survivor transfer with the 64 most recent offspring that reach
  maturity. Capture every tick on the GPU, break same-tick ties by identity hash,
  retain exact child genomes, and never refresh an entry for lingering longer.
- Treat worlds without viable offspring as failures: mutate the incoming bank
  without survivor selection, or draw a fresh random bank. Reproduction stays paid.
- Show viable offspring and distinct descendants that actually reproduced.
- Preserve selection state in experiment receipts and standalone checkpoint
  sidecars. Explicitly migrate older loops without promoting historical survivors.
- Verify actual births, recent-cohort replacement, sterile tails, retry behavior,
  archive isolation, and save/resume eligibility. Update backups for both protocols.

## 0.7.0

- Dispatch birth construction only for eligible attempts and body updates only
  for living slots. Preserve dead records with a bulk GPU copy. Cache archived
  genomes by identity and avoid redundant small-population observations between
  demographic changes and the regular 128-tick refresh. Add an opt-in per-pass
  GPU timing diagnostic. Paid reproduction and late-survivor selection remain.
- Replace the fixed dense brain with inherited sparse gated networks: four initial
  units, 1–64 units and up to 512 explicit connections. Duplication preserves the
  existing computation before mutation; deletion and connection edits can shrink it.
- Charge energy for encoded units/connections and copying the child's actual
  genome. Unused GPU allocation costs nothing; inactive encoded structure still does.
- Replace neural mutation-request outputs with shared world mutation settings.
  Ordinary births and survivor replicas use the same structural and parameter law.
- Show brain counts, birth changes, and costs in the inspector. Reports describe
  architecture distributions instead of averages over unrelated gene positions.
- Use model primitive-v6-variable-brain, checkpoint 18, and founder bank 7.
  Older worlds require their matching engine and are never silently converted.


## 0.6.1

- Fix slow startup and menu freezes caused by parsing save receipts with
  unbuffered file reads on the UI thread. Receipts are now read in bounded blocks
  and parsed from memory; the 16 MiB size limit remains enforced.
- Save-library traversal runs in a background worker with non-blocking polling,
  coalesced refresh requests, and visible loading status. Returning to Main Menu
  no longer triggers a redundant library scan.
- Existing V5 worlds, checkpoints, and evolution rules are unchanged.

## 0.6.0

- Primitive-v5 replaces eight isolated food probes with eight compass sectors and
  near/far regional food means and body counts. Food uses every in-range grid-cell center.
- The nearest individual per sector is observable and targetable; all in-range
  bodies contribute to crowding. No tick-dependent sampling or visible neighbor inventory.
- Sixteen evolved update gates allow exact retention, replacement, or blending
  of private memory, with no assigned semantics or mandatory forgetting.
- The inspector exposes the sensory regions, sector targets, signal presence,
  and memory update gates. Controllers have 108 inputs, 22 outputs, and 2,646 weights.
- Checkpoints advance to format 17 and founder banks to format 6. V4 and earlier
  files are rejected without conversion; start a fresh evolutionary experiment.
- Survival, reproduction, mutation controls, and rolling survivor selection are unchanged.

## 0.5.0

- Primitive-v4 adds brain-controlled offspring mutation probability and magnitude.
  Exact copying is a valid reproductive choice.
- In-world births and survivor transfer use the parent's latest controller
  requests; no standalone mutation-rate gene or mandatory mutation floor exists.
- Automatic survivor transfer keeps a rolling archive of up to 64 bodies, retaining
  earlier entries as a population shrinks and refreshing them as it recovers.
- A new home screen, experiment library, and docked inspector support starting,
  saving, resuming, and branching visual experiments. Saves retain the survivor
  archive as well as the current world.
- Opt-in comparison tools test memory and signal dependence in matched worlds.
- Save and founder-bank formats advance for the expanded 18-output brain. V3
  files are intentionally rejected.

## 0.4.0

- Local recurrent agents choose collection, reproduction, scalar signals, and
  contact displacement, with automatic digestion and continuous movement.
- Random founder genomes and inherited mutations drive population evolution.
- An extinction-only evolution loop carries survivor genomes between worlds in
  one window, retaining playback speed and physical settings.
- Full loop checkpoints save at startup, every five minutes, and on normal close.
- The agent inspector shows local inputs, decisions, energy, and ancestry.
- Headless reports and optional read-only tools support journey analysis,
  communication audits, and verified local backups.
- Save and export commands create unique files or refuse existing destinations.
- Source checks cover formatting, linting, and CPU tests. GPU simulation and
  visual release checks run separately.

See [release status](docs/release.md) for supported formats and distribution checks.
