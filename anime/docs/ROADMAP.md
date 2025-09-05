# Anime Engine Roadmap

## Current Status

- Vulkan feature-gated NPR path using Ash (1.3 dynamic rendering).
- Build-time GLSL → SPIR-V compilation via shaderc; embedded in binaries.
- Offscreen renders (no window):
  - Fullscreen toon test with push constants.
  - Synthetic G-buffer (albedo + normal), toon sampling pass from G-buffer.
  - Mesh path: UV-sphere → G-buffer (albedo + normal), toon sampling pass → PNG.
- Asset DNA schema + loader (YAML) for character parameters (proportions, hair, clothes, palette, shading, lines).

## Guiding Principles

- Offline-first: generate high-quality sprite sheets/flipbooks and metadata.
- Stylization-first: toon ramps, region-aware thresholds, outlines, temporal stability.
- Parametric assets: use compact DNA to vary character looks reproducibly.
- Procedural motion: idle, walk cycles; hair/cloth secondary motion with simple physics.

## Phase 1 — Solidify NPR Pipeline (Offline)

- G-buffer
  - Add region/material ID buffer (R8Uint) and optional depth.
  - Output view-space normals in R10G10B10A2 or RG16F for precision.
  - Add motion vectors (RG16F) groundwork (optional for offline).
- Toon shading
  - Sample albedo/normal/region; add a small LUT texture to map region → thresholds/bands.
  - Expose push constants or UBO to tweak thresholds/rim at runtime/CLI.
- Outlines
  - Implement backface expansion pass using `outline.vert`; composite with toon.
  - Optional depth/normal edge post-process for accents.
- Temporal coherence (optional offline)
  - Keep thickness stable in screen-space; seed stylization by IDs to avoid shimmer.

Deliverables

- CLI: `vk-toon-mesh --out toon-mesh.png` with toon + outlines + LUT.
- CLI: `vk-sprite-export` to render N frames around Y-axis and write atlas + JSON.

## Phase 2 — Character Creation Pipeline

- Asset DNA
  - Expand schema (hair styles, eye/face maps, cloth patterns, palette ramps).
  - Validators + defaults; presets for quick variations.
- Character rig
  - Skeleton definition (JSON/YAML) + simple skinning.
  - Pose import or simple param-driven idle/walk motions.
- Procedural secondary motion
  - Hair: verlet chains; tunable stiffness/damping; collision with head.
  - Cloth-lite: skirt folds oscillation + wind noise.
- Face/eye system
  - SDF-driven face shadow and highlights; eye spec/reflection ramps.
  - Region masks to drive toon thresholds (face vs. cloth differentiation).

Deliverables

- CLI: `char-build --dna examples/char_*.yml --out <folder>` → renders spritesheets for a set of poses/turntable angles.
- DNA presets and example assets.

## Phase 3 — Tooling and Export

- Sprite export
  - Compute-based atlas packing; or CPU packing fallback.
  - Write PNG/WebP atlas + JSON (frame rects, anchor points).
- Metadata
  - Animation lists (idle/walk/blink), frame timings, event tags (impact frames).
- Validation
  - CLI inspection, quick previews, diff across variants.

## Phase 4 — Game-Centered Aspects (Design Outline)

- Runtime data model
  - Entity components: Transform, SpriteAnimation, Collider, StylizationConfig.
  - Event bus: input, animation events, FX triggers.
- Collision & physics (2D-first)
  - Simple AABB/circle colliders; broadphase grid; events on contact.
- Animation graph
  - Blendspaces (idle-walk-run), state machine, parameterized transitions.
- FX
  - Procedural smears/speedlines triggered by animation events.
- Rendering integration
  - Start with 2D sprites; later experiment with real-time NPR 3D in-window.

## Phase 5 — Real-Time Windowed Preview (Later)

- Platform: winit + Vulkan surface; swapchain management.
- Real-time NPR path
  - G-buffer, toon, outline passes adapted to on-screen.
  - Camera controls; hot-reload tweakables (thresholds, rim, LUT).

## Milestones / Checklist

- [ ] Region/material ID buffer + LUT in toon pass.
- [ ] Outline pass (mesh backface expansion) + composite.
- [ ] UV-sphere sprite export demo (turntable) + JSON metadata.
- [ ] DNA-driven color palettes and shading thresholds.
- [ ] Verlet hair chains from DNA; bake into sprite sequences.
- [ ] Face SDF mask for stylized face shadows.
- [ ] CLI UX polish + docs.

