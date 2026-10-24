"""Utility-teacher traces for 3v3 Founders pressure fields. Seeds stay whole for the later split."""
import argparse, concurrent.futures, hashlib, json, platform, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()

def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--output", required=True)
    p.add_argument("--seeds", type=int, default=64)
    p.add_argument("--workers", type=int, default=4)
    p.add_argument("--release", action="store_true")
    args = p.parse_args()
    if args.seeds < 1 or args.workers < 1:
        p.error("positive seeds and workers required")
    output = Path(args.output).resolve()
    if output.exists():
        raise ValueError("output directory already exists; choose a fresh directory")
    output.mkdir(parents=True)
    profile = "release" if args.release else "debug"
    binary = ROOT / "target" / profile / "phalanx-batch"
    subprocess.run(["cargo", "build", "--locked", "--no-default-features", "--bin", "phalanx-batch"] + (["--release"] if args.release else []), cwd=ROOT, check=True)
    source = {str(path.relative_to(ROOT)): digest(path) for path in sorted((ROOT / "src").glob("*.rs"))}
    manifest = {"schema_version": 1, "study": "founders-pressure-3v3", "seeds": args.seeds, "teacher": "utility",
                "git_commit": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
                "source_sha256": source, "binary_sha256": digest(binary), "python": platform.python_version()}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    def trial(item):
        seed, swapped = item
        stem = f"pressure-{seed:04}-{'swap' if swapped else 'normal'}"
        cmd = [str(binary), "--founders", "--pressure", "--seed", str(seed), "--output", str(output / (stem + ".json")), "--trace", str(output / (stem + ".jsonl"))]
        if swapped:
            cmd.append("--swap")
        subprocess.run(cmd, check=True, capture_output=True, text=True)
        return stem
    items = [(seed, swap) for seed in range(args.seeds) for swap in (False, True)]
    done = 0
    with concurrent.futures.ThreadPoolExecutor(max_workers=args.workers) as pool:
        for _stem in pool.map(trial, items):
            done += 1
            if done % 8 == 0:
                print(f"founders pressure: {done}/{len(items)}", flush=True)
    manifest["completed_battles"] = done
    manifest["files_sha256"] = {path.name: digest(path) for path in sorted(output.iterdir()) if path.name != "manifest.json"}
    (output / "manifest.json").write_text(json.dumps(manifest, indent=2) + "\n")
    print(f"Saved {done} battles to {output}")

if __name__ == "__main__":
    main()
