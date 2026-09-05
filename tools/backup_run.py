"""Read-only world monitoring plus verified, non-destructive local backup archives."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
from pathlib import Path
import shutil
import struct
import uuid
import zipfile

from run_io import exclusive_run, save_state


def read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def checkpoint_header(path):
    size = path.stat().st_size
    with path.open("rb") as stream:
        if stream.read(12) != b"PRIMWORLD015":
            raise ValueError(f"Unexpected checkpoint version: {path}")
        seed, tick, settings_size = struct.unpack("<III", stream.read(12))
        if not 1 <= settings_size <= 32 * 1024 * 1024:
            raise ValueError("Invalid settings length")
        json.loads(stream.read(settings_size))
        for _ in range(9):
            length, = struct.unpack("<Q", stream.read(8))
            if stream.tell() + length > size:
                raise ValueError("Incomplete checkpoint buffer")
            stream.seek(length, 1)
        if stream.tell() != size:
            raise ValueError("Checkpoint has trailing bytes")
    return dict(seed=seed, tick=tick, bytes=size,
                validation="Schema15 header, JSON settings and all nine buffer boundaries; not a GPU semantic load test")


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


def summarize(state):
    rows = list(state["worlds"].values())
    checkpoints = list(state["checkpoints"].values())
    def average(part, key):
        return sum(r[key] for r in part) / len(part) if part else None
    return dict(checked_at=state["checked_at"], completed_worlds=len(rows),
        total_elapsed_ticks=sum(r["elapsed_ticks"] for r in rows),
        longest_observed_world_ticks=max((r["tick"] for r in rows), default=0),
        sampled_max_ancestry=max((r["sample_max_ancestry"] for r in rows), default=0),
        physical_setting_variants=len({r["settings_signature"] for r in rows}),
        first_ten_mean_elapsed=average(rows[:10], "elapsed_ticks"),
        last_ten_mean_elapsed=average(rows[-10:], "elapsed_ticks"),
        full_checkpoints_backed_up=len(checkpoints),
        latest_checkpoint=checkpoints[-1] if checkpoints else None,
        newest_world=state["newest_world"], free_disk_bytes=state["free_disk_bytes"],
        deferred=state["deferred"],
        warning="Worlds have different seeds and may have user-edited settings; these are descriptive results, not proof of learning. Ancestry is sampled late-survivor ancestry, not the maximum ever born.")


def run(run_dir, destination):
    destination.mkdir(parents=True, exist_ok=True)
    archive_dir = destination / "archives"
    archive_dir.mkdir(exist_ok=True)
    with exclusive_run(destination):
        path = destination / "summary.json"
        state = read(path) if path.exists() else dict(worlds={}, checkpoints={})
        state.update(checked_at=datetime.now(timezone.utc).isoformat(), deferred=[],
                     free_disk_bytes=shutil.disk_usage(destination).free)
        if state["free_disk_bytes"] < 10 * 1024**3:
            raise RuntimeError("Less than 10 GiB free; no deletion or automatic simulation restart authorized")
        directories = sorted(p for p in run_dir.glob("world-*") if p.is_dir())
        state["newest_world"] = directories[-1].name if directories else None
        for directory in directories:
            if directory.name in state["worlds"]:
                continue
            report_path = directory / "report.json"
            if not report_path.exists():
                continue
            try:
                report = read(report_path)
                sample_path = directory / "survivors.bank.json"
                sample = read(sample_path)
                sources = [report_path, sample_path, directory / "ready.json"]
                if report["termination_reason"] == "extinction":
                    # The report precedes the handoff; don't archive a partial transfer.
                    transfer = directory / "transfer.json"
                    read(transfer)
                    bank = directory / "next.bank.json"
                    read(bank)
                    sources += [transfer, bank]
                settings = dict(report["final_settings"])
                settings.pop("founder_genomes", None)
                settings.pop("founder_name", None)
                row = dict(tick=report["end"]["tick"], elapsed_ticks=report["elapsed_ticks"],
                    births=report["end"]["events"][3], harvested=report["end"]["harvested"],
                    reason=report["termination_reason"], invalid_outputs=report["end"]["invalid_outputs"],
                    sampled=len(sample["bodies"]), sampled_descendants=sum(b["ancestry_depth"] > 0 for b in sample["bodies"]),
                    sample_max_ancestry=max((b["ancestry_depth"] for b in sample["bodies"]), default=0),
                    settings=settings, settings_signature=hashlib.sha256(json.dumps(settings, sort_keys=True).encode()).hexdigest(),
                    backup=archive(archive_dir, directory.name, sources))
                state["worlds"][directory.name] = row
                save_state(destination, state)
            except (FileNotFoundError, json.JSONDecodeError) as exc:
                state["deferred"].append(dict(world=directory.name, reason=str(exc)))
        checkpoints = sorted((run_dir / "checkpoints").glob("*.checkpoint"), key=lambda p: p.stat().st_mtime_ns)
        checkpoints += sorted(run_dir.glob("world-*/paused.checkpoint"))
        for checkpoint in checkpoints:
            key = checkpoint.relative_to(run_dir).as_posix()
            if key in state["checkpoints"]:
                continue
            metadata = checkpoint_header(checkpoint)
            backup = archive(archive_dir, "checkpoint-" + hashlib.sha256(key.encode()).hexdigest()[:16], [checkpoint])
            state["checkpoints"][key] = dict(source=str(checkpoint), **metadata, backup=backup)
            save_state(destination, state)
        save_state(destination, state)
        summary = summarize(state)
        latest = destination / "latest.next.json"
        latest.write_text(json.dumps(summary, indent=2) + "\n", encoding="utf-8")
        latest.replace(destination / "latest.json")
        print(json.dumps(summary, indent=2))


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--run", type=Path, required=True)
    parser.add_argument("--backup", type=Path, required=True)
    args = parser.parse_args()
    run(args.run.resolve(strict=True), args.backup.resolve())
