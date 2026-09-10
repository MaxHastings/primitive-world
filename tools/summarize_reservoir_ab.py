"""Summarize the two saved-state admission arms without changing either run.

Usage: python tools/summarize_reservoir_ab.py REPORT_ROOT
Can inspect a running arm; a partially written final journal line may need retry.
"""
import json
import statistics
from collections import Counter
import sys
from pathlib import Path


def arm(directory):
    if not (directory / "worlds.jsonl").exists():
        return None, []
    rows = [json.loads(line) for line in (directory / "worlds.jsonl").read_text().splitlines()]
    if not rows:
        return None, []
    assert all(b["world"] == a["world"] + 1 for a, b in zip(rows, rows[1:]))
    assert sum(r["segment_ticks"] for r in rows) == rows[-1]["elapsed_ticks"]
    fields = ["packets_produced", "births", "juvenile_transfer_ticks", "juvenile_received_milli",
              "juveniles_matured", "matured_descendants_gathered", "matured_descendant_packets",
              "births_involving_descendant_parents"]
    children = []
    first = {"fed_juvenile": None, "maturation": None, "matured_descendant_reproduction": None}
    offset = 0
    for row in rows:
        raw = json.loads((directory / f"world-{row['world']}.json").read_text())
        assert len(raw["funnel"]["individuals"]) == row["funnel"]["births"]
        deliveries = Counter(e[2] for e in raw["funnel"]["life_events"] if e[0] == 8)
        for child in raw["funnel"]["individuals"].values():
            child["delivery_ticks"] = deliveries[child["lineage"]]
            children.append(child)
        for event in sorted(raw["funnel"]["life_events"], key=lambda e: e[1]):
            key = {8: "fed_juvenile", 2: "maturation"}.get(event[0])
            if key and first[key] is None:
                first[key] = {"world": row["world"], "lineage": event[2],
                              "world_tick": event[1], "ticks_since_fork": offset + event[1] - row["start_tick"]}
        for child in sorted(raw["funnel"]["individuals"].values(), key=lambda c: c["birth_tick"]):
            if child["closed_depth_at_birth"] > 0 and first["matured_descendant_reproduction"] is None:
                first["matured_descendant_reproduction"] = {
                    "world": row["world"], "child_lineage": child["lineage"], "parents": child["parents"],
                    "ticks_since_fork": offset + child["birth_tick"] - row["start_tick"]}
        offset += row["segment_ticks"]
    completed = [r for r in rows if r["natural_extinction"] and r["start_tick"] == 0]
    durations = [r["progress"]["completed"]["duration"] for r in completed]
    births = [r["funnel"]["births"] for r in completed]
    quarter = max(1, len(completed) // 4)
    bins = [(0,0,"0"),(1,1,"1"),(2,5,"2-5"),(6,20,"6-20"),(21,50,"21-50"),
            (51,100,"51-100"),(101,200,"101-200"),(201,399,"201-399"),(400,10**9,"400+")]
    feeding_distribution = []
    for low, high, label in bins:
        group = [c for c in children if low <= c["delivery_ticks"] <= high]
        dead_ages = [c["death_age"] for c in group if c["dead"]]
        feeding_distribution.append({"delivery_ticks": label, "offspring": len(group),
            "alive_at_censor": sum(not c["dead"] for c in group),
            "median_death_age": statistics.median(dead_ages) if dead_ages else None,
            "maximum_death_age": max(dead_ages, default=None),
            "total_food_milli": sum(c["juvenile_received_milli"] for c in group)})
    completion = directory / "completion.json"
    return {
        "observed_worlds": len(rows), "elapsed_ticks": offset,
        "completion": json.loads(completion.read_text()) if completion.exists() else None,
        "natural_extinctions": sum(r["natural_extinction"] for r in rows),
        "funnel": {k: sum(r["funnel"][k] for r in rows) for k in fields},
        "closed_worlds": [r["world"] for r in rows if r["funnel"]["maximum_closed_life_cycle_depth"] > 0],
        "maximum_closed_depth": max(r["funnel"]["maximum_closed_life_cycle_depth"] for r in rows),
        "first_milestones": first,
        "per_juvenile_feeding_distribution": feeding_distribution,
        "offspring_ever_received": sum(c["juvenile_received_milli"] > 0 for c in children),
        "offspring_dead": sum(c["dead"] for c in children),
        "offspring_with_repeated_deliveries": sum(c["delivery_ticks"] > 1 for c in children),
        "maximum_delivery_ticks_to_one_offspring": max((c["delivery_ticks"] for c in children), default=0),
        "maximum_food_milli_to_one_offspring": max((c["juvenile_received_milli"] for c in children), default=0),
        "maximum_offspring_death_age": max((c["death_age"] for c in children if c["dead"]), default=None),
        "natural_world_duration_median": statistics.median(durations) if durations else None,
        "natural_world_duration_maximum": max(durations, default=None),
        "first_quarter_mean_births": statistics.mean(births[:quarter]) if births else None,
        "last_quarter_mean_births": statistics.mean(births[-quarter:]) if births else None,
        "pool_distinct_genomes_initial": rows[0]["initial_search"]["distinct_pool_genomes"],
        "pool_distinct_genomes_final": rows[-1]["final_search"]["distinct_pool_genomes"],
        "pool_changed_record_slots_summed": sum(r["final_search"]["pool_slots_with_changed_record_since_sample"] for r in rows),
        "scope": "Funnel counts only events after the shared fork. Duration/quarter summaries exclude the partially shared first world and censored last world. Changed slots underestimate intervening replacements. Single paired fork, not replicated causal confidence."
    }, rows


def summarize(root):
    result, by_arm = {}, {}
    for label in ["A-birth", "B-mature-parent"]:
        result[label], by_arm[label] = arm(root / label)
    a = {r["world"]: r["seed"] for r in by_arm["A-birth"]}
    b = {r["world"]: r["seed"] for r in by_arm["B-mature-parent"]}
    common = a.keys() & b.keys()
    assert all(a[w] == b[w] for w in common)
    result["matching_world_seeds_checked"] = len(common)
    return result


if __name__ == "__main__":
    print(json.dumps(summarize(Path(sys.argv[1])), indent=2))
