# Authored war hound

The supplied `assets/characters/War_Hound.glb` is preserved unchanged. It contained
one static mesh, empty eye sockets, no skins and no animations. An exact source
copy and an imported Blender scene are under `assets/characters/source/`.
The source has UVs and a 1024-square base-color image, but every pixel in that
image is white. The concept colors were absent from the supplied export.

The editable result is `assets/characters/war-hound-rig.blend`; the game loads
`assets/characters/war-hound-rigged.glb`. The original collision asset is preserved.
The current companion rules use the existing ground-clearance graph rather than
claiming full skeletal collision or a physical animal ragdoll.

The 28-bone rig includes pelvis, spine/chest/neck/head, jaw, two tongue segments,
three tail segments, four three-segment legs, two ears and independent `Eye.L`
and `Eye.R` joints. Eye globes, amber irises, slit pupils and catchlights are fully
weighted to their own eye bones. The asymmetric sculpt's sockets are fitted
individually. Thin drool strands were removed; the tongue and teeth were retained.

Authored clips, in the fixed export order checked by the graphics test:

1. Idle
2. Walk
3. Run
4. Recall
5. Bite
6. Sniff
7. Down
8. LookAround

Blender uses Z up and front toward -Y. The export supplies Y up / +Z forward.
`HoundOrigin` scales the source by 0.25 and seats the paws at game height zero:
about 49 cm tall including the oversized ears, with a much lower shoulder height.
The physical profile uses a 34 cm eye height, 70 cm bite windup reach and 80 cm
resolution reach, with ground speeds of 0.65–2.8 m/s by order and distance.
The shared navigation graph remains conservative for a body this small.

Walk uses four-beat footfalls, Run a diagonal trot, and Recall a paired bound.
The animation is baked from two-bone IK, with stance targets anchored in the
floor plane, lifted swing paws, body weight shifts, spine flexion, head balance,
and ear/tongue/tail follow-through. Runtime cadence tracks actual travel distance.
Flat-floor baked stance error is below 9 mm in the sampled clips; there is no
per-paw terrain solver or full animal physics yet.

A new 2048-square color map is baked through the original UVs and embedded in the
GLB, with a separate `war-hound-basecolor.png` and packed Blender image. The palette
was reconstructed from the supplied concept: charcoal-purple flesh, ivory bone,
red membranes/tongue and a dark nose. This is a first material pass, not recovery
of a hidden original paint map or a finished hand-painted skin.
The runtime selects clips from actual companion motion, orders, bite windup and
health, with pause-aware timing and short pose transitions. This replaces the
articulated primitive proxy. Companion consequences remain governed by Theus.

## Verification and reproduction

```sh
blender --background --factory-startup --threads 4 --python tools/rig_war_hound.py
cargo test --locked --bin phalanx visuals::kennel::tests
cargo run --locked --bin phalanx -- --hound-witness
```

The Blender script validates every body vertex has normalized weights, moves a
leg bone and measures actual evaluated mesh deformation, and rotates one eye
while verifying the opposite eye stays still. It also evaluates baked stance
keyframes and measures ankle error against the authored floor targets. Numerical results are retained in
[the rig validation receipt](war-hound-rig-validation.json). A posed Blender
render is separate from an in-game render; local artifacts are under
`outputs/war-hound/`.

`--hound-witness` is an explicitly staged asset-pose review: it advances animation
clocks while retaining byte-identical Battle state. `--hound-demo` is an isolated
playable four-fighter fixture using real companion AI and combat. Both avoid the
player campaign save. Normal campaigns obtain a hound through the Houndwarden
class and the retained Pack Call skill; injury/recovery rules still apply.
