"""Finite actuator calibration with old costs; outcomes do not prove learning."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import shutil

ROOT = Path(__file__).resolve().parents[1]
DIRECTORY = ROOT / "reports/motor-calibration-20260904"
EXE = DIRECTORY / "calibration.exe"
GAINS = [4, 8, 16]


def sha(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    global DIRECTORY, EXE
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--directory", type=Path, default=DIRECTORY)
    parser.add_argument("--exe", type=Path, help="Copy this built executable into a fresh output directory")
    args = parser.parse_args()
    DIRECTORY = args.directory.resolve()
    EXE = DIRECTORY / "calibration.exe"
    if args.exe:
        DIRECTORY.mkdir(parents=True, exist_ok=False)
        shutil.copy2(args.exe.resolve(), EXE)
    identity = sha(EXE)
    plan = dict(scope="Three declared physical-response calibrations, not learned-policy validation.",
                seed=1, gains=GAINS, ticks=131072, max_runs=3, max_ticks=393216,
                metabolism=0.06, movement_cost=0.01, regeneration=0.01,
                population=1000, sample=1024, executable_sha256=identity,
                founder_sha256=sha(ROOT / "policies/recurrent-v1.json"),
                selection="Use smallest gain with living descendants at the full horizon and no invalid outputs; no promotion based on this alone.",
                failure="No retries/rescue. If no candidate persists, record failure; do not select a winner.")
    with (DIRECTORY / "plan.json").open("x", encoding="utf-8") as stream:
        json.dump(plan, stream, indent=2)
    state = dict(status="running", runs=[], selected_gain=None)
    def save():
        (DIRECTORY / "summary.json").write_text(json.dumps(state, indent=2), encoding="utf-8")
    save()
    try:
        for gain in GAINS:
            assert sha(EXE) == identity
            label = f"gain-{gain}"
            report = DIRECTORY / (label + ".json")
            bank = DIRECTORY / (label + "-descendants.json")
            command = [str(EXE), "--headless", "--seed", "1", "--ticks", "131072", "--sample", "1024",
                       "--population", "1000", "--motor-gain", str(gain), "--metabolic-cost", "0.06",
                       "--movement-cost", "0.01", "--founders", str(ROOT / "policies/recurrent-v1.json"),
                       "--output", str(report), "--export-founders", str(bank)]
            print(json.dumps(dict(start=label)), flush=True)
            with (DIRECTORY / (label + ".log")).open("x", encoding="utf-8") as log:
                subprocess.run(command, cwd=ROOT, stdout=log, stderr=log, check=True,
                               creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
            result = json.loads(report.read_text(encoding="utf-8"))
            final = result["history"][-1]
            row = dict(gain=gain, tick=final["tick"], living=final["living"], births=final["events"][3],
                       max_population=max(m["living"] for m in result["history"]),
                       invalid=final["invalid_outputs"], ancestry=result["evolution"]["maximum_ancestry_depth"],
                       export=result["founder_export"], report_sha256=sha(report),
                       bank_sha256=sha(bank) if bank.exists() else None, command=command)
            state["runs"].append(row)
            save()
            print(json.dumps(row), flush=True)
            assert not row["invalid"], "Numerical fault; no retry"
        survivors = [r["gain"] for r in state["runs"] if r["living"] and r["tick"] == 131072 and r["export"] == {"Ok": None}]
        state.update(status="complete", selected_gain=min(survivors) if survivors else None)
        save()
    except Exception as exc:
        state.update(status="failed", error=str(exc))
        save()
        raise


if __name__ == "__main__":
    main()
