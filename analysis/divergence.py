"""Summarize same-state disagreements on learned-team trajectories in a duel batch."""
import argparse, collections, hashlib, json
from pathlib import Path


def summarize(directory):
    root = Path(directory)
    manifest = json.loads((root / "manifest.json").read_text())
    pairs = collections.Counter()
    compared = 0
    battles = 0
    for name, expected in manifest["files_sha256"].items():
        if not name.startswith("choke-") or not name.endswith(".json"):
            continue
        path = root / name
        if hashlib.sha256(path.read_bytes()).hexdigest() != expected:
            raise ValueError(f"battle hash mismatch: {name}")
        battle = json.loads(path.read_text())
        if battle["learned_team"] is None or battle["learned_fallbacks"] != 0:
            raise ValueError("requires a duel batch with valid predictions at every learned decision")
        team = battle["learned_team"]
        compared += sum(sum(u["decision_counts"]) for u in battle["units"] if u["team"] == team)
        pairs.update((r["utility"], r["learned"]) for r in battle["disagreements"] if r["unit"] // 5 == team)
        battles += 1
    if battles != 2 * len(manifest["seeds"]) or not compared:
        raise ValueError("incomplete duel batch")
    from analysis.experiments import ROOT
    actions = json.loads((ROOT / "assets/policy.json").read_text())["actions"]
    return {"scope": "Learned-controlled teams only; same-state Utility/PyTorch comparison",
            "source_manifest_sha256": hashlib.sha256((root / "manifest.json").read_bytes()).hexdigest(),
            "arena": manifest["map"], "battles": battles, "compared": compared,
            "agreements": compared - sum(pairs.values()), "unavailable": 0,
            "agreement_rate": (compared - sum(pairs.values())) / compared,
            "top_disagreements": [{"utility": actions[u], "learned": actions[l], "count": count}
                                  for (u, l), count in sorted(pairs.items(), key=lambda p: (-p[1], *p[0]))]}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory")
    parser.add_argument("--output")
    args = parser.parse_args()
    report = json.dumps(summarize(args.directory), indent=2) + "\n"
    if args.output:
        Path(args.output).write_text(report)
    else:
        print(report)
