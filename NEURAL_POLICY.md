# Archived learned-policy contract

The active project direction is now an inherited local controller. This GRU
remains available through `--neural` as a comparison and historical experiment;
it is not the default evolutionary mechanism.

The simulator has one physical world and two decision implementations. The
authored controller is a readable control for debugging the world. The learned
controller is a shared GRU with one private state per individual. A live run
does not update weights. It only changes the individual's hidden state through
what that individual can sense and what the body actually does.

The policy is an archived experiment, not part of the active evolutionary
kernel. It remains useful for numerical parity and controller comparisons, but
its older observation schema contains reserved social slots that are zeroed by
the current raw-perception pass. Do not use this path as evidence for social
emergence.

## First principles

1. **Physics comes first.** Energy, carried food, movement cost, metabolism,
   harvest, eating, death, birth, and resource growth are the causes of every
   learned consequence. Ground food is not carried food. Carried food is not
   energy until an eat action converts it.
2. **Perception is local.** The policy receives bounded samples and summaries;
   it never receives a map, population-wide food totals, slot ids, private
   reserves of another agent, or an observer's group labels.
3. **Memory is private and causal.** The GRU sees the current body and local
   situation, then retains a compressed trace of what it experienced. Births
   and deaths clear that state. Recycled slots cannot inherit another life.
4. **Actions are affordances.** An action is masked when the body or nearby
   world cannot perform it. Transfer, force, and emit use only locally visible
   raw-body candidates; the policy receives no global relationship map.
5. **Reproduction is the selection boundary.** Survival keeps an individual in
   the world. Reproduction is what lets a behavioral policy persist across
   generations. We will not reward a desired group shape, migration distance,
   or social label.

## Version 3 contract

The exported model is `version: 3`. It has a 32-unit GRU, 24 observations, and
14 action logits. The recurrent gate order is the same as
`torch.nn.GRUCell` and the Rust/WGSL inference path.

Observations at each eight-tick decision are:

| slots | signal |
| --- | --- |
| 0–1 | own energy and carried food, normalized |
| 2 | food underfoot, including dropped food |
| 3–10 | food samples at eight directions at sensory radius |
| 11 | local occupancy density |
| 12–15 | four clipped boundary distances |
| 16–21 | reserved compatibility slots; zero under the active raw-perception pass |
| 22–23 | previous body velocity, normalized by maximum speed |

The last two values are proprioception. Without them, a policy cannot tell the
difference between choosing a direction and successfully moving in that
direction. Social values are observations of experienced consequences, not
instructions to cooperate or attack.

Actions are `wait`, `collect`, `ingest`, eight movement headings, `transfer`,
`apply force`, and `emit`. The choice is held for eight physical ticks so a
direction has time to produce a visible trip. If a held action becomes
impossible, the body waits; the policy is not silently replaced by an authored
score.

The policy can select transfer, force, or emit only when the local raw-body pass
exposes a target and the corresponding affordance is enabled. Transfer moves
food, force spends energy and can spill food, and emit creates a bounded local
signal. These consequences do not update relationship evidence.

## What is and is not learned today

The bridge already runs the exact GPU physics used by the application and
records pre-action observations, masks, logits, hidden state, executed action,
body reserves, and death cause. Python checks recurrent numerical parity with
the shader. Only the current v3 contract is loadable; old experiments are
kept as historical evidence outside the runtime.

Training uses the same ordinary world reset as the application: the same
habitat, weather, crowding, social passes, births, and death rules. The headless
bridge adds exact readback and diagnostics; it does not create a second ecology.
Controlled food replacement remains an explicit observer intervention for a
focused experiment, never an implicit training fixture. A short run can still
fail to exercise reproduction, so long-term claims require episodes long enough
to cross that lifecycle boundary.

Survival feedback is the default experiment. Physiological reserve scoring is
available only when explicitly requested because it encourages hoarding and
does not transfer cleanly to ordinary-world reproduction. Neither is a target
number; they are ways to inspect which consequences the policy currently uses.

## Reading an extinction

Use the three food quantities separately:

- **vegetation** is renewable ground resource;
- **dropped food** is ground inventory released by death or force;
- **carried food** is an individual's inventory.

Only carried food can be eaten. The bridge reports `ground_food_observed`,
`food_before`, `energy_before`, `executed_action`, and `death_cause` per life
sequence. If vegetation or dropped food is present while agents starve, the
first question is whether the policy reached it and then chose `harvest`, and
whether it later chose `eat`. That is a policy/observation failure, not proof
that the resource accounting is wrong.

## Reproduce

```powershell
cargo build --release
python -m pip install -r training/requirements.txt
python training/train.py --updates 80 --population 128 --steps 192
python training/train.py --evaluate-only --load policies/forager-v3.json --output policies/evaluation-v3.json
cargo run --release -- --neural --neural-greedy
cargo test -- --test-threads=1
```

The evolved local controller is the default. The v3 GRU remains opt-in through
`--neural` or `--neural-weights` and should not be interpreted as evidence for
multi-generation behavioral evolution.
`--neural-weights PATH` selects one v3 model. Use sampled and greedy runs as
different observations, not as optimization targets.
