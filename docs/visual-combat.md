# Character and veteran combat slice

## Current models

The runtime now uses the artist's new rigged male and female bases with fitted
independent eyes. Originals remain in `assets/characters/phalanx_base_male.glb`
and `phalanx_female_base.glb`; edited `male-eyed.blend` / `female-eyed.blend`
and matching GLBs preserve the original rig and skin weights, adding eye bones.
`male.glb` / `female.glb` are the derived animated game exports. Older body
sources and experimental replacement heads/face parts have been deleted.

```sh
blender -b --python tools/fit_rigged_eyes.py
blender -b --python tools/rig_character_faces.py
blender -b --python tools/prepare_combat_characters.py
python tools/validate_character_assets.py
python tools/validate_face_assets.py
```

The preparation step normalizes centered bases to feet-at-zero coordinates
and bakes 52 clips in stable order. The first twenty-two are Idle, Walk, Run, Attack,
Throw, Hit and Death, plus AttackQuick, AttackHeavy, AttackDesperate,
ThrowUnderhand, HitHeavy, LimbRecoil, DeathBackward, DeathDelayed, NearDeathPanic,
WoundedStagger, WireLunge, WhirlwindCut, VaultStrike, WireRecoil and WireDeath.
Thirty further Theus clips are described below; the generated manifest records their names.
Both sexes retain their own artist-authored skin weights; no weight transfer
from the older male or decimation of the female is performed.

## Appearance and facial rig

`tools/rig_character_faces.py` separates each artist-authored head along existing
triangles, opens the existing lip seam and adds a mouth interior, teeth, tongue,
lip rim, ink-like brows and spherical eyelid shells. It does not replace the
head with primitive geometry. The original source GLBs and eyed Blends remain
unchanged. Editable sources are `male-facial-rig.blend` / `female-facial-rig.blend`.

Six live controls provide rage, asymmetric pain, fear, grin/laughter, shout and
blink. Eleven identity shapes provide visible nose length/width/upturn, jaw,
cheeks, round/tall silhouettes, sweet/pouting mouths and brow shape. The creator
offers 17 named faces, 17 named noses, separate cheeks/mouth/brows, 16 iris colours,
ten hair styles plus bald, skin and dye. Previous/next arrows, face/body framing
and portrait turns make the choices inspectable before class selection. Existing
saves default the three new cosmetic fields. Neutral mouths hide teeth and tongue
until an expression opens the mouth.
Eye bones carry the real eyeballs and curved irises. Faces and skin
materials update after body animation, before transform propagation; combat
clips contain no morph-weight channels that could overwrite those controls.
Damage, attack cooldown, retreat, low health, victory and barks drive the poses.
Time freezes on pause and advances when stepping. Expressions are presentation
only and do not consume the combat RNG.

Eight skin tones, six hair dyes plus automatic black/copper, sixteen iris colours,
ten volumetric hair styles and sculpted face/nose variations are available. Skin
and Dye buttons save by fighter identity, alongside existing appearance choices;
older saves default the new fields. The portrait photographs the actual fighter.
Warrior steel, Guardian blue steel, Ranger green, Scout dark leather, Healer
ivory, Berserker red, Hexer purple and Banner Bearer ochre distinguish the eight
core classes. Rock Thrower keeps earth tones and Paladin gold. Team colour is
retained as an accent; class weapons and heavy-class shoulder plates remain.

These are a first 3D interpretation of the supplied comic sheets, not a faithful
finished reconstruction. Wide mouth poses and the neck transition still need
sculpt/retopology polish; hair remains a chunky procedural starter asset.

WASD pans relative to camera orientation, Shift moves faster, Q/E orbits, RMB
orbits, MMB or Shift+RMB pans and the wheel zooms. Manual movement exits follow.
F follows the selected fighter, Home restores arena framing. Controls work while
paused. J now cycles filtered disagreements, freeing D for camera movement.

## Blood, limbs and prosthetics

Real mesh triangles supply detached arm, leg and head pieces. Nonfatal Founders wounds remove the affected organic section; the fighter keeps acting with the persistent penalties documented in [progression](progression.md). Fatal extra limb loss is presentation only and occurs for one fifth of seeded deaths, with at most one extra detached limb; ordinary deaths keep the head and remaining limbs. Flying pieces tumble under simple gravity and terrain collision; these are rigid visual pieces, not articulated ragdolls. Cutting uses authored position bands. Edges shared by retained and removed triangles are welded across position seams and closed with a flesh rim, muscle and bone-coloured center. Live caps retain joint indices and normalized skin weights; detached pieces receive rigid interior caps. This closes the authored cut instead of leaving a hollow shell.

Blood emits on every real damaging hit, independently of amputation. Deep hits and rare severing events also create pulsed jets. A separate artistic blood reservoir is eight times larger than the preceding pass; ordinary hit deposits are ten times larger. Droplets use substepped gravity and swept contacts against triangle meshes for walls, bridge decks/rails/supports, scenery, equipment and detached pieces. Moving skin uses animated bone-sphere contact proxies. Impacts create wet projected splashes and drip marks that follow the receiver and disappear with destroyed or hidden geometry. Ground impacts feed persistent stains and downhill pools. Wet flattened puddles expand and orient to the terrain slope. This remains bounded presentation physics, not a fluid solver. Lobes and loops suggest spilled organs/guts; this is stylized gore, not an anatomical organ simulation. There are at most **12,288 blood droplets, 512 surface stains and 128 detached pieces**. Nearby surface marks merge; the oldest marks are reused when the fixed pool fills. Resets clear presentation debris; effects do not change collision, pathfinding or utility scores.

Iron I prosthetics attach to the animated limb bones. The first crafting recipe, salvage flow and 3× damage / 15% movement tradeoff are real Founders mechanics. Higher tiers, crafting stations and the reserve roster are proposals.

## Arena pass

Earth/grass variation, shrubs inside authored cover, ruin stones inside existing blocked footprints and stone wall battlements improve the arena presets. Three additional large maps are Highlands (rolling high ground), Ruins (split barricade corridors), and Grand Arena (60×48, multiple choke lanes). Citadel retains its raised decks, ramps, railings and support pillars. Decorative solid rubble does not add hidden walkable-space obstacles. There is still one navigable elevation per horizontal position; true stacked underpasses remain unfinished.

## Evidence

The earlier facial-rig witness completed combat with the original seven clip states observed, blood active, detached limbs present and wounded units still alive. It used 40 loaded male/female animation players, with only the chosen body visible per unit. Run a separate QA copy of the save if testing Founders rewards:

```sh
cargo run --bin phalanx -- --lab --arena ridge --visual-witness
```

This mode runs offscreen, starts at 4× speed, saves the actual portrait to `outputs/portrait-witness.png`, prints a `VISUAL_WITNESS` JSON record and exits after completion or a 40-second limit. It reports observations; inspect the record rather than treating process exit alone as proof of every effect.

The standard Lab does not acquire persistent wounds or salvage, so its published matched-seed studies remain historical baseline evidence. The frozen learned policy has not been trained on the new Founders injury/equipment distribution.

## Barks and AI boundaries

Comic text bubbles react to damage, limb loss, retreat, attacks and healing, with class and personality-specific lines. At most four are visible; per-fighter cooldowns limit repetition. Their lifetime follows simulation ticks and pauses with combat. These lines are authored and event-driven, not LLM-generated. Optional future LLM flavor should use real event context, generate between battles, cache results and retain authored fallbacks; it must not determine combat truth.

## Sound

A local seeded composer now generates a sixteen-bar, 124 BPM action score in 4/4. Six equal-length parts provide a sparse plucked courtyard motif, a low ostinato and answering strings for approach, accented skin drums and metallic strikes for clashes, tense upper figures under pressure, and separate victory/defeat parts. Actual contact distance, health, and resolved results drive the crossfades. The score leaves space for combat foley; it remains synthesized prototype music that needs listening review. Twenty-seven synthesized effects cover swings, impacts, cutting, blood spray, slurps, blops, gurgles, splats, throws and blessing, plus male/female pain, fear, victory and screams. Voice choice follows the selected 3D body; barks continue to supply words. These are original synthetic placeholders, including vocal-like oscillators rather than recorded human screams. They need listening and mix/art direction; they are not production foley.

Effects follow real HP loss, wounds, cooldown changes and projectile hits. The mixer admits at most four new voices per frame and sixteen active effects. Effects pause with combat; music remains available in the menu. F8 toggles music, F9 toggles effects, and the menu exposes both controls. Preferences save locally in `outputs/audio-settings.json`. Visual witnesses stay silent unless `--audible-witness` is supplied.

`cargo run --bin phalanx -- --audio-witness --seed 42` exports the score and twenty-four effect WAV files to `outputs/audio-witness/` without starting graphics or altering the campaign.

## Class equipment

![Starter class equipment photographed in the game](images/starter-class-equipment.png)

Starter garments are extracted from each body's actual surface, expanded slightly and rendered with the same skin weights, inverse bind poses and joint entities. Tunics/chest armour and trousers/boots follow motion and remove the matching sections after limb loss. Leather bracers and belts add a shared silhouette; heavy classes have shoulder plates. Clothing color follows the phalanx color, and metallic chest materials distinguish Warrior, Guardian and Paladin. This is a starter fitted shell, not finished cloth topology or simulated fabric.

Weapons attach to animated hand bones and change immediately with class: Warrior sword/shield; Guardian mace/shield; Ranger bow; Scout two daggers; Healer staff; Berserker axe; Hexer orb staff; Banner Bearer pennant lance; Rock Thrower sling; Paladin hammer/shield. They visualize existing class mechanics and do not add hidden item bonuses, projectile physics or contact-perfect weapon collisions.

The user-provided [character reference sheets](art-direction/README.md) now define the art target. The 3D faces, hair and equipment are still starter interpretations. Facial poses now deform the actual new sculpts; the deleted prototype heads are not used.

## Animation validation

The asset validator checks changing animation samples, not just clip names.
The live witness also measures actual joint rotation separately for male and
female, and counts replayed gestures. Older screenshots and witness records
show earlier models and are historical evidence, not previews of the new bases.

## Combat motion variants

The additional clips are authored bone keyframes on the new male and female
rigs, not downloaded mocap or renamed copies of one motion. Editable action
libraries are saved as `assets/characters/male-combat.blend` and
`female-combat.blend`. Hand motion still lacks contact/foot IK and physical
weapon contact; these are stylized in-place gestures, not finished stunt work.

Class, discipline, courage, patience, health and attack ordinal select attack
variants without consuming battle RNG. Scouts favor quick strikes; heavy
classes favor wind-ups; wounded fighters can flail desperately. Throws have
overhand/underhand choices. Limb loss and large hits interrupt attacks, while
a simultaneous light hit allows the strike to remain visible. Repeated gestures
restart; death clips play once and hold. Front collapse, backward fall and a
delayed side topple vary deterministically with fighter identity and battle seed.
Simulation damage and its timing remain authoritative; visual wind-up is not
a new hit-frame or projectile-contact mechanic.

For isolated art review, set `PHALANX_ANIMATION=AttackHeavy` and
`PHALANX_ANIMATION_TIME=0.48` with `--visual-witness --face-witness`. This forces
a review pose, so it must not be reported as evidence of normal clip selection.

External options: [Mixamo](https://www.mixamo.com/) and the
[ActorCore free selection](https://actorcore.reallusion.com/3d-motion/free?motionTech=Mocap&saleType=Items).
This pass contains no assets downloaded from either service.

The local [combat-motion witness](results/v05/combat-motion-witness.json)
observed fourteen of fifteen clips during a completed battle, including both
sexes moving and all three death variants. HitHeavy was checked with a forced
review pose; that battle did not produce a hit above its 16% health threshold.

The RPG character-sheet preview reuses the actual body masking, armor, equipment, flag and fitted iron-limb renderers from saved member state. Unfitted missing limbs remain absent. Blood bursts now emit ten times the previous counts, within a 12,288-droplet pool, with larger splats/pools and wound trails every 0.1 simulated seconds. See [RPG workshop](rpg-workshop.md).

## Wire-fu presentation pass

Five new authored skeletal clips add rising lunges, full-turn cuts, aerial vaults, suspended recoils and a falling death. The shared guard now uses a two-bone solve to bring the forearms forward, turns the wrists into a weapon grip, splits the stance and curls the rigs' index/thumb chains into a grip. Weapon handles mount across the palm instead of using a world-upright rotation measured from one idle frame. A fighter with an unreplaced missing leg uses grounded attack variants. Bow drawing retains its separate two-hand solver.

A fixed pool of 384 instances supplies short blade-tip ribbons, impact sparks, landing rings and dust. Blade motion produces the ribbons, real damage events produce sparks, and the aerial clips' recovery phase produces landing dust. The existing blood treatment is retained. These arcs, turns and offsets are choreography; they do not add simulated jumps, knockback, hits or damage. This is a first art pass, with contact and foot placement still requiring review.

Cinema now starts for Founders squads of any supported size and resets for a new battle. Text entry no longer triggers camera shortcuts. **Cinema [V]** restores automatic framing after manual movement; **Home** restores the tactical field view. Contact shots frame the midpoint of a duel, leaving room for airborne motion. The battle toolbar is compact, and roster cards, floating HP labels and large tactical rings withdraw during automatic cinema. **Fighters** brings back the roster and the detailed controls.

For a disposable rendered review, make a fresh directory with an `assets` symlink to this repository, then run:

```sh
/path/to/Phalanx/target/debug/phalanx --visual-witness --wirefu-demo --seed 6
```

The fixture uses six actual fighters and real combat at normal speed. It writes 24 FPS PNGs and an audit under `outputs/wirefu-demo`, with observed camera shots, aerial clips, impacts, trails, landings and music-layer peaks. It disables campaign autosave. The fixture is an authored review setup, not a player playtest or a completed fight. `--audio-witness --seed 6` exports the new score and each individual part for listening.


## Theus choreography and receiver geometry

The current derived rigs contain 52 clips. Thirty additional clips cover idle
shifts, guarded and low gaits, sidesteps, parries, cartwheels, butterfly kicks,
corkscrew cuts, wall kicks, launchers, aerial pursuit and slams, recovery,
hit reactions and victory/defeat. Runtime transitions blend over 0.14 seconds.
Theus wind-ups start strikes before their resolution; actual defensive and
combination receipts select the corresponding reactions. Missing legs retain
grounded alternatives. These remain authored procedural animation, with contact
and foot placement still requiring further art work.

Automatic cinema follows successful combinations and tries clear views around
solid walls before lifting above dense scenery. Captions name the actual
participants. Weapon ribbons, parry sparks and combination rings use the
bounded presentation pool. The earlier wire-fu clips remain available for Lab
characters; campaign combat now uses the Theus statuses and opportunities
documented in [the shared rules](design/shared-rules.md).

The face pipeline welds the sculpt's seams and subdivides once for continuous
shape deformation: roughly 27,000 head vertices per body. All four eyelid
surfaces now face outward. The Pain warp is continuous, and eyelid poses use
one bounded closure instead of stacking several collapsing expressions.
Four closure stages keep interpolated lids outside the eyeballs. The
previous 15% head reduction is retained. Original body sources are preserved.

Blood retains its 12,288-droplet pool and rare cutting severs. Rigid receiver
stains now clip to the actual triangles and carry local UVs, with a 2 mm world
normal offset. This replaces depth projection on scenery and equipment, so a
camera change cannot stretch a mark onto unrelated geometry. Moving skin still
uses small, shallow projected stains. Blood sources follow Theus aerial height.
Receiver marks reuse 512 mesh slots; they do not allocate a new mesh per drop.
