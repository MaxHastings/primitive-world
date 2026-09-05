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
    return (b"PRIMWORLD016" + struct.pack("<III", 42, 128, len(settings))
            + settings + b"".join(struct.pack("<Q", 4) + b"data" for _ in range(9)))


class BackupTests(unittest.TestCase):
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
            checkpoints = run / "checkpoints"
            checkpoints.mkdir(parents=True)
            source = checkpoints / "save.checkpoint"
            source.write_bytes(checkpoint_bytes())
            (checkpoints / "unfinished.partial").write_bytes(b"incomplete")
            backup = root / "backup"
            with redirect_stdout(StringIO()):
                backup_run.run(run, backup)
                backup_run.run(run, backup)
            latest = json.loads((backup / "latest.json").read_text())
            self.assertEqual(latest["full_checkpoints_backed_up"], 1)
            self.assertEqual(len(list((backup / "archives").glob("*.zip"))), 1)
            self.assertEqual(source.read_bytes(), checkpoint_bytes())

    def test_partial_world_handoff_is_deferred(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            world = root / "run" / "world-000001"
            world.mkdir(parents=True)
            (world / "report.json").write_text(json.dumps({"termination_reason": "extinction"}))
            (world / "survivors.bank.json").write_text("{}")
            with redirect_stdout(StringIO()):
                backup_run.run(root / "run", root / "backup")
            latest = json.loads((root / "backup/latest.json").read_text())
            self.assertEqual(latest["completed_worlds"], 0)
            self.assertEqual(len(latest["deferred"]), 1)
            self.assertTrue((world / "report.json").exists())

    def test_summary_does_not_hide_different_settings(self):
        def row(ticks, signature):
            return dict(elapsed_ticks=ticks, tick=ticks, sample_max_ancestry=2,
                        settings_signature=signature)
        state = dict(worlds={"world-1": row(10, "easy"), "world-2": row(20, "hard")},
                     checkpoints={}, checked_at="now", newest_world="world-3",
                     free_disk_bytes=123, deferred=[])
        summary = backup_run.summarize(state)
        self.assertEqual(summary["physical_setting_variants"], 2)
        self.assertIn("not proof of learning", summary["warning"])

    def test_lock_releases_and_state_replacement_leaves_valid_json(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            with exclusive_run(root):
                save_state(root, {"round": 1})
            with exclusive_run(root):
                save_state(root, {"round": 2})
            self.assertEqual(json.loads((root / "summary.json").read_text()), {"round": 2})
            self.assertFalse((root / "summary.next.json").exists())


if __name__ == "__main__":
    unittest.main()
