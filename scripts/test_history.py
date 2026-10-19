import gzip
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import history
from collect import append_run, collect_results


def sample_run(identifier="1"):
    return {"id": identifier, "date": "2026-10-19T00:00:00+00:00", "results": [{"mean_ns": 10}]}


class HistoryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.previous_directory = Path.cwd()
        os.chdir(self.directory.name)
        self.addCleanup(self.directory.cleanup)
        self.addCleanup(os.chdir, self.previous_directory)

    def write_history(self, runs):
        history.HISTORY.parent.mkdir(parents=True, exist_ok=True)
        history.HISTORY.write_text(json.dumps({"schema": 1, "runs": runs}))

    def test_first_run_initializes_only_when_release_is_absent(self):
        with patch("history.find_release", return_value=None):
            history.download("owner/repo")
        self.assertEqual(json.loads(history.HISTORY.read_text()), {"schema": 1, "runs": []})

    def test_api_failure_does_not_initialize_history(self):
        with patch("history.find_release", side_effect=subprocess.CalledProcessError(1, "gh")):
            with self.assertRaises(subprocess.CalledProcessError):
                history.download("owner/repo")
        self.assertFalse(history.HISTORY.exists())

    def test_missing_asset_fails_closed(self):
        with patch("history.find_release", return_value={"assets": []}):
            with self.assertRaisesRegex(ValueError, "asset is missing"):
                history.download("owner/repo")
        self.assertFalse(history.HISTORY.exists())

    def test_download_failure_does_not_initialize_history(self):
        release = {"assets": [{"name": history.ASSET}]}
        with patch("history.find_release", return_value=release), patch("history.gh", side_effect=OSError("network")):
            with self.assertRaises(OSError):
                history.download("owner/repo")
        self.assertFalse(history.HISTORY.exists())

    def test_corrupt_archive_does_not_replace_local_history(self):
        self.write_history([sample_run()])
        original = history.HISTORY.read_bytes()
        release = {"assets": [{"name": history.ASSET}]}

        def download(*arguments):
            (history.ARCHIVE / history.ASSET).write_bytes(b"invalid gzip")

        with patch("history.find_release", return_value=release), patch("history.gh", side_effect=download):
            with self.assertRaises(gzip.BadGzipFile):
                history.download("owner/repo")
        self.assertEqual(history.HISTORY.read_bytes(), original)

    def test_download_restores_and_backs_up_history(self):
        content = json.dumps({"schema": 1, "runs": [sample_run()]})
        archive = gzip.compress(content.encode())
        release = {"assets": [{"name": history.ASSET}]}

        def download(*arguments):
            (history.ARCHIVE / history.ASSET).write_bytes(archive)

        with patch("history.find_release", return_value=release), patch("history.gh", side_effect=download):
            history.download("owner/repo")
        self.assertEqual(history.HISTORY.read_text(), content)
        self.assertEqual((history.ARCHIVE / "previous-history.json.gz").read_bytes(), archive)

    def test_upload_preserves_old_runs_and_verifies_replacement(self):
        self.write_history([sample_run(), sample_run("2")])
        calls = []

        def command(*arguments):
            calls.append(arguments)
            if arguments[1] == "download":
                verification = history.ARCHIVE / "verification" / history.ASSET
                verification.write_bytes((history.ARCHIVE / history.ASSET).read_bytes())

        with patch("history.find_release", return_value={}), patch("history.gh", side_effect=command):
            history.upload("owner/repo")
        self.assertIn("--clobber", calls[0])
        self.assertEqual(calls[1][1], "download")
        archive = json.loads(gzip.decompress((history.ARCHIVE / history.ASSET).read_bytes()))
        self.assertEqual(len(archive["runs"]), 2)

    def test_upload_rejects_dropped_runs(self):
        self.write_history([sample_run("2")])
        history.ARCHIVE.mkdir()
        (history.ARCHIVE / "previous-history.json.gz").write_bytes(
            gzip.compress(json.dumps({"schema": 1, "runs": [sample_run()]}).encode()))
        with self.assertRaisesRegex(ValueError, "drops existing runs"):
            history.upload("owner/repo")

    def test_empty_history_is_never_published(self):
        self.write_history([])
        with self.assertRaisesRegex(ValueError, "empty history"):
            history.upload("owner/repo")

    def test_invalid_or_duplicate_ids_are_rejected(self):
        with self.assertRaises(ValueError):
            history.validate_history({"schema": 1, "runs": [sample_run(), sample_run()]})

    def test_history_keeps_more_than_ninety_runs_and_reruns_are_idempotent(self):
        data = {"schema": 1, "runs": [sample_run(str(index)) for index in range(120)]}
        append_run(data, sample_run("120"))
        self.assertEqual(len(data["runs"]), 121)
        append_run(data, sample_run("120"))
        self.assertEqual(len(data["runs"]), 121)

    def test_missing_scenarios_fail_collection(self):
        Path("benchmarks/pangui").mkdir(parents=True)
        Path("benchmarks/pangui/example.txt").touch()
        with self.assertRaisesRegex(ValueError, "incomplete run"):
            collect_results(Path("criterion"))

    def test_collects_native_library_names_and_confidence_intervals(self):
        Path("benchmarks/pangui").mkdir(parents=True)
        Path("benchmarks/pangui/example.txt").touch()
        for library in ["Taffy main", "Morphorm", "Torin"]:
            directory = Path("criterion") / library / "new"
            directory.mkdir(parents=True)
            (directory / "benchmark.json").write_text(json.dumps({
                "group_id": "PanGUI", "function_id": "example", "value_str": library}))
            (directory / "estimates.json").write_text(json.dumps({"mean": {
                "point_estimate": 10, "confidence_interval": {"lower_bound": 9, "upper_bound": 11}}}))
        results = collect_results(Path("criterion"))
        self.assertEqual({result["library"] for result in results}, {"taffy", "morphorm", "torin"})
        self.assertTrue(all(result["upper_ns"] == 11 for result in results))


if __name__ == "__main__":
    unittest.main()
