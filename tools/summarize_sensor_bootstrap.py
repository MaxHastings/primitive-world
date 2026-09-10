"""Summarize the completed, predeclared 100-world bootstrap cohort.

Usage: python tools/summarize_sensor_bootstrap.py REPORT_DIRECTORY
Raw reports are preserved. Refuse incomplete or altered seed sets.
"""
import json
import statistics
import sys
from pathlib import Path


def summarize(directory: Path) -> dict:
    cohort = json.loads((directory / "random-cohort.json").read_text(encoding="utf-8"))
    worlds = cohort["worlds"]
    assert [w["seed"] for w in worlds] == list(range(2001, 2101))
    assert all(w["tick"] <= 20000 for w in worlds)
    assert all(not w["censored"] or w["tick"] == 20000 for w in worlds)
    funnel_fields = [
        "packets_produced", "compatible_packet_coexist_ticks", "births",
        "juvenile_transfer_ticks", "juvenile_received_milli", "juveniles_matured",
        "matured_descendants_gathered", "matured_descendant_packets",
        "matured_descendants_produced_packets", "births_involving_descendant_parents",
        "adult_descendant_gathered_milli", "juvenile_gathered_milli",
        "founder_food_seen_ticks", "founder_harvest_ticks", "all_harvested_milli",
    ]
    totals = {k: sum(w["funnel"][k] for w in worlds) for k in funnel_fields}
    reached = {k: sum(w["funnel"][k] > 0 for w in worlds) for k in funnel_fields}
    signal_fields = [
        "emissions", "nonzero_aggregate_exposure_ticks",
        "transfer_selected_during_exposure", "reproduction_selected_during_exposure",
        "actual_packet_productions_with_exposure_within_16_ticks",
    ]
    signals = {k: sum(w["funnel"]["signals"][k] for w in worlds) for k in signal_fields}
    for w in worlds:
        w["funnel"]["signals"].pop("actual_transfers_with_exposure_within_16_ticks", None)
    individuals = []
    for w in worlds:
        raw = json.loads((directory / f"random-{w['seed']}.json").read_text(encoding="utf-8"))
        assert raw["funnel"]["births"] == w["funnel"]["births"]
        assert raw["settings"]["population"] == 4096
        assert raw["funnel"]["births"] == len(raw["funnel"]["individuals"])
        for child in raw["funnel"]["individuals"].values():
            individuals.append({"seed": w["seed"], **child})
    dead = [p["death_age"] for p in individuals if p["dead"]]
    depths = [w["funnel"]["maximum_closed_life_cycle_depth"] for w in worlds]
    return {
        "worlds": 100,
        "seeds": [2001, 2100],
        "horizon": 20000,
        "naturally_extinct_worlds": sum(not w["censored"] for w in worlds),
        "censored_worlds": sum(w["censored"] for w in worlds),
        "worlds_reaching_10000": sum(w["at_10000"] is not None for w in worlds),
        "closure_worlds": sum(d > 0 for d in depths),
        "maximum_closed_life_cycle_depth": max(depths),
        "maximum_genealogical_depth": max(w["funnel"]["maximum_genealogical_depth"] for w in worlds),
        "funnel_totals": totals,
        "worlds_reaching_each_measure": reached,
        "signals": signals,
        "offspring": {
            "born": len(individuals),
            "ever_received_food": sum(p["juvenile_received_milli"] > 0 for p in individuals),
            "dead": len(dead),
            "maximum_recorded_death_age": max(dead, default=None),
            "median_recorded_death_age": statistics.median(dead) if dead else None,
        },
        "world_records": worlds,
        "scope": (
            "100 fresh independent random-policy worlds; ordinary population4096, "
            "default frozen rules.20k or natural extinction; no selection of seeds. "
            "This measures bootstrap, not evolutionary search across reservoir restarts. "
            "Global compatible coexistence is not a claim of local funded contact. "
            "Signal associations are observational. The prototype raw observer's "
            "actual_transfers_with_exposure_within_16_ticks field is excluded: its "
            "inventory-debit proxy can include terminal food drops, so it is not "
            "a validated count of actual transfers. Transfer selection and actual "
            "packet-production associations remain reported."
        ),
    }


def markdown(result: dict) -> str:
    totals = result["funnel_totals"]
    worlds = result["worlds_reaching_each_measure"]
    stages = [
        ("Packets produced", "packets_produced"),
        ("Compatible packet coexistence (world-ticks)", "compatible_packet_coexist_ticks"),
        ("Births", "births"),
        ("Juvenile delivery ticks", "juvenile_transfer_ticks"),
        ("Juveniles matured", "juveniles_matured"),
        ("Matured descendants that gathered", "matured_descendants_gathered"),
        ("Matured descendants that produced packets", "matured_descendants_produced_packets"),
        ("Packets produced by matured descendants", "matured_descendant_packets"),
        ("Births involving descendant parents", "births_involving_descendant_parents"),
    ]
    lines = [
        "# Random-policy bootstrap: seeds 2001–2100",
        "",
        f"Horizon:20,000 ticks or natural extinction. {result['naturally_extinct_worlds']} "
        f"extinct; {result['censored_worlds']} censored. "
        f"{result['worlds_reaching_10000']} reached tick10,000.",
        "",
        "| Measurement | Total | Worlds with positive result |",
        "| --- | ---: | ---: |",
    ]
    lines += [f"| {label} | {totals[key]:,} | {worlds[key]}/100 |" for label, key in stages]
    lines += [
        "",
        f"Closed-loop worlds: **{result['closure_worlds']}/100**. Maximum confirmed "
        f"closed-life-cycle depth: **{result['maximum_closed_life_cycle_depth']}**. "
        f"Maximum genealogical depth: {result['maximum_genealogical_depth']}.",
        "",
        f"Juveniles received {totals['juvenile_received_milli']/1000:.3f} food in total; "
        f"{result['offspring']['ever_received_food']} offspring received any. "
        f"Maximum recorded death age: {result['offspring']['maximum_recorded_death_age']} ticks.",
        "",
        "Signals are optional observations:",
        "",
    ]
    lines += [f"- {k.replace('_', ' ')}: {v:,}." for k, v in result["signals"].items()]
    lines += ["", result["scope"], ""]
    return "\n".join(lines)


if __name__ == "__main__":
    directory = Path(sys.argv[1])
    result = summarize(directory)
    with (directory / "bootstrap-summary.json").open("x", encoding="utf-8") as handle:
        json.dump(result, handle, indent=2)
    with (directory / "bootstrap-summary.md").open("x", encoding="utf-8") as handle:
        handle.write(markdown(result))
    print(markdown(result))
