# Changelog

## 0.7.0

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
