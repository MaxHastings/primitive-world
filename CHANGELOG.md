# Changelog

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
