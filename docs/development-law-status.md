# Development law: implementation status

The [Core Player-Fantasy and Development Law](../PHALANX_CORE_PLAYER_FANTASY_AND_DEVELOPMENT_LAW.md)
is the governing design document, preserved verbatim from the author. Its
long-term examples describe the intended experience. This page records the
current implementation boundary; neither the examples nor passing tests prove
years of depth or player attachment.

| Law | Working foundation | Work still required |
| --- | --- | --- |
| 1. Play is the real currency | Campaign gold, XP, skill practice, salvage and history are earned in play. There is no real-money store. | Any future store must sell cosmetic expression only. Earned gold must not become a purchased route to power or veteran history. |
| 2. Attachment to individuals | Stable identities, names, attributes, classes, practice, equipment, wounds, prosthetics, personality, event memories and battle history persist. Flesh wounds retain source-hit provenance, recovery and scars. The first witnessed legends and cosmetic epithets persist too. | Persistent personal relationships, richer confidence and mastery, and player tests of attachment. |
| 3. Predictable rules, unpredictable lives | Seeded combat, partial observations, eligibility checks and decision inspection expose why actions occur. | Better explanations of interrupted plans and more consequence combinations. |
| 4. Theus as foundation | Attributes, skills, opposed checks, defences, effort and critical outcomes constrain campaign combat. | A separately reusable shared framework and integration into other games; current movement geometry remains Phalanx-specific. |
| 5. Emergent extraordinary acts | A bounded search composes ground approach, wall vault and strike from perceived contacts and geometry. Completion requires physical execution and a real opposed hit. | The full Black Bridge chain, interception, ledge throw and collapse sequence. |
| 6. Opportunity Composition | Four-step, 32-expansion search; per-step execution checks; interruptions and completion receipts. | More primitives, alternative objectives, relationship pressure and broader risk reasoning. |
| 7. Requirements and costs | Training, equipment, limbs, effort, terrain clearance, landing and current attack eligibility constrain the first primitives. Normal strike consequences apply. | Weapon breakage, fall/collision injury and independent mixed outcomes such as successful rescue with self-injury. |
| 8. Relationships alter risk | Personality affects ordinary tactics and the planner's injury/caution cost. | Stable pairwise bonds, rivals, protection obligations and relationship-triggered risk changes. Teamwork alone is not a personal bond. |
| 9. History records moments | A landed vault plus a committed hit can produce a factual ledger entry, post-battle report and cosmetic Wallrunner epithet. | A richer history browser, event-specific names and more kinds of accomplishment. |
| 10. Legend is earned | A provisional detector considers completed wall assaults under starting injury, low health or an actual critical. It retains the first qualifying moment chronologically, without a damage ranking. All completed compositions remain in the battle receipts. | Detector calibration and multiple meaningful event families. The first heuristic does not establish that players consider these moments legendary. |
| 11. Telemetry supports story | Study traces retain decisions, plan steps, cancellations, world completions, vault endpoints/timing, opposed dice, damage, health, flesh wounds and separate source-linked bleeding receipts. Campaign settlement writes a ledger alongside learning reports. | Richer resource/skill context at each step, relationship context, calibrated risk, battle significance and reconstruction across archived battles. Missing context must remain unknown. |
| 12. Systems create future causes | Wounds affect movement/weapons, bleeding drains health/effort, healing stanches bleeding, and severe leg trauma can prevent vaults. Prosthetics trade power for mobility, practice changes skill, and event memories influence later tactics. Utility campaign fighters scout, assemble, detect a losing/stalled approach, withdraw with nearby screens/medics and try another lane. Decisions, reached positions and actual treatment have separate receipts. | More interactions with equipment, personal relationships and broader adaptive plans. Legendary titles currently carry meaning only. The bounded recovery loop is not a complete long-term strategy planner or proof of successful rescue. |
| 13. Depth emerges through play | The playable opening grows a solo adolescent into the same three-person founding group; the existing campaign then unlocks matched four-fighter play; inspection and reports expose deeper systems. | Usability and longitudinal playtests. The 10/50/200/1000-hour examples are design targets, not measured retention. |
| 14. Long-term player identity | Local snapshots carry source-reported veteran profiles, retained legends and epithets. Current-bout replay has explicit provenance limits. | Permanent queryable archives, larger histories and authenticating published historical claims. Profiles currently retain the latest 32 ledger entries plus a lifetime count; study files preserve exported receipts. |
| 15. Systems drive replayability | Terrain, traits, injuries, practice, equipment and perception interact across battles. | Evidence of sustained discovery and diversity beyond current test fixtures and rollout studies. |
| 16. Commercial principle | This remains an early playable prototype with inspectable AI work and reproducible evidence. | Release pricing, sustained depth and polish decisions evaluated against the stated priority order. |
| 17. Raising a fighting organism | Persistent squads change through actual battles; plans can succeed or be interrupted; the report shows consequences. | The complete relationship/history feedback loop and proof that players feel the intended attachment. |

## Outcome-independent progression

Match results are recorded as history and story/tutorial gates. They do not
reinforce power or personality. Both sides receive equal completion XP/class
practice, gold and salvage. Actual Theus practice and witnessed event memories
remain. Automatic temperament drift and result-based confidence are retired;
historical confidence has no decision effect. The policy is also recorded in
[AGENTS.md](../AGENTS.md) and checked by
[outcome-neutral settlement tests](../tests/outcome_neutral.rs).

## Evidence and next dependencies

The [Opportunity Composition guide](opportunity-composition.md) links the live
code, integration tests and reproducible staged demonstration. That fixture
grants training, remembered contact and an injured starting state, suppresses
the opponent's attacks and downs the other fighters. It proves a traversal and
opposed strike can execute and persist; it does not prove a natural rescue,
combat superiority or calibrated survival odds.

The [AI engineering guide](ai/README.md) distinguishes deployed PyTorch control,
campaign adaptation and separate native Bonsai studies. No model may invent a
successful event or waive a failed legality check. Combat barks retain their
20-word limit.

The [persistent-world and learning design](persistent-world-and-learning.md)
records the tabletop-to-close-up world direction and autonomous ambitions. The
new foundation adds ordered motion/effort frames and opposed-roll receipts to
studies, plus a separate causal PyTorch decision-sequence experiment. A persistent
world, live opponent modelling and online neural training are not yet implemented.

The next dependencies for the larger fantasy are persistent relationship
identities and consequences, richer surface/anchor/support geometry, and
step-level telemetry for new primitives. Each needs a real world effect and an
inspectable witness before narration can claim it happened.
