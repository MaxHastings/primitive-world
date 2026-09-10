"""Exact inherited-record membership across an A/B fork; read-only checkpoints.
Usage: python tools/audit_admission_membership.py FORK_CHECKPOINT LATER_CHECKPOINT
The eligible starting set includes the pool AND still-live founder/packet records.
"""
import json
import sys
from audit_reservoir_reentry import Checkpoint, body_layout


def audit(fork_path, later_path):
    fork, later = Checkpoint(fork_path), Checkpoint(later_path)
    fields, body_size = body_layout()
    stride = 1247 * 4
    def field(body, name):
        start, size = fields[name]
        return body[start:start + size]
    def integer(body, name):
        return int.from_bytes(field(body, name), "little")
    def genome(c, slot, pool=False):
        return b"".join(c.row(bank, slot, stride) for bank in ((13, 14) if pool else (8, 9)))
    def traits(body):
        return field(body, "active_mask") + bytes(8) + b"".join(field(body, name) for name in [
            "packet_size", "plasticity_rate", "trace_retention", "learned_weight_retention",
            "parameter_mutation_rate", "parameter_mutation_step", "topology_mutation_rate"])
    def pool(c):
        return [genome(c, i, True) + c.row(15, i, 100) for i in range(4096)]
    try:
        assert fork.metadata["settings"] == later.metadata["settings"]
        originals = set(pool(fork))
        pool_only = set(originals)
        for slot in range(16384):
            body = fork.row(0, slot, body_size)
            state = integer(body, "alive")
            if state == 1:
                assert integer(body, "ancestry_depth") == 0, "Fork has a living descendant; eligibility must be audited explicitly"
            if state in (1, 2):
                originals.add(genome(fork, slot) + traits(body))
        records = pool(later)
        return {"fork_world": fork.metadata["progress"]["world"], "fork_tick": fork.tick,
                "later_world": later.metadata["progress"]["world"], "later_tick": later.tick,
                "fork_distinct_pool_records": len(pool_only),
                "fork_distinct_eligible_records_including_live_founders_and_packets": len(originals),
                "later_pool_records_absent_from_fork_eligible_set": sum(r not in originals for r in records),
                "later_distinct_pool_records_absent_from_fork_eligible_set": len(set(records) - originals),
                "comparison": "Exact complete inherited bank bytes plus cognitive traits, excluding all lifetime learning and state.",
                "interpretation": "With independently verified zero maturation, B must retain only eligible starting records. A may admit new recombinants regardless of development. Membership alone does not establish ancestry or adaptive improvement."}
    finally:
        fork.close()
        later.close()


if __name__ == "__main__":
    print(json.dumps(audit(*sys.argv[1:]), indent=2))
