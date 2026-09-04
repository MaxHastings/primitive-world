# Primitive World: current research plan

This project is an evolutionary ecology experiment, not a behavior generator.
The active contract is [KERNEL_SPEC.md](KERNEL_SPEC.md).

## Architectural law

**Physics may decide what is possible. Physics may not decide what is desirable.**

- The world owns geometry, visibility, possession, range, collision, costs,
  transfers, energy conversion, death, birth allocation, and environmental
  consequences.
- An agent owns its intent: destination, target, action, bounded payload, and
  amount. The agent interprets only local observations and private memory.
- Heredity owns which controller traits persist and how copying mutates them.
- The observer owns labels, metrics, lineage reconstruction, replay, and
  ablations. Observer output is one-way and never becomes agent input or a
  reproductive objective.

Every new rule must answer:

> Can this rule be justified without naming the behavior we hope to see?

If not, it belongs in an explicitly isolated experiment or it does not belong
in the kernel.

## Current kernel

The world contains continuous bounded space, renewable spatially varied food,
fertility, weather, local occupancy, carried food, energy, movement, collection,
ingestion, physical transfer, physical force, bounded local signals, death,
single-parent reproduction, private place memory, and imperfect heredity.

The active action vocabulary is:

`wait`, `move`, `collect`, `ingest`, `transfer`, `apply force`, `emit`.

The default candidate-v1 controller has independent signed, inherited action
weights over local observations and private movement proposals. A transparent
physiological bootstrap starts with standing variation. Offline preparation can
carry actual living descendants into fresh seeds; held-out evaluation never
feeds weights back. Reproduction is the selection boundary. See CONTROLLER.md.

The fixed feature extractor and movement proposals remain architectural biases.
This is finite controller evolution, not open-ended strategy invention. Neither
survival on a few seeds nor attractive trails demonstrate general intelligence.

## What we measure

Observer-only measurements include population and resource histories, births
and deaths, energy and matter flows, action attempts and resolutions, local
signals, lineage branching, genome means and variance, behavioral repertoire,
and environmental interventions.

Novelty, complexity, diversity, ecological activity, and phylogenetic branching
are descriptions of the resulting dynamics. They must not be fed back into the
world as objectives, selection bonuses, or emergency corrections.

## Immediate research priorities

1. Evaluate the signed candidate controller across unseen seeds and stress
   regimes. Compare its fixed feature extractor with a broader recurrent
   architecture only after the latter has credible multigeneration validation.
2. Measure causal effects of the raw event inputs already exposed to the
   candidate controller. Do not interpret a scalar as a shared map or social label.
3. Decide whether reproduction should become an explicit agent intent. The
   current world-side eligibility gate is useful ecology, but it means agents
   do not literally choose reproduction.
4. Keep fixed nonzero mutation unless a physical copying-cost experiment is
   explicitly designed; the previous cost-free fidelity gene has been removed.
5. Remove dormant relationship, guide, report, and archived-controller paths
   from the kernel once their comparison value is no longer needed. Historical
   experiments belong outside the active contract.

## Experimental discipline

- Keep the kernel small and physically auditable.
- Compare treatments across multiple seeds and preserve surprising failures.
- Use controls to explain causes, not to tune toward a desired population
  curve or social pattern.
- Treat extinction, collapse, coexistence, conflict, and recovery as valid
  outcomes until a physical or implementation error is demonstrated.
- Test conservation, locality, generation identity, checkpoint replay, and
  batched-clock equivalence before interpreting behavior.

## Release gate

Before changing the main branch, require:

- `cargo fmt -- --check`
- `cargo check --all-targets`
- `cargo test -- --test-threads=1`
- a release headless smoke run with a saved local report
- a manual GUI pass covering pause, step, reset, selection, intervention,
  checkpoint save/load, evolution inspection, and recent events

The current project is GPU-first and validated on NVIDIA Vulkan. Atomic
ordering means population-scale runs may diverge across devices even with the
same seed; reports are observations, not cross-GPU determinism claims.
