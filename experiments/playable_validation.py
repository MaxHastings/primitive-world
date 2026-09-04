"""Registered three-arm physical-calibration versus inheritance evaluation."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import shutil

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "reports/playable-validation-20260904"
EXE = DIRECTORY / "validation.exe"
CALIBRATION = ROOT / "reports/motor-calibration-20260904"
ORDER = [(808, "historical"), (808, "calibrated"), (808, "prepared"),
         (909, "prepared"), (909, "calibrated"), (909, "historical"),
         (1001, "calibrated"), (1001, "prepared"), (1001, "historical")]


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def write_new(path, obj):
    with path.open("x", encoding="utf-8") as stream:
        json.dump(obj, stream, indent=2)


def main():
    global DIRECTORY, CALIBRATION, EXE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=DIRECTORY)
    parser.add_argument("--calibration", type=Path, default=CALIBRATION)
    parser.add_argument("--exe", type=Path, help="Copy this built executable into a fresh output directory")
    args = parser.parse_args()
    DIRECTORY, CALIBRATION = args.directory.resolve(), args.calibration.resolve()
    EXE = DIRECTORY / "validation.exe"
    if args.exe:
        DIRECTORY.mkdir(parents=True, exist_ok=False)
        shutil.copy2(args.exe.resolve(), EXE)
    calibration = json.loads((CALIBRATION / "summary.json").read_text(encoding="utf-8"))
    assert calibration["status"] == "complete" and calibration["selected_gain"] is not None
    gain = calibration["selected_gain"]
    original = ROOT / "policies/recurrent-v1.json"
    calibration_plan = json.loads((CALIBRATION / "plan.json").read_text(encoding="utf-8"))
    assert sha(original) == calibration_plan["founder_sha256"], "Released baseline changed since preparation"
    candidate = CALIBRATION / f"gain-{gain}-descendants.json"
    banks = {a: candidate if a == "prepared" else original for _, a in ORDER}
    identity = sha(EXE)
    hashes = {a: sha(p) for a, p in banks.items()}
    write_new(DIRECTORY / "registration.json", dict(
        plan_sha256=sha(ROOT / "experiments/PLAYABLE_VALIDATION_PLAN.md"),
        executable_sha256=identity, bank_hashes=hashes, selected_gain=gain,
        order=ORDER, max_runs=9, max_ticks=1800000, ticks=200000, sample=1024,
        promotion="At least two extra surviving seeds, no capped survival-time regression, no invalids; else retain released bank."))
    state = dict(status="running", runs=[], promote_bank=False, selected_gain=gain)
    def save():
        (DIRECTORY / "summary.json").write_text(json.dumps(state, indent=2), encoding="utf-8")
    save()
    try:
        for seed, arm in ORDER:
            assert sha(EXE) == identity and sha(banks[arm]) == hashes[arm]
            label = f"{seed}-{arm}"
            report = DIRECTORY / (label + ".json")
            command = [str(EXE), "--headless", "--seed", str(seed), "--ticks", "200000", "--sample", "1024",
                       "--population", "1000", "--motor-gain", str(1 if arm == "historical" else gain),
                       "--metabolic-cost", "0.06", "--movement-cost", "0.01", "--regeneration", "0.01",
                       "--founders", str(banks[arm]), "--output", str(report)]
            print(json.dumps(dict(start=label)), flush=True)
            with (DIRECTORY / (label + ".log")).open("x", encoding="utf-8") as log:
                subprocess.run(command, cwd=ROOT, stdout=log, stderr=log, check=True,
                               creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
            result = json.loads(report.read_text(encoding="utf-8"))
            history = result["history"]
            final = history[-1]
            assert result["initial_tick"] == 0 and (final["tick"] == 200000 or final["living"] == 0)
            assert 1000 + final["events"][3] - sum(final["events"][i] for i in [1,2,7]) == final["living"]
            auc = sum((b["tick"]-a["tick"])*(a["living"]+b["living"])/2 for a,b in zip(history, history[1:]))
            peak, drawdown = 1000, 0.0
            for m in history:
                peak = max(peak, m["living"])
                drawdown = max(drawdown, (peak-m["living"])/peak)
            travel = result["travel_observer"]["stats"]
            row = dict(seed=seed, arm=arm, gain=1 if arm == "historical" else gain,
                       tick=final["tick"], living=final["living"], survived=bool(final["living"]),
                       births=final["events"][3], energy_deaths=final["events"][1], age_deaths=final["events"][2],
                       max_population=peak, mean_population_200k=auc/200000, maximum_drawdown=drawdown,
                       invalid=final["invalid_outputs"], max_ancestry=result["evolution"]["maximum_ancestry_depth"],
                       capacity_samples=sum(m["living"]>=0.95*result["capacity"] for m in history[1:]),
                       eligible_unresolved_births=final["birth_gates"][5]-final["birth_gates"][6],
                       travel=travel, net_progress_per_tracked_tick=travel["net_displacement"]/max(1,travel["tracked_agent_ticks"]),
                       path_per_tracked_tick=travel["path_distance"]/max(1,travel["tracked_agent_ticks"]),
                       report_sha256=sha(report), command=command, wall_seconds=result["wall_seconds"])
            state["runs"].append(row)
            save()
            print(json.dumps(row), flush=True)
            assert not any(m["invalid_outputs"] for m in history), "Numerical fault; no retry"
            assert not travel["invalid_observations"], "Invalid observer values"
        lookup = {(r["seed"],r["arm"]):r for r in state["runs"]}
        seeds = [808,909,1001]
        extra = sum(lookup[s,"prepared"]["survived"]-lookup[s,"calibrated"]["survived"] for s in seeds)
        no_regression = all(lookup[s,"prepared"]["tick"]>=lookup[s,"calibrated"]["tick"] for s in seeds)
        state.update(status="complete", promote_bank=extra>=2 and no_regression,
                     prepared_extra_surviving_seeds=extra, no_survival_time_regression=no_regression)
        save()
    except Exception as exc:
        state.update(status="failed", error=str(exc))
        save()
        raise


if __name__ == "__main__":
    main()
