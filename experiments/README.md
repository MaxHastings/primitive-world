# Experiments

The current primitive-v3 experiment is
[`training/founding_ecology.py`](../training/founding_ecology.py), under the
[founding ecology protocol](../training/FOUNDING_ECOLOGY_PLAN.md). It compares
24 independent worlds without external genome selection or breeding. Actual
births carry parent weights with mutation. Output directories must be new.

The older [`training/prepare.py`](../training/prepare.py) adds external family
ranking and breeding across worlds. Its random-origin and authored-starter
campaigns were intentionally stopped; preserve them as historical evidence and
do not automatically resume them.

The older two-patch travel/sensing diagnostics tested candidate-v1 and its
authored place/destination machinery. Their driver and runtime branches were
removed with that model. Historical findings remain in `reports/`; their
commands can be reproduced from Git tag `pre-recurrent-cutover`, not this tree.

Do not interpret old travel results as tests of recurrent-v1. New route or
memory studies should observe this model and state their question explicitly;
they must not quietly reintroduce a destination scorer or training reward.
