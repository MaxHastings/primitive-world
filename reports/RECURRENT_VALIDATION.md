# recurrent-v1 cutover validation

Date: 2026-09-04. Architecture delivered; prepared founder bank released.
This is a finite research-model release, not evidence of organized routes,
autonomous discovery from a blank slate or long-term population equilibrium.

## Implementation delivered

One inherited recurrent controller with 64 raw inputs, 16 private state values,
16 outputs and 1,568 weights per body. Direct movement, controllable sensor
orientation, chosen body action/target/amount/payload and explicit reproductive
investment. Weights mutate at birth; state changes during life.

Removed active candidate scoring, place/relation memory, travel commitments,
automatic birth lottery, shared-GRU/bridge/trainer, old founder banks and
diagnostic runtime branches. The old revision is preserved by
`pre-recurrent-cutover` (commit `2a46eb1`). Existing user .pt files and old
checkpoints/reports were not deleted. No GUI was controlled or restarted.

Ecology and its ordinary default rates were retained. Capacity changed from
100,000 to 16,384 to accommodate independent genomes. Cold genome storage is
102,760,448 bytes (98 MiB), separate from hot body records. Putting the entire
genome in hot body records initially caused a GPU shader/device failure; that
implementation was replaced before any registered campaign. The failed local
wiring report is not evidence of a completed run.

## Registered campaign

[Registered plan](recurrent-cutover-20260904/plan.json) and
[complete compact summary](recurrent-cutover-20260904/summary.json).

The plan was written before its first simulation process. Budget: two
preparation worlds (seeds 11,22), three held-out worlds (101,202,303), one
no-force control (101) and one famine stress world (303), each at most 12,000
ticks. Exactly 84,000 ticks ran, with no retries, seed shopping, rescue,
reseeding, policy tuning between runs or feedback from evaluation into the bank.
Each world began with 1,000 bodies at ordinary settings.

Preparation started from the explicitly declared mutable bootstrap. Stage two
used up to 128 living descendants sampled from stage one at actual abundance.
All evaluation runs used the exact stage-two bank. Reproduction, not an external
reward/fitness ranking, generated the inherited families. This does not
establish that preparation outperforms its starting biases: a matched
unprepared evaluation was not part of this campaign.

The no-force control disables a capability. Famine removes vegetation and
stops regeneration at tick 6,000, restoring growth at 6,500; carried/dropped
supplies remain. It is not equivalent to an unassisted ordinary world.

## Recorded results at tick 12,000

Ancestry is the maximum depth among **living** bodies, counted from each world's
initial founders, not lifetime historical depth across preparation resets.

| World | Living | Births | Max ancestry | Deaths starvation/age/force | Living energy | Carried food |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| prepare-0-seed11 | 457 | 1251 | 14 | 1794/0/0 | 17844.5 | 182.351 |
| prepare-1-seed22 | 390 | 1201 | 13 | 1811/0/0 | 16102.9 | 171.174 |
| evaluate-seed101 | 965 | 3338 | 15 | 3373/0/0 | 41734.1 | 492.816 |
| evaluate-seed202 | 759 | 2109 | 17 | 2350/0/0 | 31838.9 | 341.333 |
| evaluate-seed303 | 986 | 3667 | 17 | 3680/1/0 | 40027.2 | 415.632 |
| control-no-force | 1042 | 3221 | 17 | 3179/0/0 | 44208.6 | 502.968 |
| stress-famine | 817 | 2563 | 15 | 2746/0/0 | 33384.9 | 367.778 |

All seven runs had zero invalid controller outputs. Initial population plus
births minus recorded deaths equals final population in every run.
All three ordinary held-out worlds retained multigeneration descendants beyond
the maximum initial-founder lifespan. Populations remained far below the
16,384-slot storage limit; there was no capacity-based population stabilization.

**No transfer, force or emit action was selected in any of these seven runs.**
Therefore the no-force comparison says nothing useful about conflict costs or
social balance. Its population differs despite zero force in both conditions;
atomic allocation/collection order makes same-seed population runs non-bitwise
deterministic. Do not attribute that difference to a nonexistent fight.

The controller mostly selected collection and ingestion, with relatively rare
explicit reproductive requests. No route completion, memory ablation, group
coordination, useful signaling or within-life weight learning was demonstrated.
The initial low social-action biases and weak local steering remain important
authored dispositions, even though they are mutable weights.

On NVIDIA GeForce RTX 4070 SUPER, Vulkan, driver 591.86, individual measured
simulation/readback spans were 8.68–9.20 seconds for 12,000 ticks; full campaign
wall time was 73.6 seconds including setup/process overhead. These are this
machine's measurements, not a portable performance guarantee.

## Integrity and release checks

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test -- --test-threads=1`: **18 passed**
- `cargo build --release`
- Python preparation runner syntax validation.
- Release `--help` / `--version` identify recurrent-v1/checkpoint 12.
- Removed `--legacy-controller` is rejected with exit code 2.
- Final packaged default: 1,000 living, zero invalid outputs after eight ticks.
- Save and headless resume: tick 8 to 16, same founder/settings identity, zero
  invalid outputs. Empty-world founder export fails without creating a bank.

Tests verify CPU/GPU recurrence agreement, observer-field isolation, immutable
within-life weights, changed state, real sample offsets and locality, controller
attention/amount/payload wiring, locomotion independent of actions, no automatic
birth, physical collection/ingestion/motion accounting, concurrent collection,
disjoint transfers, force expenditure/spill, target incarnation/range checks,
disabled-action arbitration, post-force reproductive reserve revalidation,
child-state reset/incarnation reuse, checkpoint replay/batching, corrupt-file
rejection without live mutation, and safe founder export/default identity.

These conservation tests use isolated physical cases. Campaign reports contain
reserves, production/weather loss, action counts and force expenditure, but do
not close a full long-run energy budget: ingestion/event counters are rounded
and some counters are u32. Force/death drop rounding has a bounded matter
residual. No claim of exact real-number conservation is made.

Rendering pipeline construction/validation runs headlessly. UI logic compiles;
manual window interaction, display layout and interactive save/load clicks
were **not manually tested**. No computer control was used.

## Provenance

Campaign executable/model fingerprint:
`eeb603edd7303bd2522e8b5e3867d275cfe98ce59b1b2bf93db42ca5567cb2e2`.

Final release executable SHA-256:
`6e6e1458e6e5269f87823d751b94ae0ecce084827d8554a914ab525393874122`.
The later packaging embeds the evaluated bank as the default, adds stricter
settings/checkpoint validation, formats UI code and applies mechanical lint
cleanup; the registered campaign used explicit bootstrap/bank paths. No
controller weights, mutation, action physics or ecology were retuned afterward.

Stage-one bank SHA-256:
`30ab2cafa15cb1d057d93504230e8396c505fab1b0ac3443ee2e3d7ad21fd848`.

Stage-two exported bank SHA-256:
`42f3025938e4ffac4b8c1c59b448a5508231024bf64f150173c72da6683191b5`.

Released `policies/recurrent-v1.json` SHA-256:
`b99a0682a3f9bfc4593446a3d297ad3bbc879060e1cb52c8d41d8b864e3edd0a`.
It is the same stage-two JSON plus a terminating LF, not a reselected bank.
The bank has 128 genomes and identifies seed 22/tick 12,000. Filename and model
identity are intentionally incompatible with the removed banks.

Raw run JSON/logs remain local in `reports/recurrent-cutover-20260904/`.
These report hashes identify the recorded inputs and outcomes:
- `prepare-0-seed11.json`: `1c51b2ffd59233eb0323a46a5bd33d5bae5a140c13050ede379ac767ce2c9eac`
- `prepare-1-seed22.json`: `75006a79c9b42580c791d5f293adebd2c0d10728c7a4f2b17775374285671f8e`
- `evaluate-seed101.json`: `00ee8ea3fb538cf99ee07039c0646f56a34101d98ded5edb6e99b7b0547c38c7`
- `evaluate-seed202.json`: `0c9e280c31b8016ea4c17b7160ca8340b239dce58eb8626bc9037b7ff98c7371`
- `evaluate-seed303.json`: `6f4dbbd187cfff8c061483dd9c1fdf792dfd886a37bc465de2587a0065bc5d4b`
- `control-no-force.json`: `0707574fbdbb01eb7626c02709fd489783b815ee954ba44d9acac45f9872d9b9`
- `stress-famine.json`: `ac1b287696f28c4aac52bb560d930422d589cb7acb55f92ed880c48fb40ac2a4`

Reproduce the procedure into a new directory:

```powershell
cargo build --release
python training/prepare.py --directory reports/new-campaign
```

Exact populations are not guaranteed across replays/devices. The committed
plan, summary, source, bank and checks make the bounded claim inspectable.
Future travel or social experiments must be explicitly new questions, not
retroactive edits to this campaign or hidden policy machinery.
