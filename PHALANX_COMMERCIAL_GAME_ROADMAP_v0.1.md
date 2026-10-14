# PHALANX — COMMERCIAL GAME ROADMAP v0.1

## Project Identity

**PHALANX** is an asynchronous competitive game about building a persistent fighting team, shaping its minds, equipping it, letting it learn through battle, and sending a snapshot of that team into other players' games.

The player does not directly command fighters during combat.

The player builds the Phalanx.

The Phalanx fights.

The long-term fantasy is:

> **Raise a small warband until its history, injuries, psychology, equipment, class synergies, and learned behavior make it unmistakably yours.**

The game should be:
- easy to understand,
- hard to master,
- highly replayable,
- deeply systemic,
- visually readable,
- commercially viable,
- technically compact enough to build quickly,
- and rich enough that veteran Phalanxes become legendary.

---

# 1. Core Law

> **Do not command the Phalanx. Build the minds and conditions from which the Phalanx emerges.**

Combat remains autonomous.

Player skill comes from:
- roster construction,
- class combinations,
- psychological shaping,
- equipment,
- crafting,
- team composition,
- progression choices,
- learning from defeats,
- and understanding how a Phalanx behaves under pressure.

---

# 2. Commercial Core Loop

```text
RECRUIT / BUILD TEAM
        ↓
SET MINDS
        ↓
CHOOSE CLASSES
        ↓
EQUIP / CRAFT
        ↓
ENTER LEAGUE
        ↓
CHALLENGE ANOTHER PHALANX SNAPSHOT
        ↓
PROCEDURAL PRESSURE MAP
        ↓
AUTONOMOUS BATTLE
        ↓
RESULTS + INJURIES + PSYCHOLOGICAL EFFECTS
        ↓
GOLD + EXPERIENCE + HISTORY
        ↓
ADAPT TEAM
        ↓
FIGHT AGAIN
```

The combat loop must remain readable enough that the player can understand what happened, why it happened, who failed, who adapted, what synergy worked, and what the team should change next.

---

# 3. Asynchronous Online Competition

PHALANX is online, but battles do not require both players to be present at the same time.

A player's active team is published as a **frozen challenge snapshot**.

Another player can challenge that snapshot locally.

The result is then sent back to the owner.

The challenged player's live team is not directly modified by a remote client.

Conceptually:

```text
PLAYER A
publishes Phalanx snapshot
        ↓
SERVER STORES SNAPSHOT
        ↓
PLAYER B selects challenge
        ↓
B fights local copy of A
        ↓
result packet produced
        ↓
server validates result
        ↓
A receives battle history / rewards / consequences
```

The system must eventually protect against forged results.

---

# 4. Persistent Phalanx

A Phalanx is a persistent roster.

Every fighter should preserve:
- identity,
- sex / body base,
- class,
- mind traits,
- equipment,
- battle record,
- injuries,
- psychological history,
- relationships,
- experience,
- learned tendencies,
- cosmetic state,
- and veteran status.

The team should become more individual over time, not less.

---

# 5. League Progression

Every player begins with:

```text
3 MEMBERS
```

Everyone starts in the **3-Phalanx League**.

Gold can be used to recruit an additional permanent member.

That moves the active Phalanx into the next league.

```text
3-member league
4-member league
5-member league
...
20-member league
```

Players are **not forced to increase team size**.

A player may remain in a preferred league indefinitely.

Every league should have:
- matchmaking,
- ladder ranking,
- records,
- rewards,
- prestige.

A player could become famous specifically as a 5-member or 8-member specialist.

---

# 6. Maximum Team Size

Initial maximum:

```text
20 MEMBERS
```

Twenty is large enough for formations, multiple roles, support networks, scouting, class synergies, redundancy, and multiple tactical fronts, while still small enough that individual fighters can remain emotionally meaningful.

If playtesting later proves that 20 damages readability or simulation quality, reduce it.

---

# 7. Procedural Pressure Arenas

Players do not edit arenas in the commercial game.

Arena editing remains useful in **PHALANX Lab** for controlled experiments.

The game instead generates battlefields procedurally for each matchup.

The arena generator has one purpose:

> **Apply pressure to both Phalanxes so their weaknesses are exposed and they are forced to adapt.**

It analyzes:
- team size,
- power level,
- class composition,
- ranged / melee balance,
- mobility,
- cohesion,
- tactical habits,
- strengths,
- repeated weaknesses,
- recent battle history,
- and possibly learned-policy tendencies.

Then it builds terrain that creates meaningful friction.

Core law:

> **Generate the battlefield most likely to reveal something the Phalanx still needs to learn.**

The generator should challenge a team without arbitrarily hard-countering it.

---

# 8. Map Scaling

Arena scale grows with league size.

Do not merely multiply dimensions.

Higher leagues should increase **tactical complexity**.

A 3-member arena might emphasize tight positioning, direct class synergy, and a small number of meaningful routes.

A 20-member arena may support multiple fronts, elevated positions, choke networks, flanking corridors, split engagements, scouting routes, and reserves.

---

# 9. Class System

PHALANX should eventually contain **many classes**.

The purpose is to create a deep combination space.

Individual classes should be easy to understand.

The complexity should emerge from interaction.

A class should have:
- a clear battlefield identity,
- a small number of strong mechanics,
- recognizable strengths,
- recognizable weaknesses,
- interactions with mind traits,
- interactions with equipment,
- interactions with other classes.

Examples only:

```text
Guardian
Berserker
Scout
Ranger
Rock Thrower
Healer
Banner Bearer
Hexer
Illusionist
Executioner
Duelist
Engineer
Summoner
Beastmaster
Alchemist
Ward Mage
Battle Mage
Trickster
Skirmisher
Juggernaut
```

The system must make adding new classes straightforward.

---

# 10. Synergy Principle

The game should be:

> **easy to play, hard to master**

A class alone should be understandable.

Mastery comes from:

```text
CLASS
×
MIND
×
EQUIPMENT
×
INJURY
×
PSYCHOLOGY
×
TEAM COMPOSITION
×
MAP PRESSURE
×
EXPERIENCE
```

The player should constantly discover strange combinations.

---

# 11. Mind System

The existing mind system remains fundamental.

Traits:

```text
AGGRESSION
CAUTION
COURAGE
TEAMWORK
PATIENCE
SELF-PRESERVATION
DISCIPLINE
```

Traits directly affect utility scoring, path choice, target choice, willingness to retreat, formation behavior, support behavior, aggression, and reaction to fear or injury.

The player shapes tendencies.

The simulation decides the moment-to-moment behavior.

---

# 12. Learned Behavior

PHALANX may use learned policies where they provide value.

The deterministic simulation remains authoritative.

Learned behavior can:
- imitate strong tactical patterns,
- adapt from battle telemetry,
- influence action preference,
- improve decision quality within validated action spaces.

It must not invent illegal actions or override world state.

---

# 13. Veteran Learning

Veterans should become frightening because they have experienced more situations.

Veteran value should come from:
- accumulated tactical history,
- refined responses,
- learned tendencies,
- better adaptation,
- psychological conditioning,
- established team relationships,
- familiarity with recurring pressures.

Old Phalanxes should feel like they have **seen things**.

---

# 14. Physical Injuries

There is no permanent death.

Fighters can suffer injuries.

Injuries should create history and adaptation rather than merely temporary inconvenience.

Possible effects:
- movement limitation,
- reduced reach,
- reduced perception,
- slower recovery,
- increased pain response,
- altered class effectiveness,
- changed tactical preferences.

Injuries may recover partially or fully.

Some can leave long-term scars or tradeoffs.

---

# 15. Psychological Damage

Avoid a generic sanity meter.

Fighters can develop specific reactions to events.

Possible triggers:
- isolation,
- encirclement,
- repeated defeat,
- severe injury,
- ally injury,
- failed charge,
- ranged suppression,
- specific classes,
- specific terrain,
- formation collapse,
- abandonment,
- humiliation,
- repeated survival against impossible odds.

Possible outcomes:
- fear,
- hesitation,
- over-aggression,
- increased caution,
- stronger attachment,
- reduced willingness to pursue,
- panic in specific contexts,
- increased confidence,
- obsessive tactical habits.

The same event should affect different personalities differently.

---

# 16. Relationships

Fighters can develop persistent bonds:
- trust,
- rivalry,
- loyalty,
- resentment,
- dependence,
- protective attachment.

Relationships can influence support decisions, regrouping, target priorities, willingness to retreat, psychological reaction to injury, combat barks, and cohesion.

Keep the first version narrow and combat-relevant.

---

# 17. Combat Bark System

Fighters communicate through short comic-style text bursts.

No voice acting is required initially.

Barks can represent commands, pain, fear, rage, warning, encouragement, tactical communication, rivalry, loyalty, panic, and victory.

Examples:

```text
CHAAARGE!
HOLD!
LEFT FLANK!
GET BEHIND ME!
I'M HIT!
FALL BACK!
WITH ME!
NOT AGAIN...
BREAK THEM!
```

Barks should be short, readable, event-driven, personality-aware, class-aware, and psychological-state-aware.

The bark system is not random flavor spam.

It is the visible emotional surface of the simulation.

---

# 18. Equipment

Every fighter can use:

```text
WEAPON
ARMOR
MAGIC ITEM
```

Equipment should create build choices rather than merely increasing numbers.

It can alter:
- physical stats,
- class mechanics,
- mind weighting,
- psychological thresholds,
- team synergy.

---

# 19. Crafting

Players can craft weapons, armor, and magic items.

Start simple:

```text
MATERIALS
+
BASE ITEM / RECIPE
+
MODIFIER
=
ITEM
```

The goal is build variety, not inventory bureaucracy.

---

# 20. Economy

Primary currency:

```text
GOLD
```

Gold can be used for:
- recruiting new members,
- crafting,
- equipment,
- recovery.

Avoid currency proliferation.

---

# 21. Progression

Progression has three layers.

## Player Progression
Unlocks more strategic and crafting possibilities.

## Phalanx Progression
Tracks league, ranking, reputation, battle history, and team achievements.

## Fighter Progression
Tracks experience, veteran status, injuries, psychological changes, class mastery, learned tendencies, and relationships.

Progression should create identity more than raw vertical power.

---

# 22. Matchmaking

Matchmaking should aim for **similar effective power**, not identical level.

Possible inputs:

```text
league size
equipment strength
class composition
historical win rate
veteran strength
learned-policy quality
team synergy
recent performance
```

The exact formula must be empirical.

---

# 23. Ranking and Ladders

Every league should support:
- ladder ranking,
- win/loss record,
- streaks,
- reputation,
- historical best.

A player should be able to specialize permanently in a league.

---

# 24. Challenge Flow

```text
SELECT OPPONENT
↓
VIEW PHALANX SUMMARY
↓
GENERATE PRESSURE ARENA
↓
OPTIONAL PRE-BATTLE REVIEW
↓
RUN BATTLE
↓
WATCH / INSPECT
↓
RESULTS
↓
LEARN / ADAPT
```

Avoid excessive lobby friction.

---

# 25. Visual Direction

Serious visual production is delayed until the system spine works, but PHALANX should not remain visually miserable for long.

## Early
- simple stylized bodies,
- readable silhouettes,
- basic class differentiation,
- minimal arena materials,
- comic text bursts.

## Mid
- chibi male/female base bodies,
- stronger class silhouettes,
- basic cosmetics,
- expressive animation,
- readable wounds,
- stronger VFX.

## Later
- polished whimsical ultra-violence,
- class-specific visual language,
- richer cosmetics,
- distinctive equipment,
- expressive injury states,
- polished arena themes.

Visual production must follow system needs.

---

# 26. Character Bases

Initial commercial character art:

```text
MALE BASE
FEMALE BASE
```

Both should share enough structural compatibility that equipment, armor, and animation can be reused efficiently.

Individuality comes from face, hair, skin, body details, class silhouette, armor, accessories, scars, veteran marks, color, and cosmetics.

---

# 27. Cosmetics

Possible cosmetic categories:
- hair,
- faces,
- armor skins,
- weapon skins,
- banners,
- victory poses,
- emotes,
- team colors,
- heraldry,
- visual effects.

Cosmetics should not determine combat power.

---

# 28. Monetization Principle

Do not sell combat superiority.

Prefer identity:
- cosmetics,
- visual customization,
- team presentation,
- banners,
- effects,
- skins.

The persistent attachment system makes cosmetic ownership valuable naturally.

---

# 29. PHALANX Lab vs PHALANX Game

## PHALANX Lab
Purpose:
- controlled experimentation,
- authored arenas,
- behavioral analysis,
- telemetry,
- utility / learned controller comparison,
- AI research,
- evaluation.

## PHALANX Game
Purpose:
- persistent teams,
- asynchronous competition,
- progression,
- classes,
- equipment,
- injuries,
- psychology,
- procedural pressure arenas,
- cosmetics,
- long-term player attachment.

Do not destroy the Lab to make the Game.

---

# 30. Development Roadmap

Each phase must produce something playable.

## PHASE A — Preserve the Lab Core

Preserve:
- autonomous combat,
- utility AI,
- learned controller,
- terrain-aware navigation,
- perception,
- memory,
- communication,
- telemetry,
- reproducible seeds,
- inspection,
- controller comparison.

Before expanding heavily:
- isolate persistent fighter data,
- isolate battle-state data,
- ensure battle results can be serialized.

**Success criterion:**  
A battle can be started from a complete serialized team snapshot and produce a complete serialized result.

---

## PHASE B — Persistent Fighters

Add:

```text
fighter_id
name
body base
class
mind traits
equipment
experience
battle count
injury state
psychological state
relationships
history summary
```

**Success criterion:**  
The same fighter can survive multiple battles and return changed.

---

## PHASE C — Phalanx Roster

Add:
- roster screen,
- 3 starting fighters,
- team identity,
- team record,
- selectable active roster,
- team save/load.

**Success criterion:**  
A player can own and manage a persistent 3-member Phalanx across repeated local battles.

---

## PHASE D — Classes

Create an extensible class architecture.

Start with enough classes to prove synergy:

```text
Warrior
Guardian
Ranger
Scout
Healer
Berserker
Hexer
Banner Bearer
```

Do not begin with 40 classes.

Make adding class 9 easy.

**Success criterion:**  
Different class combinations visibly produce different team behavior.

---

## PHASE E — Equipment + Simple Crafting

Add:
- weapon,
- armor,
- magic item.

Create a small crafting system.

**Success criterion:**  
Equipment creates meaningful build choices without overwhelming the player.

---

## PHASE F — Injury System

Start with a small number of injury families:

```text
leg
arm
vision
torso
pain
```

**Success criterion:**  
A wounded veteran fights differently after recovery than before the injury.

---

## PHASE G — Psychological Consequences

Start with:

```text
isolation fear
encirclement fear
ally-loss response
failed-charge response
confidence
```

Personality modulates outcomes.

**Success criterion:**  
Two fighters exposed to the same event can develop different long-term reactions.

---

## PHASE H — Combat Barks

Tie comic-style text bursts to:
- actions,
- perception,
- fear,
- injury,
- support,
- communication,
- relationships.

**Success criterion:**  
Watching combat communicates both tactical state and fighter personality without opening the inspector.

---

## PHASE I — Gold + Progression

Add:

```text
gold
battle rewards
recruit cost
basic recovery cost
crafting cost
```

Start every new player with 3 fighters.

Allow purchase of fighter #4.

**Success criterion:**  
A player can progress naturally from the 3-member league into the 4-member league.

---

## PHASE J — League System

Support:

```text
3 → 20 member leagues
```

Initially only a few adjacent leagues need to be playable.

**Success criterion:**  
Team size changes battle scale and matchmaking category without breaking persistence.

---

## PHASE K — Procedural Pressure Arenas

Inputs:
- team size,
- team power,
- composition,
- class balance,
- recent history,
- tactical habits.

Outputs:
- terrain,
- cover,
- elevation,
- routes,
- choke points,
- spawn geometry.

**Success criterion:**  
The generator produces tactical problems that expose team weaknesses without arbitrary counter-picking.

---

## PHASE L — Asynchronous Snapshots

Add local snapshot workflow first:

```text
export team snapshot
import enemy snapshot
fight
export result
```

Do this before networking.

**Success criterion:**  
Two separate game installations can exchange a Phalanx snapshot and reproduce a valid challenge locally.

---

## PHASE M — Online Challenges

Add server-backed:
- account identity,
- snapshot publishing,
- challenge discovery,
- result submission,
- validation,
- result delivery.

**Success criterion:**  
Player A can publish a team, go offline, and later receive a valid battle result from Player B.

---

## PHASE N — Matchmaking + Ladders

Add:
- effective-power rating,
- league ladder,
- rankings,
- opponent recommendations,
- anti-farming rules,
- basic exploit detection.

**Success criterion:**  
Players regularly receive opponents near enough in strength to create meaningful battles.

---

## PHASE O — Veteran Learning

Expand learned behavior beyond the current imitation baseline.

Possible directions:
- policy refinement,
- bounded tactical adaptation,
- per-fighter or per-team learned tendencies,
- offline training between battles.

Every learned update must be:
- versioned,
- validated,
- bounded,
- reversible.

**Success criterion:**  
Veteran teams demonstrably change behavior from experience without losing reproducibility or legality.

---

## PHASE P — Serious Visual Production Pass

Now invest heavily in art.

Build:
- final chibi proportion language,
- male/female base models,
- class silhouettes,
- armor system,
- weapons,
- cosmetic attachment points,
- animation set,
- blood / impact language,
- comic bark presentation.

**Success criterion:**  
A screenshot immediately communicates PHALANX's identity before the viewer knows the rules.

---

## PHASE Q — Content Expansion

Only after the core game loop works.

Expand:
- classes,
- items,
- crafting materials,
- injuries,
- psychological reactions,
- bark libraries,
- procedural arena grammars,
- cosmetics.

This phase can continue for years.

---

# 31. Immediate Build Order

For the next development sessions:

```text
1. persistent fighter schema
2. persistent Phalanx roster
3. snapshot serialization
4. class architecture
5. 3-member local battle mode
6. gold + fourth-member progression
7. injury persistence
8. psychological persistence
9. combat barks
10. procedural pressure-map prototype
```

Do not begin networking before local snapshot exchange works.

Do not build 40 classes before eight classes reveal whether the architecture is good.

---

# 32. First Commercial Prototype Target

The first genuinely game-like build should contain:

```text
3 persistent fighters
8 classes
editable minds
basic equipment
simple crafting
physical injuries
basic psychological consequences
comic combat barks
gold rewards
recruitable fourth fighter
3-member and 4-member leagues
procedural pressure arenas
local enemy snapshots
battle history
```

No full online backend is required yet.

If this local build is already addictive, the commercial idea is alive.

---

# 33. Design Test

The project is succeeding when a player says something like:

> "My Guardian developed a fear of being isolated after that horrible choke battle, so I paired her with a Banner Bearer and gave her a mobility item. Now she stays in formation and protects my Hexer."

That sentence contains:
- history,
- psychology,
- equipment,
- class synergy,
- adaptation,
- attachment.

That is PHALANX.

---

# 34. Final Commercial Identity

PHALANX should not feel like a conventional autobattler.

It should feel like raising a fighting organism.

The player builds a team that accumulates:

```text
skill
history
wounds
habits
fear
confidence
relationships
equipment
reputation
```

Then sends a copy of that organism into another player's world.

The strongest Phalanx is not merely the one with the biggest numbers.

It is the one that has been pressured, broken, repaired, trained, and adapted so many times that very little surprises it anymore.

> **Build them. Break them. Let them learn.**
