# Historical shared-GRU experiment

The shared GRU, Python trainer, bridge and `--neural` switch were removed from
the active project in the recurrent-v1 cutover. This file is a historical
signpost, not an alternate launch guide.

For the old contract and implementation, inspect Git tag
`pre-recurrent-cutover`. Personal .pt files and old checkpoints were not deleted.
They are incompatible with the current model and are never loaded implicitly.

The current neural controller is **inherited per body**, not a shared policy
trained through the archived GRU pipeline. Its complete contract is
[CONTROLLER.md](CONTROLLER.md). Offline descendant preparation is
`training/prepare.py`, with no machine-learning Python dependencies.
