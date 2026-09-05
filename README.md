# Primitive World

A small artificial-life sandbox with no script for how to survive.

Watch tiny neural-network agents find food, reproduce, exchange signals, move one
another, and sometimes go extinct. Food patches shift. Families inherit mutated
brains. In evolution mode, the last survivors seed another world without closing
the window.

**V3 is the project’s one active model.** This is an experimental, local GPU
simulation—not a claim of general intelligence or guaranteed cooperation.

## Play

Install Rust **1.93.1 or newer** and use a GPU/driver that supports wgpu compute.
Windows is locally tested; other platforms need verification. No Python, account,
model download, or server is required to play.

```sh
cargo run --release
```

On Windows, you can also double-click `Play.cmd`. It builds the current source
before opening it. The window identifies **primitive-v3 / checkpoint 15**.

For an ongoing evolution run, choose a **new** directory:

```sh
cargo run --release -- --random-founders --seed 42 --watch-loop runs/my-first-run --view-speed 16x
```

Worlds end only at extinction. The same window starts the next world with actual
survivor genes and mutated copies. Closing saves and stops. There is no deadline,
automatic difficulty escalation, or hidden population rescue.

**Expect early failures.** Random neural weights are not a competent starter
policy, nor random action sampling. Some agents repeat ineffective actions.
Selection takes generations, and improvement is not guaranteed.

## Things to try

- Start at 1x and click an agent to inspect its real inputs, energy, and decisions.
- Speed up to watch generations, population collapses, and recoveries.
- Change physical costs or food growth to test a population under pressure.
- Save a checkpoint before intervening; compare what happens afterward.
- Watch whether departures from depleted food lead to feeding and offspring elsewhere.

Space pauses; WASD/arrows pan; mouse wheel zooms; Home fits the world; L changes
the information lens. See the [play guide](docs/play.md) for saving and resuming.

## What evolves?

Each agent has **1,760 inherited weights**, **16 private recurrent state values**,
local senses, six discrete action choices, and independent continuous movement.
Weights stay fixed during life; offspring receive small mutations. Memory state
changes during life and resets at birth. Survival and reproduction determine
which lineages remain. There is no migration reward, authored vocabulary, or
handwritten destination planner.

Interesting-looking movement is not necessarily navigation. Sending signals is
not proof of language. We distinguish those questions with observations and
controlled comparisons, not by requiring every ability to be used.

## Learn more

- [Play, controls, and saves](docs/play.md)
- [How evolution and carryover work](docs/evolution.md)
- [The agent’s inputs, memory, and outputs](docs/agents.md)
- [Physical and ecological rules](docs/world.md)
- [Headless observation and evidence limits](docs/observing.md)
- [Contributing and verification](CONTRIBUTING.md)

The repository contains `src/`, `shaders/`, `docs/`, and optional `tools/`.
Your `runs/`, `reports/`, checkpoints, and build outputs are local data, not source.
Retired models, trainers, and research plans live in Git history rather than a
second set of current instructions. See [release status](docs/release.md).
