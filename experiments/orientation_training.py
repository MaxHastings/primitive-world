"""Registered equal-budget continuation versus orientation-diverse experience."""
import argparse
import json
from pathlib import Path
import secrets
import shutil
import subprocess

from cumulative_preparation import bank_statistics, physical, read_json, sha, summarize, validate_report, write_new

ROOT = Path(__file__).resolve().parents[1]
START_SHA = '89790e952e2e91fc1b5af0c0173e95724ecda7fd767fec840af0fedd1f995fc2'
TRAIN_SEEDS = [1201, 1202, 1203, 1204]
TRAIN_TICKS = 65536
ROTATIONS = dict(ordinary=[0, 0, 0, 0], varied=[1, 2, 3, 0])


def expected_settings(base, rotation):
    assert rotation in range(4)
    result = dict(base)
    result.pop('environment_rotation', None)
    if rotation:
        result['environment_rotation'] = rotation
    return result


def execution_schedule(seeds):
    training = [dict(arm=arm, episode=episode+1, seed=seed, rotation=ROTATIONS[arm][episode])
                for episode, seed in enumerate(TRAIN_SEEDS)
                for arm in (['ordinary', 'varied'] if episode % 2 == 0 else ['varied', 'ordinary'])]
    banks = ['reference', 'ordinary', 'varied']
    evaluation = []
    for case, (seed, rotation) in enumerate((s, r) for s in seeds for r in (0, 2)):
        for arm in banks[case % 3:] + banks[:case % 3]:
            evaluation.append(dict(arm=arm, seed=seed, rotation=rotation, case=case))
    return training, evaluation


def unused_seeds():
    listing = subprocess.check_output(['git', 'worktree', 'list', '--porcelain'], cwd=ROOT, text=True)
    roots = [line.removeprefix('worktree ') for line in listing.splitlines() if line.startswith('worktree ')]
    selected, audits = [], []
    while len(selected) < 2:
        seed = 1_000_000 + secrets.randbelow(1_000_000_000)
        if seed in selected:
            continue
        # Conservative token search: also rejects numbers in registered seed lists.
        command = ['rg', '--no-ignore', '--files-with-matches', '--glob', '*.json', '--glob', '*.jsonl',
                   '--glob', '*.md', '--glob', '!**/target/**', '--glob', '!**/.git/**',
                   '--regexp', rf'\b{seed}\b', *roots]
        result = subprocess.run(command, capture_output=True, text=True)
        assert result.returncode in (0, 1), result.stderr
        audits.append(dict(candidate=seed, search_command=command, matching_files=result.stdout.splitlines()))
        if result.returncode == 1:
            selected.append(seed)
    return selected, audits


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--directory', type=Path, required=True)
    args = parser.parse_args()
    assert not subprocess.check_output(['git', 'diff', 'HEAD', '--name-only'], cwd=ROOT), 'Commit tracked source first'
    directory = args.directory.resolve()
    source_bank = ROOT/'reports/cumulative-preparation-20260904/bank-after16.json'
    assert sha(source_bank) == START_SHA
    base = physical(read_json(ROOT/'reports/physiology-development-20260904/zero-tick.json')['initial_settings'])
    assert base['population'] == 1000 and base['metabolic_cost'] == .05999999865889549
    # Preserve canonical float32 serialized physical settings instead of decimal coercion.
    directory.mkdir(parents=True, exist_ok=False)
    executable, start_path = directory/'world.exe', directory/'start-bank.json'
    shutil.copy2(args.exe.resolve(), executable)
    shutil.copy2(source_bank, start_path)
    exe_hash = sha(executable)
    version = subprocess.check_output([str(executable), '--version'], text=True).strip()
    assert version == 'Primitive World 0.3.2-dev / physiology-v2 / checkpoint 14', version
    seeds, seed_audits = unused_seeds()
    training, evaluation = execution_schedule(seeds)
    registration = dict(source_commit=subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=ROOT, text=True).strip(),
        executable_sha256=exe_hash, version=version, start_bank_sha256=START_SHA,
        runner_sha256=sha(Path(__file__)), shared_validator_sha256=sha(ROOT/'experiments/cumulative_preparation.py'),
        plan_sha256=sha(ROOT/'experiments/ORIENTATION_TRAINING_PLAN.md'),
        physical_settings=base, train_ticks=TRAIN_TICKS, training_schedule=training,
        evaluation_seeds=seeds, seed_record_audits=seed_audits, evaluation_schedule=evaluation,
        scope='Development trial. Two previously unobserved seeds, four oriented cases per bank; not final eight-seed validation.')
    write_new(directory/'registration.json', registration)
    state = dict(status='running', active_job=None, runs=[], banks=[], failed_arms=[],
                 actual_training_ticks=dict(ordinary=0, varied=0), evaluation_seeds=seeds, final_validation=False)
    endpoints = dict(reference=start_path, ordinary=start_path, varied=start_path)
    known_hashes = {str(start_path): START_SHA}
    starting_bank = read_json(start_path)

    def save():
        temporary = directory/'summary.pending.json'
        temporary.write_text(json.dumps(state, indent=2), encoding='utf-8')
        temporary.replace(directory/'summary.json')

    def run(job, is_evaluation):
        arm, seed, rotation = job['arm'], job['seed'], job['rotation']
        bank_path = endpoints[arm]
        if bank_path is None:
            return
        phase = 'eval' if is_evaluation else f"train{job['episode']}"
        label = f'{phase}-{arm}-seed{seed}-r{rotation}'
        report_path, journey_path = directory/f'{label}.json', directory/f'{label}.jsonl'
        export_path = directory/f"bank-{arm}-after{job.get('episode', 0)}.json"
        assert sha(executable) == exe_hash and sha(bank_path) == known_hashes[str(bank_path)]
        bank = read_json(bank_path)
        ticks = 200000 if is_evaluation else TRAIN_TICKS
        command = [str(executable), '--headless', '--seed', str(seed), '--environment-rotation', str(rotation),
                   '--ticks', str(ticks), '--sample', '1024', '--population', '1000', '--metabolic-cost', '0.06',
                   '--movement-cost', '0.01', '--motor-gain', '4', '--regeneration', '0.01',
                   '--founders', str(bank_path), '--output', str(report_path)]
        command += ['--journeys', str(journey_path), '--journey-sample', '32'] if is_evaluation else ['--export-founders', str(export_path)]
        write_new(directory/f'{label}-command.json', command)
        state['active_job'] = label
        save()
        print(json.dumps(dict(start=label, source_bank_sha256=known_hashes[str(bank_path)])), flush=True)
        with (directory/f'{label}.log').open('x', encoding='utf-8') as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=log, check=True,
                           creationflags=getattr(subprocess, 'CREATE_NO_WINDOW', 0))
        assert sha(bank_path) == known_hashes[str(bank_path)]
        report = read_json(report_path)
        assert report['build_version'] == '0.3.2-dev'
        validate_report(report, bank, expected_settings(base, rotation), seed, ticks, is_evaluation)
        if is_evaluation:
            footer = None
            with journey_path.open() as stream:
                for line in stream:
                    footer = json.loads(line)
            assert footer['type'] == 'summary' and footer['observer'] == report['journey_observer']
        row = dict(label=label, arm=arm, seed=seed, rotation=rotation, phase=phase,
                   source_bank_sha256=known_hashes[str(bank_path)], report_sha256=sha(report_path), **summarize(report))
        if is_evaluation:
            row['journey_sha256'] = sha(journey_path)
        else:
            state['actual_training_ticks'][arm] += report['elapsed_ticks']
            if not export_path.exists():
                assert row['living'] == 0, 'Export failed despite surviving descendants; infrastructure failure'
                endpoints[arm] = None
                state['failed_arms'].append(dict(arm=arm, episode=job['episode'], export_result=report['founder_export']))
            else:
                assert report['founder_export'] == {'Ok': None}
                exported = read_json(export_path)
                assert exported['source_seed'] == seed and exported['source_tick'] == TRAIN_TICKS
                bank_row = dict(arm=arm, episode=job['episode'], filename=export_path.name, bank_sha256=sha(export_path),
                                statistics=bank_statistics(exported, starting_bank))
                known_hashes[str(export_path)] = bank_row['bank_sha256']
                endpoints[arm] = export_path
                state['banks'].append(bank_row)
        state['runs'].append(row)
        state['active_job'] = None
        save()
        print(json.dumps({k: v for k, v in row.items() if k != 'journey'}), flush=True)

    save()
    try:
        for job in training:
            run(job, False)
        for job in evaluation:
            run(job, True)
        state['status'] = 'complete_with_training_failure' if state['failed_arms'] else 'complete'
    except BaseException as error:
        state['status'] = 'failed'
        state['error'] = repr(error)
        raise
    finally:
        save()


if __name__ == '__main__':
    main()
