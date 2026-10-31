# Local Bonsai test

Bonsai 1.7B has a local native inference test. It is **not connected to the
Phalanx game**. The game's Utility/Learned switch uses the existing exported
PyTorch policy; comic speech bubbles use authored event lines.

Run three combat-bark prompts from the project root:

```bash
python3 tools/bonsai_smoke.py
```

Or describe a situation yourself:

```bash
python3 tools/bonsai_smoke.py --prompt "You are a frightened rock thrower. Your general orders a retreat."
```

The command compiles a temporary native runner, loads the local GGUF directly
through `libllama`, and prints the generated lines. It uses four CPU threads,
zero GPU layers, greedy sampling and at most 48 generated tokens per case.
Each case starts a fresh process and reloads the model. The reported wall time
includes model loading; it is not a measurement of a persistent in-game worker.

`outputs/bonsai-smoke/report.json` records the model and library hashes,
runtime revision, raw output, elapsed time and Linux peak RSS. Prompts and full
logs are saved beside it. Output checks cover one nonempty line, at most twenty
words and no markup/control characters. Passing these checks does not prove
that the line fits the event; read the output. Reaching the timeout, a runtime
failure or an invalid output makes the command exit nonzero. A token cap can
truncate a line; the raw result is retained for review. Repeated runs overwrite
the selected output directory; use `--output outputs/bonsai-other` to retain a
separate run.

## Local prerequisites

The existing workstation setup is Linux with a C++ compiler and these files:

- `models/bonsai-1.7b/Bonsai-1.7B.gguf`
- `models/llama.cpp/examples/simple/simple.cpp`
- llama.cpp headers and shared libraries in `models/llama.cpp/build/bin/`

Model and runtime directories are ignored by Git and are **not delivered by a
clone of Phalanx**. Nothing is downloaded or installed by the test command.
`--model` and `--runtime` accept alternative local paths.

For a fresh Linux setup, using the installed `hf` command:

```bash
hf download prism-ml/Bonsai-1.7B-gguf Bonsai-1.7B.gguf LICENSE NOTICE.txt README.md \
  --revision 210a9e99f79cb184909d49595906526eb2b3dd9a \
  --local-dir models/bonsai-1.7b
git clone https://github.com/PrismML-Eng/llama.cpp models/llama.cpp
git -C models/llama.cpp checkout c96baf4abe24d11a50f326e40b6223b687449d63
cmake -S models/llama.cpp -B models/llama.cpp/build \
  -DBUILD_SHARED_LIBS=ON -DGGML_CUDA=OFF -DLLAMA_CURL=OFF \
  -DLLAMA_BUILD_TESTS=OFF -DLLAMA_BUILD_EXAMPLES=OFF -DLLAMA_BUILD_TOOLS=OFF
cmake --build models/llama.cpp/build --target llama --parallel 1
python3 tools/bonsai_smoke.py
```

These are the revisions present on the workstation for this test. The runner
adapts the runtime's simple example in a temporary directory and fails with an
explicit error if its expected source markers change. It leaves the downloaded
runtime sources untouched. Setup requires network access; inference is local
and needs no Ollama, LM Studio or HTTP server.

The [Bonsai model](https://huggingface.co/prism-ml/Bonsai-1.7B-gguf) includes
Apache 2.0 `LICENSE` and `NOTICE.txt` files. Keep them with any model distribution.
The [native runtime](https://github.com/PrismML-Eng/llama.cpp) has its own license
and third-party notices. This test does not bundle either dependency.

## Next integration boundary

[Battle reports and tactical shadow evaluation](ai-studies.md) now support
studying Bonsai action proposals on real decision-time observations. This does
not give Bonsai live control of the game.

An optional background worker could prepare event-specific lines between
battles and cache accepted text. Authored lines remain available immediately.
Generated text must not change combat outcomes or claim events that did not
happen. That worker, cache, game controls and runtime fallback are not implemented
by this smoke test. Tactical LLM proposals also remain a separate experiment.
