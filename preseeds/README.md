# Pre-seeds

These small JSON founder banks are versioned biological starting points. They
contain genomes and inherited cognitive traits only; they do not contain a
world, population state, learned memory, or ecology. That keeps them suitable
for Git and makes each run a fresh experiment.

Start the bundled sample in the wallpaper viewer with:

```powershell
.\Play.ps1 --wallpaper --pre-seed evolved-sample --seed 3001 --view-speed MAX
```

`--pre-seed NAME` resolves to `preseeds/NAME.json`. Use `--founders PATH` when
you need an external or larger founder bank. A pre-seed is used only when
starting a fresh world; it cannot be combined with `--resume` or a checkpoint.

`evolved-sample.json` is one naturally living descendant sampled from the
long-running experiment around world 753. Its provenance fields are retained
inside the bank. Its numerical genome and inherited traits are unchanged from
the v45 sample; the bank header identifies the v46 model that loads it. The
format/model are validated before use.
