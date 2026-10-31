"""Run local Bonsai inference without a server or game integration.

Uses the installed llama.cpp simple example and native shared libraries.
Results, compiler output and inference logs go under outputs/bonsai-smoke.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import re
import shutil
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]
CASES = [
    ("protect_ally", "Your ally is hurt. You move between the ally and the attacker."),
    ("retreat", "You are badly hurt and retreat toward your living teammates."),
    ("ranged_warning", "You are throwing a rock toward an enemy. Warn your teammates."),
]
SYSTEM = (
    "You write short combat barks for Phalanx. Speak as the fighter. "
    "Use only the supplied situation; do not invent an outcome. "
    "Answer with one spoken line, twenty words maximum. Do not explain."
)


def sha256(path):
    with path.open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--model", type=Path,
                        default=ROOT / "models/bonsai-1.7b/Bonsai-1.7B.gguf")
    parser.add_argument("--runtime", type=Path, default=ROOT / "models/llama.cpp")
    parser.add_argument("--output", type=Path, default=ROOT / "outputs/bonsai-smoke")
    parser.add_argument("--prompt", help="Test one custom combat situation instead of the three examples.")
    parser.add_argument("--trace", type=Path, help="Evaluate tactical proposals against a complete decision trace.")
    parser.add_argument("--samples", type=int, default=12)
    parser.add_argument("--budget-ms", type=float, default=1500,
                        help="Tactical proposal deadline including model loading.")
    parser.add_argument("--threads", type=int, default=4)
    parser.add_argument("--tokens", type=int, default=48)
    parser.add_argument("--timeout", type=float, default=30,
                        help="Seconds allowed per inference process.")
    args = parser.parse_args()
    if args.trace is not None and args.prompt is not None:
        parser.error("Choose --trace or --prompt.")
    if args.samples < 1 or not math.isfinite(args.budget_ms) or not 0 < args.budget_ms <= 120000:
        parser.error("Require positive samples and a budget in (0, 120000] ms.")
    if not (1 <= args.threads <= 16 and 1 <= args.tokens <= 128
            and math.isfinite(args.timeout) and 0 < args.timeout <= 120):
        parser.error("Require 1–16 threads, 1–128 tokens and a timeout in (0, 120].")
    if args.prompt is not None and not (args.prompt.strip() and len(args.prompt) <= 2000):
        parser.error("The custom situation must contain 1–2000 characters.")
    model, runtime, output = (p.resolve() for p in (args.model, args.runtime, args.output))
    source_path = runtime / "examples/simple/simple.cpp"
    libraries = runtime / "build/bin"
    required = [model, source_path, runtime / "include/llama.h",
                libraries / "libllama.so", libraries / "libggml.so",
                libraries / "libggml-base.so", libraries / "libggml-cpu.so"]
    for path in required:
        if not path.is_file():
            parser.error(f"Missing {path}. See docs/bonsai.md for local setup.")
    compiler = shutil.which("c++")
    if compiler is None:
        parser.error("A C++ compiler is required (for example, the build-essential package).")

    # Adapt the installed example in a temporary build directory. Do not modify
    # or redistribute third-party sources; retain their original license locally.
    source = source_path.read_text()
    thread_marker = "    ctx_params.n_batch = n_prompt;"
    echo_start = "    // print the prompt token-by-token\n"
    echo_end = "    // prepare a batch for the prompt\n"
    if (source.count(thread_marker) != 1 or source.count(echo_start) != 1
            or source.count(echo_end) != 1):
        parser.error("Incompatible llama.cpp simple example. See docs/bonsai.md for the tested revision.")
    source = source.replace(thread_marker, thread_marker +
                            f"\n    ctx_params.n_threads = {args.threads};" +
                            f"\n    ctx_params.n_threads_batch = {args.threads};")
    start, end = source.index(echo_start), source.index(echo_end)
    if end <= start:
        parser.error("Incompatible llama.cpp simple example: prompt echo markers are out of order.")
    source = source[:start] + source[end:]
    # Measure this inference process, excluding the separate compiler process.
    rss_marker = "    llama_model_free(model);"
    if source.count(rss_marker) != 1:
        parser.error("Incompatible llama.cpp simple example: missing cleanup marker.")
    source = "#include <sys/resource.h>\n" + source.replace(
        rss_marker, rss_marker +
        '\n    struct rusage usage{};\n'
        '    if (getrusage(RUSAGE_SELF, &usage) == 0) {\n'
        '        fprintf(stderr, "PHALANX_PEAK_RSS_KIB=%ld\\n", usage.ru_maxrss);\n'
        '    }')
    output.mkdir(parents=True, exist_ok=True)
    revision = subprocess.run(["git", "-C", str(runtime), "rev-parse", "HEAD"],
                              capture_output=True, text=True, check=True).stdout.strip()
    report = {
        "schema_version": 1,
        "model": str(model), "model_bytes": model.stat().st_size,
        "model_sha256": sha256(model), "runtime_commit": revision,
        "native_library_sha256": sha256(libraries / "libllama.so"),
        "example_sha256": sha256(source_path),
        "cpu_threads": args.threads, "gpu_layers": 0, "sampler": "greedy",
        "external_service": False, "game_integration": False,
        "scope": "Native inference and output shape only; semantic quality requires human review.",
        "cases": [],
    }
    cases = [("custom", args.prompt)] if args.prompt is not None else CASES
    system, metadata = SYSTEM, {}
    if args.trace is not None:
        import bonsai_tactics
        try:
            header, selected = bonsai_tactics.load_cases(args.trace, args.samples)
        except (ValueError, OSError, KeyError) as error:
            parser.error(f"Cannot load tactical study: {error}")
        cases = [(row["name"], row["situation"]) for row in selected]
        metadata = {row["name"]: row for row in selected}
        system = bonsai_tactics.SYSTEM
        report.update({"scope": "Tactical shadow evaluation on recorded decisions; no actions changed.",
                       "trace": str(args.trace.resolve()), "trace_sha256": sha256(args.trace),
                       "trace_seed": header["seed"], "budget_ms": args.budget_ms,
                       "selection": "Uniform decision indices, including endpoints when samples > 1.",
                       "prompt_contract": "same 29 observation features and eligibility; no reference labels or utility scores"})
    report["runner_sha256"] = sha256(Path(__file__))
    with tempfile.TemporaryDirectory(prefix="phalanx-bonsai-") as scratch:
        work = Path(scratch)
        (work / "smoke.cpp").write_text(source)
        executable = work / "smoke"
        command = [compiler, "-O2", str(work / "smoke.cpp"),
                   f"-I{runtime / 'include'}", f"-I{runtime / 'ggml/include'}",
                   f"-L{libraries}", f"-Wl,-rpath,{libraries}",
                   "-lllama", "-lggml", "-lggml-base", "-o", str(executable)]
        with (output / "build.log").open("w") as log:
            try:
                subprocess.run(command, stdout=log, stderr=subprocess.STDOUT,
                               check=True, timeout=60)
            except (subprocess.CalledProcessError, subprocess.TimeoutExpired) as error:
                parser.exit(1, f"Native build failed: {error}. See {output / 'build.log'}\n")
        for name, situation in cases:
            # Qwen3 non-thinking ChatML, matching the earlier native smoke.
            prompt = (f"<|im_start|>system\n{system}<|im_end|>\n"
                      f"<|im_start|>user\n{situation}\n/no_think<|im_end|>\n"
                      "<|im_start|>assistant\n<think>\n\n</think>\n\n")
            (output / f"{name}.prompt.txt").write_text(prompt)
            command = [str(executable), "-m", str(model), "-ngl", "0",
                       "-n", str(args.tokens), prompt]
            started = time.monotonic()
            timed_out = False
            with (output / f"{name}.log").open("w") as log:
                try:
                    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=log,
                                            timeout=min(args.timeout, args.budget_ms / 1000) if args.trace else args.timeout)
                    raw = result.stdout.decode("utf-8", errors="replace")
                    code = result.returncode
                except subprocess.TimeoutExpired as error:
                    raw = (error.stdout or b"").decode("utf-8", errors="replace")
                    code, timed_out = None, True
            elapsed = round(time.monotonic() - started, 3)
            (output / f"{name}.raw.txt").write_text(raw)
            text = raw.strip()
            issues = []
            if timed_out:
                issues.append("inference_timeout")
            elif code != 0:
                issues.append("inference_failed")
            if not text:
                issues.append("empty_output")
            if "\n" in text or "\r" in text:
                issues.append("multiple_lines")
            if len(text.split()) > 20:
                issues.append("more_than_twenty_words")
            if re.search(r"<[^>]*>|[\x00-\x1f\x7f]", text):
                issues.append("markup_or_control_characters")
            rss = re.search(r"PHALANX_PEAK_RSS_KIB=(\d+)", (output / f"{name}.log").read_text())
            peak = int(rss[1]) if rss else None
            row = {"name": name, "situation": situation, "text": text,
                   "exit_code": code, "timed_out": timed_out, "wall_seconds": elapsed,
                   "peak_rss_kib": peak, "format_ok": not issues, "issues": issues}
            if args.trace is not None:
                row.update(metadata[name])
                row.update(bonsai_tactics.govern(text, row["action_mask"], row["utility"],
                           exit_code=code, timed_out=timed_out, elapsed_ms=elapsed * 1000,
                           budget_ms=args.budget_ms))
                # Bark-shape checks do not apply to the tactical JSON contract.
                row.pop("format_ok")
                row.pop("issues")
            report["cases"].append(row)
            status = (row["fallback_reason"] or "accepted") if args.trace else ("format OK" if not issues else str(issues))
            print(f"{name}: {text!r} ({elapsed:.3f}s; {status})",
                  flush=True)
    if args.trace is not None:
        bonsai_tactics.write_summary(output, report)
        report["study_complete"] = True
    else:
        report["passed"] = all(row["format_ok"] for row in report["cases"])
    destination = output / "report.json"
    destination.write_text(json.dumps(report, indent=2) + "\n")
    print(f"Report: {destination}")
    raise SystemExit(0 if report.get("study_complete", report.get("passed")) else 1)


if __name__ == "__main__":
    main()
