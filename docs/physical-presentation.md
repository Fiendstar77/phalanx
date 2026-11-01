# Fluid combat and physical death reactions

This pass improves the presentation of existing simulation results. It does not
grant new jumps, damage, collision advantages or legendary events.

## Motion

Both character rigs are rebaked at 60 samples per second. The grounded walk/run
cycles solve each two-bone leg to a stance/swing foot target. The solver uses the
actual imported joint directions rather than assuming that Blender's displayed
bone axis points at the next skinning joint. Ordinary cuts use a pelvis-led coil,
fast chest/weapon contact, planted feet and a longer recovery.

Runtime gait cadence follows actual travel speed. Gait changes preserve cycle
phase, transitions use eased weights, and hit reactions enter more quickly.
Facing interpolation uses elapsed time rather than a fixed fraction per rendered
frame. Melee windup and wall-vault poses follow the actual simulation timing.
The wall-vault clip no longer adds a second visual jump above the real trajectory.

Ordinary Theus strikes select grounded attacks. Aerial clips remain associated
with actual launch, vault and combination state; clip variety cannot stand in
for an extraordinary physical action.

## Blood detail

| Surface | Previous | This pass |
| --- | ---: | ---: |
| Ground stain canvas | 2048 × 2048 | 4096 × 4096 |
| Scenery/skin splash texture | 512 × 512 | 1024 × 1024 |
| Puddle simulation/mesh grid | 256 × 256 | 384 × 384 |
| Droplet mesh | 6 × 4 UV sphere | 12 × 8 UV sphere |

Ground splashes have continuous ragged boundaries and antialiased coverage
instead of pixel-coordinate checker noise. The 4K ground canvas uploads only
after a new stamp; flowing puddles update separately. Pools still conserve
deposited volume, respect blocked cells and pause with combat. This remains
stylized blood presentation, not a medical fluid simulation.

## Physics foundation

[Avian 0.7 supports Bevy 0.19](https://github.com/avianphysics/avian/tree/v0.7.0).
The optional graphics feature now includes its 3D rigid-body solver and joints;
headless battle execution retains its existing deterministic world rules.

Actual deaths hand the visible imported rig to up to 13 capsule/sphere bodies.
The pelvis, chest, head, upper/lower arms, thighs, shins and feet follow that
physical pose through the original skinning joints. Elbows and knees use hinge
constraints; the remaining connections use limited ball joints. Missing
unfitted limbs are excluded. Bodies from one corpse ignore each other to avoid
self-collision explosions; separate corpses and the terrain can collide.

The handover takes the current pose and direction of the recorded hit. Bodies
collide with a half-metre terrain sample mesh and wall volumes. Pause freezes
physics; a new run clears its bodies and constraints. A terrain change rebuilds
the collision surfaces while retaining existing corpses. Dead
bodies continue settling after the result screen opens.

This is an initial passive ragdoll, not a complete active-ragdoll controller.
It does not yet solve living balance, contact-aware foot placement on every
slope, grasping, weapon breakage, environmental destruction or gameplay falls.
The physics terrain approximates the authoritative height field; the current
bridge model is a top surface, not a fully volumetric underpass. Corpses are
visual physical bodies and do not become new tactical blockers. Any future
physics-driven gameplay consequence needs explicit simulation integration and
receipts before it can affect AI planning or legends.

## Verification

```bash
python3 tools/validate_character_assets.py
python3 tools/validate_face_assets.py
python3 tools/validate_motion.py
cargo test --locked --bin phalanx
```

The motion check reconstructs exported joint transforms and measures planted
ankle height and vault-local displacement. It detects floating feet and duplicate
visual jumps; it does not establish that the animation feels graceful. The
physics tests exercise actual gravity, terrain contact, joint connectivity,
pause and skin-pose writeback. Runtime captures and player testing are still
required to assess the result.

The subsequent [young king opening](young-king-opening.md) adds playable solo,
duo and trio chapters and a stylized adolescent profile on these same rigs.
It preserves the boundary between story transitions and real combat receipts.
