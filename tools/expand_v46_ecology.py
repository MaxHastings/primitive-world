"""Expand one v46 256-grid save to the 512-grid v46 checkpoint format.

The source receipt/checkpoint are read-only. Every non-ecology buffer is copied
byte-for-byte, including the population, genomes, lifetime learning, and
closed-chain counters. Extensive stocks are split exactly across four cells;
intensive fields are copied to each. The new receipt is published only after a
headless load check succeeds.
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

OLD_GRID = 256
NEW_GRID = 512
BUFFER_COUNT = 18
GRID_BUFFERS = {1, 2, 3, 17}
OLD_MAGIC = b"PRIMWORLD061"
NEW_MAGIC = b"PRIMWORLD062"
MODEL = "primitive-v46-fast-evolution"
OLD_RECEIPT_VERSION = 5
NEW_RECEIPT_VERSION = 6
EXPECTED_OLD_LENGTHS = {1: OLD_GRID**2 * 4, 2: OLD_GRID**2 * 4,
                        3: OLD_GRID**2 * 32, 17: OLD_GRID**2 * 16}


def split_u32(total):
    quotient, remainder = divmod(total, 4)
    return (quotient + (remainder > 0), quotient + (remainder > 1),
            quotient + (remainder > 2), quotient)


def fine_indices(coarse):
    x, y = coarse % OLD_GRID, coarse // OLD_GRID
    top = (y * 2) * NEW_GRID + x * 2
    return top, top + 1, top + NEW_GRID, top + NEW_GRID + 1


def words(blob):
    result = array.array("I")
    result.frombytes(blob)
    if sys.byteorder != "little":
        result.byteswap()
    return result


def word_bytes(values):
    if sys.byteorder != "little":
        values.byteswap()
    return values.tobytes()


def expand_ecology(blobs):
    for index, expected in EXPECTED_OLD_LENGTHS.items():
        if len(blobs[index]) != expected:
            raise ValueError(f"Unexpected source ecology buffer {index} length")
    old_food = words(blobs[1])
    new_food = array.array("I", [0]) * (NEW_GRID**2)
    new_soil = bytearray(NEW_GRID**2 * 4)
    new_ground = bytearray(NEW_GRID**2 * 32)
    new_pools = bytearray(NEW_GRID**2 * 16)
    food_before = food_after = 0
    dropped_before = dropped_after = 0
    extracted_before = extracted_after = 0
    for coarse in range(OLD_GRID**2):
        indices = fine_indices(coarse)
        food_parts = split_u32(old_food[coarse])
        soil = struct.unpack_from("<f", blobs[2], coarse * 4)[0]
        ground = struct.unpack_from("<IIfIIIff", blobs[3], coarse * 32)
        pools = struct.unpack_from("<ffff", blobs[17], coarse * 16)
        dropped_parts = split_u32(ground[0])
        extracted_parts = split_u32(ground[1])
        produced_parts = split_u32(ground[3])
        loss_parts = split_u32(ground[4])
        collected_parts = split_u32(ground[5])
        food_before += old_food[coarse]
        dropped_before += ground[0]
        extracted_before += ground[1]
        for part, fine in enumerate(indices):
            new_food[fine] = food_parts[part]
            struct.pack_into("<f", new_soil, fine * 4, soil)
            struct.pack_into("<IIfIIIff", new_ground, fine * 32,
                             dropped_parts[part], extracted_parts[part],
                             ground[2] / 4.0, produced_parts[part],
                             loss_parts[part], collected_parts[part],
                             ground[6], ground[7])
            struct.pack_into("<ffff", new_pools, fine * 16,
                             pools[0], pools[1] / 4.0,
                             pools[2] / 4.0, pools[3] / 4.0)
            food_after += food_parts[part]
            dropped_after += dropped_parts[part]
            extracted_after += extracted_parts[part]
    if (food_before, dropped_before, extracted_before) != (
        food_after, dropped_after, extracted_after
    ):
        raise ValueError("Ecology mass changed during expansion")
    return {1: word_bytes(new_food), 2: new_soil, 3: new_ground,
            17: new_pools}, {
        "natural_food_milli_before": food_before,
        "natural_food_milli_after": food_after,
        "dropped_food_milli_before": dropped_before,
        "dropped_food_milli_after": dropped_after,
        "pending_extraction_milli_before": extracted_before,
        "pending_extraction_milli_after": extracted_after,
    }


def copy_n(source, target, length, digest):
    while length:
        chunk = source.read(min(length, 8 * 1024 * 1024))
        if not chunk:
            raise ValueError("Truncated checkpoint")
        digest.update(chunk)
        target.write(chunk)
        length -= len(chunk)


def convert(receipt_path, destination):
    receipt = json.loads(receipt_path.read_text(encoding="utf-8"))
    if receipt.get("version") != OLD_RECEIPT_VERSION or receipt.get("model") != MODEL:
        raise ValueError("Expected a v46 256-grid receipt")
    checkpoint_name = receipt.get("checkpoint")
    if (not isinstance(checkpoint_name, str)
            or Path(checkpoint_name).name != checkpoint_name
            or not checkpoint_name.endswith(".checkpoint")):
        raise ValueError("Invalid checkpoint name")
    checkpoint = receipt_path.parent / checkpoint_name
    if not checkpoint.is_file():
        raise ValueError("Source checkpoint is missing")
    destination.mkdir(parents=True, exist_ok=False)
    stamp = time.time_ns()
    new_name = f"save-{stamp}.checkpoint"
    partial = destination / f"{new_name}.partial"
    complete = destination / new_name
    digest = hashlib.sha256()
    with checkpoint.open("rb") as source, partial.open("x+b") as target:
        old_magic = source.read(12)
        if old_magic != OLD_MAGIC:
            raise ValueError("Expected v46 256-grid checkpoint magic")
        digest.update(old_magic)
        header = source.read(12)
        if len(header) != 12:
            raise ValueError("Truncated checkpoint header")
        digest.update(header)
        seed, tick_low, metadata_length = struct.unpack("<III", header)
        if metadata_length > 16_777_216:
            raise ValueError("Invalid metadata length")
        metadata_bytes = source.read(metadata_length)
        if len(metadata_bytes) != metadata_length:
            raise ValueError("Truncated checkpoint metadata")
        digest.update(metadata_bytes)
        metadata = json.loads(metadata_bytes)
        tick = tick_low | (metadata.get("tick_high", 0) << 32)
        if (seed, tick, metadata["progress"]["world"]) != (
            receipt["seed"], receipt["tick"], receipt["world"]
        ):
            raise ValueError("Receipt and checkpoint identity disagree")
        target.write(NEW_MAGIC)
        target.write(header)
        target.write(metadata_bytes)
        for index in range(BUFFER_COUNT):
            length_bytes = source.read(8)
            if len(length_bytes) != 8:
                raise ValueError("Truncated buffer header")
            digest.update(length_bytes)
            length = struct.unpack("<Q", length_bytes)[0]
            if index in GRID_BUFFERS:
                if length != EXPECTED_OLD_LENGTHS[index]:
                    raise ValueError(f"Unexpected grid buffer {index} length")
                blob = source.read(length)
                if len(blob) != length:
                    raise ValueError(f"Truncated grid buffer {index}")
                digest.update(blob)
                target.write(struct.pack("<Q", length * 4))
                target.write(b"\0" * (length * 4))
                if index == 1:
                    grid_blobs = {}
                grid_blobs[index] = blob
            else:
                target.write(length_bytes)
                copy_n(source, target, length, digest)
        if source.read(1):
            raise ValueError("Unexpected trailing checkpoint data")
        expanded, conservation = expand_ecology(grid_blobs)
        for index, blob in expanded.items():
            target.seek(24 + metadata_length)
            for _ in range(index):
                length = struct.unpack("<Q", target.read(8))[0]
                target.seek(length, 1)
            length = struct.unpack("<Q", target.read(8))[0]
            if length != len(blob):
                raise ValueError("Expanded buffer length mismatch")
            target.write(blob)
        target.flush()
        os.fsync(target.fileno())
    partial.replace(complete)
    now_ms = int(time.time() * 1000)
    new_receipt = dict(receipt)
    new_receipt.update({
        "version": NEW_RECEIPT_VERSION,
        "name": receipt["name"] + " · 512 food",
        "checkpoint": new_name,
        "saved_at_ms": now_ms,
    })
    provenance = {
        "source_receipt": str(receipt_path),
        "source_checkpoint": str(checkpoint),
        "source_sha256": digest.hexdigest(),
        "source_tick": tick,
        "source_total_ticks": receipt["total_ticks"],
        "source_living": receipt["living"],
        "ecology_expansion": conservation,
    }
    archive = destination / "frozen-v46-grid256-source"
    archive.mkdir()
    archived_checkpoint = archive / checkpoint.name
    try:
        os.link(checkpoint, archived_checkpoint)
    except OSError:
        shutil.copyfile(checkpoint, archived_checkpoint)
    shutil.copyfile(receipt_path, archive / receipt_path.name)
    with archived_checkpoint.open("rb") as verify:
        if hashlib.file_digest(verify, "sha256").hexdigest() != digest.hexdigest():
            raise ValueError("Frozen checkpoint differs from the source")
    provenance["frozen_source_checkpoint"] = str(archived_checkpoint)
    (destination / "ecology-expansion-provenance.json").write_text(
        json.dumps(provenance, indent=2), encoding="utf-8"
    )
    return destination / f"save-{stamp}.json", new_receipt, provenance


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt", type=Path)
    parser.add_argument("--output-root", type=Path, default=Path(os.environ["LOCALAPPDATA"]) / "PrimitiveWorldV46" / "experiments")
    parser.add_argument("--v46-exe", type=Path, default=Path(__file__).resolve().parents[1] / "target/release/primitive_world.exe")
    args = parser.parse_args()
    args.output_root.mkdir(parents=True, exist_ok=True)
    destination = args.output_root / f"experiment-grid512-{time.time_ns()}-{os.getpid()}"
    receipt_path, receipt, provenance = convert(args.receipt, destination)
    validation = destination / "expansion-validation.json"
    command = [str(args.v46_exe), "--headless", "--single-world", "--ticks", "1",
               "--sample", "1", "--checkpoint", str(destination / receipt["checkpoint"]),
               "--output", str(validation)]
    result = subprocess.run(command, capture_output=True, text=True, timeout=120)
    if result.returncode:
        raise RuntimeError(f"Expanded checkpoint failed to load: {result.stderr}")
    report = json.loads(validation.read_text(encoding="utf-8"))
    if report.get("initial_tick") != receipt["tick"]:
        raise ValueError("Expanded checkpoint resumed at the wrong tick")
    pending_receipt = receipt_path.with_suffix(".json.partial")
    with pending_receipt.open("x", encoding="utf-8") as output:
        output.write(json.dumps(receipt, ensure_ascii=False, separators=(",", ":")))
        output.flush()
        os.fsync(output.fileno())
    pending_receipt.replace(receipt_path)
    print(json.dumps({"receipt": str(receipt_path), "tick": receipt["tick"],
                      "living": receipt["living"], "conservation": provenance["ecology_expansion"]}))


if __name__ == "__main__":
    main()
