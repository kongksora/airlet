# AAA Lighting Research For Airlet

Updated: 2026-06-28

This document summarizes common lighting and rendering features in modern
real-time projects that pursue high realism, then maps them to Airlet's music
box scene.

## Research Sources

- Unreal Engine Lumen documentation:
  https://dev.epicgames.com/documentation/unreal-engine/lumen-global-illumination-and-reflections-in-unreal-engine
- Unreal Engine real-time ray tracing documentation:
  https://dev.epicgames.com/documentation/unreal-engine/real-time-ray-tracing
- Unity HDRP Screen Space Global Illumination documentation:
  https://docs.unity3d.com/Packages/com.unity.render-pipelines.high-definition@14.0/manual/Override-Screen-Space-GI.html
- Unity HDRP lighting/environment update:
  https://unity.com/blog/lighting-and-environments-hdrp-updates-unity-6
- NVIDIA RTXGI example:
  https://developer.nvidia.com/blog/justice-adds-nvidia-rtx-global-illumination-expanding-its-roster-of-ray-traced-effects/
- Filament physically based rendering notes:
  https://google.github.io/filament/Filament.md.html

## What Realism-Focused Projects Usually Implement

### Physically Based Materials

Realistic lighting starts with plausible material response. A project normally
needs:

- base color / albedo that is not pre-lit;
- metallic and roughness channels;
- normal maps or measured normals;
- ambient occlusion or cavity maps for small creases;
- clear-coat or layered specular for lacquered surfaces;
- calibrated exposure and tone mapping so material values stay physically
  meaningful.

Airlet relevance:

- The music box has brass/gold metal, grey crank metal, lacquered wood/body,
  dark cavities, and possibly glass/translucent decorative parts.
- The immediate target should be material separation and roughness/metallic
  calibration before adding many post effects.

### Dynamic Global Illumination

Unreal Lumen is a useful reference point: it targets fully dynamic global
illumination and reflections, including diffuse color bleeding, indirect
shadowing, sky lighting, emissive contribution, and reflections across roughness
ranges. Unity HDRP documents a similar production concern through SSGI/RTGI:
screen-space diffuse bounces, ray-traced modes, mixed tracing, denoising, and
fallback behavior when rays miss.

Common implementation shapes:

- screen-space GI for near-field bounce and contact richness;
- ray-traced or probe-based GI for off-screen contribution;
- reflection/irradiance probes as fallback;
- update-speed controls because high-quality GI often caches results;
- denoising or temporal accumulation to stabilize low-sample lighting.

Airlet relevance:

- The music box is a compact object on a plinth. The high-payoff GI target is
  not a huge world solution; it is contact bounce between brass, wood, comb,
  cylinder, lid, and platform.
- A pragmatic phase can use reflection probes, environment lighting, baked or
  screen-space AO, and carefully placed area/fill lights before attempting
  ray-traced GI.

### Reflections

High-realism scenes usually combine multiple reflection tiers:

- screen-space reflections for cheap on-screen detail;
- reflection captures or image-based lighting for off-screen environment;
- ray-traced reflections for mirror/metal accuracy;
- roughness-aware reflection blur;
- clear-coat reflections for layered materials;
- special handling for translucent reflection/refraction.

Airlet relevance:

- The cylinder, comb, crank, pins, and latch need believable metal reflection.
- A high-quality cubemap/IBL plus per-material roughness will likely move the
  scene more than adding many small lights.
- Later, a ray/path-traced reference screenshot can be used as a visual target
  even if runtime stays rasterized.

### Shadows And Contact

Realistic projects invest heavily in shadows:

- soft area shadows whose penumbra changes with light size/distance;
- contact shadows under small parts;
- high-resolution or virtual shadow maps for detailed geometry;
- ray-traced shadows where budget allows;
- ambient occlusion for creases and self-shadowing;
- separate handling for translucent/subsurface shadowing.

Airlet relevance:

- The current object has many small features: pins, comb tines, crank shaft,
  hinge/latch pieces, and teeth. Contact shadow quality is crucial.
- The scene should prioritize crisp near-contact shadows and soft outer
  penumbra from a large key light.

### Image-Based Lighting

Filament's PBR notes emphasize image-based lighting: the environment around an
object can be encoded as lighting, often using cubemaps, irradiance data, and
prefiltered specular maps. In practice, AAA assets often rely on IBL for stable
realistic reflections and ambient fill.

Airlet relevance:

- Use an HDRI or generated studio environment as the main ambience.
- Add a tuned reflection environment that makes metal readable from all camera
  angles.
- Keep the visible background restrained; the music box should remain the hero.

### Volumetrics And Atmosphere

AAA lighting often includes:

- volumetric fog or participating media;
- light shafts and visible spotlight cones;
- atmospheric haze for depth;
- dust/sparkle particles in hero shots.

Airlet relevance:

- A subtle volumetric cone around the spotlight can make the exhibit feel
  premium, but it should not wash out the small mechanical details.
- The first implementation should use restrained, controllable atmosphere.

### Post Processing And Camera Response

Realism-focused projects usually tune the camera pipeline:

- HDR rendering and automatic or fixed exposure;
- filmic tone mapping;
- bloom for bright metal glints and emissive highlights;
- color grading / LUT;
- depth of field for close-up macro shots;
- motion blur only where it supports animation readability;
- temporal anti-aliasing / upscaling for stable subpixel geometry.

Airlet relevance:

- The music box is a macro/mechanical subject, so depth of field and glint
  control can contribute strongly.
- Motion blur should be conservative; cylinder pins and comb tines must remain
  inspectable.

### Reference/Validation Path

High-end projects commonly keep a ground-truth path:

- offline path tracing or high-sample ray tracing for reference frames;
- side-by-side runtime vs reference screenshots;
- buffer/debug views for albedo, normals, roughness, metalness, AO, exposure,
  and shadow maps;
- scalability tiers so cinematic quality and interactive quality are both
  testable.

Airlet relevance:

- The project should store screenshot evidence in `target/` and record outcomes
  in `docs/roadmap.md`.
- A future reference renderer can be separate from the Bevy runtime if needed,
  but material/light decisions should be validated visually.

## Airlet AAA Lighting Target

Target mood:

- premium tabletop product photography;
- warm key light on brass/wood;
- cool or neutral rim/fill to separate silhouette;
- clean dark-to-mid background, not a flat black void;
- physically plausible reflections on metal;
- strong but not crushed contact shadows.

Core feature stack:

1. PBR material audit for body, lid, cylinder, comb, crank, teeth, pins, latch,
   and platform.
2. HDR environment / IBL setup.
3. Three-point studio light rig: large soft key, fill, and rim/accent.
4. High-quality shadow tuning with contact emphasis.
5. Ambient occlusion/cavity pass for small mechanical creases.
6. Filmic tone mapping, exposure lock, subtle bloom, and color grade.
7. Optional volumetric spotlight cone.
8. Screenshot validation presets: full product view, crank close-up, comb
   close-up, lid-open view, and low-angle metal/reflection view.

## Implementation Phases

### Phase 1: Material And Lighting Audit

- Dump current material groups and colors from GLB/spec.
- Add a debug/material report for each mesh group.
- Define target roughness/metallic values for wood, brass, grey metal, and
  platform.
- Capture baseline screenshots before changes.

### Phase 2: Studio Lighting Rig

- Replace one-note lighting with named key/fill/rim lights.
- Use physically plausible intensities and color temperatures where Bevy allows.
- Add camera presets for product, comb, crank, and lid-open shots.
- Validate nonblack screenshots and inspect composition manually.

### Phase 3: Reflection And Environment

- Add HDRI/IBL or an equivalent environment-lighting strategy.
- Tune metal roughness and reflection readability.
- Add a dark but readable background and plinth interaction shadows.

### Phase 4: Contact Detail

- Tune shadow map sizes/biases and contact-shadow equivalents.
- Add AO/cavity where feasible.
- Validate pins, teeth, comb tines, hinge/latch, crank shaft, and lid seam.

### Phase 5: Camera And Post

- Set deterministic exposure.
- Add filmic tone/color curve and subtle bloom.
- Add optional macro depth of field for screenshots only if it does not hide
  functional mechanism detail.

### Phase 6: Reference And Regression

- Store named screenshot recipes.
- Compare screenshots by brightness and visual inspection.
- Keep a short `docs/roadmap.md` validation note per lighting batch.

## Non-Goals

- Do not make the scene a generic neon/cyberpunk demo.
- Do not hide geometry flaws with excessive bloom, haze, or depth of field.
- Do not make the background more visually important than the music box.
- Do not tune lighting only from one camera angle.
