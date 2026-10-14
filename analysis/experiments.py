"""Reproducible trait contrasts and teacher-data collection, using the real Rust simulator."""
import argparse, concurrent.futures, csv, hashlib, json, platform, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ARENAS = ("ridge", "choke", "open")

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def metrics(battle, team):
    units = [u for u in battle["units"] if u["team"] == team]
    ticks = sum(u["alive_ticks"] for u in units)
    samples = battle["ally_distance_samples"][team]
    return {"survival": sum(u["health"] > 0 for u in units) / len(units),
            "damage": sum(u["damage_dealt"] for u in units),
            "cover_fraction": sum(u["cover_ticks"] for u in units) / ticks if ticks else 0,
            "high_ground_fraction": sum(u["high_ground_ticks"] for u in units) / ticks if ticks else 0,
            "ally_distance": battle["ally_distance_sum"][team] / samples if samples else None,
            "support_decisions": sum(u["decision_counts"][8] for u in units),
            "retreat_decisions": sum(u["decision_counts"][5] for u in units)}

def run(args):
    output = Path(args.output).resolve()
    if output.exists():
        raise ValueError("output directory already exists; choose a fresh directory")
    output.mkdir(parents=True)
    binary = ROOT / "target/debug/phalanx-batch"
    subprocess.run(["cargo", "build", "--locked", "--no-default-features", "--bin", "phalanx-batch"], cwd=ROOT, check=True)
    source = {str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / "src").glob("*.rs"))}
    for name in ("Cargo.toml", "Cargo.lock"):
        source[name] = digest(ROOT / name)
    manifest = {"schema_version": 1, "study": args.study, "seed_start": args.seed_start, "seeds": args.seeds,
                "low": args.low, "high": args.high, "arenas": ARENAS, "orientations": [False, True],
                "primary_metric": "high-minus-low match score (win=1, draw=0.5, loss=0), averaged across sides per seed",
                "git_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "tracked_dirty": bool(subprocess.check_output(["git", "status", "--porcelain", "--untracked-files=no"], cwd=ROOT)),
                "source_sha256": source, "binary_sha256": digest(binary), "python": platform.python_version()}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    def trial(item):
        arena, seed, swapped = item
        stem = f"{arena}-{seed:04}-{'swap' if swapped else 'normal'}"
        summary = output / (stem + ".json")
        cmd = [str(binary), "--arena", arena, "--seed", str(seed), "--output", str(summary)]
        if args.study != "profiles":
            cmd += ["--trait", args.study, "--low", str(args.low), "--high", str(args.high)]
        if swapped:
            cmd += ["--swap"]
        if args.trace:
            cmd += ["--trace", str(output / (stem + ".jsonl"))]
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        battle = json.loads(summary.read_text())
        high_team = 0 if swapped else 1
        winner = {"Red wins": 0, "Blue wins": 1, "Draw": None, "Timeout / draw": None}[battle["result"]]
        score = 0.5 if winner is None else float(winner == high_team)
        row = {"arena": arena, "seed": seed, "swapped": int(swapped), "score_high": score,
               "score_advantage": 2 * score - 1, "duration": battle["tick"] / 10,
               "timeout": int(battle["result"] == "Timeout / draw"), "summary": summary.name}
        for label, team in (("high", high_team), ("low", 1 - high_team)):
            row.update({label + "_" + k: v for k, v in metrics(battle, team).items()})
        return row
    items = [(a, s, swap) for a in ARENAS for s in range(args.seed_start, args.seed_start + args.seeds) for swap in (False, True)]
    rows = []
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for row in pool.map(trial, items):
            rows.append(row)
            if len(rows) % 48 == 0:
                print(f"{args.study}: {len(rows)}/{len(items)} battles", flush=True)
    with (output / "battles.csv").open("w", newline="") as file:
        writer = csv.DictWriter(file, fieldnames=list(rows[0]))
        writer.writeheader(); writer.writerows(rows)
    manifest["completed_battles"]=len(rows)
    manifest["files_sha256"]={p.name:digest(p) for p in sorted(output.iterdir()) if p.name!="manifest.json"}
    (output / "manifest.json").write_text(json.dumps(manifest,indent=2)+"\n")
    print(f"Saved {len(rows)} battles to {output}")

if __name__ == "__main__":
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--study", choices=("teamwork", "caution", "profiles"), default="teamwork")
    p.add_argument("--output", required=True)
    p.add_argument("--seeds", type=int, default=48)
    p.add_argument("--seed-start", type=int, default=0)
    p.add_argument("--low", type=float, default=0.2); p.add_argument("--high", type=float, default=0.9)
    p.add_argument("--workers", type=int, default=4); p.add_argument("--trace", action="store_true")
    args = p.parse_args()
    if args.seeds < 1 or args.workers < 1 or not 0 <= args.low < args.high <= 1:
        p.error("positive seeds/workers and 0 <= low < high <= 1 required")
    run(args)
