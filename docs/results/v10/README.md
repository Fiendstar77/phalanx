# Outcome-independent progression

Match results must not reinforce power or personality. This implementation
retains real Theus skill practice and witnessed event memories, while retiring
automatic victory/survival/damage XP bonuses, temperament drift and confidence
boosts. Both sides receive 25 XP, 25 class practice, equal league-based gold and
10 salvage per completed bout. Wins still count in history and story gates.

The source rule is explicit in [AGENTS.md](../../../AGENTS.md). The
[settlement tests](../../../tests/outcome_neutral.rs) compare otherwise identical
settlement states with win/loss/draw labels. Those are staged counterfactual
tests, not claims that a changed label is a real combat result. Separate tests
run real bouts, retain actual practice and event memories, and check that
historical confidence has no decision effect.

## Actual rematch witness

`summary.json`, `campaign-after.json` and eight full studies come from actual
solo combat with authored starting people, Utility control and seeds 42–49.
The diagnostic deliberately defers the next story meeting after a win so the
same two people can spar eight times. No hit or outcome is forced.

| Side | Wins | XP | Class practice | Gold | Salvage |
| --- | ---: | ---: | ---: | ---: | ---: |
| Player | 1 | 200 | 200 | 340 | 80 |
| Opponent | 6 | 200 | 200 | 340 | 80 |

There was one draw. Both temperaments remained unchanged. Actual Theus practice
differed (final melee practice 179 versus 183), and the same isolation exposure
produced fear for one personality and attachment for the other. Those are
experience receipts, not bonuses for winning. This small diagnostic does not
establish opening difficulty, AI superiority or the cause of a particular loss.

Reproduce into a new output directory:

```bash
cargo run --locked --no-default-features --example outcome_neutral_study -- outputs/equal-credit-study
```

The example never loads the player save. New runs generate fresh identities,
timestamps and step timings. JSON/trace exports retain their captured bytes;
trailing blank lines in Markdown learning summaries are normalized. The
manifest records artifact and source hashes for this snapshot.

## Existing saves and historical evidence

Stored XP, mastery, currencies, temperament and history are retained rather than
rewritten. Earlier credits are not retroactively recalculated. Historical
confidence remains readable but is inactive in decisions. New reports carry
`progression_policy: 1`; older reports default to zero and the UI marks their
earlier reward policy. Archives v06–v09 retain the rules and results captured
at their own revisions; their outcome-based adjustments are historical.

This policy makes results independent of completion credit. It does not freeze
fighters or erase consequences: actual practice, wounds, equipment choices,
event memories and authored rival preparation can still change later bouts.
