"""Historical V3 authored starter, retained for reproduction of old experiments.

Not recommended for new training: its social actions are unreachable under argmax
until mutation changes the score ordering. Use prepare.py without --initial-bank
for random-origin training. No runtime behavior overrides.
"""
import argparse
import math
from pathlib import Path
import random
import prepare

ROW, BASE = 93, 1488


def starter_genome(rng):
    # Leave all circuits mutable and weakly connected, including unused abilities.
    g = [prepare.f32(v * .08) for v in prepare.random_genome(rng)]
    threshold = rng.uniform(.74, .86)
    investment = rng.uniform(.82, .96)
    near_gain, far_gain = rng.uniform(.45, .75), rng.uniform(.12, .25)
    angle, drift = rng.uniform(0, 2 * math.pi), rng.uniform(.008, .025)
    # Hidden0: reserve threshold. Hidden1..4: near/far local food differences.
    for h in range(5):
        g[h*ROW:(h+1)*ROW] = [0.0] * ROW
    g[0], g[ROW-1] = 2.0, -2.0 * threshold
    for h, positive, negative in [(1, 23, 29), (2, 26, 20), (3, 35, 41), (4, 38, 32)]:
        g[h*ROW+positive], g[h*ROW+negative] = 2.0, -2.0
    # Historical defect: these social logits are unreachable under argmax until
    # sufficient mutation. Preserve exact weights for experiment reproducibility.
    for action in range(6):
        g[BASE+action*17+16] = -.5
    g[BASE+1*17+16] = .5
    g[BASE+5*17+16], g[BASE+5*17] = .5, 2.0
    g[BASE+6*17+1], g[BASE+6*17+3] = near_gain, far_gain
    g[BASE+7*17+2], g[BASE+7*17+4] = near_gain, far_gain
    g[BASE+6*17+16], g[BASE+7*17+16] = drift*math.cos(angle), drift*math.sin(angle)
    g[BASE+8*17+16] = math.log(investment/(1-investment))
    return [prepare.f32(v) for v in g]


def starter_bank(seed=9042603, count=256):
    rng = random.Random(seed)
    bank = prepare.make_bank([starter_genome(rng) for _ in range(count)],
                             f"authored-v3-survival-starter-seed{seed}")
    bank["provenance"] = dict(kind="authored_initializer_not_evolved", seed=seed,
        priors=["collection preference", "reserve-conditioned reproduction",
                "82..96 percent offspring investment actuator", "near/far local food response",
                "small isotropically varied drift", "initially lower social action logits"],
        scope="All entries are ordinary mutable weights. No destination, map, social semantics or runtime override.")
    return bank


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--seed", type=int, default=9042603)
    ap.add_argument("--count", type=int, default=256)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args()
    assert 8 <= args.count <= 256
    prepare.write_new(args.output, starter_bank(args.seed, args.count))
    print(f"Created {args.output}; authored and NOT yet validated")


if __name__ == "__main__":
    main()
