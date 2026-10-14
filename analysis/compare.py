"""Descriptive comparisons with seed-block bootstrap confidence intervals."""
import argparse, csv, json, random, statistics
from collections import defaultdict
from pathlib import Path

def interval(values, repetitions=4000, seed=2026):
    if len(values) < 2:
        return None
    rng = random.Random(seed)
    samples = sorted(statistics.fmean(rng.choices(values, k=len(values))) for _ in range(repetitions))
    return [samples[int(repetitions * .025)], samples[int(repetitions * .975)]]

def summarize(path):
    with (Path(path) / "battles.csv").open() as stream:
        rows = list(csv.DictReader(stream))
    groups = defaultdict(list)
    for row in rows:
        groups[row["arena"]].append(row)
    result = {}
    for arena, trials in groups.items():
        blocks = defaultdict(list)
        for row in trials:
            blocks[int(row["seed"])].append(row)
        if any(len(v) != 2 or {r["swapped"] for r in v} != {"0", "1"} for v in blocks.values()):
            raise ValueError("each seed must include exactly both orientations")
        effects = [statistics.fmean(float(r["score_advantage"]) for r in v) for v in blocks.values()]
        survival = [statistics.fmean(float(r["high_survival"]) - float(r["low_survival"]) for r in v) for v in blocks.values()]
        result[arena] = {"seed_blocks": len(blocks), "battles": len(trials), "high_wins": sum(float(r["score_high"]) == 1 for r in trials),
                         "draws": sum(float(r["score_high"]) == .5 for r in trials), "timeouts": sum(int(r["timeout"]) for r in trials),
                         "score_advantage": statistics.fmean(effects), "score_advantage_95ci": interval(effects),
                         "survival_advantage": statistics.fmean(survival), "survival_advantage_95ci": interval(survival)}
    return result

if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__); p.add_argument("directory"); p.add_argument("--output")
    a = p.parse_args(); report = summarize(a.directory)
    text = json.dumps(report, indent=2) + "\n"
    if a.output: Path(a.output).write_text(text)
    else: print(text)
