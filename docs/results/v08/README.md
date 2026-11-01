# Physical presentation witness

The optional graphics build now uses Avian rigid bodies and joints for actual
deaths. The deterministic Theus battle remains authoritative.

`physics-witness.json` was captured on the AMD Vulkan backend with
`--visual-witness --opportunity-demo --seed 6`. This is the existing staged
opportunity fixture, not a natural-play balance study. It observed a finished
battle, 13 bodies, 12 joints, 507 skin-pose updates and zero invalid poses.
The four fighters already down at fixture initialization do not get a new death
handover. Ground blood resolution was 4096 and the pool grid was 384.

Local checks passed: focused graphics tests (61), the headless Rust suite,
strict headless Clippy, formatting and character/face/motion asset validation.
The exported grounded clips sampled at 60 Hz; the motion validator measured
planted-ankle errors below 0.000001 m in the baked stance samples.

A separate close-start `--wirefu-demo --gore-demo` capture was inspected for
blood, combat poses and settling bodies. Its first run hit a 60-second wall
timeout while still capturing frames; it is not a completed-battle witness.
Audio buffer underruns occurred during that recording. No frame-rate or audio
quality claim is made from the capture.

These checks establish implementation and bounded runtime behavior. They do
not establish graceful motion by player judgment, active balance, anatomical
blood flow or physical gameplay consequences. See the
[physical presentation boundary](../../physical-presentation.md).
