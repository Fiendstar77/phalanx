# Theus

**Theus** is Guillaume Heinerth's shared RPG framework for Phalanx, Zeitgeist,
and future games. Phalanx is its first playable integration. The name was chosen
October 26, 2026. These games are personal projects by Guillaume Heinerth,
outside Cognis 11 Inc.

The design draws from the broad ideas of skill checks and active defence in
GURPS and tactical combat with open rolls and critical consequences in
Rolemaster. Theus uses original code, terminology, formulas and effect rules.
It does not reproduce either game's tables, text, character options or setting.
It is not an official adaptation or a claim of affiliation. A working name
does not establish trademark clearance.

## Playable v1

The renderer-independent, serializable rules live in `src/theus.rs`.
`src/theus_combat.rs` applies them to Phalanx's perception, equipment, roles
and simultaneous damage. Persistent campaign battles, including tutorials,
rivals and local snapshot exhibitions, use Theus. The original Lab battles
retain their research baseline without Theus.

| Rule | Current implementation |
| --- | --- |
| Attributes | Might, Agility, Insight, Resolve; each 6–16 with a 48-point creation budget. Might affects impact power; Agility affects attacks, defence and acrobatics; Insight affects combination recovery through Coordination; Resolve affects defence and effort capacity. |
| Skills | Melee, Marksmanship, Acrobatics, Awareness, Coordination, Medicine. Six bounded practice tracks. Attacks, evasions, combinations and healing earn relevant practice; Awareness is reserved for the later perception pass. |
| Resolution | Seeded 3d6 against a skill rating; attack and defence margins oppose each other. Misses, dodges and parries cause no strike damage. |
| Effort | Endurance derives from Might and Resolve; attacks, defences, bursts and combinations spend it. It regenerates, with faster recovery while holding. Exhaustion lowers movement and attack/defence ratings. |
| Impacts | Margin affects quality, current equipment supplies armour, and an original bounded open percentile roll selects off-balance, guard-split or launch consequences. |
| Timing | Melee strikes require a two-tick wind-up, cancelled when contact, range or sight is lost. Existing physical projectile wind-ups, flight and interception remain authoritative. |
| Advancement | Skill practice persists at settlement and save/load. Existing class mastery, levels, injuries and equipment remain in place. |

The creator's class stage exposes attribute point allocation. Existing saves
without a Theus sheet derive a valid starting sheet from the saved temperament
when preparing a battle. They retain fighter identity, class, wounds and XP.
Recruitment and authored rival resets start fresh practice rather than copying
a veteran's earned ranks. The inspector and squad workshop show the sheet.

## Cooperative combat

Combinations require a living, willing partner who can directly see the same
target and is actively engaging it. Ally memories and positions behind walls
cannot create an opening. Effort and a Coordination-dependent recovery timer
limit repetition.

- **Aegis launch:** a Guardian or Paladin opens a target for a nearby melee ally.
- **Meteor relay:** an agile ally follows that partner's launch with an aerial finisher.
- **Pincer:** melee allies attacking from opposite sides exploit their angles.
- **Crossfire:** ranged partners follow a real projectile impact during its exposure window.

Nearby visible melee contacts also allow effort-limited burst steps and wall
kicks through the normal walkable navigation system. The separately equipped Wall dance art now permits actual checked wall traversal. An unfitted missing leg
prevents acrobatic mobility and the aerial relay. No combination grants sight
through walls or invents an extra fighter.

Theus events carry the attack/defence rolls, actual damage, technique and partner.
The renderer uses these receipts for poses, captions, sparks and automatic
camera focus. Airborne status creates a vulnerable target and a follow-up
opportunity. The visible arcs reach roughly four metres; world collision,
most legacy projectile interception remains planar. The later [ninjutsu pass](../ninjutsu.md) adds checked 3D wall traversal, smoke visibility and physical returning blades.

## Current boundaries

This is the first combat implementation, not a complete universal tabletop
rules book. Noncombat tasks, spell construction, social conflict, broad skill
lists, economic conversion and a universal injury model remain future work.
Zeitgeist has not been changed. The portable core has no Bevy dependency and is
the intended extraction point for that later integration.

The original design references are [GURPS Lite](https://www.sjgames.com/gurps/lite/3e/gurpslite.pdf)
and the [Rolemaster overview](https://ironcrown.com/rolemaster/).
The distinction between methods and protected expression is described by the
[US Copyright Office](https://www.copyright.gov/circs/circ31.pdf).

## Verification

`tests/theus.rs` exercises real battle steps: deterministic rolls and wind-ups,
a shield launch followed by an ally, effort use, blocked/dead partner rejection,
projectile crossfire, and old-save migration through settlement and reload.
The graphical creator tests exercise the actual attribute buttons and acceptance.
Run both `cargo test --offline --locked --no-default-features` and
`cargo test --offline --locked --bin phalanx`.

For a disposable render review, run the built binary from a separate directory
with an `assets` symlink:

```sh
WGPU_ADAPTER_NAME=AMD /path/to/Phalanx/target/debug/phalanx --visual-witness --wirefu-demo --gore-demo --seed 6
```

The fixture uses real 3v3 combat and disables campaign autosave. Its JSON audit
records observed combinations, defensive outcomes, animation choices, aerial
height, blood contacts and camera shots. It is a scripted fixture, not a human
playtest. `--creator-review --visual-witness` exercises creator acceptance and
attribute persistence; `--face-witness` allows an isolated rendered expression.
