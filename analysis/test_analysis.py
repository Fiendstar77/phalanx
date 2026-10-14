import tempfile,unittest,csv
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
if __name__=="__main__":unittest.main()
