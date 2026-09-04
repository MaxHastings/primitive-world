"""Finite, preregistered recurrent-v1 founder preparation and held-out evaluation.

Only actual living descendants enter the next preparation world. No reward,
teacher, population floor, reseeding, fitness ranking or evaluation feedback.
Python standard library only. A failed preparation still evaluates the ORIGINAL
bootstrap (or supplied initial bank), not a cherry-picked intermediate survivor.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--exe", type=Path, default=ROOT / "target/release/primitive_world.exe")
    ap.add_argument("--directory", type=Path, required=True)
    ap.add_argument("--bank", type=Path)
    ap.add_argument("--prepare-seeds", nargs="*", type=int, default=[11, 22])
    ap.add_argument("--eval-seeds", nargs="+", type=int, default=[101, 202, 303])
    ap.add_argument("--prepare-ticks", type=int, default=12000)
    ap.add_argument("--eval-ticks", type=int, default=12000)
    ap.add_argument("--population", type=int, default=1000)
    args = ap.parse_args()
    if not 12000 <= args.prepare_ticks <= 1000000 or not 12000 <= args.eval_ticks <= 1000000:
        ap.error("Run limits must be 12000..1000000 ticks, beyond maximum founder lifespan")
    if set(args.prepare_seeds) & set(args.eval_seeds):
        ap.error("Preparation and held-out seeds must be disjoint")
    seeds = args.prepare_seeds + args.eval_seeds
    if any(s < 0 or s > 0xFFFFFFFF for s in seeds):
        ap.error("Seeds must fit u32")
    if not 1 <= args.population <= 16384:
        ap.error("Population must be 1..16384")
    exe = args.exe.resolve()
    initial_bank = args.bank.resolve() if args.bank else None
    version = subprocess.check_output([str(exe), "--version"], cwd=ROOT, text=True).strip()
    if "recurrent-v1" not in version:
        ap.error("Executable is not recurrent-v1")
    args.directory.mkdir(parents=True, exist_ok=False)
    directory = args.directory.resolve()
    plan = dict(schema=2, model="recurrent-v1", version=version,
                executable_sha256=sha(exe),
                initial_bank=str(initial_bank) if initial_bank else "unprepared-bootstrap",
                initial_bank_sha256=sha(initial_bank) if initial_bank else None,
                preparation_seeds=args.prepare_seeds, evaluation_seeds=args.eval_seeds,
                preparation_tick_limit=args.prepare_ticks, evaluation_tick_limit=args.eval_ticks,
                population=args.population, sample_interval=1000,
                controls=["no-force on first held-out seed", "500-tick famine at 6000 on last held-out seed"],
                maximum_total_ticks=len(args.prepare_seeds)*args.prepare_ticks+(len(args.eval_seeds)+2)*args.eval_ticks,
                failure_rule="Stop preparation on no living descendants; evaluate original initialization. Never retry or rescue.",
                interpretation="Architecture integrity and founder competence are separate; routes and population targets are not gates.")
    (directory / "plan.json").write_text(json.dumps(plan, indent=2), encoding="utf-8")
    runs = []
    status = "running"
    bank = initial_bank
    preparation_failed = False

    def save_summary(error=None):
        evaluation = [r for r in runs if r["label"].startswith("evaluate")]
        summary = dict(schema=2, model="recurrent-v1", status=status,
                       preparation_failed=preparation_failed, error=error,
                       evaluated_bank=str(bank) if bank else "unprepared-bootstrap",
                       bank_sha256=sha(bank) if bank else None,
                       executable_sha256=plan["executable_sha256"], runs=runs,
                       held_out_multigeneration_observed=bool(evaluation) and all(r["living"] > 0 and r["max_ancestry_depth"] >= 2 for r in evaluation),
                       scope=plan["interpretation"])
        (directory / "summary.json").write_text(json.dumps(summary, indent=2), encoding="utf-8")
        return summary

    def run(label, seed, ticks, extra, export=None):
        if sha(exe) != plan["executable_sha256"]:
            raise RuntimeError("Executable changed during the registered campaign")
        report = directory / f"{label}.json"
        cmd = [str(exe), "--headless", "--seed", str(seed), "--ticks", str(ticks),
               "--sample", "1000", "--population", str(args.population), "--output", str(report)]
        cmd += ["--founders", str(bank)] if bank else ["--bootstrap"]
        if export:
            cmd += ["--export-founders", str(export)]
        cmd += extra
        print(json.dumps(dict(run=label, command=cmd)), flush=True)
        with (directory / f"{label}.log").open("x", encoding="utf-8") as log:
            subprocess.run(cmd, cwd=ROOT, check=True, stdout=log, stderr=log,
                           creationflags=getattr(subprocess, "CREATE_NO_WINDOW", 0))
        result = json.loads(report.read_text(encoding="utf-8"))
        if result["model"] != "recurrent-v1":
            raise RuntimeError("Unexpected report model")
        final = result["history"][-1]
        row = dict(label=label, seed=seed, requested_ticks=ticks, elapsed_ticks=result["elapsed_ticks"],
                   living=final["living"], births=final["events"][3],
                   deaths=dict(starvation=final["events"][1], age=final["events"][2], force=final["events"][7]),
                   max_ancestry_depth=result["evolution"]["maximum_ancestry_depth"],
                   force=final["events"][5], force_energy_spent=final["force_energy_spent"],
                   energy=final["energy"], carried_food=final["carried_food"], vegetation=final["vegetation"],
                   dropped_food=final["dropped_food"], birth_gates=final["birth_gates"],
                   invalid_outputs=final["invalid_outputs"], wall_seconds=result["wall_seconds"],
                   report=report.name, report_sha256=sha(report),
                   input_bank_sha256=sha(bank) if bank else None,
                   founder_export=result["founder_export"], command=cmd)
        runs.append(row)
        save_summary()
        print(json.dumps(row), flush=True)
        if row["invalid_outputs"]:
            raise RuntimeError("Numerical integrity failure; campaign stopped")
        return row

    started = time.monotonic()
    try:
        for i, seed in enumerate(args.prepare_seeds):
            export = directory / f"founders-{i}.json"
            row = run(f"prepare-{i}-seed{seed}", seed, args.prepare_ticks, [], export)
            if row["founder_export"] != {"Ok": None} or not export.exists():
                preparation_failed = True
                bank = initial_bank
                save_summary()
                break
            bank = export
        for seed in args.eval_seeds:
            run(f"evaluate-seed{seed}", seed, args.eval_ticks, [])
        run("control-no-force", args.eval_seeds[0], args.eval_ticks, ["--no-force"])
        run("stress-famine", args.eval_seeds[-1], args.eval_ticks,
            ["--famine-at", "6000", "--restore-at", "6500"])
        status = "complete"
        summary = save_summary()
        print(json.dumps(dict(campaign=summary, wall_seconds=time.monotonic()-started)), flush=True)
    except Exception as exc:
        status = "failed"
        save_summary(str(exc))
        raise


if __name__ == "__main__":
    main()
