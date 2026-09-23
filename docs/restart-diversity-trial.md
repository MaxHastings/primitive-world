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

"Heavily mutated" now means one quarter of the selected pool genome's
expressed parameters are chosen without replacement and replaced by the
corresponding parameters of an independent freshly randomized genome. The
other three quarters remain exactly inherited. One active plasticity
coefficient is redrawn; both retention traits move halfway toward fresh
random values. Topology, packet size, latent genes, and inherited
mutation controls remain rooted in the pool record. This is deliberately much
stronger than a normal birth mutation, especially because 4,095 of 4,096
records in the sampled pool had parameter-mutation-rate multipliers in the
lowest histogram bin (0.25–0.354).
Ordinary births and all physical, cognitive, ecology, food, and presentation
rules remain as in the running four-unit build.

A read-only check of 128 evenly spaced records from the saved evolved pool
compared each mutated genome with its source and the same fresh random genome,
using only expressed parameters. The ratio of mutant-to-source distance to
fresh-to-source distance was 0.500 at the median (10th–90th percentiles
0.430–0.561); the median fraction of expressed parameters left exactly intact
was 0.748. This calibrates genetic distance, not behavior or reproductive
success. The metric is computed by the ignored
`profile_restart_mutation_distance_on_saved_pool` diagnostic.

The previous four-unit executable and exact stopped checkpoint were archived
before the 20/40/40 installation. Checkpoint format 65 accepted prior v46
formats 62–64. The stronger mutant version uses format 66 and accepts formats
62–65; older executables reject format 66. Each handoff resumes the current
world exactly, and the revised founder mix begins at its next rollover.

The exact handoff saved world 158, tick 15,218, 300 living, and cumulative v46
macro step 75,849,894. The old format-64 receipt, 417,660,249-byte checkpoint,
old DT4 executable, and verified SHA-256 manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\bio-dt4-before-restart-diversity-20260923`.
The format-65 executable resumed that same world at MAX. A copied format-64
checkpoint loaded and advanced one step in the new build, saved as format 65,
and reloaded; the targeted founder-mix and mutation tests passed.

After the owner clarified that the mutants should be closer to the middle
between selected and fresh genomes, the format-65 wallpaper took its normal
save-and-close path at world 184, tick 33,183, 92 living, cumulative v46 macro
step 77,008,013. Its exact receipt, 417,660,254-byte checkpoint, executable,
and verified SHA-256 manifest are under
`%LOCALAPPDATA%\PrimitiveWorldV46\archive\restart-mix-before-strong-mutants-20260923`.
The format-66 build resumed that exact world at MAX. A copied format-65
checkpoint advanced one step, saved as format 66, and reloaded successfully.

This is an exploration trial, not an established improvement. The large novel
cohort can consume resources and dilute viable inherited behavior; heavily
mutated controllers can also lose useful coordination. Judge the run by mature
descendants, naturally closed reproductive chains and their retention across
worlds per real hour, alongside population and rollover duration. Faster
rollover or more founder diversity alone is not success.
