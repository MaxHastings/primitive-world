import hashlib
import json
from pathlib import Path
import struct
import tempfile
import unittest
import zipfile
from contextlib import redirect_stdout
from io import StringIO

import backup_run
from run_io import save_state, exclusive_run


def checkpoint_bytes():
    settings = b"{}"
    return (b"PRIMWORLD022" + struct.pack("<III", 42, 128, len(settings))
            + settings + b"".join(struct.pack("<Q", 4) + b"data" for _ in range(11)))


class BackupTests(unittest.TestCase):
    def test_fixed_brain_checkpoint_and_receipt_are_backed_up(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            run = root / "run"
            run.mkdir()
            metadata = b'{"settings":{},"progress":{"world":2}}'
            data = (b"PRIMWORLD022" + struct.pack("<III", 42, 128, len(metadata))
                    + metadata + b"".join(struct.pack("<Q", 4) + b"data" for _ in range(11)))
            (run / "save.checkpoint").write_bytes(data)
            (run / "save-1.json").write_text(json.dumps(dict(
                version=4, model="primitive-v8-population-search", world=2,
                checkpoint="save.checkpoint", seed=42, tick=128)))
            with redirect_stdout(StringIO()):
                backup_run.run(run, root / "backup")
            latest = json.loads((root / "backup" / "latest.json").read_text())
            self.assertEqual(latest["full_checkpoints_backed_up"], 1)
            self.assertEqual((run / "save.checkpoint").read_bytes(), data)

    def test_previous_checkpoint_models_are_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "old.checkpoint"
            path.write_bytes(b"PRIMWORLD019" + checkpoint_bytes()[12:])
            with self.assertRaises(ValueError):
                backup_run.checkpoint_header(path)

    def test_header_accepts_complete_layout_and_rejects_truncation(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "test.checkpoint"
            path.write_bytes(checkpoint_bytes())
            self.assertEqual(backup_run.checkpoint_header(path)["tick"], 128)
            path.write_bytes(checkpoint_bytes()[:-1])
            with self.assertRaises(ValueError):
                backup_run.checkpoint_header(path)

    def test_header_rejects_unknown_version_and_trailing_data(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "test.checkpoint"
            for data in (b"unknown magic", checkpoint_bytes() + b"extra"):
                path.write_bytes(data)
                with self.assertRaises(ValueError):
                    backup_run.checkpoint_header(path)

    def test_archive_preserves_original_and_verifies_uncompressed_digest(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            original = root / "source.json"
            original.write_bytes(b'{"test":1}')
            result = backup_run.archive(root, "saved", [original])
            with zipfile.ZipFile(result["path"]) as archive:
                info = json.loads(archive.read("manifest.json"))[original.name]
                self.assertEqual(info["sha256"], hashlib.sha256(original.read_bytes()).hexdigest())
                self.assertEqual(archive.read(original.name), original.read_bytes())
            self.assertEqual(backup_run.archive(root, "saved", [original]), result)
            self.assertEqual(original.read_bytes(), b'{"test":1}')
            self.assertFalse(list(root.glob("*.partial.zip")))

    def test_monitor_is_incremental_and_ignores_partial_saves(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            run = root / "run"
            checkpoints = run
            checkpoints.mkdir(parents=True)
            source = checkpoints / "save.checkpoint"
            source.write_bytes(checkpoint_bytes())
            (checkpoints / "unfinished.partial").write_bytes(b"incomplete")
            (run / "save-100.json").write_text(json.dumps(dict(version=4,model="primitive-v8-population-search",checkpoint="save.checkpoint",seed=42,tick=128)))
            backup = root / "backup"
            with redirect_stdout(StringIO()):
                backup_run.run(run, backup)
                backup_run.run(run, backup)
            latest = json.loads((backup / "latest.json").read_text())
            self.assertEqual(latest["full_checkpoints_backed_up"], 1)
            self.assertEqual(len(list((backup / "archives").glob("*.zip"))), 1)
            self.assertEqual(source.read_bytes(), checkpoint_bytes())

    def test_incomplete_current_receipt_is_deferred(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            run = root / "run"
            run.mkdir()
            (run / "save-100.json").write_text(json.dumps(dict(version=4,model="primitive-v8-population-search",checkpoint="missing.checkpoint",seed=42,tick=128)))
            with redirect_stdout(StringIO()):
                backup_run.run(run,root/"backup")
            latest=json.loads((root/"backup"/"latest.json").read_text())
            self.assertEqual(latest["full_checkpoints_backed_up"],0)
            self.assertEqual(len(latest["deferred"]),1)

    def test_truncated_checkpoint_header_is_deferred(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            run = root / "run"
            run.mkdir()
            (run / "save.checkpoint").write_bytes(checkpoint_bytes()[:15])
            (run / "save-100.json").write_text(json.dumps(dict(
                version=4, model="primitive-v8-population-search", checkpoint="save.checkpoint",
                seed=42, tick=128)))
            with redirect_stdout(StringIO()):
                backup_run.run(run, root / "backup")
            latest = json.loads((root / "backup" / "latest.json").read_text())
            self.assertEqual(latest["full_checkpoints_backed_up"], 0)
            self.assertEqual(len(latest["deferred"]), 1)

    def test_lock_releases_and_state_replacement_leaves_valid_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with exclusive_run(root):
                save_state(root, {"world": 1})
            with exclusive_run(root):
                save_state(root, {"world": 2})
            self.assertEqual(json.loads((root / "summary.json").read_text()), {"world": 2})
            self.assertFalse((root / "summary.next.json").exists())


if __name__ == "__main__":
    unittest.main()
