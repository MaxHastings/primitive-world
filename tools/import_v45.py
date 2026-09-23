"""One-time, read-only v45 checkpoint conversion into an isolated v46 save.

The source receipt and checkpoint are only opened for reading. The output is
published after its full payload has been written and synced. V46 must load and
validate the result before it is used as a wallpaper experiment.
"""

import argparse
import array
import hashlib
import json
import os
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import time

OLD_GRID = 512
NEW_GRID = 512
BUFFER_COUNT = 18
GRID_BUFFERS = {1, 2, 3, 17}
OLD_MAGIC = b"PRIMWORLD059"
NEW_MAGIC = b"PRIMWORLD062"
MODEL_OLD = "primitive-v45-depth-retention"
MODEL_NEW = "primitive-v46-fast-evolution"


def old_root():
    return Path(os.environ["LOCALAPPDATA"]) / "PrimitiveWorld" / "experiments"


def new_root():
    override = os.environ.get("PRIMITIVE_WORLD_SAVES")
    return Path(override) if override else Path(os.environ["LOCALAPPDATA"]) / "PrimitiveWorldV46" / "experiments"


def read_receipt(path):
    receipt = json.loads(path.read_text(encoding="utf-8"))
    if receipt.get("version") != 4 or receipt.get("model") != MODEL_OLD:
        raise ValueError("Expected a v45 receipt")
    name = receipt.get("checkpoint")
    if not isinstance(name, str) or Path(name).name != name or not name.endswith(".checkpoint"):
        raise ValueError("Invalid source checkpoint name")
    checkpoint = path.parent / name
    if not checkpoint.is_file():
        raise ValueError("Source checkpoint is missing")
    return receipt, checkpoint


def latest_receipt():
    candidates = []
    for path in old_root().glob("experiment-*/save-*.json"):
        try:
            record, _ = read_receipt(path)
            candidates.append((record["total_ticks"], record["saved_at_ms"], path))
        except (OSError, ValueError, KeyError, json.JSONDecodeError):
            continue
    if not candidates:
        raise ValueError("No complete v45 save found")
    return max(candidates)[2]


def words(blob):
    values = array.array("I")
    values.frombytes(blob)
    if sys.byteorder != "little":
        values.byteswap()
    return values


def packed_words(values):
    if sys.byteorder != "little":
        values.byteswap()
    return values.tobytes()


def regrid(food_blob, soil_blob, ground_blob, ecology_blob):
    if tuple(map(len, (food_blob, soil_blob, ground_blob, ecology_blob))) != (
        OLD_GRID * OLD_GRID * 4,
        OLD_GRID * OLD_GRID * 4,
        OLD_GRID * OLD_GRID * 32,
        OLD_GRID * OLD_GRID * 16,
    ):
        raise ValueError("Unexpected v45 ecology buffer lengths")
    food_mass = float(sum(words(food_blob)))
    for ground in struct.iter_unpack("<IIfIIIff", ground_blob):
        food_mass += ground[0] + ground[2]
    # The 512-grid ecology is transferred byte-for-byte. V46-specific
    # physiology and clocks are handled separately from these physical stocks.
    return {
        1: food_blob, 2: soil_blob, 3: ground_blob, 17: ecology_blob
    }, {"mode": "identity_512", "food_milli_before": food_mass,
        "food_milli_after": food_mass}


def copy_n(source, target, n, digest):
    left = n
    while left:
        block = source.read(min(left, 8 * 1024 * 1024))
        if not block:
            raise ValueError("Truncated v45 checkpoint")
        digest.update(block)
        target.write(block)
        left -= len(block)


def convert(receipt_path, destination):
    receipt, checkpoint = read_receipt(receipt_path)
    destination.mkdir(parents=True, exist_ok=False)
    partial = destination / "save-import.checkpoint.partial"
    complete = destination / "save-import.checkpoint"
    digest = hashlib.sha256()
    with checkpoint.open("rb") as source, partial.open("x+b") as target:
        magic = source.read(12)
        if magic != OLD_MAGIC:
            raise ValueError("Expected v45 checkpoint magic")
        digest.update(magic)
        header = source.read(12)
        if len(header) != 12:
            raise ValueError("Truncated v45 header")
        digest.update(header)
        seed, tick_low, meta_len = struct.unpack("<III", header)
        if meta_len > 16_777_216:
            raise ValueError("Invalid source metadata length")
        meta_blob = source.read(meta_len)
        if len(meta_blob) != meta_len:
            raise ValueError("Truncated source metadata")
        digest.update(meta_blob)
        metadata = json.loads(meta_blob)
        source_tick = tick_low | (metadata.get("tick_high", 0) << 32)
        if (seed, source_tick) != (receipt["seed"], receipt["tick"]):
            raise ValueError("Receipt and checkpoint clock disagree")
        metadata["progress"] = {
            "world": 1, "rng": metadata["progress"]["rng"],
            "completed": None, "history": [], "engine_saturated": False,
        }
        metadata["environment_start_age"] += source_tick
        metadata["tick_high"] = 0
        metadata["settings"]["founder_name"] = (
            f"Imported v45 world {receipt['world']} tick {source_tick}"
        )
        target.write(NEW_MAGIC)
        new_meta = json.dumps(metadata, ensure_ascii=False, separators=(",", ":")).encode()
        target.write(struct.pack("<III", seed, 0, len(new_meta)))
        target.write(new_meta)
        ecology = {}
        for index in range(BUFFER_COUNT):
            length_blob = source.read(8)
            if len(length_blob) != 8:
                raise ValueError("Truncated v45 buffer header")
            digest.update(length_blob)
            length = struct.unpack("<Q", length_blob)[0]
            if index in GRID_BUFFERS:
                blob = source.read(length)
                if len(blob) != length:
                    raise ValueError("Truncated v45 ecology buffer")
                digest.update(blob)
                ecology[index] = blob
                # The four ecology buffers are written after all source buffers
                # are read; reserve their ordered slots in the output stream.
                target.write(struct.pack("<Q", length))
                target.write(b"\0" * length)
            elif index == 0:
                blob = bytearray(source.read(length))
                if len(blob) != length or length % 312:
                    raise ValueError("Truncated or invalid v45 body buffer")
                digest.update(blob)
                living = 0
                for at in range(0, length, 312):
                    if struct.unpack_from("<I", blob, at + 40)[0] != 0:
                        living += 1
                    energy = blob[at + 16 : at + 20]
                    food = blob[at + 32 : at + 36]
                    blob[at + 72 : at + 76] = energy
                    blob[at + 76 : at + 80] = food
                    struct.pack_into("<I", blob, at + 276, 0)
                if living != receipt["living"]:
                    raise ValueError("V45 receipt and body count disagree")
                expanded = bytearray((length // 312) * 320)
                for slot in range(length // 312):
                    expanded[slot * 320 : slot * 320 + 312] = blob[slot * 312 : (slot + 1) * 312]
                target.write(struct.pack("<Q", len(expanded)))
                target.write(expanded)
            elif index == 4:
                blob = source.read(length)
                if len(blob) != length:
                    raise ValueError("Truncated v45 statistics")
                digest.update(blob)
                values = words(blob)
                keep = {10: values[10], 23: values[23], 30: values[30], 50: values[50]}
                values = array.array("I", [0] * 96)
                for slot, value in keep.items():
                    values[slot] = value
                output = packed_words(values)
                target.write(struct.pack("<Q", len(output)))
                target.write(output)
            elif index in (5, 6, 7):
                # Events, perceptions, and decisions are tick-relative scratch.
                target.write(struct.pack("<Q", length))
                target.write(b"\0" * length)
                left = length
                while left:
                    block = source.read(min(left, 8 * 1024 * 1024))
                    if not block:
                        raise ValueError("Truncated v45 scratch buffer")
                    digest.update(block)
                    left -= len(block)
            else:
                target.write(length_blob)
                copy_n(source, target, length, digest)
        if source.read(1):
            raise ValueError("Trailing v45 checkpoint data")
        converted, mass = regrid(ecology[1], ecology[2], ecology[3], ecology[17])
        target.flush()
        for index, blob in converted.items():
            target.seek(12 + 12 + len(new_meta))
            for current in range(index):
                size = struct.unpack("<Q", target_read(target, 8))[0]
                target.seek(size, 1)
            size = struct.unpack("<Q", target_read(target, 8))[0]
            if size != len(blob):
                raise ValueError("Converted ecology length mismatch")
            target.write(blob)
        target.flush()
        os.fsync(target.fileno())
    partial.replace(complete)
    now_ms = int(time.time() * 1000)
    record = {
        "version": 6, "model": MODEL_NEW, "name": "Imported v45 evolution",
        "origin": f"Read-only conversion of v45 world {receipt['world']} tick {source_tick}; cumulative {receipt['total_ticks']} ticks",
        "checkpoint": complete.name, "saved_at_ms": now_ms,
        "seed": seed, "tick": 0, "living": receipt["living"],
        "world": 1, "total_ticks": 0,
    }
    provenance = {
        "source_receipt": str(receipt_path), "source_checkpoint": str(checkpoint),
        "source_sha256": digest.hexdigest(), "source_model": MODEL_OLD,
        "source_world": receipt["world"], "source_tick": source_tick,
        "source_total_ticks": receipt["total_ticks"],
        "source_living": receipt["living"], "source_saved_at_ms": receipt["saved_at_ms"],
        "regridding": mass,
    }
    # Keep a complete frozen source beside the converted experiment. It is a
    # restart source for every imported living genome and its trait record,
    # even if subsequent v46 saves lose the original population.
    archive = destination / "frozen-v45-source"
    archive.mkdir()
    archived_checkpoint = archive / checkpoint.name
    shutil.copyfile(checkpoint, archived_checkpoint)
    archived_receipt = archive / receipt_path.name
    shutil.copyfile(receipt_path, archived_receipt)
    with archived_checkpoint.open("rb") as verify:
        archive_hash = hashlib.file_digest(verify, "sha256").hexdigest()
    if archive_hash != digest.hexdigest():
        raise ValueError("Frozen source archive differs from validated input")
    provenance["frozen_source_receipt"] = str(archived_receipt)
    provenance["frozen_source_checkpoint"] = str(archived_checkpoint)
    (destination / "import-provenance.json").write_text(
        json.dumps(provenance, ensure_ascii=False, indent=2), encoding="utf-8"
    )
    return destination / "save-import.json", record, provenance


def target_read(target, count):
    data = target.read(count)
    if len(data) != count:
        raise ValueError("Truncated output while patching ecology")
    return data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", nargs="?", type=Path, help="v45 save receipt; defaults to latest complete v45 save")
    parser.add_argument("--output-root", type=Path, default=None)
    parser.add_argument("--v46-exe", type=Path, default=Path(__file__).resolve().parents[1] / "target/release/primitive_world.exe")
    args = parser.parse_args()
    receipt = args.receipt or latest_receipt()
    root = args.output_root or new_root()
    root.mkdir(parents=True, exist_ok=True)
    destination = root / f"experiment-import-{time.time_ns()}-{os.getpid()}"
    result, record, provenance = convert(receipt, destination)
    validation = destination / "import-validation.json"
    command = [
        str(args.v46_exe), "--headless", "--single-world", "--ticks", "1",
        "--sample", "1", "--checkpoint", str(destination / "save-import.checkpoint"),
        "--output", str(validation),
    ]
    subprocess.run(command, check=True, capture_output=True, text=True)
    result.write_text(json.dumps(record, ensure_ascii=False, separators=(",", ":")), encoding="utf-8")
    print(json.dumps({"receipt": str(result), "source_total_ticks": provenance["source_total_ticks"], "source_sha256": provenance["source_sha256"], "regridding": provenance["regridding"]}))


if __name__ == "__main__":
    main()
