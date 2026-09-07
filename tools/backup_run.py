"""Verified backups of current game receipts and checkpoints."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import struct
import uuid
import zipfile

from run_io import exclusive_run, save_state


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def checkpoint_header(path):
    size = path.stat().st_size
    with path.open("rb") as stream:
        magic = stream.read(12)
        if magic != b"PRIMWORLD023":
            raise ValueError(f"Unexpected checkpoint version: {path}")
        seed, tick, settings_size = struct.unpack("<III", stream.read(12))
        if not 1 <= settings_size <= 32 * 1024 * 1024:
            raise ValueError("Invalid settings length")
        json.loads(stream.read(settings_size))
        for _ in range(11):
            length, = struct.unpack("<Q", stream.read(8))
            if stream.tell() + length > size:
                raise ValueError("Incomplete checkpoint buffer")
            stream.seek(length, 1)
        if stream.tell() != size:
            raise ValueError("Checkpoint has trailing bytes")
    return dict(seed=seed, tick=tick, bytes=size,
                validation=f"Schema{int(magic[-3:])} header, JSON settings and all buffer boundaries; not a GPU semantic load test")


def archive(destination, name, sources):
    target = destination / f"{name}.zip"
    if not target.exists():
        pending = destination / f"{name}-{uuid.uuid4().hex}.partial.zip"
        manifest = {}
        with zipfile.ZipFile(pending, "x", compression=zipfile.ZIP_DEFLATED, compresslevel=1) as out:
            for path in sources:
                before = path.stat()
                with path.open("rb") as stream:
                    digest = hashlib.file_digest(stream, "sha256").hexdigest()
                out.write(path, path.name)
                after = path.stat()
                if (before.st_size, before.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
                    raise ValueError(f"File changed while backing up: {path}")
                manifest[path.name] = dict(sha256=digest, bytes=after.st_size, source=str(path))
            out.writestr("manifest.json", json.dumps(manifest))
        with zipfile.ZipFile(pending) as check:
            for entry, info in manifest.items():
                with check.open(entry) as stream:
                    if hashlib.file_digest(stream, "sha256").hexdigest() != info["sha256"]:
                        raise ValueError("Backup checksum mismatch")
        pending.rename(target)
    else:
        with zipfile.ZipFile(target) as check:
            if check.testzip() is not None:
                raise ValueError(f"Damaged existing backup: {target}")
    return dict(path=str(target), bytes=target.stat().st_size)


def run(run_dir, destination):
    run_dir = run_dir.resolve(strict=True)
    destination = destination.resolve()
    if destination == run_dir or run_dir in destination.parents:
        raise ValueError("Backup destination must be outside the experiment folder")
    destination.mkdir(parents=True, exist_ok=True)
    archive_dir = destination / "archives"
    archive_dir.mkdir(exist_ok=True)
    with exclusive_run(destination):
        state_path = destination / "summary.json"
        state = read(state_path) if state_path.exists() else dict(checkpoints={})
        state.update(checked_at=datetime.now(timezone.utc).isoformat(), deferred=[])
        for receipt in sorted(run_dir.rglob("save-*.json")):
            key = receipt.relative_to(run_dir).as_posix()
            if key in state["checkpoints"]:
                continue
            try:
                record = read(receipt)
                if record.get("version") != 4 or record.get("model") != "primitive-v9-descendant-population-search":
                    raise ValueError("Unsupported game receipt")
                name = record["checkpoint"]
                if Path(name).name != name or any(c in name for c in '/\\:') or not name.endswith('.checkpoint'):
                    raise ValueError("Invalid checkpoint name")
                checkpoint = receipt.parent / name
                metadata = checkpoint_header(checkpoint)
                if (metadata["seed"], metadata["tick"]) != (record["seed"], record["tick"]):
                    raise ValueError("Receipt and checkpoint disagree")
                result = archive(archive_dir, "save-" + hashlib.sha256(key.encode()).hexdigest()[:16], [receipt,checkpoint])
                state["checkpoints"][key] = dict(source=str(receipt), **metadata, backup=result)
                save_state(destination,state)
            except (FileNotFoundError, json.JSONDecodeError, ValueError, KeyError, struct.error) as exc:
                state["deferred"].append(dict(save=key,reason=str(exc)))
        save_state(destination,state)
        summary = dict(checked_at=state["checked_at"], full_checkpoints_backed_up=len(state["checkpoints"]),deferred=state["deferred"])
        latest = destination / "latest.next.json"
        latest.write_text(json.dumps(summary,indent=2)+"\n",encoding="utf-8")
        latest.replace(destination / "latest.json")
        print(json.dumps(summary,indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--backup", type=Path, required=True)
    args = parser.parse_args()
    run(args.run.resolve(strict=True), args.backup.resolve())
