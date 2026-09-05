"""Check CPU/GPU probe agreement, then summarize measured cue reversals (not fitness)."""
import argparse
import hashlib
import json
from pathlib import Path
import statistics
from inspect_bank_direction import movement, probe


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check(path):
    report = json.loads(path.read_text())
    bank_path = Path(report['bank_path'])
    bank = json.loads(bank_path.read_text())
    assert bank['name'] == report['bank_name']
    worst = 0.0
    cases = []
    for case in report['cases']:
        direction = dict(bare=None, north=0, right=1, south=2, left=3)[case['food_side']]
        sensory = probe(case['energy'], case['inventory'], direction, case['food_on_probes'])
        assert len(case['motors']) == len(bank['genomes'])
        for genome, actual in zip(bank['genomes'], case['motors']):
            predicted = movement(genome, sensory)
            worst = max(worst, *(abs(a-b) for a, b in zip(actual, predicted)))
        cases.append({k: v for k, v in case.items() if k != 'motors'} | dict(
            mean_vx=statistics.fmean(m[0] for m in case['motors']),
            mean_vy=statistics.fmean(m[1] for m in case['motors']),
            left=sum(m[0] < -1e-6 for m in case['motors']),
            right=sum(m[0] > 1e-6 for m in case['motors'])))
    assert worst < 2e-5, f'CPU/GPU surrogate mismatch: {worst}'
    sequences = []
    for sequence in report['sequences']:
        assert len(sequence['steps']) == 128
        phases = []
        for steps in (sequence['steps'][48:64], sequence['steps'][112:128]):
            average = [statistics.fmean(step['motors'][i][0] for step in steps)
                       for i in range(len(bank['genomes']))]
            side = steps[0]['food_direction']
            assert all(step['food_direction'] == side for step in steps)
            phases.append(dict(food_side='right' if side == 1 else 'left',
                               mean_vx=statistics.fmean(average), left=sum(x < -1e-6 for x in average),
                               right=sum(x > 1e-6 for x in average)))
        sequences.append(dict(food_on_probes=sequence['food_on_probes'], phases=phases))
    return dict(bank_name=bank['name'], bank_sha256=digest(bank_path), gpu_report_sha256=digest(path),
                maximum_cpu_gpu_motor_error=worst, cases=cases, reversal_sequences=sequences,
                scope=report['scope'])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('reports', type=Path, nargs='+')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    results = [check(path) for path in args.reports]
    with args.output.open('x') as stream:
        json.dump(results, stream, indent=2)
    for result in results:
        print(json.dumps({k: v for k, v in result.items() if k not in ('cases', 'scope')}))


if __name__ == '__main__':
    main()
