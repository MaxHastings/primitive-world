"""Registered founding and spatial-challenge checks for the authored V3 starter."""
import argparse
from pathlib import Path
import shutil
import subprocess
import initialize
import prepare


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--exe", type=Path, required=True)
    ap.add_argument("--directory", type=Path, required=True)
    args = ap.parse_args()
    directory = args.directory.resolve()
    directory.mkdir(parents=True, exist_ok=False)
    exe = directory / "world.exe"
    shutil.copy2(args.exe.resolve(), exe)
    bank = initialize.starter_bank()
    bank_path = directory / "starter.bank.json"
    prepare.write_new(bank_path, bank)
    cases = [dict(label=f"{suite}-{i}", job=dict(seed=9042701+i, ticks=16384,
                  contrast=contrast, regeneration=.01, rotation=i, population=1000))
             for suite, contrast in [("founding", .25), ("challenge", 1.0)] for i in range(4)]
    registration = dict(schema=1, model=prepare.MODEL, executable_sha256=prepare.sha(exe),
        bank_sha256=prepare.sha(bank_path), initializer_sha256=prepare.sha(Path(initialize.__file__)),
        runner_sha256=prepare.sha(Path(__file__)), validator_sha256=prepare.sha(Path(prepare.__file__)),
        protocol_sha256=prepare.sha(prepare.ROOT / "training/STARTER_PLAN.md"), cases=cases,
        final_validation=False, selection=False)
    prepare.write_new(directory / "registration.json", registration)
    results = {}
    for case in cases:
        label, job = case["label"], case["job"]
        output = directory / f"{label}.json"
        command = [str(exe), "--headless", "--families", "--founders", str(bank_path),
                   "--seed", str(job["seed"]), "--ticks", str(job["ticks"]), "--sample", "1024",
                   "--population", str(job["population"]), "--habitat-contrast", str(job["contrast"]),
                   "--regeneration", str(job["regeneration"]), "--environment-rotation", str(job["rotation"]),
                   "--output", str(output)]
        prepare.write_new(directory / f"{label}.command.json", command)
        print(f"Starting {label}", flush=True)
        with (directory / f"{label}.log").open("x", encoding="utf-8") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True,
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        result = prepare.validate(prepare.read(output), job, bank)
        result.update(report_sha256=prepare.sha(output),
                      established=result["living"] > 0 and result["tick"] == job["ticks"]
                          and result["diagnostics"]["births_to_descendant_parents"] > 0
                          and result["diagnostics"]["maximum_depth"] >= 3)
        assert prepare.sha(exe) == registration["executable_sha256"]
        assert prepare.sha(bank_path) == registration["bank_sha256"]
        results[label] = result
        print(label, {k: result[k] for k in ["tick", "living", "births", "established", "capacity_sample_fraction"]}, flush=True)
    prepare.write_new(directory / "summary.json", dict(status="complete_unpromoted", results=results,
        founding_gate_passed=sum(results[f"founding-{i}"]["established"] for i in range(4)) >= 3,
        final_validation=False))


if __name__ == "__main__":
    main()
