"""Matched two-patch travel diagnostics. No learning, ranking, or founder export."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--exe', type=Path, default=ROOT/'target/release/primitive_world.exe')
    parser.add_argument('--ticks', type=int, default=3000)
    parser.add_argument('--distances', nargs='+', type=int, default=[120, 300])
    parser.add_argument('--regenerations', nargs='+', type=float, default=[0.002, 0.02])
    parser.add_argument('--seeds', nargs='+', type=int, default=[7, 19])
    parser.add_argument('--modes', nargs='+', choices=['discovery', 'known-target'], default=['discovery', 'known-target'])
    parser.add_argument('--food', type=float, default=0.3)
    parser.add_argument('--radius', type=float, default=16)
    parser.add_argument('--sensing', choices=['baseline', 'near', 'sweep'], default='baseline')
    args = parser.parse_args()
    if not 1 <= args.ticks <= 10000:
        parser.error('ticks must be 1..10000')
    if args.directory.exists():
        parser.error('Use a new directory; existing experiment evidence is not overwritten')
    args.directory.mkdir(parents=True)
    executable = args.exe.resolve()
    fingerprint = hashlib.sha256(executable.read_bytes()).hexdigest()
    rows = []
    for mode in args.modes:
        for separation in args.distances:
            for regeneration in args.regenerations:
                for index, seed in enumerate(args.seeds):
                    for erase in [False, True]:
                        label = f'{mode}-d{separation}-r{regeneration}-s{seed}-erase{int(erase)}'
                        report = (args.directory/f'{label}.json').resolve()
                        command = [str(executable), '--headless', '--travel-diagnostic', '--seed', str(seed),
                                   '--ticks', str(args.ticks), '--travel-distance', str(separation),
                                   '--regeneration', str(regeneration), '--travel-genome', str(index % 128),
                                   '--travel-mode', mode, '--travel-food', str(args.food), '--travel-radius', str(args.radius),
                                   '--travel-sensing', args.sensing, '--output', str(report)]
                        if erase:
                            command.append('--erase-place-memory')
                        print(json.dumps({'run': label}), flush=True)
                        subprocess.run(command, cwd=ROOT, check=True,
                                       creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
                        data = json.loads(report.read_text())
                        row = dict(label=label, mode=mode, separation=separation, regeneration=regeneration,
                                   seed=seed, erase=erase, genome=index % 128, sensing=args.sensing, report=str(report), command=command,
                                   **data['outcome'])
                        rows.append(row)
                        summary = dict(schema=1, executable_sha256=fingerprint, complete=False, runs=rows,
                                       scope='Isolated diagnostic: births suppressed; custom two-patch geography; normal body costs and resource shader; no migration reward')
                        (args.directory/'summary.json').write_text(json.dumps(summary, indent=2))
    if fingerprint != hashlib.sha256(executable.read_bytes()).hexdigest():
        raise RuntimeError('Executable changed during the experiment')
    summary['complete'] = True
    (args.directory/'summary.json').write_text(json.dumps(summary, indent=2))
    print(json.dumps({'complete': True, 'runs': len(rows)}), flush=True)

if __name__ == '__main__':
    main()
