# Experiments

The current whole-model preparation/evaluation runner is
[`training/prepare.py`](../training/prepare.py). It registers a bounded campaign,
keeps preparation separate from held-out worlds, and preserves failure reports.
Use a new output directory for each explicitly planned campaign.

The older two-patch travel/sensing diagnostics tested candidate-v1 and its
authored place/destination machinery. Their driver and runtime branches were
removed with that model. Historical findings remain in `reports/`; their
commands can be reproduced from Git tag `pre-recurrent-cutover`, not this tree.

Do not interpret old travel results as tests of recurrent-v1. New route or
memory studies should observe this model and state their question explicitly;
they must not quietly reintroduce a destination scorer or training reward.
