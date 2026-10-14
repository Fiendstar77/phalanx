# PHALANX

## Terrain-Aware Autonomous Combat Simulation

### Project Status

**Portfolio prototype / experimental game / AI-behavior laboratory**

### Primary Stack

- **Rust**
- **Bevy**
- **Python** for telemetry analysis and experiments
- Optional local LLM integration only after deterministic combat intelligence works

---

# 1. Core Idea

**PHALANX** is a small autonomous-combat simulation about the relationship between:

```text
MIND
+
TERRAIN
+
TEAM COHESION
+
SITUATION
=
BEHAVIOR
```

The player does not directly command soldiers during battle.

Instead, the player:

- builds the battlefield,
- shapes terrain,
- creates two opposing teams,
- modifies the psychological traits of individual fighters,
- starts the battle,
- watches the units make their own tactical decisions,
- analyzes why they behaved as they did,
- changes the experiment,
- and runs it again.

The fundamental fantasy is not controlling an army.

It is **designing the conditions from which intelligent collective behavior emerges**.

---

# 2. Why the Name PHALANX

The name is inspired by the idea of highly disciplined, cohesive fighting groups such as the legendary Myrmidons associated with Achilles.

The important concept is not historical simulation.

It is:

```text
discipline
cohesion
mutual support
formation
adaptation
terrain use
collective intelligence
```

A group of ordinary fighters behaving together with extraordinary precision can become more dangerous than individually superior combatants acting independently.

That is the central theme of PHALANX.

---

# 3. Core Design Question

PHALANX asks:

> **How much can personality, tactical awareness, terrain, and team cohesion change the outcome of autonomous combat?**

The project should make this answer visible.

Changing an agent's mind should visibly change its behavior.

Changing the terrain should visibly change team strategy.

Changing the psychological composition of a team should visibly change how the entire unit fights.

---

# 4. Player Experience

The basic loop:

```text
BUILD ARENA
      ↓
CREATE TEAMS
      ↓
SHAPE MINDS
      ↓
RUN BATTLE
      ↓
WATCH AUTONOMOUS DECISIONS
      ↓
INSPECT BEHAVIOR
      ↓
CHANGE VARIABLES
      ↓
RUN AGAIN
```

The player should naturally experiment.

Examples:

> What happens if every fighter is highly aggressive?

> What happens if everyone is cautious but highly disciplined?

> Can a weaker team win by controlling high ground?

> Will selfish fighters abandon injured allies?

> Will patient fighters refuse to cross exposed terrain?

> What happens if one member is reckless while the rest are disciplined?

> Can a tightly coordinated team defeat individually stronger opponents?

> Can I design a terrain configuration that rewards one psychological profile over another?

That experimentation **is the game**.

---

# 5. Project Goal

The first public version should be:

- small,
- polished enough to understand immediately,
- visually readable,
- technically interesting,
- reproducible,
- measurable,
- fun to experiment with.

It should demonstrate competence in:

- game simulation,
- autonomous agents,
- behavioral systems,
- tactical reasoning,
- terrain navigation,
- Rust,
- Bevy,
- telemetry,
- experimentation,
- statistics,
- Python,
- interpretable AI,
- optional generative-AI integration.

---

# 6. Scope Law

PHALANX is deliberately small.

It is not:

- an RPG,
- a campaign game,
- a civilization simulator,
- an economy simulator,
- a narrative game,
- Zeitgeist,
- an RTS,
- a giant procedural world.

Do not add systems merely because they would be interesting.

The project exists to study:

> **terrain-aware autonomous team combat under different psychological configurations.**

Anything that does not strengthen that experiment should be treated suspiciously.

---

# 7. Core Battle Format

Initial target:

```text
TEAM A
5 fighters

vs.

TEAM B
5 fighters
```

Later:

```text
4–8 fighters per team
```

Small battles make behavior easier to observe and debug.

The project should prioritize behavioral clarity over spectacle.

---

# 8. Autonomous Combat

Once battle begins, the player does not issue movement or attack commands.

Every fighter chooses its own actions.

Possible actions include:

```text
ADVANCE
ATTACK
HOLD
RETREAT
SEEK COVER
TAKE HIGH GROUND
FLANK
REGROUP
SUPPORT ALLY
PURSUE
REPOSITION
DEFEND POSITION
```

Each action receives a utility score derived from:

- personality,
- health,
- terrain,
- nearby allies,
- nearby enemies,
- tactical danger,
- team condition,
- current objectives.

The best legal action is selected.

---

# 9. Mind System

Every fighter has adjustable mental traits.

Initial traits:

```text
AGGRESSION
CAUTION
COURAGE
TEAMWORK
PATIENCE
SELF-PRESERVATION
DISCIPLINE
CURIOSITY
```

Each value:

```text
0.0 → 1.0
```

These are not cosmetic statistics.

They directly modify tactical decisions.

---

# 10. Aggression

Aggression increases preference for:

- attacking,
- closing distance,
- pursuing,
- maintaining pressure,
- accepting tactical risk.

High aggression should not automatically equal intelligence.

A highly aggressive fighter with low caution may repeatedly choose dangerous approaches.

---

# 11. Caution

Caution increases sensitivity to:

- exposure,
- enemy numbers,
- dangerous paths,
- unfavorable terrain,
- low health,
- uncertain situations.

High caution encourages:

- cover,
- flanking,
- waiting,
- retreat,
- safer positioning.

---

# 12. Courage

Courage represents willingness to remain functional under threat.

Low courage may produce:

- retreat,
- hesitation,
- avoidance.

High courage enables:

- holding position,
- continued fighting under pressure,
- confronting stronger opponents.

Courage and aggression must remain distinct.

Example:

```text
HIGH AGGRESSION
LOW COURAGE
```

The fighter attacks enthusiastically until serious resistance appears.

Another:

```text
LOW AGGRESSION
HIGH COURAGE
```

The fighter may remain calm and hold position without charging recklessly.

---

# 13. Teamwork

Teamwork influences willingness to:

- remain near allies,
- support threatened teammates,
- coordinate attacks,
- regroup,
- wait for reinforcement,
- protect vulnerable fighters.

A low-teamwork agent may pursue personal tactical opportunities even when doing so harms the team.

---

# 14. Discipline

Discipline measures adherence to collective tactical behavior.

High discipline encourages:

- maintaining formation,
- respecting rally positions,
- remaining with the squad,
- following team objectives,
- resisting impulsive action.

Low discipline gives personality and immediate opportunity greater influence.

---

# 15. Patience

Patience affects willingness to wait for better conditions.

High patience favors:

- holding terrain,
- waiting for allies,
- delaying attack,
- ambush behavior,
- avoiding premature engagement.

Low patience favors immediate action.

---

# 16. Self-Preservation

Self-preservation influences concern for personal survival.

High values encourage:

- retreat when injured,
- avoiding overwhelming enemies,
- seeking safer routes,
- using cover,
- disengaging.

Low values allow extreme risk-taking.

---

# 17. Curiosity

Curiosity influences willingness to investigate unusual opportunities.

Later applications may include:

- unexplored terrain,
- unusual routes,
- unknown enemy positions,
- tactical experimentation.

It should remain secondary during the first milestone.

---

# 18. Emergent Psychological Profiles

Do not hard-code personality classes.

Profiles emerge from numeric combinations.

Examples:

## Berserker-like

```text
Aggression        0.95
Courage           0.90
Caution           0.10
Self-Preservation 0.15
Teamwork          0.35
```

Likely behavior:

- rapid engagement,
- pursuit,
- dangerous pushes,
- limited retreat.

---

## Cohesive Veteran

```text
Aggression        0.55
Courage           0.80
Caution           0.65
Teamwork          0.90
Discipline        0.95
Patience          0.75
```

Likely behavior:

- maintains group structure,
- exploits favorable terrain,
- waits for allies,
- commits decisively when conditions improve.

---

## Cowardly Opportunist

```text
Aggression        0.65
Courage           0.20
Caution           0.85
Self-Preservation 0.95
Teamwork          0.20
```

Likely behavior:

- attacks weak targets,
- retreats quickly from danger,
- abandons allies,
- avoids uncertain engagements.

---

# 19. Mind Editing

Before battle, the player can select any fighter.

Display:

```text
UNIT A-03

Aggression        [████████░░] 0.82
Caution           [███░░░░░░░] 0.31
Courage           [███████░░░] 0.74
Teamwork          [████████░░] 0.79
Patience          [██░░░░░░░░] 0.21
Self-Preservation [████░░░░░░] 0.43
Discipline        [██████░░░░] 0.63
```

Allow:

- slider editing,
- copying profiles,
- applying one profile to a team,
- randomizing traits,
- saving profiles.

---

# 20. Team Personality

The player should be able to create experiments such as:

```text
TEAM A
high aggression
low caution
medium teamwork
```

versus:

```text
TEAM B
medium aggression
high caution
high discipline
high teamwork
```

The resulting tactical difference should be visible without reading statistics.

---

# 21. Arena Builder

The player can modify the battlefield before battle.

Initial tools:

```text
RAISE
LOWER
FLATTEN
RIDGE
DEPRESSION
WALL
COVER
DIFFICULT TERRAIN
SPAWN ZONE
```

The editor should be simple.

The battlefield does not need full volumetric terrain.

The point is tactical geometry.

---

# 22. Terrain Must Matter

Terrain is not decoration.

Every terrain feature must affect decisions or movement.

---

# 23. Elevation

Elevation can affect:

- visibility,
- tactical advantage,
- ranged effectiveness,
- exposure,
- movement cost.

Agents should recognize high ground as potentially valuable.

---

# 24. Cover

Cover reduces exposure.

Agents with high caution or self-preservation should value it more strongly.

Aggressive fighters may ignore it.

---

# 25. Difficult Terrain

Examples:

- mud,
- rubble,
- dense vegetation.

Effects:

```text
reduced movement
+
increased path cost
```

---

# 26. Choke Points

Choke points create:

- concentration,
- congestion,
- defensive opportunity,
- risk.

Disciplined teams may exploit them more effectively.

Reckless teams may pile directly into them.

Perfect.

---

# 27. Open Ground

Open terrain provides:

- fast movement,
- clear visibility,
- little protection.

A cautious fighter should perceive open ground very differently from an aggressive one.

---

# 28. Navigation

Pathfinding should not merely seek minimum distance.

Agents evaluate:

```text
DISTANCE
+
MOVEMENT COST
+
EXPOSURE
+
ENEMY THREAT
+
COVER
+
ELEVATION
+
ALLY SUPPORT
+
TACTICAL OBJECTIVE
```

Conceptually:

```text
PATH COST =
    distance
  + terrain difficulty
  + exposure
  + enemy threat
  - cover value
  - elevation value
  - ally support
```

Weights vary according to personality.

Thus two fighters may choose different routes through the same battlefield.

---

# 29. Example

Two routes lead toward an enemy.

### Route A

```text
short
open
exposed
```

### Route B

```text
longer
protected by terrain
offers flanking position
```

Aggressive low-caution fighter:

```text
Route A
```

Patient cautious fighter:

```text
Route B
```

This is exactly the kind of visible behavioral difference PHALANX should produce.

---

# 30. Combat Model

Keep combat simple initially.

Each fighter has:

```text
health
damage
attack range
attack cooldown
movement speed
```

All fighters may initially use identical combat statistics.

This isolates the effect of psychological differences.

---

# 31. Tactical Perception

Each fighter needs awareness of nearby conditions.

Possible inputs:

```text
visible enemies
nearby allies
relative strength
health
cover
elevation
escape routes
current path
enemy concentration
team cohesion
```

These become the tactical state used by decision logic.

---

# 32. Tactical Maps

The simulation may derive local spatial fields such as:

```text
THREAT
COVER
ELEVATION
EXPOSURE
ALLY SUPPORT
TACTICAL VALUE
```

Agents use these fields during decisions.

The player should eventually be able to visualize them.

---

# 33. Utility-Based Decisions

Each possible action receives a score.

Example:

```text
ATTACK =
    aggression
  + courage
  + ally_support
  + enemy_vulnerability
  - danger
  - injury_penalty
```

Example:

```text
RETREAT =
    caution
  + self_preservation
  + enemy_superiority
  + injury
  - courage
```

Example:

```text
TAKE_HIGH_GROUND =
    tactical_value
  + caution
  + discipline
  + patience
  - travel_cost
```

The formulas should remain transparent and tunable.

---

# 34. Noise and Variation

Behavior should not become perfectly predictable.

Use controlled stochastic variation.

For example:

```text
final_score =
utility_score
+
small_random_component
```

The random component must be small enough that personality remains meaningful.

Every battle stores its random seed.

---

# 35. Reproducibility

The simulation should support:

```text
RUN AGAIN — NEW SEED
```

and:

```text
REPLAY — SAME SEED
```

Same seed + same parameters should reproduce the battle as closely as practical.

This is important for debugging and experimentation.

---

# 36. Agent Inspection

This is one of the most important portfolio features.

Click a fighter during battle.

Display:

```text
UNIT A-03

HEALTH
72%

CURRENT ACTION
TAKE HIGH GROUND

TARGET
Western ridge

MIND

Aggression        0.42
Caution           0.81
Courage           0.67
Teamwork          0.88
Patience          0.72
Self-Preservation 0.61
Discipline        0.91

TACTICAL CONTEXT

Enemies visible     3
Allies nearby       2
Threat              HIGH
Cover               AVAILABLE
High ground         NEARBY

DECISION SCORES

Attack              0.36
Retreat             0.28
Hold                0.54
Take High Ground    0.86
Flank               0.67
Regroup             0.43

SELECTED

TAKE HIGH GROUND
```

The intelligence should be observable.

---

# 37. Debug Overlays

Allow visualization of:

```text
PATHS
THREAT
COVER
ELEVATION
TACTICAL VALUE
CURRENT GOALS
TEAM COHESION
```

This transforms PHALANX from merely an autobattler into an interpretable behavioral laboratory.

---

# 38. Team Cohesion

A central metric should be:

```text
COHESION
```

Possible components:

- average distance between allies,
- formation integrity,
- isolated units,
- coordinated target selection,
- mutual support frequency.

This directly reinforces the PHALANX identity.

---

# 39. Collective Intelligence

The core research interest is whether individually simple agents can produce sophisticated group behavior.

Examples:

```text
one fighter sees danger
↓
moves toward cover
↓
high-teamwork allies remain close
↓
team naturally consolidates defensive position
```

Or:

```text
one aggressive fighter advances
↓
disciplined team refuses to follow
↓
fighter becomes isolated
↓
enemy destroys them
```

The system should allow these consequences to emerge rather than scripting them explicitly.

---

# 40. Team Information

Teams may eventually maintain shared tactical information:

```text
enemy positions
known threat zones
rally point
high-value terrain
team strength
```

High discipline/teamwork agents use shared information more strongly.

Low discipline agents rely more heavily on personal perception.

---

# 41. Battle Controls

Required:

```text
RUN
PAUSE
STEP
RESET
```

Simulation speeds:

```text
x1
x2
x4
x10
```

Later:

```text
MAX SPEED
```

for experiment batches.

---

# 42. Arena Presets

Ship with several examples.

## THE RIDGE

Central high ground dominates the map.

## THE CHOKE

Two large regions connected by a narrow passage.

## THE BASIN

Low central ground surrounded by elevation.

## OPEN FIELD

Minimal terrain.

Useful baseline.

## TWIN HILLS

Two competing elevated positions.

## THE CROSSING

Teams must traverse an exposed central route.

These should emphasize different psychological behaviors.

---

# 43. Arena Serialization

Custom arenas can be saved.

Possible formats:

```text
RON
```

or:

```text
JSON
```

Store:

```text
terrain
obstacles
spawn zones
arena settings
```

---

# 44. Personality Serialization

Save reusable mind profiles.

Example:

```text
berserkers.ron
disciplined_veterans.ron
cowardly_opportunists.ron
patient_defenders.ron
```

This also makes experiments reproducible.

---

# 45. Telemetry

Every battle should generate structured telemetry.

Prefer JSON initially.

Record:

```text
battle_id
seed
map
winner
duration
team profiles
agent profiles
survivors
damage
kills
movement distance
cover usage
high-ground usage
retreat events
flanking events
regroup events
decision counts
cohesion
```

Do not log meaningless quantities merely because logging them is easy.

---

# 46. Event Log

Important events should be recorded.

Example:

```text
00:07 A-03 chooses TAKE_HIGH_GROUND
00:09 B-01 begins FLANK
00:12 A-05 retreats below 30% health
00:15 Team A cohesion rises
00:22 B-02 isolated
00:25 B-02 eliminated
```

This helps debugging and demonstrates behavioral causality.

---

# 47. Python Analysis Layer

Python consumes battle telemetry.

Initial structure:

```text
analysis/
    loader.py
    metrics.py
    compare.py
    plots.py
```

Use Python to produce:

- descriptive statistics,
- controlled comparisons,
- charts,
- experiment summaries.

---

# 48. First Scientific Experiment

Question:

> **Does higher teamwork improve combat performance when terrain rewards coordinated movement?**

Set:

```text
TEAM A teamwork = low
TEAM B teamwork = high
```

Keep other personality values and combat statistics approximately equal.

Run repeated battles across:

```text
THE CHOKE
THE RIDGE
OPEN FIELD
```

Measure:

- win rate,
- survival rate,
- cohesion,
- ally distance,
- cover use,
- battle duration.

Do not claim significance unless actually tested.

---

# 49. Second Experiment

Question:

> **Does caution improve survival at the cost of offensive effectiveness?**

Compare:

```text
low caution
vs.
high caution
```

Measure:

```text
survival
damage
retreat frequency
cover use
battle duration
```

---

# 50. Third Experiment

Question:

> **Can terrain reverse the advantage between two psychological profiles?**

For example:

```text
aggressive team
vs.
disciplined team
```

Run on:

```text
OPEN FIELD
```

then:

```text
THE CHOKE
```

then:

```text
THE RIDGE
```

This directly demonstrates interaction between personality and environment.

---

# 51. Optional Local LLM Experiment

The deterministic system must work first.

Only then introduce a local LLM.

Do not make PHALANX depend on it.

---

# 52. LLM Purpose

The LLM receives high-level tactical decisions only when useful.

It does not control:

- movement,
- physics,
- pathfinding,
- damage,
- health,
- world state.

It may help choose among valid tactical options.

---

# 53. Selective Cognitive Activation

Most decisions remain deterministic.

Invoke the LLM when:

- multiple actions have similar scores,
- the battlefield situation is unusually complex,
- a fighter is trapped,
- the team suffers a major unexpected loss,
- several tactical opportunities compete.

This creates a miniature version of **dynamic cognitive resolution**.

---

# 54. Example LLM Input

```text
UNIT A-03

TRAITS
cautious
high teamwork
moderate aggression
high discipline

STATE
health: 62%

ALLIES
2 nearby

ENEMIES
3 visible

TERRAIN
north: exposed ground
east: elevated flank
south: partial cover
west: blocked

VALID ACTIONS
ATTACK
HOLD
FLANK_EAST
SEEK_COVER
RETREAT
```

Required output:

```text
ACTION: FLANK_EAST
CONFIDENCE: 0.82
```

Optional:

```text
REASON:
Eastern elevation offers positional advantage without crossing exposed ground.
```

---

# 55. Validation

The simulation verifies that the proposed action remains legal.

Architecture:

```text
WORLD STATE
      ↓
TACTICAL STATE
      ↓
LLM
      ↓
PROPOSED ACTION
      ↓
VALIDATOR
      ↓
SIMULATION
```

Core law:

> **The model proposes. PHALANX decides what reality permits.**

---

# 56. Controller Comparison

Eventually provide:

## Controller A

Simple heuristics.

## Controller B

Full deterministic utility AI.

## Controller C

Utility AI + selective small local LLM.

Then compare them using the same maps and seeds.

---

# 57. LLM Evaluation

Possible metrics:

```text
valid action rate
decision latency
behavioral diversity
combat outcome
tokens used
VRAM usage
LLM calls per battle
```

The project should investigate whether the generative layer actually creates value.

Do not assume that it does.

If deterministic AI performs better for a particular task, that result is valuable.

---

# 58. Visual Style

Keep the aesthetic deliberate but inexpensive.

Suggested direction:

- low-poly,
- strong silhouettes,
- abstract fighters,
- simple terrain materials,
- readable elevation,
- minimal effects.

Agents can initially be:

- capsules,
- stylized pawns,
- simple low-poly warriors.

PHALANX should look like an experimental tactical game, not an unfinished asset-store project.

---

# 59. Camera

Initial camera:

```text
3D tactical perspective
```

Allow:

- orbit,
- zoom,
- pan.

The entire arena should remain easy to inspect.

---

# 60. UI Layout

Possible arrangement:

```text
┌────────────────────────────────────────────┐
│                                            │
│                 ARENA                      │
│                                            │
│   Team A                  Team B           │
│                                            │
└────────────────────────────────────────────┘

SELECTED UNIT: A-03

Mind:
Aggression      0.72
Caution         0.31
Teamwork        0.89
Discipline      0.91

Current action:
REGROUP

Decision:
Attack       0.41
Hold         0.52
Regroup      0.84
Flank        0.61

[Run] [Pause] [Step] [Reset]

Overlay:
[Paths] [Threat] [Cover] [Decisions]
```

---

# 61. Repository Structure

Suggested:

```text
phalanx/
│
├── Cargo.toml
├── README.md
├── LICENSE
│
├── src/
│   ├── main.rs
│   │
│   ├── arena/
│   │   ├── mod.rs
│   │   ├── terrain.rs
│   │   └── builder.rs
│   │
│   ├── agents/
│   │   ├── mod.rs
│   │   ├── mind.rs
│   │   ├── perception.rs
│   │   └── state.rs
│   │
│   ├── cognition/
│   │   ├── mod.rs
│   │   ├── utility.rs
│   │   ├── actions.rs
│   │   └── scheduler.rs
│   │
│   ├── navigation/
│   │   ├── mod.rs
│   │   └── tactical_path.rs
│   │
│   ├── combat/
│   │   ├── mod.rs
│   │   └── resolution.rs
│   │
│   ├── telemetry/
│   │   ├── mod.rs
│   │   └── logger.rs
│   │
│   └── ui/
│       ├── mod.rs
│       ├── inspector.rs
│       └── overlays.rs
│
├── arenas/
├── personalities/
├── experiments/
│
├── analysis/
│   ├── requirements.txt
│   ├── loader.py
│   ├── metrics.py
│   ├── compare.py
│   └── plots.py
│
├── tests/
│
└── docs/
    ├── architecture.md
    ├── mind-system.md
    └── experiments.md
```

This structure is aspirational.

Do not create empty architecture merely to satisfy the diagram.

Add modules as implementation requires them.

---

# 62. README Opening

Suggested:

> # PHALANX
>
> **PHALANX** is an experimental autonomous-combat simulation built in Rust with Bevy.
>
> Players shape battlefields and psychological profiles rather than directly commanding fighters. Autonomous agents evaluate terrain, threat, allies, and individual mental traits to decide how to fight.
>
> The project explores terrain-aware navigation, interpretable agent decision-making, emergent team cohesion, behavioral simulation, automated experimentation, and optional selective local-LLM reasoning.

---

# 63. Portfolio Principle

Someone opening the repository should understand the project within roughly one minute.

They should immediately see evidence that the developer can:

```text
design simulation systems
build interactive software
reason about autonomous agents
work in Rust
use Python for evaluation
collect telemetry
design experiments
make AI behavior interpretable
think about generative AI without blindly using it everywhere
```

---

# 64. Milestone 1 — Make It Alive

**Do this first.**

Nothing else.

Required:

```text
small 3D arena
simple elevation
two teams
5 vs 5
basic navigation
basic combat
mind stats
utility-based decisions
Run
Pause
Reset
clickable unit inspection
```

Success criterion:

> Altering personality values visibly changes combat behavior.

No LLM.

No Python analytics.

No elaborate graphics.

No procedural world generator.

No grand architecture ceremony.

Make ten little idiots fight intelligently first.

---

# 65. Milestone 2 — Make Terrain Matter

Add:

```text
arena editing
terrain types
cover
high ground
difficult terrain
better tactical navigation
debug overlays
saved arenas
saved personalities
```

Success criterion:

> Altering terrain visibly changes tactical behavior.

---

# 66. Milestone 3 — Make Behavior Measurable

Add:

```text
telemetry
event logs
batch battles
seed control
Python analysis
behavior metrics
plots
controlled experiments
```

Success criterion:

> Claims about behavior can be supported by measured results.

---

# 67. Milestone 4 — Add Selective Generative Cognition

Only after the previous systems are stable:

```text
small local LLM
structured tactical input
valid action list
selective activation
output validation
controller comparison
model benchmarking
```

Success criterion:

> Determine experimentally whether selective generative reasoning improves any meaningful aspect of autonomous combat.

---

# 68. First Build Target

The first target should be brutally simple:

```text
flat or slightly sculpted arena

5 red agents
5 blue agents

all identical physically

different psychological traits

basic movement
basic attacks

visible selected-agent decision scores
```

That is enough to establish the heart of PHALANX.

---

# 69. First Demonstration Scenario

Create one ridge in the center.

Team A:

```text
high aggression
low caution
low patience
```

Team B:

```text
moderate aggression
high caution
high discipline
high teamwork
```

Expected hypothesis:

Team A tends toward rapid direct engagement.

Team B should show a stronger preference for advantageous positioning.

This is a **hypothesis**, not a promised outcome.

If the simulation does not produce the expected difference, adjust the behavioral model or investigate why.

Do not fake the result.

---

# 70. Core Engineering Rule

Every interesting behavior should be explainable.

If a fighter chooses an action, the simulation should be capable of answering:

```text
WHY?
```

Not with mystical AI language.

With actual state:

```text
High caution increased exposure penalty.

High teamwork increased regroup utility.

Nearby ridge increased positional utility.

Therefore TAKE_HIGH_GROUND scored highest.
```

Interpretability is part of the game.

---

# 71. Final Identity

PHALANX is not about making individual NPCs appear magically intelligent.

It is about asking whether simple autonomous minds, interacting with:

```text
terrain
danger
allies
personality
discipline
uncertainty
```

can produce convincing collective tactics.

The player designs the minds.

The player designs the battlefield.

Then the player relinquishes control.

The simulation decides what happens.

---

# 72. Final Law

> **Do not command the phalanx. Build the minds and terrain from which the phalanx emerges.**

PHALANX should ultimately make one phenomenon irresistible to watch:

> **individual psychology becoming collective behavior.**