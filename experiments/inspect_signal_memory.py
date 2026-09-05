"""Read-only event-input pulse assay. Sensitivity/retention are not communication utility."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics
from inspect_bank_choices import inputs


def response(genome, sensory, pulse, steps=64):
    projection = [genome[h*80+79] + sum(genome[h*80+k]*sensory[k] for k in range(63))
                  for h in range(16)]
    recurrent = [genome[h*80+63:h*80+79] for h in range(16)]
    hidden = [0.0]*16
    trace = []
    for step in range(steps):
        event = pulse if step == 0 else 0
        hidden = [math.tanh(projection[h] + genome[h*80+12]*event
                           + sum(w*v for w, v in zip(recurrent[h], hidden))) for h in range(16)]
        outputs = [genome[1280+o*17+16] + sum(genome[1280+o*17+h]*hidden[h] for h in range(16))
                   for o in range(8)]
        motor = [math.tanh(4*outputs[6]), math.tanh(4*outputs[7])]
        norm = max(1, math.hypot(*motor))
        trace.append((hidden, [1.2*v/norm for v in motor], max(range(6), key=lambda i: outputs[i])))
    return trace


def inspect(path):
    raw = path.read_bytes()
    bank = json.loads(raw)
    assert bank['model'] == 'physiology-v2' and bank['version'] == 3
    cases = []
    for energy, inventory in [(10, 0), (50, 0), (50, 2)]:
        sensory = inputs(energy, inventory, 0)
        differences = {pulse: {step: [] for step in (1, 2, 4, 8, 16, 32, 64)} for pulse in (-.5, .5)}
        for genome in bank['genomes']:
            assert len(genome) == 1518 and all(math.isfinite(v) for v in genome)
            control = response(genome, sensory, 0)
            for pulse in differences:
                changed = response(genome, sensory, pulse)
                for step in differences[pulse]:
                    a, b = control[step-1], changed[step-1]
                    differences[pulse][step].append((max(abs(x-y) for x, y in zip(a[0], b[0])),
                                                     math.dist(a[1], b[1]), a[2] != b[2]))
        for pulse, delays in differences.items():
            cases.append(dict(energy=energy, inventory=inventory, pulse=pulse,
                measurements=[dict(update=step, mean_max_hidden_delta=statistics.fmean(d[0] for d in values),
                    maximum_hidden_delta=max(d[0] for d in values), mean_motor_delta=statistics.fmean(d[1] for d in values),
                    maximum_motor_delta=max(d[1] for d in values), action_changes=sum(d[2] for d in values))
                    for step, values in delays.items()]))
    return dict(bank=bank['name'], bank_sha256=hashlib.sha256(raw).hexdigest(), genomes=len(bank['genomes']), cases=cases,
        scope='Float64 direct recurrent-state diagnostic: adult age500, bare field, empty initial state, no neighbors, identical fixed sensory inputs except one +/-0.5 pulse on event input12 at update1. Afterward event is zero; movement/action feedback and body physiology are deliberately held fixed to isolate private recurrence. This shared event channel also carries transfer/force feedback. A response is not proof of a learned message convention, useful memory, truth, deception, or live-world communication benefit. No genes or simulation state are modified.')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('banks', type=Path, nargs='+')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    results = [inspect(path) for path in args.banks]
    with args.output.open('x') as stream:
        json.dump(results, stream, indent=2)
    for result in results:
        print(json.dumps(dict(bank=result['bank'], cases=[c for c in result['cases']
            if c['energy'] == 50 and c['inventory'] == 2 and c['pulse'] == .5])))


if __name__ == '__main__':
    main()
