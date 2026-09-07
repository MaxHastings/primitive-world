"""Summarize read-only directional-lock-in evidence from evolution reports.

This script makes no causal claim: an agent population can collapse for many
reasons.  It labels a world only when repeated sampled horizontal movement has
the same strong sign before a recorded extinction in that same world.
"""

import argparse
import json
import re
from collections import defaultdict
from pathlib import Path


def classify(report, threshold):
    """Return per-world repeated direction evidence and report-level totals."""
    worlds = defaultdict(list)
    for sample in report.get("history", []):
        progress = sample.get("progress", {})
        evolution = sample.get("evolution", {})
        world = progress.get("world")
        if world is not None:
            worlds[world].append((sample, evolution))

    results = []
    for world, samples in sorted(worlds.items()):
        strong = []
        for sample, evolution in samples:
            if sample.get("metrics", {}).get("living", 0) <= 0:
                continue
            bias = evolution.get("horizontal_directional_bias", 0.0)
            if abs(bias) >= threshold:
                strong.append((1 if bias > 0 else -1, bias, sample))
        repeated = any(a[0] == b[0] for a, b in zip(strong, strong[1:]))
        extinct = any(sample.get("metrics", {}).get("living", 0) == 0 for sample, _ in samples)
        if repeated:
            sign = next(a[0] for a, b in zip(strong, strong[1:]) if a[0] == b[0])
            results.append(
                {
                    "world": world,
                    "direction": "right" if sign > 0 else "left",
                    "strong_samples": len(strong),
                    "extinction_recorded": extinct,
                    "biases": [round(item[1], 4) for item in strong],
                    "living": [item[2]["metrics"]["living"] for item in strong],
                }
            )
    return results


def summarize(path, threshold):
    report = json.loads(path.read_text(encoding="utf-8"))
    locks = classify(report, threshold)
    seed = re.search(r"seed(\d+)", path.stem)
    return {
        "file": str(path),
        "seed": int(seed.group(1)) if seed else None,
        "completed_comparisons": report.get("completed_comparisons"),
        "accepted_challengers": report.get("final_progress", {}).get("accepted_challengers"),
        "earned_environment_floor": report.get("earned_environment_floor"),
        "termination_reason": report.get("termination_reason"),
        "lock_in_worlds": locks,
        "lock_in_world_count": len(locks),
        "lock_in_extinction_count": sum(item["extinction_recorded"] for item in locks),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("reports", nargs="+", help="Evolution JSON reports")
    parser.add_argument("--threshold", type=float, default=0.25)
    args = parser.parse_args()
    if not 0.0 < args.threshold <= 1.0:
        parser.error("--threshold must be in (0, 1]")
    summaries = [summarize(Path(name), args.threshold) for name in args.reports]
    total = {
        "reports": len(summaries),
        "reports_with_lock_in": sum(bool(item["lock_in_world_count"]) for item in summaries),
        "lock_in_worlds": sum(item["lock_in_world_count"] for item in summaries),
        "lock_in_extinctions": sum(item["lock_in_extinction_count"] for item in summaries),
    }
    print(json.dumps({"threshold": args.threshold, "total": total, "runs": summaries}, indent=2))


if __name__ == "__main__":
    main()
