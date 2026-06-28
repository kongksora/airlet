# Airlet Roadmap

This document keeps the next infrastructure steps explicit so sound-design
iterations do not erase backend/API goals or the final 3D performance-app
direction.

## Current Baseline

- `Composition` contains notes and musical durations only.
- `ComposedScore` is created by binding a `Composition` to a `Tempo`.
- `Timeline` can expand absolute musical onsets and durations.
- The current best sound-design target is the `a-dry` modal model, ported from
  the Python probe into Rust and stored as a bundled JSON preset.
- The top-level app is now the 3D Bevy music-box performance app shell.

## 3D Exhibit Demo Roadmap

Airlet's final top-level executable is the 3D music-box performance app. Extra
operational capabilities should be exposed as crate APIs rather than new binary
entry points. The top-level `src/main.rs` is therefore the Bevy app surface; the
core crate remains responsible for score, audio, presets, and mechanism data.

### Entry Policy

- `src/main.rs` is the only top-level app entry for the product.
- Do not add separate `play_*`, `render_*`, or `export_*` bins for normal
  workflows.
- Existing helper bins should be removed or converted into crate functions when
  this app shell lands.
- `crates/airlet` must stay Bevy-free and remain a lightweight backend crate.
- Bevy, egui, camera control, model loading, and presentation state belong to
  the top-level app crate.

### Demo Acceptance Criteria

- [x] Top-level app uses Bevy `0.19`.
- [x] `bevy-egui` is enabled and visible as a control panel.
- [x] The downloaded music-box model is loaded from
  `assets/models/converted/music_box.glb`.
- [x] The music box is statically placed on a platform.
- [x] A spotlight illuminates the music box and points at the exhibit target.
- [x] The camera always looks at the music box.
- [x] Mouse orbit rotates around the target.
- [x] Mouse or UI controls adjust pitch/elevation within sensible limits.
- [x] The egui panel controls spotlight angle.
- [x] The egui panel controls performance volume.
- [x] The egui panel has start/stop performance controls.
- [x] Audio playback uses the existing `airlet` rendered default sound.
- [x] The app builds without adding additional binary entry points.

### Initial Implementation Shape

- Top-level `Cargo.toml` depends on:
  - `bevy = "0.19.0"`
  - `bevy_egui`
  - `airlet`
  - `rodio` for generated-buffer playback until Bevy-native generated audio is
    introduced.
- Top-level app modules may be organized under `src/` as normal Rust modules
  rather than separate bins.
- The first demo can display both imported open/closed model states if the GLB
  contains both; separation into named product states can happen after asset
  inspection.
- Later animation work should consume `airlet::mechanism` hints instead of
  hardcoding tooth/cylinder timing in Bevy.

### Screenshot-Driven Visual Debugging

The first Bevy demo appeared dark because the imported GLB scene was not being
instantiated through Bevy 0.19's `WorldAssetRoot(Scene0)` path, and the recorded
GLB bounds had the vertical/depth axes swapped. The GLB measured bounds are:

- min: `[-1.17, -1.13, -1.63]`
- max: `[0.83, -0.54, -0.79]`
- center: `[-0.17, -0.83, -1.21]`

Fix and validation checklist:

- [x] Recenter the model so the measured model center lands on the exhibit
  target.
- [x] Lower the model so the measured bottom rests on the platform top.
- [x] Increase ambient/fill/spot lighting enough for immediate visibility.
- [x] Add an environment-variable screenshot mode that saves a primary-window
  PNG and exits automatically.
- [x] Use `identify`/`magick` statistics on the screenshot to catch black or
  near-black frames.
- [x] Iterate camera, model placement, and lighting until the screenshot is
  visibly non-dark.

## Roadmap Status

- [x] Timeline-driven audio engine
- [x] Unified engine API
- [x] Performance and arrangement layer
- [x] Score DSL expansion
- [x] Mechanism hint export
- [x] Preset serialization
- [x] Golden checks

## Asset Segmentation Roadmap

Large 3D/modeling tasks must start by recording a roadmap before implementation.
The current goal is to turn the downloaded two-state music-box asset into a
usable parametric exhibit model: choose the closed model, center it, identify
the lid/hinge/body parts, and prepare for `t=0..1` lid animation.

### Current Scope

This first pass should not guess an animation rig in Rust before the asset has
been measured. It should produce evidence:

- [x] Probe the GLB with Python/uv and export per-node bounds, centers, sizes,
  and cluster assignments.
- [x] Decide which spatial cluster is the closed model and which is the open
  reference model.
- [x] Classify closed-model nodes into `body`, `lid`, `hinge`, `handle`,
  `mechanism`, or `unknown` using geometry heuristics.
- [x] Estimate a lid pivot axis and an open-angle target from the open-model
  reference.
- [x] Generate a human-readable classification report under `target/`.
- [x] Generate a debug render or image that colors the classified groups so the
  decision can be visually checked.

Current probe result:

- closed model: lower-height left spatial cluster, 37 meshes;
- open reference model: higher right spatial cluster, 38 meshes;
- closed lid: `Mesh`, already separate from the body mesh;
- open reference lid: `Mesh.037`;
- estimated lid pivot: `[-0.854749, -0.854495, -1.386203]`;
- estimated lid axis: `[1.0, 0.0, 0.0]`;
- generated files: `target/model-probe.json`, `target/model-probe.md`, and
  `target/model-probe-debug.png`.

### Follow-Up Scope

After the classification is validated visually:

- [ ] Add a model-paired `spec.toml` that records closed/open clusters, mesh
  roles, lid pivot, lid axis, and musical cylinder axis.
- [ ] Move closed-model rendering to grouped Bevy entities driven by the spec.
- [ ] Add `LidAnimation { t, pivot, axis, closed_angle, open_angle }`.
- [ ] Add parameterized musical cylinder rotation.
- [ ] Add egui sliders/toggles for lid `t` and cylinder rotation/playback.
- [ ] Validate screenshots at `t=0`, `t=0.5`, and `t=1`.
- [ ] If the lid/body mesh is fused, add an explicit preprocessing step instead
  of pretending Bevy transforms can separate it cleanly.

### Spec-Driven Rig Roadmap

The app should use the closed model as the runtime exhibit state and treat the
open model only as rigging reference data. Model-specific rigging belongs in a
model-adjacent `spec.toml`, so later downloaded or custom music-box models can
ship their own mesh roles and axes without requiring code changes.

Implementation checklist:

- [x] Create `assets/models/converted/spec.toml` for the current GLB.
- [x] Extend the Python probe to emit a spec draft with closed model roles,
  lid pivot/axis, open angle, and cylinder axis.
- [x] Add an independent `crates/airlet-model` crate that owns model spec
  loading, mesh grouping, movable-model state, and parametric rig poses.
- [x] Keep `crates/airlet-model` Bevy-free; the app converts its data into Bevy
  entities and transforms.
- [x] Expose APIs to redefine cylinder and comb modeling data for the future
  hint-to-model path.
- [x] Load the spec through `airlet-model` from the Bevy app.
- [x] Instantiate only the closed-model mesh primitives rather than the whole
  GLB scene.
- [x] Group lid meshes under a pivot entity and rotate them with `lid_t`.
- [x] Group musical cylinder meshes under an axis pivot and rotate them with a
  parameter.
- [x] Add egui controls for lid `t`, cylinder angle, and cylinder auto-spin.
- [x] Screenshot-validate visible closed, half-open, and open states.

### Basis-Aligned Rig Roadmap

The downloaded closed model is not aligned to world X/Y/Z. Rotating the lid or
cylinder around world axes is therefore wrong. The model pipeline must establish
a model-local basis first, align the closed model into Airlet's standard
coordinates, and then apply part rotations in that aligned space.

Implementation checklist:

- [x] Extend the Python probe to calculate a closed-model basis from PCA and
  emit `[basis]` into the generated spec.
- [x] Add `[basis]` to `assets/models/converted/spec.toml`.
- [x] Extend `airlet-model` so it parses the basis and exposes root alignment,
  local-to-rig point conversion, and local-to-rig axis conversion.
- [x] Update the Bevy adapter so the root transform aligns the closed model to
  standard coordinates before lid and cylinder rotations.
- [x] Recompute lid and cylinder pivots/axes through the basis instead of
  assuming the model is world-axis aligned.
- [x] Screenshot-validate `lid_t=0`, `lid_t=0.5`, and `lid_t=1` after alignment.

Current caveat:

- The closed model is basis-aligned and parameterized, but the lid pivot remains
  an inferred rear-lower-edge point. It opens in the correct direction; future
  polish should identify the hinge hardware meshes and move the pivot exactly
  onto that hinge line.

### Horizontal Frame Correction Roadmap

The first basis pass overused full 3D PCA. That can tilt the complete box and
break both lid and cylinder motion. The model should stay physically horizontal:
source world Y remains up, and the rig only applies yaw alignment around Y.

Implementation checklist:

- [x] Change probe basis estimation to a yaw-only horizontal frame:
  `up = [0, 1, 0]`, with front/right derived from the closed model's horizontal
  footprint.
- [x] Preserve a right-handed frame convention: right maps to +X, up maps to
  +Y, front maps to -Z.
- [x] Compute cylinder axis from the cylinder mesh geometry/PCA, not from the
  model basis.
- [x] Update `spec.toml` with horizontal basis and corrected cylinder axis.
- [x] Screenshot-validate that the box sits flat, lid opens around a plausible
  hinge, and cylinder rotation follows the cylinder length axis.

### Paired-Geometry Joint Fitting Roadmap

The model contains closed and open instances of the same music box. Once the
closed and open body meshes are paired, the open lid can be aligned into the
closed model frame and used to solve the lid joint geometrically. The target is
not a plausible hinge; it is a deterministic joint derived from paired geometry,
with only floating-point error in the fitted values.

Coordinate contract:

- Mesh indices, raw bounds, joint pivots, and joint axes in the model-adjacent
  `spec.toml` are recorded in GLB asset-local coordinates unless a field
  explicitly says otherwise.
- `[basis]` defines the asset-local to Airlet rig-space frame. The Bevy adapter
  currently applies it at the closed-model root, so child pivots must stay in
  asset-local coordinates.
- `airlet-model` should expose converted rig-space poses for future procedural
  modeling and hint-to-model paths instead of making app code infer the frame.

Implementation checklist:

- [x] Identify paired closed/open body meshes and compute the body alignment
  transform.
- [x] Align the open lid mesh into the closed model frame using the body
  transform.
- [x] Extract the open angle from the closed/open lid plane normals after body
  alignment.
- [x] Extract the revolute axis and pivot from the closed-model hinge hardware
  centerline.
- [x] Identify hinge hardware meshes and include the moving hinge side in the
  lid-following transform group.
- [x] Emit the fitted joint into `model-probe.json`, `model-probe.md`, and
  `model-spec.toml`.
- [x] Update `assets/models/converted/spec.toml` from the fitted values.
- [x] Validate screenshots until the hinge error is not visually noticeable.

### Hinge And Cylinder Axis Refinement

The first paired-geometry pass moved every detected hinge mesh with the lid and
used the cylinder AABB center as the rotation pivot. That is visibly wrong: the
lower hinge leaves should stay fixed on the body, and the cylinder pivot should
lie on the fitted cylinder centerline.

Implementation checklist:

- [x] Split hinge hardware into fixed lower leaves and lid-following upper
  leaves using closed-model geometry.
- [x] Keep fixed hinge leaves in the static body group.
- [x] Fit the cylinder pivot from the cylinder point cloud centroid projected
  onto the cylinder PCA axis, not from the AABB center.
- [x] Regenerate and apply `assets/models/converted/spec.toml`.
- [x] Screenshot-validate lid opening and cylinder rotation again.

### Hinge Pairing And Axle-Hole Axis Refinement

The upper/lower hinge split still has at least one misclassified small part, and
the cylinder still has a slight eccentric wobble. The next correction should use
paired open/closed geometry for hinge membership and the axle/hole geometry for
the cylinder axis.

Implementation checklist:

- [x] Classify each hinge submesh by comparing fixed-body alignment error
  against lid-motion alignment error.
- [x] Keep only submeshes with lower lid-motion error in the lid-following
  group.
- [x] Detect the cylinder axle/hole support meshes and derive the cylinder
  centerline from those coaxial small cylinders.
- [x] Regenerate and apply `assets/models/converted/spec.toml`.
- [x] Screenshot-validate open-lid hinges and cylinder 90/180 degree poses.

## API Polish Roadmap

This second pass turns the working backend into a cleaner crate surface for the
future 3D app, CLI tools, Python-driven sound probes, and mechanical exporters.

### Status

- [x] Rendered audio product type
- [x] Thin defaults layer
- [x] Preset library API
- [x] Legacy compatibility boundary
- [x] App and exporter migration
- [x] API-level tests

### 1. Rendered Audio Product Type

`Engine::render` should return a structured render product instead of a bare
`Vec<f32>`:

```rust
RenderedAudio {
    sample_rate,
    channels,
    samples,
}
```

The type should own common metadata and utility methods:

- `duration()`
- `peak()`
- `rms()`
- `into_samples()`
- `as_samples()`

This keeps playback, wav export, tests, and future UI code from each
recomputing duration and levels differently.

### 2. Thin Defaults Layer

The binary app should be a thin starter. Defaults should move into the crate:

```rust
defaults::air_intro_plan()
defaults::air_intro_audio(sample_rate)
```

The default plan is currently:

- `songs::air::intro_composition()`
- `songs::air::intro_tempo()`
- `ModelPreset::ADry`

### 3. Preset Library API

Bundled and filesystem presets should be accessed through explicit preset APIs:

```rust
PresetLibrary::bundled().load_model(ModelPreset::ADry)
PresetLibrary::load_model_from_path(path)
```

The bundled `a-dry` JSON remains the canonical current sound target. This gives
sound-design iteration a stable place to load edited preset files without
touching the synthesis code.

### 4. Legacy Compatibility Boundary

Old `NoteEvent`/`Score`/`Performance`/`render_events` paths should not remain
mixed into the main API surface without context. Keep them available for now,
but move them behind a clearly named `compat` module and re-export only what the
existing tests and demos still need.

The long-term direction is:

- composition goes through `score`;
- playback/rendering goes through `performance` + `engine`;
- old millisecond event streams are compatibility glue only.

### 5. App and Exporter Migration

`src/main.rs`, `render_air`, and tests should consume `RenderedAudio` and the
defaults/preset APIs where appropriate. The app should still play the current
default `a-dry` sound immediately after startup.

### 6. API-Level Tests

Add tests that lock the API contracts, not raw wav contents:

- `RenderedAudio` duration/level helpers;
- default plan renders with the current model;
- bundled preset library loads `a-dry`;
- legacy compatibility functions remain deterministic while they exist.

## Important Diagnosis: Direct Playback Timing

The direct playback bin in `src/main.rs` now runs:

```rust
let samples = Engine::new(sample_rate).render(&plan);
stream_handle.mixer().add(SamplesBuffer::new(..., samples));
```

That means direct playback uses the same timeline-driven backend as the wav
exporter. It no longer sleeps between individual notes, so the note onsets are
scheduled by absolute sample offsets during rendering.

Remaining timing caveat:

- playback start still depends on `rodio` device scheduling, but once the buffer
  starts, the internal musical timing is fixed in the rendered samples.

## Completed Infrastructure

### 1. Timeline-Driven Audio Engine

Both `legacy` and `a-dry` consume `Timeline` through `Engine`.

```rust
let composition = songs::air::intro_composition();
let performance = PerformancePlan::new(composition).tempo(songs::air::intro_tempo());
let audio = Engine::new(sample_rate).render(&performance);
```

Implemented behavior:

- use absolute `TimelineEvent.onset`;
- schedule notes at sample offsets, not with sleeps;
- preserve velocity and voice information;
- keep legacy conversion only as compatibility glue.

### 2. Unified Engine API

The app and wav exporter both use one backend surface:

```rust
Engine::new(sample_rate).render(&performance)
Engine::new(sample_rate).source(&performance)
```

### 3. Performance and Arrangement Layer

`PerformancePlan` sits between composition and engine:

```rust
PerformancePlan {
    composition,
    tempo,
    ornament_policy,
    velocity_policy,
    model_preset,
}
```

The policy fields are intentionally conservative placeholders; they give later
ornament, velocity, quantization, and humanization work a stable place to land.

### 4. Score DSL Expansion

The DSL now supports:

- `Dur::pattern([1, 1, 2])`
- tuplets over arbitrary total duration;
- `velocity(...)`;
- `tie(...)` and slur metadata;
- repeat helpers;
- more natural ornament helpers.

### 5. Mechanism Hint Export

A planner consumes `Timeline` and exports JSON:

```rust
ToothHint {
    midi_note,
    onset_tick,
    angle_rad,
    axial_position,
    protrusion,
    width,
}
```

Current planner assumptions:

- pitch maps to comb/tine track and axial position;
- onset maps to cylinder angle;
- velocity maps to protrusion/depth;
- short-distance repeated notes on the same track emit diagnostics;
- dense adjacent notes at the same angle emit spacing diagnostics.

### 6. Preset Serialization

The current `a-dry` model is stored at `crates/airlet/presets/a-dry.json`.
`MusicBoxModel` can load from JSON and serialize back to JSON, while the
hardcoded Rust constructor remains as a reference for now.

### 7. Golden Checks

The test suite avoids locking full wav contents, but checks:

- output duration;
- no NaN/inf;
- peak and RMS range;
- deterministic seeded render;
- timeline onset correctness.
