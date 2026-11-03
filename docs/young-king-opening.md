# Before the crown: playable opening

**New Game / Before the crown** opens a creator for one persistent 16-year-old
aspiring king. Edit the phalanx name and King name, seed, body and facial features;
then choose a class and Theus attributes. **Accept your King / begin the opening**
starts the story. Back to main menu pauses the draft; Continue resumes it. The
existing save remains available until acceptance. **Classic three-founder start**
retains the original three-person creator. Established
saves keep their rosters and earned history.

The opening has three playable encounters using ordinary autonomous Theus
combat. No victory, rescue, opposed roll or legendary event is scripted into
existence. Each chapter needs a recorded player victory before its next meeting
becomes available. A loss or draw permits another attempt with earned practice,
memories and permanent wounds retained. Normal between-battle recovery restores
HP; the workshop can fit a replacement for a missing limb.

| Chapter | Your roster | Transition after a witnessed win |
| --- | --- | --- |
| Before the crown | One aspiring king, age 16 | One winter passes; the future queen joins |
| A promise shared | Two people; protagonist age 17 | Another year passes; the future general joins |
| The third standard | Three people; protagonist age 18 | Royal offices are earned; the existing campaign systems open |

Use **Opening chapters** to read the current stage, then **Play this chapter**.
**Space** runs or pauses combat. After the report, use **Next chapter / tutorial**
or **Opening chapters > Next meeting / retry after battle**. The current chapter
and pending transition survive saving and loading; the live combat itself is
restarted from preparation on load, as in the existing campaign save system.
Names, appearance, equipment and temperament remain editable through Character.

The queen and general have stable identities while waiting offstage. Joining
receipts identify each person and the actual preceding victory. Existing
fighters retain their identities, equipment, learning, wounds and history as
roster size changes. These are story meetings and shared combat histories;
persistent pairwise bonds and relationship-driven rescue behavior remain future
work. The three short chapter introductions are the first playable foundation,
not a finished narrative campaign with dialogue or cinematic scenes.

Before the final victory, all participants have the Member office: no crown or
royal combat bonus. After the opening, the same veterans enter matched **3v3**,
then can unlock **4v4**. The five NPC tutorial teams remain five sequential
opponents. They are not these three opening chapters or a five-person league.

## Recruitment

**Character / Workshop > Recruit / slot 4: 250 gold** is available from the squad
screens and reports. The original main-menu purchase remains too. During the
opening, the button explains that companions join through the story. Afterward,
250 earned gold adds a fourth fighter and expands opponents equally. Leagues
5–20 have a published price ladder, but their gameplay remains planned content.
There is no purchase with real money.

## Age presentation

Saved appearance includes an explicit age; older saves default to 24. The
opening uses two explicit story time skips, not aging per battle tick. Late
adolescents use a slimmer regional torso profile and a different head profile;
the fitted garments use the same deformation and skeleton. Body height, hands
and joint locations are retained. Turning 18 restores the adult presentation.
This is a stylized adolescent treatment on the existing rig, not a separately
sculpted, anatomically validated age library or a general growth simulation.
Age adds no hidden combat modifier.

## Inspectable implementation

- `src/prologue.rs`: chapter state, waiting identities, victory and join receipts,
  roster transitions, explicit time skips and validation.
- `src/progression.rs`: solo/duo campaign support, settlement and save migration.
  Independent league snapshots remain restricted to three/four-person rosters.
- `src/front.rs`, `src/camp.rs`: new-game entry, chapter UI and saved transitions.
- `tests/prologue.rs`: actual seeded combat across all chapters, loss/retry,
  save round trips, persistent history, malformed state and legacy saves.

Reproduce a real combat journey and export per-battle study reports:

```bash
cargo run --locked --no-default-features --example prologue_journey
```

This writes chapter saves, decision studies, learning reports and
`opening-receipts.json` under `outputs/prologue-journey`; it uses a bounded retry
budget and stops if no victory occurs. It does not alter the player's save.
For an isolated interactive start, run:

```bash
cargo run --locked --bin phalanx -- --prologue-demo
```

Demo autosaving is disabled; manual saves use `outputs/prologue-demo-save.json`.
