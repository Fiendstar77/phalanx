# Measurement and PyTorch baseline

This pipeline collects real Rust simulation output and trains a small imitation policy. The game uses utility AI; the learned network is an offline action-selection baseline, not an installed game controller or a generative model.

## Setup

```bash
python3 -m venv .venv
.venv/bin/python -m pip install -r analysis/requirements.txt
```

The requirements select CPU PyTorch. The small network does not require a GPU.

## Collect reproducible data

```bash
.venv/bin/python -m analysis.experiments --study teamwork --seeds 48 --trace --output outputs/teamwork
.venv/bin/python -m analysis.experiments --study caution --seeds 48 --trace --output outputs/caution
.venv/bin/python -m analysis.experiments --study profiles --seeds 48 --trace --output outputs/profiles
```

Each study includes Ridge, Choke, and Open Field, with both spawn orientations for every seed. Trait studies change only the named trait (0.2 vs 0.9); all other traits are 0.5, and roles/combat statistics match. The profiles study collects the original aggressive/disciplined configurations and is not a single-trait experiment.

Directories must be fresh. Failed trials stop the run. Manifests record seed/configuration, source and binary hashes, Git state, and hashes of generated records. JSONL includes exact decision-time features, contacts, eligible-action masks, choices, and damage events. A replay test checks that logging does not change combat.

## Analyze trait effects

```bash
.venv/bin/python -m analysis.compare outputs/teamwork --output outputs/teamwork-report.json
.venv/bin/python -m analysis.plots --study outputs/teamwork-report.json --output outputs/teamwork.png
```

The primary contrast is high-minus-low match score (win=1, draw=0.5, loss=0), averaged across both sides within each seed. Confidence intervals use 4,000 seed-block bootstrap resamples, preserving the paired orientations. Maps are reported separately. Timeouts and draws remain in the data. This first run is exploratory, not a powered confirmatory study; intervals are not adjusted for multiple comparisons.

Cover/high-ground use is living-unit-time weighted. Ally distance is the mean pairwise distance among surviving allies, averaged over ticks with at least two allies; a zero sample count is missing data, not zero distance. Damage includes raw resolved hits and may exceed effective health loss due to simultaneous overkill.

## Train and verify

```bash
.venv/bin/python -m analysis.train --data outputs/teamwork outputs/caution outputs/profiles --output outputs/policy --epochs 20
.venv/bin/python -m analysis.predict outputs/policy
.venv/bin/python -m analysis.plots --policy outputs/policy
```

Seeds 0–31 train, 32–39 validate, and 40–47 test. All arenas, orientations, and studies for a seed stay in the same split. Feature normalization uses training data only. Best checkpoint selection uses validation macro-F1; held-out test scores are computed after selection.

The 29 inputs contain traits and perceived tactical state. Utility-score magnitudes, chosen actions, future state, and hidden enemy positions are excluded. Eligibility masks come from the existing tactical guards and are supplied to both learner and baselines. Thus masked validity is guaranteed by the interface, not learned.

The network has two 64-unit hidden layers and nine outputs. AdamW and class-weighted cross entropy train it to reproduce the utility teacher. Reports include held-out accuracy, supported-class macro-F1, per-class counts/precision/recall, raw eligibility rate, simple baselines, and single-observation CPU timing. There is no claim of better combat, new-map generalization, player preference, or generative reasoning.

The saved `policy.pt` contains model weights, feature/action names, and train-only normalization. `predict` reloads it with `weights_only=True` and verifies stored inference observations. See the [official PyTorch serialization guidance](https://docs.pytorch.org/tutorials/beginner/saving_loading_models.html).
