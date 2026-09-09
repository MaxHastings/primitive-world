# Primitive World

**A living artificial-life experiment for your Windows desktop.**

Primitive World is a local, GPU-powered ecology where small neural agents must
find food, spend energy, reproduce, communicate, and survive changing terrain.
There is no pretrained model, cloud service, or authored strategy—just a world
whose consequences decide what persists.

[![Primitive World running as a living desktop background](docs/images/primitive-world-demo.gif)](assets/primitive-world-demo.mp4)

*Autoplaying preview — click for the full 1080p video. [Watch the original Reddit post.](https://www.reddit.com/r/ArtificialInteligence/comments/1waf33e/running_an_ecology_survival_simulation_experiment/)*

## Make it your wallpaper

Primitive World currently targets **Windows**. Install [Rust 1.93.1 or newer](https://www.rust-lang.org/tools/install), use a GPU/driver with wgpu compute support, then clone this repository and run:

```powershell
.\Play.cmd --wallpaper
```

That builds the current source into `target/play/release/primitive_world.exe`
and attaches a fixed-camera habitat to the desktop. A
small HUD shows population and world state; use its menus to change the lens or
biological speed. Click empty habitat to add food, and use the tray menu to
pause or quit.

To have the newest saved experiment resume when you sign in:

```powershell
.\Play.cmd --install-startup
```

Resume manually with `.\Play.cmd --wallpaper --resume`. After updating the
source, use `Play.cmd` again to rebuild the executable used at login.
Remove the startup entry with `.\Play.cmd --uninstall-startup`. Wallpaper mode uses one
native desktop host, so multi-monitor layouts should be verified on the target
machine. The detailed [wallpaper guide](docs/play.md) covers saves, controls,
desktop behavior, and recovery.

For a regular window instead, run `cargo run --release` or double-click
`Play.cmd`.

## What is happening in the world?

Each tick is simulated on the GPU. Food grows and shifts across a bounded world;
agents wrap to the opposite edge when they cross it, sense only their local neighborhood, update private memory, choose an
action, and pay its physical cost. Food, energy, aging, movement, contact, and
reproduction are ordinary world rules—not rewards for a hidden policy.

```text
changing habitat → local perception → recurrent controller → physical action
       ↑                                                        │
       └──── food, energy, bodies, signals, births, and deaths ─┘
```

The available actions are deliberately primitive:

- gather and digest food;
- move, reproduce, and pass inventory to a nearby body;
- push a nearby body; and
- emit a signed scalar that only nearby agents can perceive next tick.

None of those actions carries a built-in meaning such as “help,” “attack,” or
“food is here.” If a pattern emerges, it has to emerge from the agents’ local
information, memory, and the ecology.

## Inside an agent

Every body has the same 16-unit potential gated-recurrent substrate: **108 input
slots (101 active local measurements) → 1–16 active recurrent units → 20 outputs**
(**2,612 inherited controller values**). A heritable active mask decides capacity;
inactive units are inert.
Recurrent state, traces, and learned connection deltas are private to one life
and reset at birth. Inherited local plasticity can alter only active connections
through local activity, and each actual update pays energy.

Inputs include energy, nearby food and bodies, changes in their own energy and inventory, coarse
near/far food regions, and the nearest neighbor in each of eight directions.
Outputs select an action, movement, amount, contact target and displacement, or
the value of a signal. This is a compact controller with no global map, lineage
score, scripted food-seeking, online optimizer, curriculum, or semantic communication
channel. Read the exact [agent interface](docs/agents.md).

All modes use model `primitive-v32-raw-physical-reservoir`. Saves from other model
layouts are incompatible; start a new world. Current saves retain inherited
controllers and lifetime learning so the same experiment can resume.

At 32x, playback requests 1,920 ticks/s; actual throughput depends on population,
rendering, and GPU load. See [performance](docs/performance.md) for measurement limits.

## A viable search space without a scoreboard

We define a broad, reachable space of possibilities. We avoid defining which
solution is desirable. The environment determines consequences; evolution
determines what persists. Read the [core direction](docs/direction.md).

Selection happens through physical survival and reproduction. There is no
lifespan contest, behavioral reward, population ranking or optimizer. Successful
births place complete inherited records into a fixed 4,096-entry pool by blind
random replacement. After natural extinction, fresh bodies sample unchanged
records from that pool. Lifetime learning is never inherited.

Gathering can compose with movement, reproduction and other actions; it remains
controlled by the organism. Brains receive body state, changes in their own body
state, movement, local fields, and nearby signals—not labels such as “collected,”
“successful,” or “received.” Environmental dynamics operate from tick zero
without a curriculum. Body upkeep alone rises from .01 to .06 over a fresh
world's first 50,000 ticks, providing limited founding runway without granting
food or energy. Extinction remains a valid outcome; a world
that closes off nearly every viable life cycle is a design problem to investigate.
Read the exact [inheritance protocol](docs/evolution.md).

## Explore and contribute

- [Play, controls, saves, and wallpaper limitations](docs/play.md)
- [Ecology and physical rules](docs/world.md)
- [Evolution protocol](docs/evolution.md)
- [Headless experiments and evidence limits](docs/observing.md)
- [Performance notes](docs/performance.md)
- [Contributing and verification](CONTRIBUTING.md)

For unattended or reproducible experiments, the same simulation can run without
rendering:

```sh
cargo run --release -- --headless --seed 42 --ticks 200000 --sample 1024 --output reports/evolution.json --save-checkpoint reports/evolution.checkpoint
```

Generated runs, reports, checkpoints, and build products stay local; only the
source and curated documentation belong in the repository.

For a resumable unattended test, see [long-run testing](docs/long-run.md).
Start a fresh experiment for this model; older saves remain untouched.
