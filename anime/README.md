# Procedural Anime Assets (Skeleton)

This workspace sets up a minimal foundation for a procedural anime asset pipeline, following the shared plan. It includes:

- `stylize-core`: library with placeholders for DNA schema, rendering passes, rig, secondary motion, FX, and export modules.
- `stylize-cli`: simple CLI to load and inspect Asset DNA YAML files.
- `examples/char_01.yml`: example DNA matching the shared conversation.
- `assets/shaders/`: toon ramp fragment and outline vertex shader stubs.
  - Compiled at build time with `shaderc` into SPIR-V and embedded into the binary.
- `docs/procedural-anime-assets.txt`: offline copy of the prior discussion.

## Usage

- Inspect a DNA file:

```
cargo run -p stylize-cli -- inspect examples/char_01.yml
```

Note: Building will fetch crates from crates.io. If your environment blocks network, allow it or run where network is available.

### Vulkan NPR (feature-gated)

- Enumerate Vulkan devices:
  - `cargo run -p stylize-cli --features vulkan -- vk-info`
- Verify shader modules load:
  - `cargo run -p stylize-cli --features vulkan -- vk-test-shaders`
- Fullscreen toon test (procedural):
  - `cargo run -p stylize-cli --features vulkan -- vk-render-test --out out.png`
- Synthetic G-buffer (albedo + normal) PNGs:
  - `cargo run -p stylize-cli --features vulkan -- vk-gbuffer-test --width 512 --height 512 --out-prefix gbuf`
- Toon from synthetic G-buffer:
  - `cargo run -p stylize-cli --features vulkan -- vk-toon-from-gbuf --width 512 --height 512 --out toon.png`
- Mesh G-buffer (UV-sphere) PNGs:
  - `cargo run -p stylize-cli --features vulkan -- vk-gbuffer-mesh --width 512 --height 512 --out-prefix mesh`
- Toon from mesh G-buffer:
  - `cargo run -p stylize-cli --features vulkan -- vk-toon-mesh --width 512 --height 512 --out toon-mesh.png`

## Roadmap (matching the plan)

- G-buffer pass: Vulkan dynamic rendering for albedo/normal/depth (in progress).
- Toon pass: region-aware thresholds (LUT) and push constants (partial: procedural + sampled G-buffer).
- Outline pass: mesh backface expansion + crease edges (stubs in `assets/shaders/outline.vert`).
- Temporal stability: motion-vector reprojection of edges/bands (planned).
- Secondary motion: twin-tail hair chains via verlet using DNA parameters (planned).
- Sprite export: render frames → pack atlas + JSON metadata (planned).

## Structure

- `stylize-core/src/asset_dna/` — YAML schema + loaders.
- `stylize-core/src/render/` — stubs for `gbuffer`, `toon`, `outline`, `post`, `sprite_pack`.
- `stylize-core/src/rig/` — skeleton/IK (stubs).
- `stylize-core/src/secondary/` — hair/cloth-lite params (stubs).
- `stylize-core/src/fx/` — smears/accents (stubs).
- `stylize-core/src/export/` — atlas metadata (stubs).

## Next Steps

- Decide target: real-time 3D NPR or offline sprite sheets first.
- If Vulkan: continue NPR pipeline buildout.
  - Region/material IDs in G-buffer + stylization LUT.
  - Outline pass + composite with toon.
  - Motion vectors + temporal resolve.
  - Offscreen sprite export path and metadata.
- If sprite-first: render via an offscreen rasterizer or a small wgpu path, then implement `sprite_pack` + `export`.

Contributions welcome: extend schemas, add tests, wire render passes.
