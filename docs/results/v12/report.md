# PyTorch decision-sequence forecast

Research model; no live game control. Whole seeds and fighter identities are held out.

| Split | Samples | Neural NLL | Markov NLL | Neural accuracy | Persistence accuracy |
| --- | ---: | ---: | ---: | ---: | ---: |
| train | 14509 | 0.2233 | 0.2690 | 0.938 | 0.937 |
| validation | 4565 | 0.2581 | 0.2958 | 0.931 | 0.933 |
| test | 4079 | 0.2099 | 0.2565 | 0.941 | 0.943 |

Dataset authorship is recorded in provenance; this model is not an opponent-visible observation model.
Ticks are correlated; held-out seeds are the evaluation groups.
Forecast accuracy does not establish stronger combat or calibrated certainty.
No online training, Spark cluster or world simulation is implemented here.

Uses the decision maker own prior observations; requires a separate opponent-visible dataset and tactical evaluation before opponent deployment.
