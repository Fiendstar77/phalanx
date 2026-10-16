# Render-memory investigation

On October 15 the kernel killed PHALANX after about nine hours, with roughly
23 GiB of anonymous resident RAM. That established a real system-memory OOM;
it did not establish which allocation was responsible.

A paused Quarry session reproduced sustained retained-heap growth, with no
battle progression. gperftools heap snapshots attribute the dominant growth to
Khronos Vulkan validation allocations reached through wgpu-hal native object
naming: buffers, bind groups and texture views. This machine has Vulkan
validation layers 1.4.363.0-1. The archived samples summarize **live allocated
heap**, not total cumulative allocations and not GPU VRAM:

- Native labels: +41.73 MiB over 44.58 seconds, about 56.16 MiB/minute.
- Discarded native labels: +0.37 MiB over 40.01 seconds, about 0.55 MiB/minute.

See [the profile summary](results/v05/memory-profile.json). These are short local
samples and support a mitigation for the dominant growth path. They do not
certify overnight stability or prove every byte of the historical OOM came
from this path. RSS includes allocator-retained pages and driver mappings;
it must be measured separately from live heap.

## Runtime change

`presentation::player_instance_flags` adds wgpu's `DISCARD_HAL_LABELS` flag.
It preserves the other instance flags, including Vulkan validation in debug
builds; validation is not disabled to conceal rendering errors. The existing
surface initialization guard remains. Rust-side resource labels remain useful,
but native Vulkan diagnostics lose individual object names. Graphics developers
can restore them with `WGPU_DISCARD_HAL_LABELS=0` for a bounded reproduction.
User environment overrides continue to work.

There are [upstream reports of validation-layer memory retention](https://github.com/KhronosGroup/Vulkan-ValidationLayers/issues/9022),
but that older report is not proof of an identical upstream defect here.
The local A/B and allocation stacks are the evidence for this workaround.

## Reproduce

```sh
cargo build --locked --bin phalanx
python tools/memory_soak.py --seconds 300 --labels on --output outputs/memory-labels-on
python tools/memory_soak.py --seconds 300 --labels default --output outputs/memory-labels-default
```

The Linux sampler starts its own hidden, continuously rendered Quarry session
at normal presentation cadence, with a separate temporary campaign directory.
It writes `game.log`, `rss.jsonl` and `summary.json`; records whether the Vulkan
validation library actually loaded; and terminates only its own child. Default
warmup is 30 seconds and the default RSS stop limit is 6 GiB. A stopped test or
an early process exit is reported as such, not as a passing soak.

The short unprofiled comparison is recorded separately from the instrumented
heap test. Longer paused, battle/reset and menu sessions remain useful before
calling this long-session issue fully resolved.

[Unprofiled comparison results](results/v05/memory-soak.json) include both
validation-library observations and the RSS growth measurements after warmup.
