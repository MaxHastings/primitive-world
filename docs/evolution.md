# Evolution, without a behavior checklist

The simulator’s native `--watch-loop` carries survivor genomes between worlds.
All modes share the same controller and physics, with random founder weights
and no authored starter policy.

## Within a world

Brains choose actions and continuous outputs from local senses and private state.
Their weights do not learn by gradient descent during life. A birth copies the
parent’s current weights, with approximately 2% mutation probability per weight,
an additive change in [-0.03, 0.03], and clipping to [-4, 4]. The child’s private
state starts empty. Useful behavior can spread when its carriers leave descendants.

There is no loss function, survival reward buffer, action-use bonus, or requirement
to communicate, fight, cooperate, or migrate. In-world reproduction is chosen and
must be paid for. Extinction is allowed.

## Between worlds

Every 128 ticks the observer captures up to 64 hash-sampled living bodies and their
**current** genomes. It replaces its previous sample only when bodies are alive.
At extinction it keeps the latest nonempty sample, not an original ancestor’s
weights and not necessarily the exact final 64 deaths.

Each sampled genome is copied unchanged once. Balanced mutated replicas fill a
256-genome bank using the same mutation probability/range as births, with an
explicit versioned PRNG. The bank seeds fresh bodies in a new seeded world.
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
