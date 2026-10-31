"""Integrity and semantic consistency of the recruiter-facing evidence bundle."""
import hashlib
import json
from pathlib import Path
import shutil
import tempfile
import unittest
from tools.check_ai_evidence import check, same_f32_state

ROOT = Path(__file__).resolve().parents[1]
BUNDLE = ROOT / "docs/results/v06"


class EvidenceTests(unittest.TestCase):
    def test_f32_persistence_comparison_accepts_json_spelling_but_not_drift(self):
        self.assertTrue(same_f32_state({"caution": 0.9}, {"caution": 0.8999999761581421}))
        self.assertFalse(same_f32_state({"caution": 0.9}, {"caution": 0.9001}))

    def test_published_evidence_is_complete_and_consistent(self):
        result = check(BUNDLE)
        self.assertEqual(result["campaign_battles"], 3)
        self.assertEqual(result["decision_records"], 2015)

    def test_tampered_and_missing_artifacts_are_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "bundle"
            shutil.copytree(BUNDLE, bundle)
            path = bundle / "barks/retreat.raw.txt"
            original = path.read_bytes()
            path.write_text("fabricated result")
            with self.assertRaisesRegex(ValueError, "hash mismatch"):
                check(bundle)
            path.write_bytes(original)
            path.unlink()
            with self.assertRaisesRegex(ValueError, "inventory differs"):
                check(bundle)

    def test_resigning_a_changed_reference_does_not_hide_semantic_corruption(self):
        with tempfile.TemporaryDirectory() as directory:
            bundle = Path(directory) / "bundle"
            shutil.copytree(BUNDLE, bundle)
            path = bundle / "tactical/report.json"
            report = json.loads(path.read_text())
            report["cases"][0]["utility"] = 8
            path.write_text(json.dumps(report))
            manifest_path = bundle / "manifest.json"
            manifest = json.loads(manifest_path.read_text())
            manifest["files_sha256"]["tactical/report.json"] = hashlib.sha256(path.read_bytes()).hexdigest()
            manifest_path.write_text(json.dumps(manifest))
            with self.assertRaisesRegex(ValueError, "model/reference mismatch"):
                check(bundle)


if __name__ == "__main__":
    unittest.main()
