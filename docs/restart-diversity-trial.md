# V46 restart-diversity trial

The owner selected a continuing wallpaper experiment with a 20/40/40 founder
mix at each world rollover. The source of the retained founders is the rolling
4,096-record hereditary pool, which spans worlds; it is not a literal copy of
the just-extinct world's final organisms. The current living world is not
repopulated until it naturally ends.

For each five founder slots, one receives a completely new random genome and
random inherited traits, two receive unchanged independent pool samples, and
two receive independent pool samples whose **copies** are heavily mutated.
At the saved population setting of 8,192 founders, this gives 1,639 new,
3,277 unchanged, and 3,276 mutated founders. Slot classes repeat evenly across
the initial population, whose positions are independently seeded. The pool
records themselves are unchanged by founder sampling or restart mutation;
only ordinary viable births can introduce these new lineages into the pool.

"Heavily mutated" means one fifth of the selected pool genome's expressed
parameters are chosen without replacement and each receives a bounded uniform
perturbation in [-0.35, 0.35], clamped to the ordinary [-4, 4] gene range.
Two active plasticity coefficients and both retention traits receive bounded
perturbations of up to 0.04. Topology, packet size, latent genes, and inherited
mutation controls remain rooted in the pool record. This is deliberately much
stronger than a normal birth mutation, especially because the current pool's
parameter-mutation-rate multiplier is almost entirely at its 0.25 minimum.
Ordinary births and all physical, cognitive, ecology, food, and presentation
rules remain as in the running four-unit build.

The previous four-unit executable and exact stopped checkpoint are archived
before installation. Checkpoint format 65 accepts prior v46 formats 62–64 and
records the new rule boundary; the prior executable rejects format 65. The
first world after handoff is the exact continuation of the current world;
the new founder mix begins only at its next rollover.

The exact handoff saved world 158, tick 15,218, 300 living, and cumulative v46
macro step 75,849,894. The old format-64 receipt, 417,660,249-byte checkpoint,
old DT4 executable, and verified SHA-256 manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\bio-dt4-before-restart-diversity-20260923`.
The format-65 executable resumed that same world at MAX. A copied format-64
checkpoint loaded and advanced one step in the new build, saved as format 65,
and reloaded; the targeted founder-mix and mutation tests passed.

This is an exploration trial, not an established improvement. The large novel
cohort can consume resources and dilute viable inherited behavior; heavily
mutated controllers can also lose useful coordination. Judge the run by mature
descendants, naturally closed reproductive chains and their retention across
worlds per real hour, alongside population and rollover duration. Faster
rollover or more founder diversity alone is not success.
