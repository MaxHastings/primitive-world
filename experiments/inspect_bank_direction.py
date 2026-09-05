"""Read-only counterfactual directional assay; not GPU replay or ecological fitness."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

from inspect_bank_choices import inputs


def movement(genome, sensory):
    assert len(genome) == 1518 and len(sensory) == 63
    hidden = [math.tanh(genome[h*80+79] + sum(genome[h*80+k]*sensory[k] for k in range(63)))
              for h in range(16)]
    outputs = [genome[1280+o*17+16] + sum(genome[1280+o*17+h]*hidden[h] for h in range(16))
               for o in (6, 7)]
    motor = [math.tanh(4*v) for v in outputs]
    norm = max(1, math.hypot(*motor))
    return [1.2*v/norm for v in motor]


def probe(energy, inventory, direction, food):
    sensory = inputs(energy, inventory, 0)
    if direction is not None:
        # Place food at near and far probes on only one side; underfoot stays bare.
        for k in (direction, direction+4):
            sensory[15+3*k] = food
    return sensory


def inspect(path):
    raw = path.read_bytes()
    bank = json.loads(raw)
    assert bank['model'] == 'physiology-v2' and bank['version'] == 3
    genomes = bank['genomes']
    assert genomes and all(len(g) == 1518 and all(math.isfinite(v) for v in g) for g in genomes)
    cases = []
    for energy, inventory in [(10, 0), (50, 0), (50, 2), (100, 2)]:
        for food in [.02, .2]:
            for name, direction in [('bare', None), ('north', 0), ('right', 1), ('south', 2), ('left', 3)]:
                motors = [movement(g, probe(energy, inventory, direction, food)) for g in genomes]
                cases.append(dict(energy=energy, inventory=inventory, food_on_probes=food,
                                  food_side=name, mean_vx=statistics.fmean(m[0] for m in motors),
                                  mean_vy=statistics.fmean(m[1] for m in motors),
                                  left=sum(m[0] < -1e-6 for m in motors),
                                  right=sum(m[0] > 1e-6 for m in motors),
                                  mean_speed=statistics.fmean(math.hypot(*m) for m in motors)))
    return dict(bank=bank['name'], path=str(path), sha256=hashlib.sha256(raw).hexdigest(),
                genomes=len(genomes), cases=cases,
                scope='Float64 first-decision surrogate with empty hidden state, no neighbors or prior feedback, adult age500, gain4, speed cap1.2. Side probes are synthetic counterfactual inputs, not worlds. Negative x is screen-left. Does not establish the live cause of extinction or select training survivors.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('banks', type=Path, nargs='+')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    results = [inspect(p) for p in args.banks]
    with args.output.open('x', encoding='utf-8') as stream:
        json.dump(results, stream, indent=2)
    for result in results:
        print(json.dumps(dict(bank=result['bank'], cases=[c for c in result['cases']
            if c['energy'] == 50 and c['inventory'] == 0 and c['food_side'] in ('bare', 'right', 'left')
            and c['food_on_probes'] == .2])))


if __name__ == '__main__':
    main()
