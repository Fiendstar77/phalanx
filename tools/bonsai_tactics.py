"""Same-state tactical shadow evaluation. No simulation actions are changed."""
import json
import math
from pathlib import Path

ACTIONS = ["ADVANCE", "ATTACK", "HIGH GROUND", "FLANK", "REGROUP", "RETREAT", "HOLD", "SEEK COVER", "SUPPORT ALLY"]
FEATURES = ["aggression", "caution", "courage", "teamwork", "patience", "self_preservation", "discipline", "health", "thrower", "x", "z", "elevation", "cover", "rough", "enemy_distance", "enemy_visible", "known_enemies", "enemy_health", "contact_confidence", "nearby_allies", "isolation", "local_threat", "support_urgency", "engaged_allies", "ridge_arrived", "hold_allowed", "flank_ready", "contact_exists", "flank_remaining"]
SYSTEM = (
    'You propose tactical actions for a fighter in Phalanx. '
    'Use only the observed features. Values are normalized; health is the remaining fraction. '
    'Enemy information is perceived or remembered, not omniscient. '
    'Choose exactly one of eligible_actions. Reply only with JSON: {"action":"ACTION NAME"}. '
    'ADVANCE closes on a known contact or searches; ATTACK strikes a visible enemy in range; '
    'HIGH GROUND seeks elevation; FLANK finds another attack angle; REGROUP rejoins allies; '
    'RETREAT withdraws; HOLD waits; SEEK COVER reduces exposure; SUPPORT ALLY helps a teammate. '
    'Balance danger, health and the fighter personality. Do not invent actions or outcomes.'
)


def load_cases(path, samples):
    """Uniformly sample recorded decisions by index; labels never enter prompts."""
    if samples < 1:
        raise ValueError("samples must be positive")
    rows = []
    header, ended = None, False
    with Path(path).open() as source:
        for line_number, line in enumerate(source, 1):
            row = json.loads(line)
            if ended:
                raise ValueError("trace has records after end")
            if row.get("event") == "start":
                if header is not None or rows:
                    raise ValueError("expected one start record before decisions")
                header = row
                if row.get("actions") != ACTIONS or row.get("feature_names") != FEATURES:
                    raise ValueError("unsupported trace action/feature schema")
            elif row.get("event") == "end":
                ended = True
            elif row.get("event") == "decision":
                if header is None or row.get("phase") != "decision_before_movement":
                    raise ValueError("expected a decision-time observation")
                values, mask = row.get("features", []), row.get("action_mask", [])
                if (len(values) != len(FEATURES) or not all(type(v) in (int, float) and math.isfinite(v) for v in values)
                        or len(mask) != len(ACTIONS) or not all(type(v) is bool for v in mask)):
                    raise ValueError("invalid observation or action mask")
                comparison = row.get("comparison") or {}
                utility, learned = comparison.get("utility"), comparison.get("learned")
                if type(utility) is not int or not 0 <= utility < len(ACTIONS) or not mask[utility]:
                    raise ValueError("missing or ineligible utility fallback")
                if learned is not None and (type(learned) is not int or not 0 <= learned < len(ACTIONS) or not mask[learned]):
                    raise ValueError("invalid learned reference")
                # These allowlisted values are the only model input. Exclude scores,
                # chosen labels, targets, full initial world state and future events.
                observation = {name: round(value, 5) for name, value in zip(FEATURES, values)}
                situation = json.dumps({"observation": observation,
                                        "eligible_actions": [a for a, legal in zip(ACTIONS, mask) if legal]},
                                       separators=(",", ":"))
                rows.append({"name": f"decision-{line_number}", "situation": situation,
                             "trace_line": line_number, "tick": row["tick"], "unit": row["unit"],
                             "action_mask": mask, "utility": utility, "learned": learned,
                             "actual_action": row["action"]})
    if header is None or not ended or not rows:
        raise ValueError("a complete trace with decisions is required")
    count = min(samples, len(rows))
    indices = [round(i * (len(rows) - 1) / (count - 1)) if count > 1 else len(rows) // 2 for i in range(count)]
    return header, [rows[i] for i in indices]


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON key")
        result[key] = value
    return result


def govern(text, mask, utility, *, exit_code=0, timed_out=False, elapsed_ms=0, budget_ms=1000):
    """Reject rather than repair invalid proposals; select the recorded utility fallback."""
    if (len(mask) != len(ACTIONS) or not all(type(v) is bool for v in mask)
            or type(utility) is not int or not 0 <= utility < len(ACTIONS) or not mask[utility]):
        raise ValueError("invalid fallback contract")
    proposal, reason = None, None
    if timed_out:
        reason = "timeout"
    elif exit_code != 0:
        reason = "runtime_failure"
    elif not math.isfinite(elapsed_ms) or elapsed_ms > budget_ms:
        reason = "deadline_exceeded"
    else:
        try:
            value = json.loads(text, object_pairs_hook=_unique_object)
            if not isinstance(value, dict) or set(value) != {"action"} or type(value["action"]) is not str:
                reason = "invalid_schema"
            else:
                proposal = value["action"]
                if proposal not in ACTIONS:
                    reason = "unknown_action"
                elif not mask[ACTIONS.index(proposal)]:
                    reason = "ineligible_action"
        except (ValueError, TypeError):
            reason = "invalid_json"
    chosen = ACTIONS.index(proposal) if reason is None else utility
    return {"proposal": proposal, "accepted": reason is None, "fallback_reason": reason,
            "governed_action": ACTIONS[chosen], "utility_action": ACTIONS[utility]}


def write_summary(output, report):
    rows = report["cases"]
    accepted = [r for r in rows if r["accepted"]]
    learned = [r for r in accepted if r["learned"] is not None]
    report["summary"] = {
        "sampled_decisions": len(rows), "accepted": len(accepted), "fallbacks": len(rows) - len(accepted),
        "accepted_utility_agreement": sum(r["governed_action"] == r["utility_action"] for r in accepted) / len(accepted) if accepted else None,
        "accepted_learned_comparisons": len(learned),
        "accepted_learned_agreement": sum(r["governed_action"] == ACTIONS[r["learned"]] for r in learned) / len(learned) if learned else None,
        "mean_request_wall_ms": sum(r["wall_seconds"] * 1000 for r in rows) / len(rows),
    }
    text = ("# Bonsai tactical shadow study\n\n"
            f"{len(rows)} recorded decisions; {len(accepted)} accepted proposals; {len(rows) - len(accepted)} utility fallbacks.\n\n"
            "The simulation already executed these decisions. Governed actions below are hypothetical; this study changes no battle. "
            "Utility/PyTorch labels and score magnitudes were withheld from the prompt. "
            "Eligibility comes from existing rules; the model has not learned legality.\n\n"
            "| Tick / unit | Actual | Utility | PyTorch | Bonsai governed | Result | Request ms |\n"
            "| --- | --- | --- | --- | --- | --- | ---: |\n")
    for row in rows:
        reference = ACTIONS[row["learned"]] if row["learned"] is not None else "unavailable"
        text += (f"| {row['tick']} / {row['unit']} | {row['actual_action']} | {row['utility_action']} | {reference} | "
                 f"{row['governed_action']} | {row['fallback_reason'] or 'accepted'} | {row['wall_seconds'] * 1000:.1f} |\n")
    text += ("\n[Full report and provenance](report.json). Raw prompts and completions are retained beside it.\n\n"
             "Requests reload the model and include loading in their latency. They do not measure a resident worker. "
             "Timeout and malformed, extra-field, duplicate-key, unknown or ineligible proposals fall back to utility. "
             "This is a small correlated sample, not an independent test set, training, live control or evidence of stronger tactics. "
             "Win-rate comparison requires separate controlled rollouts with the model actually choosing actions.\n")
    (Path(output) / "report.md").write_text(text)
