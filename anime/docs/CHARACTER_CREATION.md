# Character Creation (Offline-First)

This document outlines how we’ll build parametric, stylized characters using Asset DNA and a Vulkan NPR pipeline, targeting offline sprite export initially.

## Asset DNA

- ID and proportions: head/eye scales, limb lengths.
- Hair: style, strand count, stiffness/damping (for verlet chains).
- Clothes: type, fold count; optional material/region IDs.
- Palette: swatches for skin/hair/cloth; ramps for toon bands.
- Shading: band count, thresholds (face vs. cloth), rim settings.
- Lines: width, crease angle for outline pass.

Example: `examples/char_01.yml` (already included).

## Geometry Sources

- Meshes: riggable low-poly bodies/clothes; sphere/capsule primitives for tests.
- SDF elements (later): eyes/face accents and masks for clean curves and face shadows.

## Rigging & Animation

- Skeleton: simple hierarchical bones; JSON/YAML definition.
- Skinning: CPU-baked transforms for offline sequences initially.
- Procedural: idle breathing and walk cycle synthesis; verlet chains for hair.

## Stylization

- G-buffer: albedo, view-space normals, region/material IDs; optional depth/motion.
- Toon: 2–3 bands with per-region thresholds from a LUT; rim highlights.
- Outlines: mesh backface expansion + crease edges; composite over toon.
- Face maps: optional SDF mask to drive face shadow intent.

## Export

- Render sequences (poses/angles) → pack into sprite atlases (PNG/WebP).
- Emit JSON metadata: frames, anchors, animation definitions.

## Immediate Tasks

- Add region/material ID buffer and a small stylization LUT.
- Wire toon pass to use LUT per-pixel; expose thresholds via CLI.
- Implement outline pass and simple composite.
- Add a turntable + pose driver and export a 4×4 sprite atlas.

## Later Tasks

- Rig importer or procedural body generator.
- Hair clip collision and constraints for stability.
- Temporal reprojection for stable edges/bands.
- Editor UX for DNA presets and live parameter tweaking.

