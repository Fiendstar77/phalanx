# Phalanx evolution study

Isolated new 3v3 campaign, using real simulation and settlement rules. No player save is loaded or changed.

Experience, practice, traits, memories and rival adaptation persist between these battles. PyTorch and Bonsai weights are unchanged.

## Battle 1: Blue wins

[Battle study](battle-200/report.md) · [Learning receipts](battle-200/learning.md)

**KING**

- Self-preservation 25.0% -> 26.0%: finishing below half health strengthens caution in danger.
- Isolation memory: none 0% -> recklessness 55%. Reacts when that situation returns.
- Encirclement memory: none 0% -> recklessness 55%. Reacts when that situation returns.
- Ally loss memory: none 0% -> recklessness 25%. Reacts when that situation returns.
- Failed charge memory: none 0% -> recklessness 20%. Reacts when that situation returns.
- Class practice 0 -> 35 [####--------] next rank at 100.

**QUEEN**

- Self-preservation 25.0% -> 26.0%: finishing below half health strengthens caution in danger.
- Isolation memory: none 0% -> recklessness 55%. Reacts when that situation returns.
- Encirclement memory: none 0% -> recklessness 36%. Reacts when that situation returns.
- Class practice 0 -> 33 [###---------] next rank at 100.

**GENERAL**

- Self-preservation 25.0% -> 26.0%: finishing below half health strengthens caution in danger.
- Isolation memory: none 0% -> recklessness 1%. Reacts when that situation returns.
- Class practice 0 -> 35 [####--------] next rank at 100.

## Battle 2: Blue wins

[Battle study](battle-201/report.md) · [Learning receipts](battle-201/learning.md)

**KING**

- Self-preservation 26.0% -> 27.0%: finishing below half health strengthens caution in danger.
- Isolation memory: recklessness 55% -> recklessness 100%. Reacts when that situation returns.
- Encirclement memory: recklessness 55% -> recklessness 100%. Reacts when that situation returns.
- Class practice 35 -> 70 [########----] next rank at 100.

**QUEEN**

- Self-preservation 26.0% -> 27.0%: finishing below half health strengthens caution in danger.
- Isolation memory: recklessness 55% -> recklessness 100%. Reacts when that situation returns.
- Encirclement memory: recklessness 36% -> recklessness 91%. Reacts when that situation returns.
- Ally loss memory: none 0% -> recklessness 35%. Reacts when that situation returns.
- Class practice 33 -> 68 [########----] next rank at 100.

**GENERAL**

- Self-preservation 26.0% -> 27.0%: finishing below half health strengthens caution in danger.
- Isolation memory: recklessness 1% -> recklessness 56%. Reacts when that situation returns.
- Encirclement memory: none 0% -> recklessness 55%. Reacts when that situation returns.
- Ally loss memory: none 0% -> recklessness 25%. Reacts when that situation returns.
- Class practice 35 -> 70 [########----] next rank at 100.

## Battle 3: Red wins

[Battle study](battle-202/report.md) · [Learning receipts](battle-202/learning.md)

**KING**

- LEVEL UP! 1 -> 2. Experience 70 -> 125.
- Courage 85.0% -> 85.5%: victory reinforces willingness to engage.
- Self-preservation 27.0% -> 28.0%: finishing below half health strengthens caution in danger.
- Ally loss memory: recklessness 25% -> recklessness 50%. Reacts when that situation returns.
- Failed charge memory: recklessness 20% -> recklessness 50%. Reacts when that situation returns.
- Pressure confidence 0.0% -> 7.8%: changed by surviving pressure or being downed.
- CLASS RANK UP! 0 -> 1. Practice is retained across retraining.
- Class practice 70 -> 125 [#-----------] next rank at 300.

**QUEEN**

- LEVEL UP! 1 -> 2. Experience 68 -> 108.
- Courage 85.0% -> 85.5%: victory reinforces willingness to engage.
- Self-preservation 27.0% -> 28.0%: finishing below half health strengthens caution in danger.
- Encirclement memory: recklessness 91% -> recklessness 100%. Reacts when that situation returns.
- Failed charge memory: none 0% -> recklessness 55%. Reacts when that situation returns.
- CLASS RANK UP! 0 -> 1. Practice is retained across retraining.
- Class practice 68 -> 108 [------------] next rank at 300.

**GENERAL**

- LEVEL UP! 1 -> 2. Experience 70 -> 125.
- Courage 85.0% -> 85.5%: victory reinforces willingness to engage.
- Self-preservation 27.0% -> 28.0%: finishing below half health strengthens caution in danger.
- Encirclement memory: recklessness 55% -> recklessness 100%. Reacts when that situation returns.
- Ally loss memory: recklessness 25% -> recklessness 50%. Reacts when that situation returns.
- Failed charge memory: none 0% -> recklessness 30%. Reacts when that situation returns.
- Pressure confidence 0.0% -> 7.8%: changed by surviving pressure or being downed.
- CLASS RANK UP! 0 -> 1. Practice is retained across retraining.
- Class practice 70 -> 125 [#-----------] next rank at 300.
