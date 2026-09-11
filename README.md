# Primitive World

**An evolving artificial-life experiment that lives on your Windows desktop.**

Small neural organisms forage, spend energy, release reproductive packets, and
produce new generations in a changing landscape. Their starting brains are random.
Movement, memory, lifetime learning, food transfer, pushing, and local signals
give evolution ingredients to work with; no controller is given a strategy for
finding food, choosing a mate, or caring for offspring.

Leave a world running, inspect an individual, or follow an experiment across
generations and extinctions. The simulation runs locally on your GPU, with no
pretrained model or cloud service.

[![Primitive World running as a living desktop background](docs/images/primitive-world-demo.gif)](assets/primitive-world-demo.mp4)

*Click the preview for the full 1080p video. [Original demo post](https://www.reddit.com/r/ArtificialInteligence/comments/1waf33e/running_an_ecology_survival_simulation_experiment/).*

## Get started

You need **Windows**, [Rust 1.93 or newer](https://www.rust-lang.org/tools/install),
and a GPU/driver that supports the application's wgpu compute requirements.
Install the Windows C++ build tools if prompted by the Rust installer.

Clone or download this repository, open PowerShell in its folder, and build:

```powershell
.\Build.cmd
```

To build and launch wallpaper from the terminal, use `.\Play.cmd --wallpaper --resume`.

Then choose how to play:

| Launch | What happens |
| --- | --- |
| Double-click **Primitive World.exe** | Opens as wallpaper and resumes the newest compatible save, or creates an experiment if none exists |
| Double-click **Play.cmd** | Builds and opens a regular window with **New Game / Load Game** |

Wallpaper runs behind your desktop icons. It does not need a terminal to stay
open. Its habitat fits the desktop host; resuming a save with different dimensions
creates a separate descendant experiment and preserves the original save.

To update from source, choose **Save and quit** from the tray menu before
rebuilding. The executable can also be copied to a permanent folder and run there.

### Wallpaper controls

Right-click the tray icon in the Windows notification area—it may be inside the
**^** overflow—for **Pause / Resume**, **Save now**, **Open saves folder**, and
**Save and quit**. Double-clicking the tray icon also toggles pause.

- Use the desktop HUD to change the view, simulation speed, or inspect **Details**.
- Enable the **Food** paintbrush to add food on empty desktop space. Drag to paint;
  adjust brush size and density in the HUD. Painting starts disabled.
- Enable **Start this copy with Windows** in the tray menu to resume at sign-in.
  Startup is opt-in; move the executable to its permanent location first.

Food painting is an intervention in the current experiment. It preserves its
organisms and history, and the added food and intervention are recorded in saves.
Turn painting off to restore normal desktop selection gestures.

### Windowed controls

| Control | Action |
| --- | --- |
| Space | Pause / resume |
| WASD / arrow keys | Pan |
| Mouse wheel | Zoom |
| Home | Fit the world |
| L | Change the information lens |
| Click an organism | Inspect its energy, senses, actions, gates, and memory |
| Esc | Open the menu |

See the [play guide](docs/play.md) for all controls, save behavior, and desktop
limitations, including multi-monitor considerations.

## What makes it interesting?

**Reproduction is a physical encounter.** Organisms spend energy manufacturing
packets with inherited, evolvable sizes. Packets drift, decay, and can be pushed.
Two packets from different producers must meet to fuse; their remaining energy
pays for construction and provisions the offspring. There is no mate button or
scripted courtship.

**Childhood has a cost.** Actual offspring begin as juveniles and mature at 1,800
ticks. Gathering starts at 1% of adult ability and rises with age; storage grows
too. Larger parental investment and ordinary food transfer can support survival.
There is no automatic feeding or built-in care policy. Fresh worlds start with
8,192 mature founders by default to bootstrap the population.

**Brains remember and can change during life.** Each organism has a gated
recurrent controller and inherited local-plasticity rules. Memory and learned
connection changes belong to that lifetime; newborns inherit the machinery,
not their parents' acquired memories. Expressed neural units and state changes
consume energy.

**Signals have no assigned meaning.** An organism can emit a signed scalar that
nearby organisms sense on the following tick. There are no words, recipient IDs,
or prescribed meanings such as “food here.” Transfer and pushing are similarly
general actions. Whether they become useful coordination, exploitation, noise,
or go unused is part of the experiment.

**The landscape changes independently of the population.** Food responds to
water, minerals, detritus, substrate, and climate varying over local, regional,
and global timescales. Fresh default worlds begin with favorable climate
conditions, then transition into ongoing variation. Weather does not adjust to
reward agent progress. Movement and local interactions wrap across world edges.

**Extinction does not erase all hereditary history.** Births contribute inherited
records to a 4,096-entry pool. After natural extinction, a new world draws founders
from it. The current pool-retention rule explicitly favors deeper within-world
reproduction; founder sampling is uniform. This is an authored selection
preference, not purely ecological selection. No behavioral reward trains the
controllers. See the [evolution protocol](docs/evolution.md).

Patterns of movement, encounter timing, packet investment, juvenile survival,
signaling, and transfer are things to investigate—not guaranteed milestones.
The project does not claim demonstrated intelligence, communication, parental
care, or indefinite survival.

## Under the hood

The application is written in **Rust**, with **wgpu/WGSL** simulation kernels and
an **egui** viewer. Agents and ecology advance on the GPU each biological tick.

| Component | Current model |
| --- | --- |
| Senses | 107 local physical inputs; 16 body-relative area samples |
| Brain | 1–16 expressed gated recurrent units; 2,494 inherited neural parameters, plus hereditary traits |
| Lifetime adaptation | Recurrent state, activity traces, and local learned-weight deltas |
| Outputs | 14 outputs controlling movement, gathering, transfer, pushing, signals, and packet production/placement |
| Ecology | 512 × 512 resource grid with ongoing climate and soil dynamics |
| Entity capacity | 16,384 shared organism/packet slots |

Movement and gathering can accompany a primary action; transfer, pushing,
signaling, and packet production compete for that primary action. Digestion is
automatic. Agents receive local physical measurements, not neighbor identities,
global maps, or labels explaining whether an action succeeded.

At storage capacity, packet requests that cannot fit are skipped without charge;
fusion can reuse a consumed packet slot. Accounting limits can also start another
world, separately from natural extinction. These are implementation limits, not
evolutionary outcomes. The [agent interface](docs/agents.md) and
[world rules](docs/world.md) describe the mechanics in detail.

## Saves and long runs

Changed state autosaves every five minutes. **Save now** and **Save and quit** are
available from the tray. Saves include live organisms, memory and learning,
ecology, hereditary state, and recent world history.

The default Windows save location is:

```text
%LOCALAPPDATA%\PrimitiveWorld\experiments
```

The library retains six recent snapshots per experiment and applies a 16 GiB
cleanup budget while preserving each experiment's newest valid snapshot. Use
**Open saves folder** to find yours. For backups, keep each `.json` receipt and
its matching `.checkpoint` together; see [saving and recovery](docs/play.md#saving).

All modes currently use model `primitive-v45-depth-retention`, checkpoint format
58, and founder-bank format 21. Incompatible biological layouts are rejected
rather than silently converted; older saves are not rewritten into this model.

### Run without a viewer

For reports and unattended experiments, run the same biology headlessly:

```powershell
cargo run --release -- --headless --seed 42 --ticks 200000 --sample 1024 --output reports/evolution.json --save-checkpoint reports/evolution.checkpoint
```

Use new output filenames for each run; existing report/checkpoint destinations
are not overwritten. Headless runs continue across worlds by default;
`--single-world` stops at extinction. Use `--help` for options, or follow the
[headless observation guide](docs/observing.md) and [long-run guide](docs/long-run.md)
for resumable experiments and diagnostics.

### Simulation speed

**1x** targets 60 ticks/second; **32x** targets 1,920; **MAX** uses available
throughput. The speed menu extends through **128x**. Requested speed is not a
guarantee: population, packet activity, rendering, and other GPU work affect the
actual rate. Higher playback speeds do not skip learning, sensing, or ecology.

To reduce rendering load while resuming wallpaper:

```powershell
& '.\Primitive World.exe' --wallpaper --resume --view-fps 30
```

Save and quit an existing wallpaper instance before relaunching with different
options. See [performance measurements](docs/performance.md) for tested workloads
and their limits.

## Explore the project

- [Design principles](docs/direction.md)
- [Agent senses, learning, actions, and juvenile physiology](docs/agents.md)
- [Ecology and physical rules](docs/world.md)
- [Inheritance and cross-world continuity](docs/evolution.md)
- [Observation tools and evidence limits](docs/observing.md)
- [Contributing, project layout, and verification](CONTRIBUTING.md)

Generated runs, personal saves, reports, and build products stay local. The
repository contains source, documentation, and curated demo assets.
