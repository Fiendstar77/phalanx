"""Model checks run in the PyTorch-equipped CI job."""
import unittest


class PolicyTests(unittest.TestCase):
    def test_mixed_traces_cannot_supply_learned_or_composed_teacher_labels(self):
        import json
        from pathlib import Path
        import tempfile
        from analysis.train import load

        for controller in ("Learned", "Opportunity"):
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "manifest.json").write_text(json.dumps({"source_sha256": {"fixture": "test"}}))
                header = {"controller": "Utility", "feature_names": ["health"], "actions": ["ADVANCE"], "seed": 0}
                decision = {"event": "decision", "features": [0.5], "action_mask": [True], "action": "ADVANCE", "comparison": {"controller": controller}}
                (root / "trace.jsonl").write_text(json.dumps(header) + "\n" + json.dumps(decision) + "\n")
                with self.assertRaisesRegex(ValueError, "utility decisions"):
                    load([root], train_end=1, val_end=2)

    def test_founders_width_is_a_two_to_five_million_parameter_mlp(self):
        from analysis.train import Policy

        count = sum(p.numel() for p in Policy(29, 9, hidden=1024, depth=3).parameters())
        self.assertGreaterEqual(count, 2_000_000)
        self.assertLessEqual(count, 5_000_000)


if __name__ == "__main__":
    unittest.main()
