"""Checks that protect the documentation build and owner metrics boundaries."""

import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.error import HTTPError

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / (name + ".py"))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class DistributionTests(unittest.TestCase):
    def test_site_has_real_canonicals_and_escaped_verification(self):
        site = load("build_site")
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp)
            site.build(output, verification='token"<')
            html = (output / "index.html").read_text(encoding="utf-8")
            self.assertIn('content="token&quot;&lt;"', html)
            self.assertIn('rel="canonical" href="' + site.SITE_URL + '"', html)
            self.assertTrue((output / "assets/demo-summary.json").is_file())
            self.assertIn(site.SITE_URL + "sitemap.xml", (output / "robots.txt").read_text())

    def test_site_rejects_broken_guide_link(self):
        site = load("build_site")
        original = Path.read_text

        def read(path, *args, **kwargs):
            value = original(path, *args, **kwargs)
            if path.name == "index.html" and "content" in path.parts:
                value += '<a href="does-not-exist.html">Missing</a>'
            return value

        with tempfile.TemporaryDirectory() as temp, patch.object(Path, "read_text", read):
            with self.assertRaisesRegex(ValueError, "Broken local link"):
                site.build(Path(temp))

    def test_public_snapshot_keeps_traffic_unknown_and_ignores_drafts(self):
        metrics = load("adoption_snapshot")
        payloads = [{"html_url": metrics.BASE.replace("api.github.com/repos", "github.com"), "stargazers_count": 7, "forks_count": 2}, [{"draft": False, "tag_name": "v1", "assets": [{"name": "binary.zip", "download_count": 3}]}, {"draft": True, "tag_name": "v2", "assets": [{"name": "preview.zip", "download_count": 9}]}]]

        class Response:
            def __init__(self, payload):
                self.payload = payload

            def read(self):
                return json.dumps(self.payload).encode()

            def __enter__(self):
                return self

            def __exit__(self, *args):
                return False

        with patch.object(metrics, "urlopen", side_effect=[Response(x) for x in payloads]), patch.object(metrics.subprocess, "run") as command:
            result = metrics.collect()
            command.assert_not_called()
        self.assertIsNone(result["traffic"])
        self.assertEqual(len(result["release_assets"]), 1)
        self.assertEqual(result["release_assets"][0]["download_count"], 3)

    def test_metrics_http_failure_does_not_expose_auth_or_body(self):
        metrics = load("adoption_snapshot")
        with patch.object(metrics, "urlopen", side_effect=HTTPError(metrics.BASE, 403, "private response", {}, None)):
            with self.assertRaisesRegex(RuntimeError, "HTTP 403 for repository") as error:
                metrics.collect()
        self.assertNotIn("private response", str(error.exception))


if __name__ == "__main__":
    unittest.main()
