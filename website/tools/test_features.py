"""Tests for website/tools/features.py's pages. Needs only Python.

Run: python3 website/tools/test_features.py
"""

import importlib.util
import json
import re
import sys
import tempfile
import unittest
import unittest.mock
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("features", HERE / "features.py")
features = importlib.util.module_from_spec(spec)
sys.modules["features"] = features
spec.loader.exec_module(features)


def entry(fid, category):
    return {
        "id": fid, "category": category, "kicker": f"{fid} kicker",
        "headline": "Line one<br>line two", "dek": f"{fid} dek",
        "description": f"{fid} description", "chapters": [[1, "Start"], [9, "Past the end"]],
        "how": [["Step", "Do it."]], "docs": ["/docs/x/", "Docs"], "keywords": "special",
    }


def narration(fid):
    return {"id": fid, "title": {"en": f"{fid} title", "fr": f"{fid} titre"},
            "segments": [{"say": {"en": f"{fid} first.", "fr": f"{fid} premier thème."}},
                         {"say": {"en": f"{fid} second.", "fr": f"{fid} deuxième."}}]}


class Build(unittest.TestCase):
    def setUp(self):
        tmp = tempfile.TemporaryDirectory()
        self.addCleanup(tmp.cleanup)
        root = Path(tmp.name)
        self.site = root / "site"
        clips = self.site / "assets/feature-clips"
        specs = root / "specs"
        clips.mkdir(parents=True)
        specs.mkdir()
        catalog = [entry("alpha", "Agents"), entry("beta", "Review"), entry("gamma", "Agents")]
        (root / "features.json").write_text(json.dumps({"features": catalog}))
        manifest = {
            "alpha": {"windows": ["moonframe", "classic"], "langs": ["en", "fr"],
                      "length": 24.4, "starts": [1.0, 5.25]},
            "beta": {"windows": ["classic"], "langs": ["en", "fr"],
                     "length": 70.0, "starts": [1.0, 4.0]},
            # gamma has no clip yet: it must be left out, not break the build.
        }
        (clips / "manifest.json").write_text(json.dumps(manifest))
        for fid in ("alpha", "beta", "gamma"):
            (specs / f"{fid}.json").write_text(json.dumps(narration(fid)))
        for name, value in {"SITE": self.site, "CLIPS": clips, "CATALOG": root / "features.json",
                            "SPECS": specs}.items():
            patch = unittest.mock.patch.object(features, name, value)
            patch.start()
            self.addCleanup(patch.stop)
        features.build()

    def read(self, path):
        return (self.site / "features" / path).read_text()

    def test_pages_only_for_features_with_clips(self):
        self.assertTrue((self.site / "features/alpha/index.html").exists())
        self.assertTrue((self.site / "features/beta/index.html").exists())
        self.assertFalse((self.site / "features/gamma").exists())

    def test_index_searches_every_language_and_keyword(self):
        index = self.read("index.html")
        data = json.loads(re.search(r'id="feature-index">(.*?)</script>', index, re.S).group(1))
        alpha = next(e for e in data if e["id"] == "alpha")
        self.assertIn("alpha premier thème.", alpha["text"])
        self.assertIn("special", alpha["text"])
        self.assertIn("Line one line two", alpha["text"])
        self.assertIn('data-category="Agents"', index)
        self.assertIn("2 of 2 features", re.sub(r"<[^>]+>", "", index))

    def test_feature_page_chapters_switches_and_pager(self):
        alpha = self.read("alpha/index.html")
        self.assertIn('data-seek="5.25"', alpha)
        self.assertNotIn("Past the end", alpha)
        self.assertIn('data-window="classic"', alpha)
        self.assertIn('data-lang="fr"', alpha)
        self.assertIn('href="/features/beta/"', alpha)
        beta = self.read("beta/index.html")
        # One window: no switch, the window is named instead.
        self.assertNotIn('data-window="moonframe"', beta)
        self.assertIn("Classic window", beta)
        self.assertIn("1:10", self.read("index.html"))
        self.assertIn('rel="next" href="/features/alpha/"', beta)

    def test_text_is_escaped(self):
        self.assertEqual(features.esc('<a & "b">'), "&lt;a &amp; &quot;b&quot;&gt;")


if __name__ == "__main__":
    unittest.main()
