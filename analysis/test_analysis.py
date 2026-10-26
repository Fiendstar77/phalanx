import tempfile,unittest,csv,json,hashlib
from pathlib import Path
from analysis.compare import interval,summarize
class AnalysisTests(unittest.TestCase):
    def test_bootstrap_is_repeatable_and_does_not_invent_uncertainty(self):
        self.assertEqual(interval([0,1,-1,0]),interval([0,1,-1,0]))
        self.assertEqual(interval([.5]*8),[.5,.5]);self.assertIsNone(interval([.5]))
    def test_counterbalanced_effect_clusters_both_sides_by_seed(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/"battles.csv"
            with path.open("w",newline="") as f:
                w=csv.DictWriter(f,fieldnames=["arena","seed","swapped","score_advantage","score_high","high_survival","low_survival","timeout"]);w.writeheader()
                for seed in range(3):
                    for swap,effect in [(0,1),(1,-1)]:w.writerow(dict(arena="ridge",seed=seed,swapped=swap,score_advantage=effect,score_high=(effect+1)/2,high_survival=.5,low_survival=.5,timeout=0))
            r=summarize(directory)["ridge"]
            self.assertEqual(r["seed_blocks"],3);self.assertEqual(r["score_advantage"],0);self.assertEqual(r["score_advantage_95ci"],[0,0])
    def test_missing_orientation_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            (Path(directory)/"battles.csv").write_text("arena,seed,swapped\nridge,1,0\n")
            with self.assertRaises(ValueError):summarize(directory)
    def test_duel_divergence_filters_utility_team_and_rejects_missing_predictions(self):
        from analysis.divergence import summarize as divergence
        for fallback in [0, 1]:
            with tempfile.TemporaryDirectory() as directory:
                root = Path(directory); hashes = {}
                for side in ["normal", "swap"]:
                    path = root / f"choke-0000-{side}.json"
                    path.write_text(json.dumps({"learned_team": 1, "learned_fallbacks": fallback,
                        "units": [{"team": 0, "decision_counts": [50]}, {"team": 1, "decision_counts": [5]}],
                        "disagreements": [{"unit": 7, "utility": 8, "learned": 1}, {"unit": 0, "utility": 0, "learned": 3}]}))
                    hashes[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
                (root / "manifest.json").write_text(json.dumps({"files_sha256": hashes, "map": "choke", "seeds": [0]}))
                if fallback:
                    with self.assertRaisesRegex(ValueError, "valid predictions"): divergence(root)
                else:
                    result = divergence(root)
                    self.assertEqual(result["compared"], 10)
                    self.assertEqual(result["agreement_rate"], .8)
                    self.assertEqual(result["top_disagreements"], [{"utility": "SUPPORT ALLY", "learned": "ATTACK", "count": 2}])
                    path.write_text("tampered")
                    with self.assertRaisesRegex(ValueError, "hash mismatch"): divergence(root)
if __name__=="__main__":unittest.main()
