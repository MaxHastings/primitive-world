# Evolution, without a behavior checklist

The default viewer and headless runner share one round-based evolution engine.
New Game starts it automatically; Load Game restores its complete training state.
Only new-model saves are accepted. There is no older execution or save protocol.

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

Inside organisms there is no loss function, survival reward buffer, action-use bonus, or requirement
to communicate, fight, cooperate, or migrate. In-world reproduction is chosen and
must be paid for. Extinction is allowed.

## Selection and lifetime records

The separate selector receives only factual lifetime measurements: action counts,
food and energy flows, movement, local observations, reproduction, and death
causes. Missing observations are explicit. Genomes, brain sizes, identities,
archive age, source-world labels, and authored fitness scores never enter its
input API. Stored genomes are opaque material for exact copying after selection.

The population model uses a shared 16-unit encoder, two attention blocks, and a
sequential decoder conditioned on the pool and previously selected copies. The
individual control is a learned linear scorer on the same measurements. Uniform
selection is the untrained control. Learned policies retain uniform exploration
and behavior-independent composition exploration. Useful combinations are not
guaranteed to be learned.

`life-reservoir-v3` measures every individual's lifetime on the GPU and retains up
to 256 individuals through one identity-hash sampling opportunity per individual,
including founders and early deaths. It does not rank behavior. Selected founding
copies receive fresh bodies and reset recurrent memory. Mutation occurs only at
ordinary births; founding slots are shuffled independently of selection order.

## Training in rounds

A round holds one candidate pool fixed across several learning batches. Each batch
proposes multiple founding populations and runs each on the same environment
seeds. Only when every world in the batch has naturally ended does world duration
update the selector. Other populations on each matched seed supply the comparison
baseline. The next batch uses the updated selector and the same pool. A failed
population cannot erase the alternatives being tested.

```sh
cargo run --release -- --train-loop runs/rounds-42 --rounds 8 --random-founders --seed 42 --ticks 200000
```

Without `--candidate-pool`, an initial world runs naturally to extinction to supply
the first pool; it earns no selector reward. Alternatively, supply a completed
`candidates.archive.json` or a round trial's `.archive.json`. Round training rejects
live archives and records of externally removed organisms. It does not invent
missing histories or accept a genome-only bank as measured evidence.

| Option | Default | Meaning |
| --- | --- | --- |
| `--rounds N` | 8 in headless mode | Total rounds, 1-100000; viewer runs until stopped |
| `--batches-per-round N` | 3 | Batches before pool refresh, 1-1000 |
| `--compositions N` | 3 | Populations per batch, 2-32 |
| `--comparison-seeds A,B,...` | `11,22` | 2-32 distinct matched training seeds |
| `--pool-retention N` | 4 | Maximum selectable rounds per record, 1-16 |
| `--ticks N` | 2000 | Total GPU tick budget, including the initial world |
| `--selector-policy` | `population` | `population`, `individual`, or `uniform` |
| `--selector-model PATH` | Fresh weights | Import weights into a new experiment |
| `--selector-frozen` | Off | Evaluate without weight updates |

Defaults mean 18 experimental worlds per round: three batches of three populations
in two environments. The seed list stays fixed throughout training. Evaluation
must use separate seeds and pools. Controls use the same trial structure,
retention rules, and declared simulation budget.

### Pool renewal

At each round boundary, remove the oldest `ceil(pool_size / retention)` records
and all records whose age reaches the retention limit. Admit up to
`ceil(256 / retention)` new records from that round's experimental worlds. At full
capacity with retention four, this replaces 64 of 256 records. Small pools can grow
as candidates become available, or remain below capacity if there are too few
distinct individuals. No synthetic padding copies are added.

Sampling allocates one candidate per contributing world before allocating second
candidates, and so on. Random world order resolves indivisible remainders; unused
allowances from small worlds go to other worlds. Individuals within each world
have random sampling order. A short failed world has the same admission opportunity
as a long one. Neither the selector nor world duration decides retention. The
incoming reservoir is bounded to the next refresh allowance regardless of trial
count or world length.

Selecting copies never renews an old record's admission date. A new organism that
actually lives in another world can contribute its own new record, even with an
identical genome. Provenance includes round, batch, population, seed, and local
identity, so separate worlds cannot accidentally merge matching local IDs.
Measurements and opaque genomes share one ordered mapping through replay and refresh.

### Pausing and resuming

A tick budget or five-minute save point pauses the current world. It receives no
extinction reward, the batch receives no partial update, and the pool stays fixed.
Use a **new** output directory for resume:

```sh
cargo run --release -- --train-loop runs/rounds-42-resumed --round-resume runs/rounds-42/round-state.json --ticks 200000
```

State preserves the pool, incoming sampling tickets and RNG, proposals, completed
durations, weights and optimizer, and current physical checkpoint. Resume cannot
override experiment settings. A completed experiment is final; importing
`selector.json` starts a separate experiment with those weights.

`round-training.json` reports progress, compute use, policy, hardware, settings,
and the current comparison. Per-batch reports contain copy counts, raw durations,
and paired differences. Trial archives and complete/advanced state receipts retain
source mappings. `work-*.state.json` provides recovery points. Completion receipts
can resume around the learner update without applying credit twice. The final
state retains the final pool without refreshing into an additional round.

Keep states with the checkpoint paths they reference. Paths are absolute; moving
only a JSON file does not relocate its checkpoint. `selector.json` alone is not
resumable training state. The backup helper preserves complete game receipts and their checkpoints.

Round training states use format 2; retained pools use format 1. Selector weights
use format 4. Game receipts use format 2 with an embedded viewer snapshot in format
1; physical checkpoints use format 20. Older saves and weights are rejected, not
converted. Physical rules and genome encoding are unchanged.

## Single comparison batches

For a single fixed-pool matched batch, use `--single-batch`:

```sh
cargo run --release -- --train-loop runs/matched --single-batch --candidate-pool runs/rounds-42/bootstrap.archive.json --comparison-seeds 101,202 --compositions 3 --ticks 200000
cargo run --release -- --train-loop runs/matched-resumed --comparison-resume runs/matched/comparison-state.json --ticks 200000
```

The default viewer runs rounds without a predetermined round count. Headless
`--train-loop` defaults to eight rounds and stops or pauses at its compute budget.
Viewer saves embed the current archive and training state beside a relative
checkpoint filename; headless resume states reference their paused checkpoint.

## Evaluation

Training reports do not establish improvement. Compare learned and uniform policies
with equal simulation budgets and matching initial sources/settings. Repeat across
independent training seeds. Separately test frozen models on unseen candidate
pools and disjoint environment seeds:

```sh
python tools/benchmark_selector.py --exe target/release/primitive_world.exe --pool runs/held-out/bootstrap.archive.json --population-model runs/rounds-42/selector.json --output reports/selector-comparison --seeds 101 202 303 --ticks 20000
```

An optional `--individual-model` supplies a separately trained linear scorer.
Omitting a model uses an untrained policy; a missing named file is an error.
Reports retain each completed duration and mark unfinished trials as null. Means
and paired differences are available only for complete batches. The tick limit is
a total compute budget, not a separate survival horizon for each world. Resume
unfinished batches before comparing their means. Evaluation worlds must not feed training.

Engineering checks cover fixed pools, failed proposals, equal-world admission,
record expiration, genome/record alignment, gradients, natural extinction, and
resume across credit and refresh boundaries. These establish mechanics, not
learned improvement or sustained evolutionary progress.

## Ancestry and evidence

The inspector’s ancestry depth counts births inside the current world. Across-world
ancestry can be reconstructed through `founder_family`, sampled source bodies,
and the candidate pool and selected-slot mappings in saved round states. Adding each world’s maximum depth is wrong:
the deepest family may not be the one that was carried forward. External exact
copies should be reported separately from biological births.

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
