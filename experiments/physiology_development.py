"""Registered v2 body development campaign; not a prepared-vs-baseline evaluation."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess

ROOT = Path(__file__).resolve().parents[1]
ORDER = [(808, 1), (909, 1), (1001, 1), (808, 2)]


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_new(path, value):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(value, stream, indent=2)


def f32_genes(genomes):
    return b"".join(struct.pack("<f", g) for genome in genomes for g in genome)


def invoke(command, log):
    with log.open("x", encoding="utf-8") as stream:
        subprocess.run(command, cwd=ROOT, stdout=stream, stderr=stream, check=True,
                       creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", required=True, type=Path)
    parser.add_argument("--directory", required=True, type=Path)
    args = parser.parse_args()
    source_commit = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    assert not subprocess.check_output(["git", "diff", "HEAD", "--name-only"], cwd=ROOT), "Commit tracked source first"
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    exe = directory / "world.exe"
    shutil.copy2(args.exe.resolve(), exe)
    initial = directory / "zero-tick.json"
    invoke([str(exe), "--headless", "--ticks", "0", "--seed", "808", "--output", str(initial)],
           directory / "zero-tick.log")
    initial_report = json.loads(initial.read_text(encoding="utf-8"))
    assert initial_report["model"] == "physiology-v2" and initial_report["elapsed_ticks"] == 0
    settings = initial_report["initial_settings"]
    assert "unprepared" in settings["founder_name"]
    assert len(settings["founder_genomes"]) == 128
    assert all(len(g) == 1518 for g in settings["founder_genomes"])
    bank = directory / "unprepared.json"
    write_new(bank, dict(version=3, model="physiology-v2", name=settings["founder_name"],
                         source_seed=0, source_tick=0, genomes=settings["founder_genomes"]))
    identity, bank_hash = sha(exe), sha(bank)
    protocol = dict(source_commit=source_commit, executable_sha256=identity,
                    bank_sha256=bank_hash, initial_report_sha256=sha(initial),
                    runner_sha256=sha(Path(__file__)),
                    plan_sha256=sha(ROOT / "experiments/PHYSIOLOGY_V2_PLAN.md"),
                    order=ORDER, horizon=200000, sample=1024, journey_sample=32,
                    initial_settings=settings,
                    scope="Unprepared v2 development only. No causal v1 comparison, pretraining or final validation.")
    write_new(directory / "registration.json", protocol)
    state = dict(status="running", runs=[], prepared=False, final_validation=False)

    def save_state():
        (directory / "summary.json").write_text(json.dumps(state, indent=2), encoding="utf-8")

    save_state()
    try:
        for seed, repeat in ORDER:
            assert sha(exe) == identity and sha(bank) == bank_hash
            label = f"seed{seed}-repeat{repeat}"
            report_path, journey_path = directory / f"{label}.json", directory / f"{label}.jsonl"
            command = [str(exe), "--headless", "--seed", str(seed), "--ticks", "200000", "--sample", "1024",
                       "--population", "1000", "--metabolic-cost", "0.06", "--movement-cost", "0.01",
                       "--motor-gain", "4", "--regeneration", "0.01", "--founders", str(bank),
                       "--output", str(report_path), "--journeys", str(journey_path), "--journey-sample", "32"]
            write_new(directory / f"{label}-command.json", command)
            print(json.dumps(dict(start=label)), flush=True)
            invoke(command, directory / f"{label}.log")
            report = json.loads(report_path.read_text(encoding="utf-8"))
            history, final_settings = report["history"], report["final_settings"]
            initial_settings = report["initial_settings"]
            assert f32_genes(initial_settings["founder_genomes"]) == f32_genes(settings["founder_genomes"])
            for key in settings:
                if key != "founder_genomes":
                    assert initial_settings[key] == settings[key], key
            assert initial_settings == final_settings
            assert report["model"] == "physiology-v2" and report["checkpoint_version"] == 14
            assert report["seed"] == seed and report["initial_tick"] == 0
            assert report["famine_at"] == report["restore_at"] == 4294967295
            assert report["founder_export"] is None
            last = history[-1]
            assert last["tick"] == 200000 or last["living"] == 0
            accounting = all(1000 + m["events"][3] - sum(m["events"][i] for i in [1, 2, 7]) == m["living"]
                             for m in history)
            cap_time = sum(b["tick"] - a["tick"] for a, b in zip(history, history[1:])
                           if b["living"] >= .95 * report["capacity"])
            row = dict(seed=seed, repeat=repeat, tick=last["tick"], living=last["living"],
                       births=last["events"][3], energy_deaths=last["events"][1], age_deaths=last["events"][2],
                       max_population=max(m["living"] for m in history),
                       invalid=max(m["invalid_outputs"] for m in history), accounting_consistent=accounting,
                       cap_sample_fraction=sum(m["living"] >= .95 * report["capacity"] for m in history[1:]) / max(1, len(history)-1),
                       cap_sampled_time_fraction=cap_time / max(1, last["tick"]),
                       max_ancestry=report["evolution"]["maximum_ancestry_depth"],
                       journey=report["journey_observer"], report_sha256=sha(report_path),
                       journey_sha256=sha(journey_path), wall_seconds=report["wall_seconds"])
            state["runs"].append(row)
            save_state()
            print(json.dumps(row), flush=True)
            assert accounting and row["invalid"] == 0, "Physical/numerical failure; preserve and stop, no retry"
        state["status"] = "complete"
    except BaseException as error:
        state["status"] = "failed"
        state["error"] = repr(error)
        raise
    finally:
        save_state()


if __name__ == "__main__":
    main()
