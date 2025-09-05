Anime-Style Toon Shading Recipe (Integrated)

This note distills a practical recipe for rich “anime” rendering while keeping physically grounded lighting cues, and documents what’s wired up in this repo.

Keep physics, then remap

- Signals: ndl = dot(N,L), ndv = dot(N,V), ndh = dot(N,H), plus albedo and normal from the G-buffer. Optional later: AO, roughness, material ID, bent normal/IBL.
- Philosophy: compute real lighting cues, then compress them into stylized ramps, hue shifts, and crisp highlights—so results read as clean cels without losing form.

Stylization components

- Diffuse ramp: 2–4 bands with soft edges to stabilize contours.
- Hue/saturation: slight cooler shadows and warmer lights via band-dependent hue/sat adjustments.
- Specular: thresholded “anime glint” from ndh for crisp highlights.
- Rim: view-based rim with width/strength controls.
- Lines: silhouettes + select creases (planned here; outline pass stubs exist).

What’s implemented now

- Shaders: `assets/shaders/toon.frag` and `assets/shaders/toon_gbuffer.frag`
  - Soft band edges via `bandSoftness`.
  - Band-dependent hue/saturation shifts.
  - Thresholded specular highlight using ndh.
  - Rim light with width and strength.
- Push constants: extended to 12 floats (48 bytes), plumbed in Vulkan pipelines.
- DNA schema: Shading block extended with optional style fields (defaults preserved).

Next targets

- Add AO/roughness/material ID to the G-buffer and use material-style LUTs.
- Outline pass completion (backface expansion + composite).
- Optional: low-order IBL and AO to enhance form before ramp.

