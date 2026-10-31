"""Model-free contract checks for the optional native tactical study."""
import json
from pathlib import Path
import tempfile
import unittest
from tools.bonsai_tactics import ACTIONS, FEATURES, govern, load_cases


class BonsaiTests(unittest.TestCase):
    def test_ineligible_malformed_and_late_proposals_fall_back(self):
        mask = [True, False] + [True] * 7
        for text, reason in [
            ('{"action":"ATTACK"}', "ineligible_action"),
            ('{"action":"TELEPORT"}', "unknown_action"),
            ('```json\n{"action":"ADVANCE"}\n```', "invalid_json"),
            ('{"action":"ADVANCE","reason":"I win"}', "invalid_schema"),
            ('{"action":"ADVANCE","action":"HOLD"}', "invalid_json"),
        ]:
            result = govern(text, mask, 0)
            self.assertFalse(result["accepted"])
            self.assertEqual(result["fallback_reason"], reason)
            self.assertEqual(result["governed_action"], "ADVANCE")
        self.assertEqual(govern('{"action":"HOLD"}', mask, 0, timed_out=True)["fallback_reason"], "timeout")
        self.assertEqual(govern('{"action":"HOLD"}', mask, 0, exit_code=1)["fallback_reason"], "runtime_failure")
        self.assertEqual(govern('{"action":"HOLD"}', mask, 0, elapsed_ms=1501, budget_ms=1500)["fallback_reason"], "deadline_exceeded")
        self.assertEqual(govern('{"action":"HOLD"}', mask, 0)["governed_action"], "HOLD")
        with self.assertRaises(ValueError):
            govern('{"action":"HOLD"}', mask, 1)

    def test_prompt_excludes_reference_labels_hidden_state_and_scores(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "trace.jsonl"
            header = {"event": "start", "actions": ACTIONS, "feature_names": FEATURES,
                      "initial_units": [{"hidden_enemy": "secret"}]}
            decision = {"event": "decision", "phase": "decision_before_movement", "tick": 10,
                        "unit": 0, "features": [0.5] * 29, "action_mask": [True] * 9,
                        "comparison": {"utility": 8, "learned": 5}, "action": "SUPPORT ALLY",
                        "scores": ["teacher-secret"], "target": [999, 999]}
            rows = [header, decision, dict(decision, tick=15), {"event": "end"}]
            path.write_text("\n".join(map(json.dumps, rows)))
            _, cases = load_cases(path, 12)
            self.assertEqual(len(cases), 2)
            payload = json.loads(cases[0]["situation"])
            self.assertEqual(set(payload), {"observation", "eligible_actions"})
            self.assertEqual(set(payload["observation"]), set(FEATURES))
            self.assertNotIn("secret", cases[0]["situation"])
            self.assertEqual(cases[0]["utility"], 8)
            self.assertEqual(cases[0]["actual_action"], "SUPPORT ALLY")
            path.write_text("\n".join(map(json.dumps, rows[:-1])))
            with self.assertRaisesRegex(ValueError, "complete trace"):
                load_cases(path, 12)


if __name__ == "__main__":
    unittest.main()
