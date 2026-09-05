"""Read-only reset-state v2 choice assay. Not fitness, learning or migration proof."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

ACTIONS = ["none", "collect", "transfer", "force", "emit", "reproduce"]


def inputs(energy, inventory, food):
    x = [0.0]*63
    x[0], x[1], x[2], x[3] = energy/100, inventory/8, food, .05  # adult age500
    for k in range(8):
        direction = [(0, -1), (1, 0), (0, 1), (-1, 0)][k % 4]
        radius = 4 if k < 4 else 24
        x[15+3*k:18+3*k] = [food, direction[0]*radius/24, direction[1]*radius/24]
    return x  # no neighbors, feedback, cooldown or previous motion


def decide(genome, x):
    # Float64 scalar surrogate of a reset-state GPU decision, not bitwise replay.
    assert len(genome) == 1518 and len(x) == 63
    hidden = [math.tanh(genome[h*80+79] + sum(genome[h*80+k]*x[k] for k in range(63))) for h in range(16)]
    output = [genome[1280+o*17+16] + sum(genome[1280+o*17+h]*hidden[h] for h in range(16)) for o in range(14)]
    action = max(range(6), key=lambda i: output[i])
    motor = [math.tanh(4*output[6]), math.tanh(4*output[7])]
    norm = math.hypot(*motor)
    movement = [v/max(1, norm) for v in motor]
    scores = sorted(output[:6], reverse=True)
    return action, 1.2*math.hypot(*movement), scores[0]-scores[1]


def inspect(path):
    raw = path.read_bytes()
    bank = json.loads(raw)
    assert bank["model"] == "physiology-v2" and bank["version"] == 3
    assert bank["genomes"] and all(len(g) == 1518 for g in bank["genomes"])
    assert all(math.isfinite(v) and abs(v) <= 4 for g in bank["genomes"] for v in g)
    conditions = []
    for energy in [10, 50, 100]:
        for inventory in [0, .5, 1, 2]:
            for food in [0, .02, .2]:
                decisions = [decide(g, inputs(energy, inventory, food)) for g in bank["genomes"]]
                conditions.append(dict(energy=energy, inventory=inventory, uniform_food=food,
                    action_counts={name: sum(d[0] == i for d in decisions) for i, name in enumerate(ACTIONS)},
                    predicted_adult_speed_mean=statistics.fmean(d[1] for d in decisions),
                    predicted_adult_speed_median=statistics.median(d[1] for d in decisions),
                    near_tied_actions=sum(d[2] < 1e-5 for d in decisions)))
    return dict(bank=str(path), bank_sha256=hashlib.sha256(raw).hexdigest(), bank_name=bank["name"],
                conditions=conditions,
                scope="Post-hoc reset-state numerical assay only. Float64 surrogate, not guaranteed GPU bitwise decisions. Hidden state, neighbors and previous feedback zero; adult age500, gain4, fixed compass point sensing of a uniform field. These counterfactual inputs are not full ecological episodes. No results enter control, founder selection or campaign continuation. A useful-looking choice change is not proof of learned survival or migration.")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("banks", type=Path, nargs="+")
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    results = [inspect(path) for path in args.banks]
    with args.output.open("x", encoding="utf-8") as stream:
        json.dump(results, stream, indent=2)
    for result in results:
        selected = [c for c in result["conditions"] if (c["energy"], c["inventory"], c["uniform_food"]) in [(100, .5, .02), (10, 0, .2), (50, 0, 0)]]
        print(json.dumps(dict(bank=result["bank_name"], selected_conditions=selected)))


if __name__ == "__main__":
    main()
