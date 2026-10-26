"""Model checks run in the PyTorch-equipped CI job."""
import unittest


class PolicyTests(unittest.TestCase):
    def test_founders_width_is_a_two_to_five_million_parameter_mlp(self):
        from analysis.train import Policy

        count = sum(p.numel() for p in Policy(29, 9, hidden=1024, depth=3).parameters())
        self.assertGreaterEqual(count, 2_000_000)
        self.assertLessEqual(count, 5_000_000)


if __name__ == "__main__":
    unittest.main()
