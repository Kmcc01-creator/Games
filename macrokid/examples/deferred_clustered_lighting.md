# Deferred + Clustered Lighting (Design Doc)

This document outlines a practical approach to add a deferred, clustered (aka tiled/clustered) lighting path in macrokid. It covers render process, data specifications, descriptor layouts, resource formats, and engine‑level additions we should implement to support it.

## Goals

- Scale to thousands of dynamic lights with predictable per‑pixel work.
- Support variable/beam lights and shadows with optional volumetrics.
- Keep the design Vulkan‑friendly and aligned with macrokid’s `ResourceBindings`, `PipelineDesc`, and `RenderGraph` abstractions.

## High‑Level Pipeline

1) G‑Buffer Pass (graphics)
- Rasterize scene geometry into multiple render targets (MRT) and a depth buffer.
- Outputs: albedo/metal, normal/roughness, optional emissive/AO; depth.

2) Depth Pyramid (compute)
- Build a min (or min/max) depth mip chain for culling and estimating per‑tile z ranges.

3) Light Clustering (compute)
- Partition the view frustum into a 3D grid: tilesX × tilesY × slicesZ.
- Cull lights to clusters; produce per‑cluster light lists.

4) Lighting Resolve (compute or fullscreen graphics)
- For each pixel: recover G‑buffer attributes, find its cluster, iterate only that cluster’s lights, evaluate BRDF, and accumulate HDR color.

5) Post/Compose (graphics)
- Tonemap, bloom, UI, transparent pass, etc.

## G‑Buffer Specification

Suggested targets (formats are examples; adjust for hardware/quality):
- GB0 Albedo/Metal: `rgba8_srgb` (rgb = albedo, a = metalness)
- GB1 Normal/Roughness: `rgba16f` (xyz = view‑space unit normal, w = roughness)
- GB2 Emissive/AO (optional): `rg16f` (or write emissive in resolve)
- Depth: `D32_SFLOAT`

RenderGraph outputs (per pass):
- Mark GBn with `UsageMask::COLOR | SAMPLED`.
- Depth: `UsageMask::DEPTH | SAMPLED`.

Fragment outputs example:
```glsl
layout(location=0) out vec4 oGB0; // albedo.rgb (srgb), metal.a
layout(location=1) out vec4 oGB1; // normal.xyz, roughness.w
```

## Cluster Grid & Z‑Slicing

- Tile size: e.g., 16×16 pixels.
- tilesX = ceil(width / 16), tilesY = ceil(height / 16).
- slicesZ: logarithmic is common (e.g., 24–64 slices). Compute slice by mapping view‑space depth to log buckets.
- Use depth pyramid min/max per tile to narrow z slice range for that tile (reduces empty clusters).

Uniforms for clustering (std140 UBO):
- View/proj, invView/invProj, viewProj/invViewProj (as needed).
- Viewport size (uvec2), near/far (vec2).
- Tile size (uvec2), tilesX, tilesY, slicesZ (uvec3 + pad).
- Log slice parameters (vec4): {logScale, logBias, …}.
- Time/exposure/etc. as needed.

## Light Data

Prefer SSBO (std430) for a large, dynamic light array:
```glsl
struct Light {
    vec3 pos;      float range;       // world or view space
    vec3 dir;      float innerCos;    // cos(innerAngle)
    vec3 color;    float outerCos;    // cos(outerAngle)
    vec4 profile;                      // e.g., amp, freq, phase, type
    uint kind;     uint shadowIdx; uint flags; uint _pad;
};

layout(std430, set=1, binding=0) buffer Lights { Light lights[]; };
```

Sizing target: ~64 bytes per light. 10k lights ≈ 640 KB.

Shadow maps: prefer 2D array samplers for cascades/spots.

## Depth Pyramid (Compute)

Inputs/Outputs:
- Input depth: `sampler2D depth` (set=2,binding=0)
- Mip levels: storage images `image2D` (r32f) per mip or a single texture with bound per‑level views (set=2,binding=1…N)

Usage flags: `SAMPLED | STORAGE`.

Algorithm: 2×2 min reduction (or min/max). Dispatch one kernel per mip level, or one kernel that iterates levels.

## Light Clustering (Compute)

Buffers (SSBO, set=3):
- `ClusterCounts` (binding=0): `uint counts[clusters]` (zeroed each frame).
- `ClusterOffsets` (binding=1): `uint offsets[clusters+1]` (filled by prefix sum).
- `ClusterIndices` (binding=2): `uint indices[capacity]` (filled with light indices).
- Optional `Counter` (binding=3): `uint head;` for single‑pass atomic append.

Workflow:
1) Pass A: per‑cluster counts (cull light vs cluster frusta, using tile frusta + z range from depth pyramid).
2) Prefix sum (exclusive scan) on counts → offsets.
3) Pass B: write light indices into `indices[offset + localIdx]`.

Capacity planning: `capacity ≈ clusters * avgLightsPerCluster * safety` (start with 1.5× safety).

## Lighting Resolve

Inputs:
- G‑buffers (sampled), Depth (sampled).
- Lights SSBO, `ClusterOffsets`, `ClusterIndices` (read‑only).
- BRDF LUT (sampled 2D) and optional IBL (cube map or lat‑long 2D).

Outputs:
- HDR color as either MRT color target (`rgba16f`) in a graphics pass, or `image2D` in compute.

Inner loop (per pixel):
1) Fetch GB attributes (albedo, normal, roughness, metal).
2) Reconstruct view‑space position from depth.
3) Find cluster id → read range `[offsets[c], offsets[c+1])`.
4) Iterate lights in that range, evaluate BRDF (Lambert/Blinn/Phong/PBR), accumulate.
5) Write HDR color.

## Descriptor Sets (Example Layout)

We split per pass to keep each pipeline’s `ResourceBindings` small and explicit.

- set=0: Scene UBO and common samplers (used by most passes).
- set=1: Lights (SSBO and shadow maps).
- set=2: Depth + pyramid (compute write/read images for reduction).
- set=3: Cluster binning buffers (counts, offsets, indices, counters).
- set=4: Resolve inputs/outputs (G‑buffers, lights/lists reuse, HDR output if compute).

Concrete Rust skeletons (ResourceBindings):
```rust
use macrokid_graphics::resources::*;

pub struct GBufferRB; // graphics pass
impl ResourceBindings for GBufferRB {
    fn bindings() -> &'static [BindingDesc] {
        static B: [BindingDesc; 1] = [
            BindingDesc { field: "scene", set: 0, binding: 0, kind: ResourceKind::Uniform, stages: Some(BindingStages{vs:true, fs:true, cs:false}) },
        ];
        &B
    }
}

pub struct LightsRB; // common lights + shadows
impl ResourceBindings for LightsRB {
    fn bindings() -> &'static [BindingDesc] {
        static B: [BindingDesc; 2] = [
            BindingDesc { field: "lights", set: 1, binding: 0, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "shadow_maps", set: 1, binding: 1, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
        ];
        &B
    }
}

pub struct DepthPyramidRB; // compute
impl ResourceBindings for DepthPyramidRB {
    fn bindings() -> &'static [BindingDesc] {
        static B: [BindingDesc; 3] = [
            BindingDesc { field: "depth", set: 2, binding: 0, kind: ResourceKind::Texture, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
            BindingDesc { field: "mip0",  set: 2, binding: 1, kind: ResourceKind::StorageImage, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
            BindingDesc { field: "mip1",  set: 2, binding: 2, kind: ResourceKind::StorageImage, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
        ];
        &B
    }
}

pub struct ClusterBinRB; // compute
impl ResourceBindings for ClusterBinRB {
    fn bindings() -> &'static [BindingDesc] {
        static B: [BindingDesc; 4] = [
            BindingDesc { field: "counts",  set: 3, binding: 0, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
            BindingDesc { field: "offsets", set: 3, binding: 1, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
            BindingDesc { field: "indices", set: 3, binding: 2, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
            BindingDesc { field: "counter", set: 3, binding: 3, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
        ];
        &B
    }
}

pub struct ResolveRB; // compute or graphics
impl ResourceBindings for ResolveRB {
    fn bindings() -> &'static [BindingDesc] {
        static B: [BindingDesc; 8] = [
            BindingDesc { field: "gb0", set: 4, binding: 0, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "gb1", set: 4, binding: 1, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "depth", set: 4, binding: 2, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "lights", set: 4, binding: 3, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "offsets", set: 4, binding: 4, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "indices", set: 4, binding: 5, kind: ResourceKind::StorageBuffer, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "brdf_lut", set: 4, binding: 6, kind: ResourceKind::CombinedImageSampler, stages: Some(BindingStages{vs:false, fs:true, cs:true}) },
            BindingDesc { field: "out_hdr", set: 4, binding: 7, kind: ResourceKind::StorageImage, stages: Some(BindingStages{vs:false, fs:false, cs:true}) },
        ];
        &B
    }
}
```

## Engine Additions (macrokid crates)

Required `ResourceKind` extensions:
- Add `StorageBuffer`, `StorageImage` (and optionally `TexelBuffer`).
- Map in Vulkan bridge:
  - `StorageBuffer` → `vk::DescriptorType::STORAGE_BUFFER`
  - `StorageImage` → `vk::DescriptorType::STORAGE_IMAGE`

Compute support (follow‑up):
- Add `Compute` pipeline creation path parallel to graphics.
- Allow `PassKind::Compute` to bind `ResourceBindings` and dispatch.
- Shader loading: allow `.comp` or inline `inline.comp:` prefixes similar to VS/FS.

RenderGraph usage:
- Define pass descriptors with explicit outputs (G‑buffers, depth, HDR) and `UsageMask` flags to enable sampling and storage.
- Plan/reserve resources using existing `plan_resources` helpers; extend when compute pipelines are wired.

## Shadows and Volumetrics

- Shadows: optionally add a shadow depth pass per light type (spot/cascade) and sample in resolve. Use 2D array samplers for compact binding.
- Volumetric beams: render cone volumes in a separate additive graphics pass, or perform low‑step raymarch in a compute pass that writes into HDR (noise‑jittered, temporal).

## Performance Notes

- Use std430 for SSBOs; std140 for UBOs with padding.
- Pack angles as cosines; avoid acos.
- Use depth pyramid to shrink z‑slice ranges per tile → fewer cluster tests.
- Prefer two‑pass binning (count + scan + fill) for lower atomic contention and stability.
- Choose HDR `rgba16f` for resolve output; tone map in a later pass.

## Implementation Plan (Phased)

1) Extend Resource Kinds
- Add `StorageBuffer`, `StorageImage` (+ Vulkan mapping).

2) Define RBs and Formats
- Create per‑pass `ResourceBindings` types as above.
- Add example `PassDesc` with G‑buffer/HDR outputs and correct `UsageMask`.

3) Shaders
- G‑buffer VS/FS.
- Compute: depth pyramid, light bin (A + B), resolve.

4) Engine Support
- Wire compute pipelines (descriptor sets, shader stages, dispatch).
- Add inline `.comp` shader support similar to `inline.vert:` / `inline.frag:`.

5) Example
- An end‑to‑end `examples/clustered_deferred_demo.rs` showcasing the full path with a small light field and debug overlays.

## Risks / Open Items

- Descriptor heap sizing for large SSBOs and per‑frame allocations.
- Scan implementation: GPU (compute) vs CPU fallback for small grids.
- Transparents: forward clustered path or weighted blended OIT.
- Validation: add simple checks for buffer capacities (indices overflow).

---

This plan keeps changes isolated and incremental: start by extending resource kinds and authoring the RBs + shaders, then add compute pipeline wiring and a demo. From there, iterate on performance and features (shadows, volumetrics, debug views).

## Current Engine Support (Status Update)

- Resource kinds: `StorageBuffer` and `StorageImage` are implemented and mapped to Vulkan descriptor types.
- Compute pipelines:
  - `ComputeDesc { shader, dispatch, push_constants, bindings }` enables multiple ordered compute passes per frame.
  - Per‑compute descriptor set layouts are supported via `ComputeDesc.bindings` (derived from a `ResourceBindings` type).
  - The backend allocates per‑frame descriptor sets for each compute pass and writes demo resources (UBO, sampled, storage) to them.
  - Compute dispatches run before the graphics pass each frame, in the order added.
- Compute‑only present path:
  - When `BackendOptions.compute_only_present = true` and compute pipelines exist, no graphics render pass is started.
  - A storage‑capable HDR image is used as a compute output, then blitted to the swapchain for presentation.
- Descriptor pool sizing:
  - Pool sizing includes global + per‑compute bindings, multiplied per frame.
  - `BackendOptions.desc_pool_multiplier` (or `MK_DESC_POOL_MULTIPLIER`) oversizes the pool to reduce rebuild pressure.

### About Dynamic Descriptor Pool Resizing

- Not yet implemented. The current pool and allocation strategy assumes fixed layouts known at initialization time.
- If allocations fail, increase `desc_pool_multiplier` (e.g., 2 or 4) or reduce per‑compute bindings.
- Planned approach for dynamic resizing (future):
  - Detect allocation failures; re‑create a larger pool with `FREE_DESCRIPTOR_SET` and re‑allocate per‑frame sets for global + compute.
  - Reapply descriptor writes from cached inputs (or via an application callback) after rebuild.
  - Ensure synchronization by rebuilding when the device is idle between frames.

### Developer Notes

- For production code, replace demo storage buffers/images with real resources and adjust descriptor writes accordingly.
- Prefer std430 for SSBOs and GENERAL layout for storage images before compute writes; the backend transitions demo storage images to GENERAL.
- Compute push constants are supported per pass; if multiple compute passes need different sizes, each has its own pipeline layout.
