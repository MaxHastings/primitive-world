# Evolution, without a behavior checklist

The simulator’s native `--watch-loop` carries survivor genomes between worlds.
All modes share the same controller and physics, with random founder weights
and no authored starter policy.

## Within a world

Brains choose actions and continuous outputs from local senses and private state.
Structure and weights stay fixed during life. A birth can duplicate or delete
one generic recurrent unit, insert or delete a connection, and perturb encoded
parameters. Default probabilities per birth are 4% for duplication, 4% for
deletion, 8% for connection insertion, and 8% for removal; these are mutually
exclusive structural proposals. A blocked proposal at the capacity or minimum
is a no-op, not a retry. Equal proposal rates do not guarantee equal accepted
changes or a flat distribution of architecture sizes.

After the structural proposal, each actual bias/weight has a 2% chance of a
uniform ±0.03 perturbation, clipped to [-4,4]. New edge weights use that same
magnitude and bounds. Mutation settings are visible world rules, with no neural
outputs controlling them. Parent state is unchanged; child memory starts empty.
Useful behavior can spread when its carriers leave descendants.

Each unit and encoded connection costs upkeep, and copying the child's actual
genome adds to the reproduction bill. Four units is the fresh starting size,
one is the minimum, and 64 units / 512 connections is the allocation ceiling.
There is no reward for growing and no success-triggered growth rule.

There is no loss function, survival reward buffer, action-use bonus, or requirement
to communicate, fight, cooperate, or migrate. In-world reproduction is chosen and
must be paid for. Extinction is allowed.

## Between worlds

The visible loop maintains a rolling archive of up to 64 distinct bodies. Every
128 ticks, and after each playback batch with 64 or fewer living bodies, it
captures current survivors and their genomes. Current sampled bodies take priority;
earlier entries fill the remaining places. Reobserving a lingering individual
updates its one entry, not its share of the archive. A recovering population can
replace older entries. Playback keeps its chosen batch size.

At extinction the archive seeds the next world automatically. It represents the
latest observed survivors plus retained earlier bodies, not an exact ranking of
the final 64 deaths. Every entry records its own observation tick, architecture size, and birth changes. An abrupt collapse retains the preceding archive. If fewer than 64
distinct bodies have been observed, all available entries are used.

Each sampled genome is copied unchanged once. Balanced replicas fill a 256-genome
bank using the same world mutation law as ordinary births. The receipt records
each replica's mutation seed and changes (`sparse-lcg32-v1`). The bank seeds fresh bodies in a new seeded world.
With 64 entries each contributes four bank genomes: one exact copy and three
offspring replicas. Brains are never averaged or merged. Equal representation
prevents one archive entry dominating transfer; related or identical brains can
still occur. The archive limit of 64 is a design choice.
Energy, age, inventory, signals, and private state reset. Genes retain inherited
changes. The user’s final physical settings carry forward.

This external serial transfer is an authored experimental protocol, not literal
uninterrupted natural evolution. It selects for late survival, which need not
maximize reproduction, diversity, or adaptation. A lone survivor can seed the
next world. There is no automatic population rescue inside a world and no
automatic difficulty increase between worlds.

## Ancestry and evidence

The inspector’s ancestry depth counts births inside the current world. Across-world
ancestry can be reconstructed through `founder_family`, sampled source bodies,
and `transfer.json` parent mappings. Adding each world’s maximum depth is wrong:
the deepest family may not be the one that was carried forward. External exact
copies and mutated replicas should be reported separately from biological births.

Longer survival is one observation, not proof of better brains. Track population,
births, successful feeding, recovery after bottlenecks, and completed journeys.
Action selection and successful execution are different measurements. Emission
does not establish useful communication; displacement does not establish cooperation.

For a comparison, freeze both an earlier and a later bank, evaluate them on the
same held-out seeds/settings and orientations, and report all results, including
extinctions and runs still alive at the evaluation horizon. Evaluation worlds must
not seed training. Changing difficulty mid-run is a valid play experiment, but it
breaks a simple before/after learning comparison.

The public tree does not ship an allegedly “smartest” bank. Share an interesting
gene pool explicitly with its model, source, settings, and limitations. Preserve
the source checkpoint if you want to preserve the whole experience.
