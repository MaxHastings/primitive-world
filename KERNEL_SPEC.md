# Kernel contract

This file is the boundary that future features must defend.

## World owns

- space, matter, energy, geometry, visibility, possession, collision, and range;
- physical action costs and resource conservation;
- the result of an accepted action;
- death, birth allocation, and persistent environmental change.

The world never asks whether an action is clever, cooperative, novel, complex,
or socially desirable.

## Agent owns

- the complete action intent: destination, target, action, bounded payload, and amount;
- interpretation of local observations and private memory;
- controller state and inherited dispositions.

The current seven-slot decision buffer is the action-intent boundary. Its
physical vocabulary is wait, move, collect, ingest, transfer, apply force, and
emit. Existing report/gift/attack language is compatibility terminology around
some of those intents, not a promise about what agents should do.

## Heredity owns

Reproduction copies signed controller weights with sparse bounded mutation.
The mutation process has fixed nonzero error; there is no free fidelity trait.
Private memories are not copied. Lineage identifiers and parent links are
observer-only and never influence decisions.

## Observer owns

Metrics, labels, event history, lineage reconstruction, replay, ablation,
shadow populations, and experiment comparison. Observer output is one-way: it
must never be read by an action, birth, mutation, or ecology shader.

## Extension rule

Any proposed mechanism must answer:

> Can this rule be justified without naming the behavior we hope to see?

If not, it is an optional experimental extension or an observer classifier.
It does not belong in the kernel.
