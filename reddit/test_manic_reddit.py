import importlib.util
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("manic_reddit.py")
SPEC = importlib.util.spec_from_file_location("manic_reddit", SCRIPT)
MODULE = importlib.util.module_from_spec(SPEC)
assert SPEC and SPEC.loader
sys.modules[SPEC.name] = MODULE
SPEC.loader.exec_module(MODULE)


class RedditPublisherTests(unittest.TestCase):
    def test_parse_videos_keeps_section_and_pipe_title(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "videos.txt"
            path.write_text(
                "# ---- examples: 3D scenes ----\n"
                "ex-demo|PLACEHOLDER|A title | with a pipe\n",
                encoding="utf-8",
            )
            rows = MODULE.parse_videos(path)
        self.assertEqual(rows[0]["name"], "ex-demo")
        self.assertEqual(rows[0]["section"], "3D scenes")
        self.assertEqual(rows[0]["title"], "A title")

    def test_wanted_accepts_catalog_and_short_names(self):
        self.assertTrue(MODULE.wanted("ex-demo", "demo"))
        self.assertTrue(MODULE.wanted("ex-demo", "ex-demo"))
        self.assertFalse(MODULE.wanted("ex-other", "demo"))

    def test_description_uses_gallery_page_and_source_anchor(self):
        row = {"name": "ex-demo", "title": "Demo — manic"}
        gallery = {
            "ex-demo": {
                "description": "One line\ncontinued.",
                "slug": "3d",
                "example": "demo",
            }
        }
        body = MODULE.make_description(row, gallery)
        self.assertIn("One line continued.", body)
        self.assertIn("/docs/ex-3d.html#demo", body)
        self.assertIn("plain-text `.manic`", body)

    def test_state_round_trip(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "state.json"
            MODULE.save_state(path, {"maniclang:ex-demo": {"post_id": "abc"}})
            state = MODULE.load_state(path)
        self.assertEqual(state["maniclang:ex-demo"]["post_id"], "abc")


if __name__ == "__main__":
    unittest.main()
