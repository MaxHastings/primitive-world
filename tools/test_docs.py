"""Keep the public entry points self-contained; no local research artifacts needed."""
from pathlib import Path
import re
import unittest


ROOT = Path(__file__).resolve().parents[1]


class DocumentationTests(unittest.TestCase):
    def test_local_document_links_resolve(self):
        files = [*sorted(ROOT.glob("*.md")), *sorted((ROOT / "docs").glob("*.md"))]
        for path in files:
            for target in re.findall(r"\]\(([^)]+)\)", path.read_text(encoding="utf-8")):
                if "://" in target or target.startswith("#"):
                    continue
                with self.subTest(file=path.name, link=target):
                    self.assertTrue((path.parent / target.split("#")[0]).exists())

    def test_documented_tools_exist(self):
        for path in ["backup_run.py", "analyze_departures.py"]:
            self.assertTrue((ROOT / "tools" / path).is_file())

    def test_front_page_has_portable_launch_commands(self):
        readme = (ROOT / "README.md").read_text(encoding="utf-8")
        self.assertNotIn("C:/Users/", readme)
        self.assertIn("cargo run --release", readme)
        self.assertIn("Play.cmd --wallpaper", readme)
        self.assertIn("--headless", readme)
        self.assertNotIn("--watch-loop", readme)


if __name__ == "__main__":
    unittest.main()
