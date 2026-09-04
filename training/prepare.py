"""Prepare inherited founder weights through ordinary reproduction, then validate.

No PPO reward, teacher labels, forced births or population target are used. A
preparation stage samples living descendants at actual abundance and starts a
fresh seeded world with their genomes. Evaluation never exports back into the bank.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--exe', type=Path, default=ROOT/'target/release/primitive_world.exe')
    ap.add_argument('--directory', type=Path, required=True)
    ap.add_argument('--bank', type=Path, help='Continue preparing this bank, or evaluate it with no preparation seeds')
    ap.add_argument('--prepare-seeds', nargs='*', type=int, default=[11, 22, 33])
    ap.add_argument('--eval-seeds', nargs='+', type=int, default=[101, 202, 303])
    ap.add_argument('--prepare-ticks', type=int, default=30000)
    ap.add_argument('--eval-ticks', type=int, default=60000)
    ap.add_argument('--population', type=int, default=1000)
    args = ap.parse_args()
    if args.prepare_ticks<12000 or args.eval_ticks<12000:
        ap.error('Lifecycle evidence requires runs exceeding the maximum founder lifespan (11000 ticks)')
    if set(args.prepare_seeds) & set(args.eval_seeds):
        ap.error('Preparation and held-out evaluation seeds must be disjoint')
    if not 1 <= args.population <= 100000:
        ap.error('population must be 1..100000')
    args.directory.mkdir(parents=True, exist_ok=True)
    bank=args.bank.resolve() if args.bank else None
    runs=[]
    def run(label, seed, ticks, extra, export=None):
        report=args.directory/f'{label}.json'
        if report.exists() or (export and export.exists()):
            raise FileExistsError(f'Refusing to replace experiment {label}; use a new directory')
        cmd=[str(args.exe.resolve()), '--headless', '--seed', str(seed), '--ticks', str(ticks),
             '--sample', '5000', '--population', str(args.population), '--output', str(report.resolve())]
        if bank:cmd+=['--founders', str(bank)]
        else:cmd+=['--bootstrap']
        if export:cmd+=['--export-founders',str(export.resolve())]
        cmd+=extra
        print(json.dumps({'run':label,'command':cmd}),flush=True)
        subprocess.run(cmd, cwd=ROOT, check=True)
        result=json.loads(report.read_text())
        final=result['history'][-1]
        row=dict(label=label, seed=seed, ticks=ticks, living=final['living'],births=final['events'][3],
                 force=final['events'][5], max_ancestry_depth=result['evolution']['maximum_generation'],
                 force_energy_spent=final['force_energy_spent'], birth_gates=final['birth_gates'],
                 report=str(report.resolve()), command=cmd)
        runs.append(row)
        print(json.dumps(row),flush=True)
    for i,seed in enumerate(args.prepare_seeds):
        export=args.directory/f'founders-{i}.json'
        run(f'prepare-{i}-seed{seed}', seed, args.prepare_ticks, [], export)
        bank=export.resolve()
    if bank is None:ap.error('A bank or preparation seeds are required')
    for seed in args.eval_seeds:
        run(f'evaluate-seed{seed}',seed,args.eval_ticks,[])
    run('control-no-force',args.eval_seeds[0],args.eval_ticks,['--no-force'])
    famine_at=min(16000,args.eval_ticks-1000)
    run('stress-famine',args.eval_seeds[-1],args.eval_ticks,
        ['--famine-at',str(famine_at),'--restore-at',str(famine_at+500)])
    evaluation=[r for r in runs if r['label'].startswith('evaluate')]
    summary=dict(schema=1, executable_sha256=hashlib.sha256(args.exe.read_bytes()).hexdigest(),
                 bank=str(bank),bank_sha256=hashlib.sha256(bank.read_bytes()).hexdigest(),runs=runs,
                 ordinary_world_checks_passed=all(r['living']>0 and r['max_ancestry_depth']>=3 for r in evaluation),
                 scope='Finite-seed multigeneration persistence; not proof of open-ended emergence or universal stability')
    (args.directory/'summary.json').write_text(json.dumps(summary,indent=2))
    print(json.dumps(summary),flush=True)

if __name__=='__main__':main()
