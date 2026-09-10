"""Read-only summary of a completed or still-running unassisted audit batch.

Usage: python tools/summarize_reservoir_sequence.py REPORT_DIRECTORY [CONTINUATION_DIRECTORY ...]
Prints JSON; never changes the run, checkpoints, or source reports.
"""
import json
import statistics
import sys
from pathlib import Path


def summarize(directories):
    located = [(directory, json.loads(line)) for directory in directories
               for line in (directory / "worlds.jsonl").read_text().splitlines()]
    rows = [row for _, row in located]
    assert rows, "No completed world observations yet"
    assert all(b["world"] == a["world"] + 1 for a, b in zip(rows, rows[1:]))
    assert all(not r["settings"]["founder_genomes"] for r in rows)
    assert all(r["settings"]["population"] == 4096 for r in rows)
    assert all(not r["progress"]["completed"]["assisted"] for r in rows if not r["censored"])
    fields = ["packets_produced", "births", "juvenile_transfer_ticks", "juvenile_received_milli",
              "juveniles_matured", "matured_descendants_gathered", "matured_descendant_packets",
              "births_involving_descendant_parents"]
    children = []
    first = {"fed_juvenile": None, "maturation": None, "matured_descendant_reproduction": None}
    offset = 0
    for directory, row in located:
        raw = json.loads((directory / f"world-{row['world']}.json").read_text())
        assert len(raw["funnel"]["individuals"]) == row["funnel"]["births"]
        children.extend(raw["funnel"]["individuals"].values())
        for event in sorted(raw["funnel"]["life_events"], key=lambda e: e[1]):
            key = {8: "fed_juvenile", 2: "maturation"}.get(event[0])
            if key and first[key] is None:
                first[key] = {"world": row["world"], "lineage": event[2],
                              "world_tick": event[1], "cumulative_tick": offset + event[1]}
        for child in sorted(raw["funnel"]["individuals"].values(), key=lambda c: c["birth_tick"]):
            if child["closed_depth_at_birth"] > 0 and first["matured_descendant_reproduction"] is None:
                first["matured_descendant_reproduction"] = {
                    "world": row["world"], "child_lineage": child["lineage"], "parents": child["parents"],
                    "world_tick": child["birth_tick"], "cumulative_tick": offset + child["birth_tick"]}
        offset += row["tick"]
        if "cumulative_ticks" in row:
            assert row["cumulative_ticks"] == offset, "Pass the full chain's directories in order"
    durations = [r["progress"]["completed"]["duration"] for r in rows if not r["censored"]]
    births = [r["funnel"]["births"] for r in rows if not r["censored"]]
    quarter = max(1, len(durations) // 4)
    return {
        "observed_worlds": len(rows), "world_range": [rows[0]["world"], rows[-1]["world"]],
        "completed_batch": (directories[-1] / "continuation.checkpoint").is_file(),
        "censored_worlds": sum(r["censored"] for r in rows),
        "total_ticks": sum(r["tick"] for r in rows),
        "funnel": {f: sum(r["funnel"][f] for r in rows) for f in fields},
        "closed_worlds": [r["world"] for r in rows if r["funnel"]["maximum_closed_life_cycle_depth"] > 0],
        "maximum_closed_depth": max(r["funnel"]["maximum_closed_life_cycle_depth"] for r in rows),
        "first_milestones": first,
        "natural_world_durations": {
            "minimum": min(durations, default=None), "maximum": max(durations, default=None),
            "median": statistics.median(durations) if durations else None,
            "first_quarter_median": statistics.median(durations[:quarter]) if durations else None,
            "last_quarter_median": statistics.median(durations[-quarter:]) if durations else None,
            "worlds_over_10000": sum(t > 10000 for t in durations),
            "worlds_over_20000": sum(t > 20000 for t in durations)},
        "births_per_completed_world": {
            "first_quarter_mean": statistics.mean(births[:quarter]) if births else None,
            "last_quarter_mean": statistics.mean(births[-quarter:]) if births else None,
            "scope": "Descriptive time comparison within one chain, not a causal estimate of selection or a replicated trend."},
        "offspring_ever_received": sum(c["juvenile_received_milli"] > 0 for c in children),
        "maximum_offspring_death_age": max((c["death_age"] for c in children if c["dead"]), default=None),
        "pool_distinct_genomes_initial": rows[0]["initial_search"]["distinct_pool_genomes"],
        "pool_distinct_genomes_final": rows[-1]["final_search"]["distinct_pool_genomes"],
        "pool_changed_record_slots_summed": sum(r["final_search"]["pool_slots_with_changed_record_since_sample"] for r in rows),
        "scope": "One consecutive reservoir sequence, not independent seeds. Changed slots underestimate intervening replacements. A completed batch is not evidence of closure; inspect closed_worlds. An in-progress file may require retry if its final line is being written."
    }


if __name__ == "__main__":
    print(json.dumps(summarize([Path(p) for p in sys.argv[1:]]), indent=2))
