"""Run isolated, equal-founder lineage contests in the v46 headless engine.

Input banks are read only. The mixed bank and reports are written only under --out.
The odd/even ancestry gate is diagnostic and must never be used by wallpaper.
"""
import argparse
import hashlib
import json
import subprocess
from pathlib import Path


def read_bank(path: Path):
    bank = json.loads(path.read_text(encoding="utf-8-sig"))
    if bank["model"] not in ("primitive-v45-depth-retention", "primitive-v46-fast-evolution"):
        raise ValueError(f"Unknown founder model: {bank['model']}")
    if len(bank["genomes"]) < 128 or len(bank["traits"]) != len(bank["genomes"]):
        raise ValueError(f"Need 128 matched genomes and traits: {path}")
    if any(len(g) != 2494 for g in bank["genomes"][:128]):
        raise ValueError(f"Unexpected genome width: {path}")
    return bank


def write_mixed(old, new, path):
    genomes, traits = [], []
    for i in range(128):
        genomes.extend((old["genomes"][i], new["genomes"][i]))
        traits.extend((old["traits"][i], new["traits"][i]))
    bank = dict(version=22, model="primitive-v46-fast-evolution", name="isolated-lineage-contest",
                source_seed=0, source_tick=0, assisted=True, genomes=genomes, traits=traits)
    with path.open("x", encoding="utf-8") as file:
        json.dump(bank, file, separators=(",", ":"), allow_nan=False)


def summarize(report_path, ancestry_path):
    report = json.loads(report_path.read_text(encoding="utf-8"))
    ancestry = json.loads(ancestry_path.read_text(encoding="utf-8"))
    if (not ancestry["gate_enabled"] or ancestry["successful_births_by_child_ancestry"]["hybrid"]
            or ancestry.get("invalid_masks", 0)
            or any(ancestry[k][m] for k in ("organisms_alive", "packets_alive")
                   for m in ("unknown", "hybrid"))):
        raise ValueError("Contest ancestry gate failed")
    families = report["family_report"]["families"]
    def score(side):
        group = [f for f in families if f["family"] % 2 == side]
        return dict(organism_biological_units=2 * sum(f["founder_body_ticks"] + f["descendant_body_ticks"] for f in group),
                    births=sum(f["births"] for f in group),
                    descendant_parent_births=sum(f["births_to_descendant_parents"] for f in group),
                    matured_descendants=sum(f["matured_descendants"] for f in group))
    return dict(seed=report["seed"], elapsed_macro_steps=report["elapsed_ticks"],
                elapsed_biological_units=report["elapsed_biological_units"],
                termination=report["termination_reason"], extinct=report["lineage_extinction"],
                old=dict(**score(0), final_organisms=ancestry["organisms_alive"]["old_only"]),
                new=dict(**score(1), final_organisms=ancestry["organisms_alive"]["latest_only"]),
                hybrid_births=ancestry["successful_births_by_child_ancestry"]["hybrid"],
                wall_seconds=report["wall_seconds"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--old", type=Path, required=True)
    parser.add_argument("--new", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--exe", type=Path, default=Path("target-lineage-contest/release/primitive_world.exe"))
    parser.add_argument("--seeds", type=int, nargs="+", default=[101, 202, 303])
    parser.add_argument("--ticks", type=int, default=50_000,
                        help="v46 macro steps; 50,000 equals 100,000 biological units")
    args = parser.parse_args()
    if not 1 <= args.ticks <= 200_000 or len(set(args.seeds)) != len(args.seeds):
        parser.error("ticks must be 1..200000 and seeds unique")
    old, new = read_bank(args.old), read_bank(args.new)
    args.out.mkdir(parents=True, exist_ok=False)
    mixed = args.out / "mixed-founders.json"
    write_mixed(old, new, mixed)
    manifest = dict(old=str(args.old.resolve()), new=str(args.new.resolve()),
                    old_sha256=hashlib.sha256(args.old.read_bytes()).hexdigest(),
                    new_sha256=hashlib.sha256(args.new.read_bytes()).hexdigest(),
                    mixed_sha256=hashlib.sha256(mixed.read_bytes()).hexdigest(),
                    old_source_model=old["model"], new_source_model=new["model"],
                    founder_count_each=128, seeds=args.seeds, max_macro_steps=args.ticks,
                    max_biological_units=args.ticks * 2,
                    method="Interleaved founder slots, shared v46 world, physical packet fusion within ancestry only; fresh world per seed.")
    (args.out / "manifest.json").write_text(json.dumps(manifest, indent=2), encoding="utf-8")
    results = []
    for seed in args.seeds:
        run_dir = args.out / f"seed-{seed}"
        run_dir.mkdir()
        report, ancestry = run_dir / "report.json", run_dir / "ancestry.json"
        famine = min(2500, args.ticks + 1)
        restore = min(3000, args.ticks + 2)
        command = [str(args.exe.resolve()), "--headless", "--single-world", "--seed", str(seed),
                   "--founders", str(mixed.resolve()), "--ticks", str(args.ticks),
                   "--sample", "128", "--families", "--ancestry-audit", str(ancestry.resolve()),
                   "--block-cross-lineage-mating", "--stop-on-lineage-extinction",
                   "--population", "8192", "--regeneration", ".01", "--metabolic-cost", ".005",
                   "--movement-cost", ".01", "--motor-gain", "4", "--habitat-contrast", "1",
                   "--environment-rotation", "0", "--famine-at", str(famine),
                   "--restore-at", str(restore), "--famine-radius", "256", "--famine-delta", "-10",
                   "--output", str(report.resolve())]
        with (run_dir / "runner.log").open("x", encoding="utf-8") as log:
            result = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, check=False)
        if result.returncode or not report.is_file() or not ancestry.is_file():
            raise RuntimeError(f"Seed {seed} failed; inspect {run_dir / 'runner.log'}")
        summary = summarize(report, ancestry)
        results.append(summary)
        (args.out / "summary.json").write_text(json.dumps(dict(manifest=manifest, results=results), indent=2), encoding="utf-8")
        print(json.dumps(summary), flush=True)


if __name__ == "__main__":
    main()
