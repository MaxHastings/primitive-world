"""Summarize unassisted offspring and paid packet investment; no selection feedback."""
import json
import statistics
import struct
import sys
from pathlib import Path


def distribution(values):
    values = sorted(values)
    return {"count": len(values), "minimum": values[0] if values else None,
            "median": statistics.median(values) if values else None,
            "maximum": values[-1] if values else None}


def summarize(root):
    rows = [json.loads(p.read_text()) for p in root.glob("world-*.json")]
    rows.sort(key=lambda r: r["elapsed_ticks"])
    children = [dict(c, world=r["world"]) for r in rows for c in r["funnel"]["individuals"].values()]
    packets = [struct.unpack("<f", struct.pack("<I", e[8]))[0]
               for r in rows for e in r["funnel"]["life_events"] if e[0] == 9]
    groups = []
    for low, high in [(0, 16), (16, 32), (32, 48), (48, 64), (64, 80), (80, 1000)]:
        group = [c for c in children if low < c["birth_energy"] <= high]
        groups.append({"birth_energy_interval": [low, high], "lower_exclusive": True,
                       "offspring": len(group), "matured": sum(c["maturity_tick"] is not None for c in group),
                       "death_age": distribution([c["death_age"] for c in group if c["dead"]]),
                       "received_food": distribution([c["juvenile_received_milli"] / 1000 for c in group])})
    fusion_investments = []
    matched_packet_ids = 0
    for r in rows:
        events = r["funnel"]["life_events"]
        made = {e[2]: struct.unpack("<f", struct.pack("<I", e[8]))[0] for e in events if e[0] == 9}
        births = {(e[1], e[10]): struct.unpack("<f", struct.pack("<I", e[8]))[0] for e in events if e[0] == 1}
        for e in events:
            if e[0] != 6:
                continue
            pair = [made[e[2]], made[e[9]]]
            matched_packet_ids += 2
            birth_energy = births.get((e[1], e[4]))
            fusion_investments.append({"manufactured_energy": pair, "birth_energy": birth_energy,
                "decay_loss_before_successful_fusion": sum(pair) - r["settings"]["fusion_loss"] - birth_energy if birth_energy is not None else None})
    certificates = []
    for r in rows:
        individuals = r["funnel"]["individuals"]
        for child in individuals.values():
            for pid in child["parents"]:
                parent = individuals.get(str(pid))
                if parent and parent["maturity_tick"] is not None and parent["maturity_tick"] <= child["birth_tick"]:
                    certificates.append({"world": r["world"], "parent": parent, "offspring": child})
    completion = root / "completion.json"
    return {"completion": json.loads(completion.read_text()) if completion.exists() else None,
            "elapsed_ticks_reported": sum(r["segment_ticks"] for r in rows),
            "worlds_reported": len(rows), "births": len(children),
            "matured": sum(c["maturity_tick"] is not None for c in children),
            "birth_energy": distribution([c["birth_energy"] for c in children]),
            "packet_manufacture_energy": distribution(packets),
            "packets_over_32_energy": sum(x > 32 for x in packets),
            "packets_over_40_energy": sum(x > 40 for x in packets),
            "packets_participating_in_fusion_attempts": matched_packet_ids,
            "fusion_initial_total_energy": distribution([sum(f["manufactured_energy"]) for f in fusion_investments]),
            "successful_fusion_decay_loss": distribution([f["decay_loss_before_successful_fusion"] for f in fusion_investments if f["birth_energy"] is not None]),
            "largest_initial_investment_fusions": sorted(fusion_investments, key=lambda f: sum(f["manufactured_energy"]), reverse=True)[:10],
            "offspring_by_endowment": groups, "actual_life_cycle_certificates": certificates,
            "scope": "Only complete reported segments. Energy intervals include the upper boundary. Manufacture energy is recorded before first packet decay. Certificates require a descendant birth record, recorded maturity and subsequent actual dual-parent fusion birth; reservoir founders never count."}


if __name__ == "__main__":
    print(json.dumps(summarize(Path(sys.argv[1])), indent=2))
