"""Verify the published AI evidence without downloading a model or running graphics."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import struct
import subprocess

try:
    from . import bonsai_tactics as tactics
except ImportError:
    import bonsai_tactics as tactics

ROOT = Path(__file__).resolve().parents[1]


def read(path):
    return json.loads(path.read_text())


def require(condition, message):
    if not condition:
        raise ValueError(message)


def inside(root, name):
    path = (root / name).resolve()
    require(path.is_relative_to(root.resolve()), f"path outside evidence bundle: {name}")
    return path


def digest(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def same_f32_state(left, right):
    """Typed Rust f32 JSON and serde_json::Value can spell the same float differently."""
    if isinstance(left, dict) and isinstance(right, dict):
        return left.keys() == right.keys() and all(same_f32_state(left[k], right[k]) for k in left)
    if isinstance(left, list) and isinstance(right, list):
        return len(left) == len(right) and all(same_f32_state(a, b) for a, b in zip(left, right))
    if type(left) in (int, float) and type(right) in (int, float):
        return struct.pack("<f", left) == struct.pack("<f", right)
    return left == right


def check(bundle, verify_source=False):
    bundle = Path(bundle).resolve()
    manifest = read(bundle / "manifest.json")
    require(manifest["schema_version"] == 1, "unsupported evidence schema")
    for flag in ("source_tree_clean_at_start", "source_tree_clean_at_end", "source_unchanged_during_capture"):
        require(manifest[flag] is True, f"capture does not attest {flag}")
    require(re.fullmatch(r"[0-9a-f]{40}", manifest["source_commit"]), "invalid source commit")
    inventory = manifest["files_sha256"]
    actual = {str(p.relative_to(bundle)) for p in bundle.rglob("*") if p.is_file() and p != bundle / "manifest.json"}
    require(set(inventory) == actual, "evidence inventory differs from files on disk")
    for name, expected in inventory.items():
        require(digest(inside(bundle, name)) == expected, f"hash mismatch: {name}")
    if verify_source:
        for name, expected in manifest["source_sha256"].items():
            blob = subprocess.check_output(["git", "show", f"{manifest['source_commit']}:{name}"], cwd=ROOT)
            require(hashlib.sha256(blob).hexdigest() == expected, f"captured source hash mismatch: {name}")

    previous = None
    decision_count = 0
    campaign_reports = sorted((bundle / "campaign").glob("battle-*/report.json"))
    require(len(campaign_reports) == 3, "expected three captured campaign battles")
    for path in campaign_reports:
        report = read(path)
        folder = path.parent
        require(read(folder / "status.json")["complete"] is True, "incomplete battle")
        rows = [json.loads(line) for line in (folder / "decisions.jsonl").read_text().splitlines()]
        require(rows[0]["event"] == "start" and rows[-1]["event"] == "end", "trace lacks start/end")
        decisions = [r for r in rows if r["event"] == "decision"]
        require(len(decisions) == report["decision_records"], "decision count mismatch")
        battle = read(folder / "battle.json")
        require(battle["finished"] and battle["result"] == report["result"] == rows[-1]["result"], "outcome mismatch")
        for row in decisions:
            require(row["phase"] == "decision_before_movement", "wrong observation phase")
            action = tactics.ACTIONS.index(row["action"])
            require(row["action_mask"][action] is True and row["comparison"]["executed"] == action, "executed action mismatch")
        if previous is not None:
            for unit in rows[0]["initial_units"]:
                receipt = previous[unit["fighter_id"]]
                require(same_f32_state(unit["mind"], receipt["mind_after"]), "next battle lost adapted mind")
                require(same_f32_state(unit["psyche"], receipt["psyche_after"]), "next battle lost memory")
                require(unit["experience"] == receipt["experience"][1], "next battle lost experience")
        receipts = [f["learning"] for team in read(folder / "learning.json")["fighters"] for f in team]
        previous = {r["fighter_id"]: r for r in receipts}
        for unit in battle["units"]:
            require(same_f32_state(unit["mind"], previous[unit["fighter_id"]]["mind_before"]), "learning before-state mismatch")
        decision_count += len(decisions)

    model_hash = None
    for category in ("tactical", "deadline"):
        report = read(bundle / category / "report.json")
        require(report["study_complete"] and not report["game_integration"], "study scope mismatch")
        trace = inside(bundle, str(Path(category) / report["trace"]))
        require(digest(trace) == report["trace_sha256"], "model study trace hash mismatch")
        _, selected = tactics.load_cases(trace, len(report["cases"]))
        for row, reference in zip(report["cases"], selected):
            for field in ("trace_line", "tick", "unit", "action_mask", "utility", "learned", "actual_action", "situation"):
                require(row[field] == reference[field], f"model/reference mismatch: {field}")
            expected = tactics.govern(row["text"], row["action_mask"], row["utility"], exit_code=row["exit_code"],
                                      timed_out=row["timed_out"], elapsed_ms=row["wall_seconds"] * 1000,
                                      budget_ms=report["budget_ms"])
            for field, value in expected.items():
                require(row[field] == value, f"governor outcome mismatch: {field}")
            raw = (bundle / category / f"{row['name']}.raw.txt").read_text().strip()
            require(raw == row["text"], "raw completion differs from report")
            require(row["situation"] in (bundle / category / f"{row['name']}.prompt.txt").read_text(), "prompt lacks recorded observation")
        accepted = [r for r in report["cases"] if r["accepted"]]
        summary = report["summary"]
        require(summary["accepted"] == len(accepted) and summary["fallbacks"] == len(report["cases"]) - len(accepted), "acceptance count mismatch")
        utility_rate = sum(r["governed_action"] == r["utility_action"] for r in accepted) / len(accepted) if accepted else None
        require(summary["accepted_utility_agreement"] == utility_rate, "utility agreement mismatch")
        learned = [r for r in accepted if r["learned"] is not None]
        learned_rate = sum(r["governed_action"] == tactics.ACTIONS[r["learned"]] for r in learned) / len(learned) if learned else None
        require(summary["accepted_learned_agreement"] == learned_rate and summary["accepted_learned_comparisons"] == len(learned), "learned agreement mismatch")
        mean = sum(r["wall_seconds"] * 1000 for r in report["cases"]) / len(report["cases"])
        require(math.isclose(summary["mean_request_wall_ms"], mean), "latency summary mismatch")
        if model_hash is None:
            model_hash = report["model_sha256"]
        require(model_hash == report["model_sha256"], "mixed model artifacts")
    examples = read(bundle / "tactical/observations.json")
    tactical = read(bundle / "tactical/report.json")["cases"]
    require(len(examples) == len(tactical), "curated example count mismatch")
    for example, row in zip(examples, tactical):
        require(example["sent_to_model"] == json.loads(row["situation"]) and example["raw_completion"] == row["text"], "curated example differs from native evidence")
    barks = read(bundle / "barks/report.json")
    require(barks["model_sha256"] == model_hash, "bark model differs from tactical model")
    for row in barks["cases"]:
        require(row["format_ok"] and 0 < len(row["text"].split()) <= 20, "bark violates word contract")
        require(row["text"] == (bundle / "barks" / f"{row['name']}.raw.txt").read_text().strip(), "bark raw completion mismatch")
    return {"files_verified": len(inventory), "campaign_battles": len(campaign_reports),
            "decision_records": decision_count, "source_commit": manifest["source_commit"]}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--bundle", type=Path, default=ROOT / "docs/results/v06")
    parser.add_argument("--verify-source", action="store_true", help="Also verify captured source against its Git commit; requires that commit in local history.")
    args = parser.parse_args()
    print(json.dumps(check(args.bundle, args.verify_source), indent=2))


if __name__ == "__main__":
    main()
