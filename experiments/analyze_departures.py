"""Read schema2 journey JSONL; summarize failed/censored attempts without scoring genes."""
import argparse
from collections import Counter
import hashlib
import json
import math
from pathlib import Path
import statistics


def distribution(values):
    values = [v for v in values if v is not None and math.isfinite(v)]
    return {"n": len(values), "min": min(values), "median": statistics.median(values),
            "max": max(values)} if values else {"n": 0}


def analyze(path, metabolic, movement):
    records, completed, footer, header = [], 0, None, None
    with path.open(encoding="utf-8") as source:
        for line in source:
            item = json.loads(line)
            if item["type"] == "header":
                header = item
            elif item["type"] == "summary":
                footer = item
            elif item["type"] == "journey":
                completed += 1
            elif item["type"] == "ended_attempt":
                records.append(item["evidence"])
    if header is None or footer is None or header["observer"]["schema"] != 2:
        raise ValueError("Require completed schema2 observation, not a partial file")
    rows = []
    for r in records:
        points = [p for p in r["waypoints"] if p["tick"] >= r["departure_tick"]]
        first, last = points[0], points[-1]
        ticks = last["tick"] - first["tick"]
        path_distance = last["distance_travelled"] - first["distance_travelled"]
        net = math.dist(first["position"], last["position"])
        reserves = first["energy"] + 8 * first["inventory"]
        speed = path_distance / ticks if ticks else 0
        age_budget = max(0, first["max_age"] - first["age"])
        max_speed = first["max_speed"]
        optimistic_range = min(reserves * max_speed / (metabolic + movement*max_speed), max_speed*age_budget)
        constant_speed_range = min(reserves * speed / (metabolic + movement*speed), speed*age_budget)
        nearest = r["nearest_destination_at_departure"]
        terminal = r["terminal_observation"]
        rows.append({"lineage_id": r["lineage_id"], "departure_tick": r["departure_tick"],
            "end_reason": r["end_reason"], "last_stage": r["last_stage"],
            "energy_at_departure": first["energy"], "inventory_at_departure": first["inventory"],
            "reserves_at_departure": reserves, "remaining_age": age_budget,
            "ticks_observed_after_departure": ticks, "path_after_departure": path_distance,
            "net_after_departure": net, "net_to_path": net/path_distance if path_distance > 0 else None,
            "mean_path_speed": speed, "optimistic_max_speed_range": optimistic_range,
            "illustrative_constant_observed_speed_range": constant_speed_range,
            "nearest_food_distance_at_departure": nearest["distance"] if nearest else None,
            "nearest_beyond_optimistic_range": nearest["distance"] > optimistic_range if nearest else None,
            "nearest_beyond_observed_speed_range": nearest["distance"] > constant_speed_range if nearest else None,
            "rich_footprint_samples_after_departure": sum(p["local_vegetation"] >= .04 for p in points),
            "collection_samples_after_departure": sum(p["collected_last_tick"] > 0 for p in points),
            "reproduction_after_departure": last["lifetime_births"] - first["lifetime_births"],
            "last_seen_energy": last["energy"],
            "terminal_energy": terminal["energy"] if terminal else None,
            "terminal_age_limit": terminal["age"] >= terminal["max_age"] if terminal else None})
    groups = {}
    for name, subset in [("all", rows), ("poor_crossing", [r for r in rows if r["last_stage"] == "poor_crossing"])]:
        numeric = ["energy_at_departure", "inventory_at_departure", "reserves_at_departure", "remaining_age",
                   "ticks_observed_after_departure", "path_after_departure", "net_after_departure", "net_to_path",
                   "mean_path_speed", "optimistic_max_speed_range", "illustrative_constant_observed_speed_range",
                   "nearest_food_distance_at_departure", "rich_footprint_samples_after_departure",
                   "collection_samples_after_departure", "reproduction_after_departure", "last_seen_energy"]
        groups[name] = {"n": len(subset), "end_reasons": dict(Counter(r["end_reason"] for r in subset)),
            "stages": dict(Counter(r["last_stage"] for r in subset)),
            "distribution": {k: distribution([r[k] for r in subset]) for k in numeric},
            "nearest_exists": sum(r["nearest_food_distance_at_departure"] is not None for r in subset),
            "nearest_beyond_max_speed_range": sum(r["nearest_beyond_optimistic_range"] is True for r in subset),
            "nearest_beyond_observed_speed_range": sum(r["nearest_beyond_observed_speed_range"] is True for r in subset),
            "observed_dead_with_zero_energy": sum(r["end_reason"] == "observed_dead" and r["terminal_energy"] == 0 for r in subset)}
    return {"schema": 1, "source": str(path), "source_sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
        "metabolic_cost": metabolic, "movement_cost": movement, "observer": footer["observer"],
        "completed_journeys": completed, "groups": groups, "attempts": rows,
        "limits": "Sampled attempts that meet explicit departure definition, not all agents. Energy+8*inventory assumes all stored food can be converted. Max-speed range is an optimistic no-new-food straight-line bound, clipped by remaining lifespan; it ignores ingestion timing, juvenile slowdown and reproduction. Observed-speed range is illustrative, not a physical impossibility bound. Nearest food is the departure-time landscape and need not persist or be discoverable. Missing identity is not a diagnosed death. No metric enters control or selection."}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("input", type=Path)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--metabolic-cost", type=float, required=True)
    parser.add_argument("--movement-cost", type=float, required=True)
    args = parser.parse_args()
    if args.metabolic_cost <= 0 or args.movement_cost < 0:
        parser.error("Require positive metabolism and nonnegative movement cost")
    result = analyze(args.input, args.metabolic_cost, args.movement_cost)
    with args.output.open("x", encoding="utf-8") as out:
        json.dump(result, out, indent=2)
    print(json.dumps(result["groups"], indent=2))
