# Primitive World

Primitive World is an artificial-life experiment about how much adaptive, complex behavior can emerge from a small set of general physical, cognitive, and evolutionary primitives.

The goal is **not to make agents intelligent, cooperative, efficient, human-like, or successful according to our preferences**.

The goal is to build a world with enough expressive freedom that evolution can discover its own strategies, while minimizing the amount of strategy we implicitly author into the simulator.

## North star

**We define a broad, reachable space of possibilities.
We avoid defining which solution is desirable.
The environment determines consequences.
Evolution determines what persists.**

We must author a viable search space, not just a neutral one. Physical budgets,
actuator composition, sensing and inherited variation must make complete life
cycles reachable. We may measure reachability across seeds and deliberately
simple test controllers while designing the world. Such tests never become
founder policies, behavioral rewards, online difficulty adjustments or selection
rules. Longer survival alone is not evidence that the search space is viable.

The simulator should provide possibilities, not preferred solutions.

A good primitive supports many incompatible strategies. Movement may be used to flee, hunt, explore, migrate, follow, orbit, or remain still. Signals may become communication, deception, coordination, noise, or be ignored. Memory and plasticity may become useful, useless, or actively selected against.

When interesting behavior appears, we should ideally be able to say:

**“There is no primitive for that behavior. It emerged from more general primitives.”**

## What we author

We must author the physics of the experiment. This includes things such as:

* energy and material;
* local sensing;
* movement and physical interaction;
* reproduction;
* heredity and blind mutation;
* neural computation and memory;
* local lifetime plasticity;
* environmental dynamics;
* physical costs;
* unavoidable simulation and persistence rules.

These mechanisms should be understandable without reference to whether their consequences are desirable.

Small mutations, bounded values, duplication, inheritance, local learning, and computation costs are acceptable because they define the structure of possibility.

## What we do not author

We should avoid encoding answers to the evolutionary problem.

The simulator should not decide that:

* intelligence is good;
* complexity is good;
* cooperation is good;
* food-seeking is the intended strategy;
* exploration deserves a bonus;
* certain individuals are “best”;
* one developmental sequence should occur before another;
* a particular mutation would help;
* a particular signal has meaning;
* a particular cognitive architecture is the destination.

There should be no behavioral reward function, intelligence score, complexity objective, or hidden optimizer guiding evolution.

## Evolution

Selection should arise primarily from ordinary ecological consequences:

**heredity → behavior → survival/reproduction → descendants → changing population**

Mutation generates possibilities; it does not generate proposals for improvement.

Mutation must be blind to behavior and outcomes.

Where cross-world hereditary continuity is required because early worlds frequently go extinct, it should also be blind: inherited records may persist through a bounded random hereditary pool, but they are never ranked by lifespan, intelligence, behavior, complexity, reproduction count, or any other merit score.

The purpose of such continuity is to avoid erasing accumulated heredity, not to decide which heredity deserves to survive.

## Learning

Agents may adapt during their lifetime, but learning must not receive an authored definition of success.

Plasticity should depend on locally available neural or physical information, not an external reward such as “food was good” or “reproduction succeeded.”

Acquired lifetime state is not inherited. Offspring inherit the machinery and predispositions for learning, not their parent's learned weights, traces, or recurrent memories.

This keeps lifetime adaptation and evolutionary adaptation distinct.

## Complexity

Capability should be available, but never free merely because we want complex agents.

Larger brains, active neural units, memory changes, movement, signaling, reproduction, and other physical processes should have costs where appropriate.

If simple organisms outperform complex ones, simplicity is a valid result.

If plasticity evolves toward zero, that is a valid result.

If signaling is ignored, that is a valid result.

If every lineage eventually goes extinct, that is also a valid result.

We are studying what evolution chooses, not trying to force an interesting answer.

## Information given to agents

Prefer physical measurements over interpretations.

Agents should ideally observe things like state, local fields, relative positions, motion, and changes in their own physical state.

Be cautious about channels that tell an agent why something happened or label an event according to our interpretation.

A useful rule is:

**Expose state and consequence; avoid exposing meaning.**

## Environment

The environment should behave according to general dynamics rather than a curriculum.

Do not deliberately make the world easy first and harder later so evolution can learn in an order chosen by us.

Dynamic geography, seasons, scarcity, motion, and environmental variation are fine when they are properties of the world itself and operate without reference to agent progress.

## Observation

Research tools may measure anything:

* survival;
* ancestry;
* behavior;
* communication;
* complexity;
* learning;
* generalization;
* migration;
* diversity;
* ecological statistics.

But observations must have **no causal path back into evolution**.

Reports, UI metrics, diagnostics, experimental forks, and analysis tools never decide reproduction, mutation, hereditary retention, founder selection, or world reset behavior.

**Evaluation may measure anything. Evolution sees none of it.**

## Engineering constraints

Implementation limits are not automatically biological laws.

GPU capacity, buffer sizes, counter limits, save limits, logging limits, and tick horizons must not silently become selection pressures.

If an engineering ceiling is reached, treat it explicitly as an engine limitation unless that constraint was intentionally designed as part of the world's physics.

## Decision test

When considering a new feature, ask:

1. **Does it increase the space of possible strategies, or does it encourage a particular strategy?**
2. **Does it behave differently because an organism was “successful”?**
3. **Does it expose physical information, or our interpretation of that information?**
4. **Could several very different behaviors make use of the same primitive?**
5. **Would the rule still make sense if all behavioral names were replaced with meaningless symbols?**
6. **Is this biology/world physics, or an engineering convenience accidentally becoming biology?**

Prefer mechanisms that increase **compositional capability** rather than mechanisms that directly make recognizable behavior easier to obtain.

## The standard

Primitive World is successful as an experiment when surprising behavior can emerge without that behavior already existing, semantically or strategically, in the source code.

We are not trying to make evolution succeed.

We are trying to create a sufficiently expressive world in which evolution is free to decide what success becomes.
