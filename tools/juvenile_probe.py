"""Fixed random-founder diagnostic; reports continuity without selecting for it.

Build with cargo build --release first. Raw per-family evidence is retained next
to the summary. No genomes are exported, selected, or fed back into the world.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def summarize(report):
    families = report["family_report"]["families"]
    keys = ["births", "juvenile_starvation_deaths", "matured_descendants",
            "juvenile_transfers_received", "juvenile_received_milli",
            "births_to_descendant_parents"]
    result = {key: sum(f[key] for f in families) for key in keys}
    result.update(seed=report["seed"], elapsed_ticks=report["elapsed_ticks"],
                  maximum_depth=max((f["maximum_depth"] for f in families), default=0))
    result["nonzero_reproductive_continuity"] = result["births_to_descendant_parents"] > 0
    return result


def main():
    root = Path(__file__).resolve().parents[1]
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--executable", type=Path, default=root / "target/release/primitive_world.exe")
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--seeds", type=int, nargs="+", default=[42, 91, 3137])
    parser.add_argument("--ticks", type=int, default=12000)
    args = parser.parse_args()
    args.output.parent.mkdir(parents=True, exist_ok=True)
    if args.output.exists():
        parser.error("output already exists; choose a new path")
    evidence = Path(tempfile.mkdtemp(prefix="juvenile-evidence-", dir=args.output.parent)).resolve()
    rows = []
    for index, seed in enumerate(args.seeds):
        path = evidence / f"{index}-{seed}.json"
        subprocess.run([str(args.executable.resolve()), "--headless", "--single-world",
                        "--seed", str(seed), "--ticks", str(args.ticks), "--sample", "3000",
                        "--families", "--output", str(path)], check=True, cwd=root,
                       creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        rows.append(summarize(json.loads(path.read_text(encoding="utf-8"))))
    summary = {"scope": "Fixed seeds, default physiology, random mature founders; no behavior tuning.",
               "evidence": str(evidence), "runs": rows,
               "any_reproductive_continuity": any(r["nonzero_reproductive_continuity"] for r in rows)}
    with args.output.open("x", encoding="utf-8") as file:
        json.dump(summary, file, indent=2)
    print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    main()
