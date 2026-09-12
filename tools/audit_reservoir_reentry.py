"""Compare immutable v58/v59 checkpoints without exporting inherited records.

Usage: python tools/audit_reservoir_reentry.py INITIAL_CHECKPOINT LATER_CHECKPOINT
Reports exact record membership, not hashes or chosen genotypes. No files change.
"""
import json
import mmap
import re
import struct
import sys
from pathlib import Path


class Checkpoint:
    def __init__(self, path):
        self.file = Path(path).open("rb")
        self.data = mmap.mmap(self.file.fileno(), 0, access=mmap.ACCESS_READ)
        assert self.data[:12] in (b"PRIMWORLD058", b"PRIMWORLD059")
        self.body_size = 296 if self.data[:12] == b"PRIMWORLD058" else 312
        self.seed, self.tick, length = struct.unpack_from("<III", self.data, 12)
        self.metadata = json.loads(self.data[24:24 + length])
        self.tick |= self.metadata.get("tick_high", 0) << 32
        offset = 24 + length
        self.buffers = []
        for _ in range(18):
            size, = struct.unpack_from("<Q", self.data, offset)
            offset += 8
            self.buffers.append((offset, size))
            offset += size
        assert offset == len(self.data)

    def row(self, buffer, index, stride):
        offset, size = self.buffers[buffer]
        assert (index + 1) * stride <= size
        return self.data[offset + index * stride:offset + (index + 1) * stride]

    def close(self):
        self.data.close()
        self.file.close()


def body_layout():
    source = (Path(__file__).resolve().parents[1] / "src/model.rs").read_text()
    body = source.split("pub struct AgentGpu {", 1)[1].split("\n}", 1)[0]
    fields = {}
    offset = 0
    for name, kind in re.findall(r"pub (\w+): ([^,]+),", body):
        count = 1
        if kind.startswith("["):
            kind, count = kind[1:-1].split(";")
            count = 16 if count.strip() == "HIDDEN" else int(count)
        assert kind.strip() in ("f32", "u32")
        fields[name] = (offset, count * 4)
        offset += count * 4
    assert offset == 312
    return fields, offset


def audit(initial_path, later_path):
    initial, later = Checkpoint(initial_path), Checkpoint(later_path)
    fields, _ = body_layout()
    bank_stride = 1247 * 4

    def field(body, name):
        offset, size = fields[name]
        return body[offset:offset + size]

    def integer(body, name):
        return int.from_bytes(field(body, name), "little")

    def genome(checkpoint, slot, pool=False):
        banks = (13, 14) if pool else (8, 9)
        return b"".join(checkpoint.row(bank, slot, bank_stride) for bank in banks)

    def traits(body):
        return field(body, "active_mask") + bytes(8) + b"".join(field(body, name) for name in [
            "packet_size", "plasticity_rate", "trace_retention", "learned_weight_retention",
            "parameter_mutation_rate", "parameter_mutation_step", "topology_mutation_rate"])

    try:
        assert initial.tick == 0 and initial.seed == 3001
        assert initial.metadata["settings"] == later.metadata["settings"]
        originals = set()
        for slot in range(4096):
            body = initial.row(0, slot, initial.body_size)
            assert integer(body, "alive") == 1 and integer(body, "ancestry_depth") == 0
            originals.add(genome(initial, slot) + traits(body))
        for slot in range(4096):
            assert genome(initial, slot, True) + initial.row(15, slot, 100) in originals
        pool_records = [genome(later, slot, True) + later.row(15, slot, 100) for slot in range(4096)]
        new_pool_records = sum(record not in originals for record in pool_records)
        living_founders, new_living_founders = 0, 0
        for slot in range(16384):
            body = later.row(0, slot, later.body_size)
            if integer(body, "alive") == 1 and integer(body, "ancestry_depth") == 0:
                living_founders += 1
                new_living_founders += genome(later, slot) + traits(body) not in originals
        return {
            "initial_seed": initial.seed, "initial_distinct_founder_records": len(originals),
            "later_world": later.metadata["progress"]["world"], "later_tick": later.tick,
            "pool_records": 4096, "pool_records_absent_from_original_founders": new_pool_records,
            "living_adult_founders": living_founders,
            "living_adult_founders_with_records_absent_from_original_founders": new_living_founders,
            "comparison": "Exact inherited genome bytes plus inherited cognitive traits; excludes learned deltas, traces, lifetime state and diagnostic ancestry labels.",
            "scope": "New records can be recombinations or mutations, not necessarily adaptive innovations. Together with the separately verified absence of all offspring maturation, adult-founder re-entry demonstrates that development was not required for these inherited records to cross extinction. Counts do not measure the causal fitness effect of a particular care mutation."
        }
    finally:
        initial.close()
        later.close()


if __name__ == "__main__":
    print(json.dumps(audit(*sys.argv[1:]), indent=2))
