"""Cumulative inherited preparation with frozen development evaluation checkpoints."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import shutil
import statistics
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
TRAIN_SEEDS = [11, 22, 303, 404, 505, 606, 707, 1, 1101, 1102, 1103, 1104, 1105, 1106, 1107, 1108]
EVALUATIONS = {4: [909, 1001, 808], 8: [1001, 808, 909], 16: [808, 909, 1001]}
TRAIN_TICKS = 65536
EXE_SHA = "7a2729ddbd68ccdad1a94d67b10e80ae2a93ce779044059bbb27c55aa6ccc4e5"
BANK_SHA = "34c32e136ed80d34845ce9a7cf298ccf7f848eb67ee6e67115c002b3f8750b65"


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def write_new(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)


def packed(genomes):
    return b"".join(struct.pack("<f", g) for genome in genomes for g in genome)


def physical(settings):
    return {k: v for k, v in settings.items() if k not in ("founder_name", "founder_genomes")}


def validate_report(report, bank, expected_physical, seed, ticks, evaluation):
    assert report["model"] == "physiology-v2" and report["checkpoint_version"] == 14
    assert report["seed"] == seed and report["initial_tick"] == 0 and report["requested_ticks"] == ticks
    assert report["famine_at"] == report["restore_at"] == 4294967295
    settings = report["initial_settings"]
    assert physical(settings) == expected_physical, "Body/ecology/initialization changed"
    assert settings == report["final_settings"], "Runtime intervention or settings change"
    assert settings["founder_name"] == bank["name"]
    assert packed(settings["founder_genomes"]) == packed(bank["genomes"]), "Wrong founding weights"
    history = report["history"]
    assert history[0]["tick"] == 0 and history[0]["living"] == expected_physical["population"]
    assert all(a["tick"] < b["tick"] for a, b in zip(history, history[1:]))
    last = history[-1]
    assert report["elapsed_ticks"] == last["tick"] <= ticks
    assert last["tick"] == ticks or last["living"] == 0, "Incomplete world, not a result"
    for m in history:
        assert expected_physical["population"] + m["events"][3] - sum(m["events"][i] for i in [1, 2, 7]) == m["living"], "Population accounting failure"
        assert m["invalid_outputs"] == 0, "Invalid controller outputs"
    assert report["travel_observer"]["stats"]["invalid_observations"] == 0
    if evaluation:
        assert report["founder_export"] is None, "Evaluation must not prepare founders"
        observer = report["journey_observer"]
        assert observer["schema"] == 2 and observer["sample_ticks"] == 32
        assert observer["stats"]["invalid_observations"] == 0
    else:
        assert report["journey_observer"] is None


def bank_statistics(bank, baseline):
    genomes = bank["genomes"]
    assert bank["version"] == 3 and bank["model"] == "physiology-v2"
    assert 1 <= len(genomes) <= 128 and all(len(g) == 1518 for g in genomes)
    assert all(math.isfinite(g) and abs(g) <= 4 for genome in genomes for g in genome)
    means = [statistics.fmean(values) for values in zip(*genomes)]
    reference = [statistics.fmean(values) for values in zip(*baseline["genomes"])]
    variance = statistics.fmean(statistics.pvariance(values) for values in zip(*genomes))
    return dict(genomes=len(genomes), unique_genomes=len({packed([g]) for g in genomes}),
                mean_within_bank_gene_variance=variance,
                mean_genome_rms_difference_from_baseline=math.sqrt(statistics.fmean((a-b)**2 for a, b in zip(means, reference))),
                scope="Parameter change/diversity, not learned competence or a training loss.")


def summarize(report):
    history, last = report["history"], report["history"][-1]
    cap = report["capacity"]
    cap_ticks = sum(b["tick"]-a["tick"] for a, b in zip(history, history[1:]) if b["living"] >= .95*cap)
    return dict(tick=last["tick"], living=last["living"], survived=last["living"] > 0,
                births=last["events"][3], energy_deaths=last["events"][1], age_deaths=last["events"][2],
                max_population=max(m["living"] for m in history),
                cap_sample_fraction=sum(m["living"] >= .95*cap for m in history[1:])/max(1, len(history)-1),
                cap_sampled_time_fraction=cap_ticks/max(1, last["tick"]),
                maximum_living_ancestry_at_end=report["evolution"]["maximum_ancestry_depth"],
                mean_living_ancestry_at_end=report["evolution"]["mean_ancestry_depth"],
                action_ticks=last["action_ticks"], birth_gates=last["birth_gates"],
                journey=report["journey_observer"], wall_seconds=report["wall_seconds"],
                accounting_consistent=True, invalid=0)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--diagnostic", required=True, type=Path)
    parser.add_argument("--directory", required=True, type=Path)
    args = parser.parse_args()
    diagnostic, directory = args.diagnostic.resolve(), args.directory.resolve()
    assert sha(diagnostic / "world.exe") == EXE_SHA and sha(diagnostic / "unprepared.json") == BANK_SHA
    development = read_json(diagnostic / "summary.json")
    assert development["status"] == "complete" and len(development["runs"]) == 4
    for row in development["runs"]:
        label = f"seed{row['seed']}-repeat{row['repeat']}"
        assert sha(diagnostic / f"{label}.json") == row["report_sha256"]
        assert sha(diagnostic / f"{label}.jsonl") == row["journey_sha256"]
    assert not subprocess.check_output(["git", "diff", "HEAD", "--name-only"], cwd=ROOT), "Commit tracked source first"
    directory.mkdir(parents=True, exist_ok=False)
    exe, baseline_path = directory / "world.exe", directory / "budget0.json"
    shutil.copy2(diagnostic / "world.exe", exe)
    shutil.copy2(diagnostic / "unprepared.json", baseline_path)
    baseline = read_json(baseline_path)
    expected_physical = physical(read_json(diagnostic / "zero-tick.json")["initial_settings"])
    write_new(directory / "registration.json", dict(
        executable_sha256=EXE_SHA, executable_source_commit=read_json(diagnostic / "registration.json")["source_commit"],
        baseline_sha256=BANK_SHA, baseline_diagnostic=str(diagnostic),
        baseline_summary_sha256=sha(diagnostic / "summary.json"),
        runner_sha256=sha(Path(__file__)), plan_sha256=sha(ROOT / "experiments/CUMULATIVE_PREPARATION_PLAN.md"),
        runner_source_commit=subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        train_seeds=TRAIN_SEEDS, episode_ticks=TRAIN_TICKS, evaluations=EVALUATIONS,
        final_baseline_repeats=[808, 909, 1001], physical_settings=expected_physical,
        baseline_statistics=bank_statistics(baseline, baseline),
        scope="Development learning curve. No final holdouts, promotion or goal completion."))
    state = dict(status="running", completed_training_episodes=0, actual_training_ticks=0,
                 runs=[], banks=[], active_job=None, final_validation=False, baseline_original=development["runs"])

    def save():
        temporary = directory / "summary.pending.json"
        temporary.write_text(json.dumps(state, indent=2), encoding="utf-8")
        temporary.replace(directory / "summary.json")

    def run_world(label, seed, bank_path, episode, evaluation, baseline_repeat=False):
        assert sha(exe) == EXE_SHA and sha(baseline_path) == BANK_SHA
        input_bank = read_json(bank_path)
        input_hash = sha(bank_path)
        expected_hash = BANK_SHA if bank_path == baseline_path else next(
            row["bank_sha256"] for row in state["banks"] if row["filename"] == bank_path.name)
        assert input_hash == expected_hash, "A previously exported bank changed"
        report_path, log_path = directory / f"{label}.json", directory / f"{label}.log"
        ticks = 200000 if evaluation else TRAIN_TICKS
        command = [str(exe), "--headless", "--seed", str(seed), "--ticks", str(ticks), "--sample", "1024",
                   "--population", "1000", "--metabolic-cost", "0.06", "--movement-cost", "0.01",
                   "--motor-gain", "4", "--regeneration", "0.01", "--founders", str(bank_path), "--output", str(report_path)]
        export_path = directory / f"bank-after{episode}.json"
        journey_path = directory / f"{label}.jsonl"
        if evaluation:
            command += ["--journeys", str(journey_path), "--journey-sample", "32"]
        else:
            command += ["--export-founders", str(export_path)]
        write_new(directory / f"{label}-command.json", command)
        state["active_job"] = label
        save()
        print(json.dumps(dict(start=label, source_bank_sha256=input_hash)), flush=True)
        with log_path.open("x", encoding="utf-8") as log:
            subprocess.run(command, cwd=ROOT, stdout=log, stderr=log, check=True,
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        assert sha(bank_path) == input_hash
        report = read_json(report_path)
        validate_report(report, input_bank, expected_physical, seed, ticks, evaluation)
        if evaluation:
            with journey_path.open(encoding="utf-8") as stream:
                footer = None
                for line in stream:
                    footer = json.loads(line)
            assert footer["type"] == "summary" and footer["observer"] == report["journey_observer"], "Partial/mismatched trajectory output"
        row = dict(label=label, phase="evaluation" if evaluation else "training", seed=seed,
                   completed_preparation_episodes=0 if baseline_repeat else episode if evaluation else episode-1,
                   source_bank_sha256=input_hash, report_sha256=sha(report_path), **summarize(report))
        if evaluation:
            row["journey_sha256"] = sha(journey_path)
        else:
            state["actual_training_ticks"] += report["elapsed_ticks"]
        state["runs"].append(row)
        state["active_job"] = None
        save()
        print(json.dumps({k: v for k, v in row.items() if k != "journey"}), flush=True)
        if not evaluation:
            if not export_path.exists():
                state["status"] = "preparation_failed_no_living_descendants"
                state["failure_episode"] = episode
                state["founder_export_result"] = report["founder_export"]
                save()
                return None
            assert report["founder_export"] == {"Ok": None}
            next_bank = read_json(export_path)
            assert next_bank["source_seed"] == seed and next_bank["source_tick"] == TRAIN_TICKS
            bank_row = dict(episode=episode, training_ticks=state["actual_training_ticks"],
                            source_bank_sha256=input_hash, bank_sha256=sha(export_path),
                            filename=export_path.name, statistics=bank_statistics(next_bank, baseline))
            state["banks"].append(bank_row)
            state["completed_training_episodes"] = episode
            save()
            print(json.dumps(dict(exported=bank_row)), flush=True)
            return export_path
        return bank_path

    save()
    bank_path = baseline_path
    try:
        for episode, seed in enumerate(TRAIN_SEEDS, 1):
            bank_path = run_world(f"train{episode:02d}-seed{seed}", seed, bank_path, episode, False)
            if bank_path is None:
                return
            for evaluation_seed in EVALUATIONS.get(episode, []):
                run_world(f"budget{episode}-seed{evaluation_seed}", evaluation_seed, bank_path, episode, True)
        for seed in [808, 909, 1001]:
            run_world(f"baseline-repeat-seed{seed}", seed, baseline_path, 0, True, baseline_repeat=True)
        state["status"] = "complete"
    except BaseException as error:
        state["status"] = "failed"
        state["error"] = repr(error)
        raise
    finally:
        save()


if __name__ == "__main__":
    main()
