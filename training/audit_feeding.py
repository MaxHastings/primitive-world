"""Replay the registered pilot pair with read-only juvenile diagnostics."""
import argparse
from pathlib import Path
import shutil
import subprocess
import prepare


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--exe", type=Path, required=True)
    ap.add_argument("--pilot", type=Path, required=True)
    ap.add_argument("--directory", type=Path, required=True)
    args = ap.parse_args()
    directory = args.directory.resolve()
    pilot = args.pilot.resolve()
    summary = prepare.read(pilot / "summary.json")
    assert summary["status"] == "complete_unpromoted"
    banks = {}
    job = dict(summary["trials"]["evaluation-0-initial"]["job"])
    assert job == summary["trials"]["evaluation-0-candidate"]["job"]
    for arm in ["initial", "candidate"]:
        path = pilot / f"{arm}.bank.json"
        assert prepare.sha(path) == summary["trials"][f"evaluation-0-{arm}"]["bank_sha256"]
        banks[arm] = prepare.read(path)
    directory.mkdir(parents=True, exist_ok=False)
    exe = directory / "world.exe"
    shutil.copy2(args.exe.resolve(), exe)
    registration = dict(job=job, executable_sha256=prepare.sha(exe),
                        banks={arm: prepare.sha(pilot / f"{arm}.bank.json") for arm in banks},
                        protocol_sha256=prepare.sha(prepare.ROOT / "training/FEEDING_CAMPAIGN.md"),
                        runner_sha256=prepare.sha(Path(__file__)), final_validation=False)
    prepare.write_new(directory / "registration.json", registration)
    results = {}
    for arm, bank in banks.items():
        bank_path = directory / f"{arm}.bank.json"
        prepare.write_new(bank_path, bank)
        output = directory / f"{arm}.json"
        command = [str(exe), "--headless", "--families", "--founders", str(bank_path),
                   "--seed", str(job["seed"]), "--ticks", str(job["ticks"]),
                   "--sample", "256", "--population", str(job["population"]),
                   "--habitat-contrast", str(job["contrast"]),
                   "--regeneration", str(job["regeneration"]),
                   "--environment-rotation", str(job["rotation"]), "--output", str(output)]
        prepare.write_new(directory / f"{arm}.command.json", command)
        with (directory / f"{arm}.log").open("x", encoding="utf-8") as log:
            subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=True,
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        result = prepare.validate(prepare.read(output), job, bank)
        result["report_sha256"] = prepare.sha(output)
        assert prepare.sha(exe) == registration["executable_sha256"]
        results[arm] = result
        print(arm, result["tick"], result["diagnostics"], flush=True)
    prepare.write_new(directory / "summary.json", results)


if __name__ == "__main__":
    main()
