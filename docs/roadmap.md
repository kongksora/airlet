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
- [x] The egui panel has product controls: `Full Wind`,
  `Pause`/`Continue`, and `Reset`.
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

- Current architecture and validation status is summarized in
  `docs/architecture-code-quality-audit.md`.
- Schema-driven debug/MCP behavior is documented in `docs/mcp.md`.
- AAA lighting research and implementation direction are documented in
  `docs/aaa-lighting-research.md`.

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

This early follow-up scope has been resolved by the later spec-driven rig,
twin-state, and winding-control sections.

- [x] Add a model-paired `spec.toml` that records mesh roles and rig axes.
- [x] Move closed-model rendering to grouped Bevy entities driven by the spec.
- [x] Add parameterized lid motion through `lid_t`.
- [x] Add parameterized musical cylinder rotation driven by twin mechanical
  phase.
- [x] Replace early lid/cylinder playback sliders with product controls:
  `Full Wind`, `Pause/Continue`, and `Reset`.
- [x] Validate visual rig states through runtime/debug screenshots recorded in
  later roadmap sections.
- [x] Confirm the lid/body mesh is separable for the current model; no fused-mesh
  preprocessing is needed for this asset.

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

### Hint-Driven Mechanism And Playback Sync

The music-box performance should not render a decorative cylinder independently
from the audio. `MechanismLayoutHint` is the shared contract: a tooth's angular
position and the playback clock must be derived from the same timeline tick, so
the visual strike and generated sound onset are phase-locked.

Implementation checklist:

- [x] Replace the tan platform with a pure white circular display plinth.
- [x] Reduce ambient/fill lighting and raise the visual role of the controllable
  spotlight.
- [x] Generate a procedural cylinder tooth mesh from `MechanismLayoutHint`.
- [x] Generate a procedural comb/tooth-line model from the same note tracks.
- [x] Use one playback clock to drive both audio playback state and cylinder
  angle.
- [x] Map `onset_tick` to cylinder angle with the same ticks-per-turn convention
  used by `MechanismPlanner`.
- [x] Validate that starting playback resets cylinder phase, so tooth contact
  and sound onset are aligned by construction.
- [x] Screenshot-validate the updated stage and mechanism model.

### Geometry-Sized Procedural Mechanism

The first hint-driven demo used temporary app constants for procedural cylinder
length and radius. That breaks visual scale and tooth axial distribution. The
procedural mechanism must inherit dimensions from the fitted model geometry and
keep app code out of model-specific sizing.

Implementation checklist:

- [x] Measure cylinder radius and length from the model geometry probe.
- [x] Store fitted cylinder dimensions in `assets/models/converted/spec.toml`.
- [x] Expose cylinder radius/length through `airlet-model`.
- [x] Render the procedural cylinder, teeth, and comb using spec dimensions
  instead of app constants.
- [x] Screenshot-validate the corrected cylinder scale and axial distribution.
- [x] Decide whether a lightweight debug action surface is enough, or whether
  Airlet should grow a dedicated MCP server for live inspection/control.

Decision: build a lightweight app/debug action surface first, then wrap it with
MCP only after the state and action vocabulary is stable. The immediate useful
actions are screenshot capture, camera/light controls, lid/cylinder controls,
winding-first playback controls, and structured mechanism-state dumps.

### Airlet Debug Action Surface And MCP Path

MCP should be an adapter over Airlet's own debug/action vocabulary, not the first
place where app state semantics are invented. The first implementation should be
a local-only JSON action endpoint that can be driven by scripts and later wrapped
by an MCP server.

Implementation checklist:

- [x] Add a local debug endpoint bound to `127.0.0.1`.
- [x] Support the first JSON action vocabulary. This has since been superseded
  by the schema-driven `describe_actions` catalog below.
- [x] Route debug actions through the same `ExhibitControls`,
  `AudioOutputState`, twin state, and screenshot resources used by egui.
- [x] Return structured state including playback phase, cylinder angle, model
  cylinder dimensions, and mechanism hint counts.
- [x] Add a lightweight client/smoke path before building a dedicated MCP
  server.
- [x] Keep the endpoint opt-out/opt-in safe for local development and document
  the future MCP adapter boundary.

### MCP, Comb Calibration, And Material Pass

The local debug endpoint is now stable enough to wrap with MCP. The next pass
must also correct the physical mechanism layout: the comb track range, cylinder
tooth axial positions, and audio onset ticks must share one mapping so a visual
tooth reaches the matching comb tine exactly when the audio note starts.

Implementation checklist:

- [x] Add a full MCP server that exposes the Airlet debug actions as tools.
- [x] Keep MCP as a separate adapter process over the JSON debug endpoint.
- [x] Calibrate comb note range from the actual `MechanismLayoutHint` events
  and fit it within the model cylinder length.
- [x] Remap tooth axial positions and comb tine positions through the same
  note-to-track function.
- [x] Add structured mechanism debug output with note range, track count, track
  spacing, cylinder dimensions, and preview tooth positions.
- [x] Keep the demo scene explicitly on a white circular platform.
- [x] Add suitable procedural metal materials for the procedural cylinder,
  teeth, and comb.
- [x] Validate MCP tools, screenshot output, and mechanism geometry.
- [x] Record what extra PBR assets are needed for full-model materialization and
  which assets can be generated locally.

Validation notes:

- MCP adapter: `uv run --project py airlet-mcp` imports through
  `mcp.server.fastmcp.FastMCP`; `dump_mechanism` was smoke-tested through the
  tool wrapper against a running app.
- Current score range: MIDI `66..86`, `21` comb tracks.
- Current model cylinder: length `0.16361199`, calibrated usable length
  `0.14070632`, track spacing `0.00703532`.
- There are no remaining `outside comb range` diagnostics. Existing mechanism
  diagnostics are same-angle dense-tooth hints caused by simultaneous notes.
- Screenshot validation:
  `target/airlet-debug-action-shot.png`, `1280x800`, mean brightness
  `0.326768`.

Full-model material assets:

- Can be generated immediately: procedural metal parameters for cylinder,
  teeth, comb, screws, and hinge hardware; generated brushed-metal normal and
  roughness maps; simple lacquered-color base materials; generated AO helper
  maps for procedural parts.
- Should be downloaded or authored for high-quality final rendering: wood or
  lacquered-box PBR texture set, brass/gold PBR texture set if procedural metal
  is not enough, velvet/felt liner texture if the open box interior is exposed,
  engraved logo/name/date artwork, and an HDRI studio environment for
  reflections.
- Preferred texture formats: PNG or TGA for base color, normal, roughness,
  metallic, AO, and height maps; 2K is enough for the demo, 4K is preferable for
  close-up gift renders. For glTF export, pack ORM as occlusion/roughness/
  metallic channels when possible. HDRI should be `.hdr` or `.exr`.

### Cylinder Time Mapping Correction

The first hint-driven cylinder pass used `PPQ * 4` as `ticks_per_turn`, which
folded every measure onto the same cylinder angle. That made different playback
times appear as simultaneous teeth. The current demo is a short, single-turn
mechanism: one full score timeline maps to one cylinder revolution.

Implementation checklist:

- [x] Derive `ticks_per_turn` from the full timeline end tick plus the
  mechanism tail rest.
- [x] Use the derived value in both `MechanismPlanner` and playback phase sync.
- [x] Add a validation report for same-onset groups and same-phase groups.
- [x] Require zero same-phase collisions when the score has no same-onset
  chords.
- [x] Expose the timing validation in `dump_mechanism`.
- [x] Test the default Air intro has no folded phase collisions.
- [x] Validate with the running app through the debug endpoint.

Validation result:

- score end tick: `29760`, equal to `last_onset + last_duration`.
- `ticks_per_turn`: `33600`, equal to score end tick plus a 2.0s mechanism
  tail rest.
- `same_onset_group_count`: `0`.
- `same_phase_group_count`: `0`.
- mechanism diagnostics: `0`.
- Runtime endpoint validation file:
  `target/airlet-time-mapping-debug.json`.

### Comb Probe, Clearance, And Contact Animation Roadmap

The procedural comb currently uses a derived range from cylinder length and
places tines by visual approximation. That is not sufficient: the source model
already contains a cylinder and comb with a visible gap, and the procedural
mechanism must inherit those dimensions rather than collapsing the comb onto
the cylinder.

Implementation checklist:

- [x] Extend the model probe to identify the source comb mesh/bounds and
  cylinder-to-comb clearance.
- [x] Recompute the real cylinder radius from cylinder geometry and axle
  constraints.
- [x] Store comb meshes, axial range, radial direction, tip radius, and
  clearance in `assets/models/converted/spec.toml`.
- [x] Extend `airlet-model` spec parsing for the measured comb geometry.
- [x] Drive procedural comb track range from measured comb bounds instead of
  `cylinder_length * 0.86`.
- [x] Place comb tines so their tips preserve the measured
  cylinder/comb clearance.
- [x] Expose clearance and comb range in `dump_mechanism` for validation.
- [x] Screenshot-validate visible cylinder/comb gap and corrected tooth range.
- [x] Draft the next-stage animation model for tooth pluck, tine deflection,
  and damped vibration.
- [x] Draft the render roadmap for high-shadow-quality and cinematic lighting.

Probe result:

- Source cylinder body mesh: `Mesh.027`, fitted radius `0.049188`, length
  `0.127228`.
- Source comb mesh: `Mesh.023`.
- Comb axial range: `-0.05439..0.057542`.
- Comb radial direction: `[-0.863441, 0.077203, 0.498507]`.
- Comb tip radius: `0.051817`; measured cylinder/comb clearance: `0.002629`.
- Runtime validation: `target/airlet-comb-clearance-debug.json` reports
  `tooth_tip_radius = 0.05134378`, leaving `0.00047322` static clearance before
  tooth strike.
- Screenshot validation: `target/airlet-comb-clearance-shot.png`, `1280x800`,
  mean brightness `0.276168`.

Tooth/comb animation model:

- Keep cylinder phase as the authoritative time source:
  `score_tick -> cylinder_degrees`.
- For each tooth, define a contact window around its `onset_tick`; contact
  begins when angular distance to the comb radial direction enters a small
  threshold.
- Compute tine deflection as a short pluck envelope: ramp while tooth passes,
  release at the note onset, then damped oscillation
  `deflection(t) = A * exp(-damping * t) * sin(2*pi*f*t + phase)`.
- Drive the visible comb tine by a per-track component containing rest pose,
  local bend axis, stiffness proxy, damping, and current displacement.
- Couple audio and visual amplitude from the same note velocity, while keeping
  audio synthesis independent enough to avoid frame-rate jitter.
- Later replace rigid cuboid tines with skinned or segmented tines so the root
  stays fixed and the tip bends.

Cinematic render roadmap:

- Already enabled this pass: 4096 directional shadow map, 4096 point/spot
  shadow map, tighter directional cascades, and camera contact shadows.
- Add a high-quality studio HDRI (`.hdr`/`.exr`) for metallic reflections and
  image-based lighting.
- Add authored PBR material sets for lacquered wood, brass, steel, and felt;
  use glTF-friendly ORM packing for occlusion/roughness/metallic.
- Enable HDR camera, filmic tonemapping, subtle bloom on highlights, and TAA
  once the material/IBL stack is stable.
- Add area-key/fill/rim light presets for gift-render shots, plus screenshot
  presets for close-up macro views of cylinder, comb, lid, and engraved gift
  details.

### Dense Comb And Tooth Modeling Correction

The first measured-comb pass preserved the original model's range and clearance,
but the generated mechanism was still visually under-modeled: teeth were too
small, comb tines were emitted only for notes that appear in the song, and the
comb lacked a fixed base region distinct from the playable tine region.

Implementation checklist:

- [x] Generate every comb track from `lowest_midi..=highest_midi`, not only
  notes present in the current melody.
- [x] Add a fixed comb base/anchor bar behind the playable tine region.
- [x] Keep the free tine tips at the measured comb tip radius and the base
  region toward the measured root radius.
- [x] Increase tooth cap radius from track spacing instead of clamping it by
  radial clearance.
- [x] Preserve static tooth/comb clearance while making teeth visually readable.
- [x] Expose rendered track count and fixed/free comb dimensions in
  `dump_mechanism`.
- [x] Screenshot-validate dense comb spacing and larger rounded teeth.

Validation result:

- `rendered_track_count = 21`, matching `track_count = 21`.
- Tine width is `0.82 * track_spacing`, so the generated comb is dense instead
  of emitting only melody notes.
- Tooth cap radius is `0.32 * track_spacing`; radial clearance still leaves
  `0.00021032` static gap before strike.
- Comb free tine length: `0.13113001`; fixed base length: `0.05099499`.
- Runtime debug file: `target/airlet-dense-comb-debug.json`.
- Screenshot: `target/airlet-dense-comb-shot.png`, `2560x1568`, mean brightness
  `0.451635`.

### Comb Tine Release Animation And Audio Alignment

The visual mechanism must not play a note when the tooth first touches or lifts
the tine. The note starts when the tine is released. Therefore each mechanism
event needs two visual phases:

- pre-release pluck: before `onset_tick`, the passing tooth bends the tine;
- release/vibration: at exactly `onset_tick`, the tine is released and audio
  begins.

Implementation checklist:

- [x] Convert free comb tines into pivoted entities with a fixed root and child
  playable tine body.
- [x] Add a per-track animation component keyed by MIDI note.
- [x] Animate pre-release tine bend during `onset_tick - pluck_window..onset_tick`.
- [x] Start damped vibration at exactly `onset_tick`.
- [x] Keep audio onset and visual release tied to the same timeline tick.
- [x] Expose release-alignment debug data in `dump_mechanism`.
- [x] Add tests proving release tick equals note onset tick.
- [x] Runtime-validate with MCP/debug dump and screenshot.

Validation notes:

- `target/airlet-comb-animation-debug.json` reports `pluck_window_ticks = 120`
  and `vibration_ticks = 1920`.
- The MCP `release_alignment_preview` rows all satisfy
  `release_tick == onset_tick` and `pluck_start_tick < release_tick`.
- Screenshots:
  `target/airlet-comb-animation-shot.png` (closed, 1280x800, mean 0.276168)
  and `target/airlet-comb-animation-open-shot.png` (open, 1280x800,
  mean 0.372433).

### Adaptive Comb Tine Vibration Visuals

The current release-aligned animation is mechanically correct at the event level
but too literal for high-frequency vibration. A display running at normal frame
rates cannot reproduce the physical oscillation cleanly, so the visual layer
should use a common motion-smear approach: multiple faint tine instances at
sampled vibration phases, while the primary tine follows the current deflection.

The chain must stay adaptive:

`melody -> timeline -> hint -> tooth geometry -> comb animation`

Implementation checklist:

- [x] Derive comb animation events from `ToothHint` instead of fixed visual
  constants.
- [x] Compute pre-release lift duration from each tooth's tangential footprint
  and the planned cylinder turn duration.
- [x] Compute maximum deflection from tooth protrusion and velocity hint.
- [x] Compute vibration duration from velocity/protrusion so stronger teeth
  linger longer.
- [x] Animate the pluck as a slow raised ramp before release.
- [x] Add semi-transparent ghost tine instances to approximate vibration smear.
- [x] Expose derived animation parameters in `dump_mechanism`.
- [x] Add tests for hint-driven animation adaptation and release alignment.
- [x] Runtime-validate with MCP/debug dump and open-lid screenshot.

Validation notes:

- `target/airlet-adaptive-comb-animation-debug.json` reports 36 animation
  events, 4 ghost samples per tine, and every preview row satisfies
  `release_tick == onset_tick`.
- Current bundled Air hint has uniform velocity/protrusion/tooth length, so the
  default song currently produces equal deflection and duration values. The
  synthetic test verifies that stronger/larger hint teeth produce longer pluck
  windows, larger deflection, and longer vibration.
- Runtime scene spawned with 212 mesh components, consistent with main comb
  tines plus ghost instances.
- Screenshots:
  `target/airlet-adaptive-comb-animation-open-shot.png` (1280x800, mean
  0.372422) and `target/airlet-adaptive-comb-animation-close-shot.png`
  (1280x800, mean 0.408263).

### Close-Up Comb Motion Validation

Single overview screenshots are not enough to judge pluck/release/vibration
quality. The next visual-quality gate is a repeatable close-up frame sequence
around one mechanism event.

Implementation checklist:

- [x] Add a Python debug helper that drives MCP to open the box, focus a close
  camera, seek around a selected comb event, and capture frames.
- [x] Add MCP controls for screenshot-focused camera target and UI visibility.
- [x] Capture pre-pluck, mid-pluck, release, early-vibration, and late-decay
  frames from the same event.
- [x] Build a contact sheet so visual changes can be compared without manually
  opening each PNG.
- [x] Include the derived animation parameters next to the generated frames.
- [x] Validate the sequence against a running Airlet demo.
- [x] Represent each free comb tine as one deformable mesh instead of multiple
  segment entities.
- [x] Merge the fixed comb base and all free tines into one primary comb mesh,
  with each tine owning a stable vertex/index range for local deformation.
- [x] Batch vibration smear into one ghost comb mesh per phase sample instead
  of spawning ghost entities per tine.
- [x] Gate the visual pluck/vibration on tooth contact length so too-short
  teeth do not fake a pluck or auto-vibrate.
- [x] Split the pre-release motion into contact, lift, max-deflection hold, and
  release phases instead of a single ramp.

Validation notes:

- The old `airlet-comb-motion-sequence` seek-driven screenshot helper has been
  removed with direct seek controls. Comb timing is now covered by Rust tests
  and the schema-driven debug action catalog.
- MCP now supports screenshot-focused camera target overrides and hiding the
  egui panel via `set_ui`.
- The side-view sequence at
  `target/comb-motion-sequence-segmented/contact_sheet.png` made the previous
  segment-entity prototype visibly readable; the implementation then moved that
  shape into a single deformable mesh per tine.
- `target/airlet-single-mesh-comb-debug.json` reports 36 animation events,
  every preview row aligned to onset, and every default Air event has
  `contact_supported = true` with `contact_window_ticks = 553` versus
  `required_pluck_ticks = 60`.
- `target/comb-motion-sequence-single-mesh/contact_sheet.png` validates the
  same close-up sequence against the single-mesh implementation.
- `target/airlet-contact-phases-debug.json` validates that the default Air
  events now have ordered `contact_start_tick <= max_deflection_start_tick <=
  release_tick == onset_tick`; the first event uses contact start `11687`, max
  deflection start `12007`, and release/onset `12240`.
- `target/comb-motion-sequence-contact-phases/contact_sheet.png` validates the
  updated seven-frame contact/lift/hold/release/vibration sequence.
- `too_short_tooth_does_not_fake_pluck_or_vibration` verifies that insufficient
  tooth length yields `contact_supported = false`, zero deflection, zero
  vibration duration, and no comb animation sample.
- `comb_pluck_window_has_contact_lift_hold_and_release_phases` verifies that
  contact starts from zero deflection, lift ramps up, max deflection is held
  until release, and release stays aligned to the audio onset.
- `comb_mesh_ranges_partition_each_tine_in_one_mesh` verifies that the primary
  comb mesh partitions fixed base and tine geometry into stable, non-overlapping
  vertex/index ranges.
- `comb_ghost_mesh_keeps_ranges_but_collapses_inactive_tines` verifies that
  ghost batches preserve tine ranges while inactive tines collapse out of view.

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

## Architecture Refactor Roadmap

This roadmap is for the next multi-agent refactor pass. The guiding rule is
behavior preservation: keep the current Air track, current audible output
direction, current model-view behavior, and existing debug/MCP workflows intact
unless a task explicitly says otherwise.

Current audit context:

- `src/lib.rs` is the largest quality risk. It currently mixes Bevy app setup,
  egui controls, playback, debug TCP actions, GLB/model spawning, procedural
  mechanism geometry, comb animation, screenshot automation, JSON dumps, and
  app-level tests.
- `crates/airlet` has the correct long-term role as the Bevy-free backend, but
  `engine.rs` and `performance.rs` still depend on a config type that lives in
  `compat`.
- `crates/airlet-model` has the correct long-term role as the Bevy-free model
  spec and rig-pose layer.
- Python model/audio probes are useful, but should remain a later cleanup item
  until the Rust module boundaries are stable.

Validation baseline for every implementation batch:

```bash
cargo fmt --all --check
cargo test --workspace
git diff --check
```

For behavior-changing visual work, also capture and inspect screenshots through
the existing `AIRLET_SCREENSHOT=<path> cargo run` path or the MCP screenshot
helpers.

### Phase 0: Baseline and Coordination

Purpose: give multiple agents a stable starting point and avoid accidental
overlap.

Checklist:

- [x] Record the current dirty worktree before starting a batch.
- [x] Keep unrelated untracked model assets out of refactor commits.
- [x] Confirm `cargo fmt --all --check`, `cargo test --workspace`, and
  `git diff --check` before the first structural split.
- [x] Assign each agent one ownership area before editing.
- [x] Do not mix file movement with behavior changes in the same batch.

Recommended agent ownership:

- App shell agent: `src/lib.rs`, app schedule wiring, module declarations.
- Playback/debug agent: playback state, rodio integration, TCP debug endpoint,
  JSON action plumbing.
- Mechanism-view agent: procedural cylinder/tooth/comb geometry, comb mesh
  ranges, comb animation view systems.
- Backend-core agent: `crates/airlet`, `compat`, `engine`, `performance`,
  synthesis and notation.
- Docs/validation agent: `docs/roadmap.md`, audit follow-ups, verification
  transcripts, screenshot artifacts.

Serial edit zones:

- `src/lib.rs` module declarations and Bevy schedule wiring.
- Public exports in `crates/airlet/src/lib.rs`.
- Shared config structs once introduced.
- `docs/roadmap.md` checklist status changes.

Parallel-safe zones after ownership assignment:

- `src/screenshot.rs`
- `src/playback.rs`
- `src/debug.rs`
- `src/comb_animation.rs`
- `src/mechanism_view.rs`
- `src/model_view.rs`
- `src/scene.rs`
- `crates/airlet/src/synthesis.rs`
- `crates/airlet/src/notation.rs`

Completion criteria:

- [x] Each agent can explain its touched files and validation command output.
- [x] No agent rewrites another agent's active ownership area without
  coordination.
- [x] Roadmap status is updated only after implementation and verification.

### Phase 1: Split the Top-Level Bevy App Without Behavior Changes

Purpose: reduce `src/lib.rs` to an app assembly layer while preserving current
behavior.

Target shape:

```text
src/
  lib.rs               # run(), plugin/schedule wiring, module declarations
  controls.rs          # ExhibitControls and egui control panel
  playback.rs          # AudioOutputState, rodio setup, rate/duration helpers
  debug.rs             # DebugEndpoint, DebugAction, request/response plumbing
  screenshot.rs        # ScreenshotState and automatic capture/exit systems
  scene.rs             # camera, lights, orbit controls, platform setup
  model_view.rs        # GLB/model loading, materials, rig transforms
  mechanism_view.rs    # procedural cylinder/tooth/comb mesh view systems
  comb_animation.rs    # timing-derived comb events and sampling
  visual_config.rs     # later phase; initially optional
```

Suggested order:

1. Screenshot extraction.
   - Move `ScreenshotState`.
   - Move automatic screenshot request and exit systems.
   - Keep system names stable where practical.

2. Playback extraction.
   - Move playback/audio output state.
   - Move `PlaybackCommand`.
   - Move rodio setup.
   - Move playback control, seek, duration, and sample-index helpers.
   - Preserve existing tests for sample alignment and formatting.

3. Debug endpoint extraction.
   - Move TCP listener and request envelope.
   - Move `DebugAction` and response types.
   - Initially allow handler functions to call back into app modules.
   - Do not change the debug JSON schema during this phase.

4. Comb animation extraction.
   - Move `CombAnimationEvent`.
   - Move `CombTineSample`.
   - Move event derivation and sampling functions.
   - Move tests for release alignment, contact/lift/hold/release phases,
     tooth strength, and too-short tooth behavior.

5. Mechanism view extraction.
   - Move procedural cylinder/tooth spawning.
   - Move `CombMeshModel`, `CombTineRange`, and `CombFixedBaseRange`.
   - Move comb mesh construction and animation systems.
   - Keep current mesh batching semantics: one primary comb mesh plus one ghost
     batch mesh per smear phase.

6. Model and scene extraction.
   - Move GLB/model spawning, material conversion, and rig-pose application.
   - Move camera, light, platform, orbit, and spotlight controls.

Completion criteria:

- [x] `src/lib.rs` only wires modules and schedules systems.
- [x] `src/main.rs` still calls `airlet_app::run()`.
- [x] Debug action names and JSON fields remain compatible.
- [x] `cargo fmt --all --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `git diff --check` passes.

### Phase 2: Introduce Visual Mechanism Config

Purpose: replace scattered top-level app constants with explicit configuration
objects that can be tested and dumped.

Target structs:

```rust
pub(crate) struct MechanismVisualConfig {
    pub tooth: ToothVisualConfig,
    pub comb: CombVisualConfig,
    pub animation: CombAnimationConfig,
}

pub(crate) struct ToothVisualConfig {
    pub width_ratio: f32,
    pub height_ratio: f32,
    pub default_clearance_ratio: f32,
}

pub(crate) struct CombVisualConfig {
    pub tine_length_ratio: f32,
    pub tine_width_ratio: f32,
    pub tine_thickness_ratio: f32,
    pub free_length_ratio: f32,
    pub tine_width_spacing_ratio: f32,
    pub track_usable_length_ratio: f32,
}

pub(crate) struct CombAnimationConfig {
    pub min_pluck_ticks: i64,
    pub max_pluck_ticks: i64,
    pub min_vibration_ticks: i64,
    pub max_vibration_ticks: i64,
    pub lift_window_ratio: f32,
    pub deflection_scale: f32,
    pub min_deflection_rad: f32,
    pub max_deflection_rad: f32,
    pub ghost_samples: &'static [f32],
    pub cylinder_playback_rotation_sign: f32,
}
```

Migration targets:

- `TOOTH_WIDTH_RATIO`
- `TOOTH_HEIGHT_RATIO`
- `DEFAULT_TOOTH_CLEARANCE_RATIO`
- `COMB_TINE_LENGTH_RATIO`
- `COMB_TINE_WIDTH_RATIO`
- `COMB_TINE_THICKNESS_RATIO`
- `COMB_FREE_LENGTH_RATIO`
- `COMB_TINE_WIDTH_SPACING_RATIO`
- `COMB_TRACK_USABLE_LENGTH_RATIO`
- `COMB_MIN_PLUCK_TICKS`
- `COMB_MAX_PLUCK_TICKS`
- `COMB_MIN_VIBRATION_TICKS`
- `COMB_MAX_VIBRATION_TICKS`
- `COMB_LIFT_WINDOW_RATIO`
- `COMB_DEFLECTION_SCALE`
- `COMB_MIN_DEFLECTION_RAD`
- `COMB_MAX_DEFLECTION_RAD`
- `COMB_GHOST_SAMPLES`
- `CYLINDER_PLAYBACK_ROTATION_SIGN`

Implementation notes:

- Start with `Default`; do not introduce external config files yet.
- Make `debug_mechanism_json` dump the effective config.
- Keep current numeric defaults identical.
- Keep comb direction derivation tied to the same cylinder playback sign used
  by cylinder rotation.

Completion criteria:

- [x] No default visual parameter changes.
- [x] Debug mechanism JSON exposes the effective visual config.
- [x] Comb timing and direction tests still pass.
- [x] Screenshot validation still produces a nonblank, correctly framed scene
  for visual changes.

### Phase 3: Fix Backend Compatibility Dependency Direction

Purpose: make `compat` a true old-API adapter instead of a dependency of the
main engine path.

Problem:

- `crates/airlet/src/engine.rs` and `crates/airlet/src/performance.rs` still use
  `ModelPlaybackConfig` from `compat`.
- That makes the main timeline-driven render path depend on a module whose name
  says legacy compatibility.

Target shape:

```text
crates/airlet/src/
  playback_config.rs   # or render_config.rs
  compat.rs            # old NoteEvent/Score glue only
  engine.rs            # depends on playback_config/performance, not compat
  performance.rs
```

Checklist:

- [x] Move `ModelPlaybackConfig` out of `compat.rs` into `render_config.rs`.
- [x] Update `engine.rs` and `performance.rs` to import the new module.
- [x] Keep a temporary `pub use` in `compat.rs` if needed for API stability.
- [x] Ensure `compat.rs` depends on the new path rather than the reverse.
- [x] Add or update a test that confirms legacy compatibility render remains
  deterministic while the compatibility layer exists.

Completion criteria:

- [x] `rg "crate::compat" crates/airlet/src/engine.rs
  crates/airlet/src/performance.rs` returns no main-path dependency.
- [x] Existing audio golden/statistical tests pass.
- [x] Public API churn is minimized or explicitly documented.

### Phase 4: Split the Backend Facade

Purpose: make `crates/airlet/src/lib.rs` a facade rather than a synthesis and
notation implementation file.

Target shape:

```text
crates/airlet/src/
  lib.rs
  synthesis.rs
  notation.rs
  audio.rs
  score.rs
  engine.rs
  performance.rs
  preset.rs
  mechanism.rs
  model.rs
  defaults.rs
  songs.rs
  compat.rs
```

Migration plan:

- Move synthesis types into `synthesis.rs`.
  - `Single`
  - `TineParams`
  - `BoxTine`
  - `ModalTine`
  - frequency and modal helper functions

- Move notation types into `notation.rs`.
  - `Pitch`
  - `CypherNotation`
  - notation parsing and display helpers

- Keep `lib.rs` as the public facade.
  - Re-export existing public types to preserve call sites.
  - Keep module declarations and crate-level documentation there.

Completion criteria:

- [x] Existing downstream imports in this workspace still compile.
- [x] `crates/airlet/src/lib.rs` contains facade wiring rather than large
  implementations.
- [x] `cargo test --workspace` passes.

### Phase 5: Add Score Validation Before External Score Inputs

Purpose: keep the current fluent internal DSL, but add a fallible validation
boundary before scores become user-editable data.

Short-term target:

```rust
pub enum ScoreError {
    InvalidTempo,
    InvalidDuration,
    InvalidVelocity,
    InvalidTie,
    InvalidTimeline,
}

impl Timeline {
    pub fn validate(&self) -> Result<(), ScoreError>;
}
```

Rules:

- Do not replace the current hardcoded song DSL in this phase.
- Do not make current song construction noisier.
- Add validation for externally loaded or generated timelines first.

Candidate validations:

- positive tempo;
- positive event duration;
- finite velocity in expected range;
- monotonic or valid onset positions where required;
- invalid tie/slur metadata if represented in the current model;
- out-of-range MIDI notes if a mechanism planner range is selected.

Completion criteria:

- [x] Existing `songs::air` construction remains ergonomic.
- [x] Validation tests cover invalid tempo, duration, velocity, and timeline
  edge cases.
- [x] External score-loading work has a clear validation entry point.

### Phase 6: Tighten Debug Endpoint Defaults

Purpose: keep the local MCP/debug workflow useful while avoiding accidental
control endpoints in packaged builds.

Options:

- Debug builds default to local endpoint; release builds default off.
- Or require explicit `AIRLET_DEBUG=1` for all builds.
- Keep `AIRLET_DEBUG_BIND` for overriding the bind address.

Checklist:

- [x] Decide the default policy: require explicit `AIRLET_DEBUG=1`.
- [x] Update `DebugEndpoint::start_from_env` to require opt-in.
- [x] Update `AGENTS.md` and MCP docs with the required env vars.
- [x] Verify Python MCP tools still work with the chosen startup command.

Completion criteria:

- [x] Debug endpoint behavior is explicit in docs.
- [x] Release behavior does not unexpectedly expose local control actions.
- [x] MCP usage remains one documented command away.

### Phase 7: Python Model Probe Status

Purpose: record the final status of the model probe after the Rust app and
schema-driven debug workflow stabilized.

Target shape:

```text
py/airlet_audio_lab/model_probe/
  __init__.py
  load.py
  geometry.py
  classify.py
  spec_writer.py
  report.py
  debug_render.py
```

Completion criteria:

- [x] Existing `uv run --project py airlet-probe-model` probe command still
  works.
- [x] Existing spec output fields remain stable for the current model pipeline.
- [x] Probe geometry behavior is covered by the generated `spec.toml`,
  `airlet-model` parsing tests, and runtime rig validation.
- [x] `py/uv.lock` does not need a refresh because dependencies did not change.

Validation notes:

- `uv run --project py airlet-probe-model` completed and wrote
  `target/model-probe.json`, `target/model-probe.md`,
  `target/model-spec.toml`, and `target/model-probe-debug.png`.

### Multi-Agent Batch Plan

Recommended first batch:

1. Agent A: extract `screenshot.rs` and `playback.rs`.
2. Agent B: extract `debug.rs`.
3. Agent C: extract `comb_animation.rs`.
4. Agent D: keep docs synchronized and run verification.

Recommended second batch:

1. Agent A: extract `mechanism_view.rs`.
2. Agent B: extract `model_view.rs`.
3. Agent C: extract `scene.rs` and simplify `src/lib.rs`.
4. Agent D: run screenshots and update validation notes.

Recommended third batch:

1. Agent A: introduce `visual_config.rs`.
2. Agent B: fix backend `compat` dependency direction.
3. Agent C: split backend `synthesis.rs` and `notation.rs`.
4. Agent D: update docs, run full tests, and check API drift.

Conflict rules:

- Only one agent should edit `src/lib.rs` at a time during schedule/module
  wiring changes.
- Only one agent should edit `crates/airlet/src/lib.rs` at a time during public
  facade changes.
- Agents may add new module files in parallel after agreeing on module names.
- Agents should not mark roadmap checkboxes complete for another agent's work.
- If a test fails after merging two agents' work, first check import visibility
  and module ownership before changing behavior.

Refactor success criteria:

- [x] `src/lib.rs` is reduced from the old single-file app shell to a small
  Bevy app assembly file.
- [x] The Bevy-free backend remains Bevy-free.
- [x] `compat` no longer sits on the main engine dependency path.
- [x] Visual mechanism constants are grouped in explicit config structs.
- [x] Current Air playback tests, mechanism tests, and model tests pass (44 tests).
- [x] Screenshot validation remains available for visual changes.

Final acceptance notes:

- Final app shell is `src/lib.rs` at 75 lines after formatting, with Bevy
  schedule wiring only.
- App behavior modules now live in dedicated files:
  `controls.rs`, `playback.rs`, `debug.rs`, `scene.rs`, `model_view.rs`,
  `mechanism_view.rs`, `comb_animation.rs`, `screenshot.rs`, and
  `visual_config.rs`.
- Backend facade split is complete for this pass:
  `synthesis.rs`, `notation.rs`, and `render_config.rs` now carry the moved
  implementation/config responsibilities.
- Final verification command:

  ```bash
  cargo fmt --all --check && cargo test --workspace && git diff --check
  ```

  Result: passed. The workspace reports 44 unit tests passing plus empty
  doctest runs for the three crates.
- Phase 7 Python model-probe splitting remains a deliberately deferred
  follow-up. It is not part of the completed Rust architecture refactor
  acceptance scope.

## Winding Playback Roadmap

This roadmap starts the next interaction model: the cylinder should no longer
free-spin from a debug/UI toggle. Playback should eventually be initiated by
winding the music-box key, accumulating a wind meter, releasing the key, and
then letting the wound amount drive automatic playback.

Important playback constraint:

- The rendered track has a long tail at the end.
- A physical cylinder period should not simply concatenate the audio buffer
  end-to-start.
- Looping playback must allow overlap between the next musical cycle and the
  previous tail, so the cylinder period stays mechanically correct while audio
  release tails can continue sounding.

### Phase 0: Remove Legacy Cylinder Free-Spin

Purpose: remove the obsolete `cylinder_spin` path so cylinder motion has one
clear source of truth before adding winding.

Current problem:

- During playback, the cylinder angle is synchronized from the mechanical twin
  clock.
- Outside playback, the old `cylinder_spin` UI/debug flag can still advance the
  model through `MovableMusicBoxModel::advance`.
- If `cylinder_spin` was enabled before or after playback, the cylinder can keep
  rolling after the performance ends.

Checklist:

- [x] Remove `ExhibitControls::cylinder_spin`.
- [x] Remove the egui `Cylinder spin` checkbox.
- [x] Remove `cylinder_spin` from debug state JSON.
- [x] Remove `DebugAction` code that manually clears `cylinder_spin`.
- [x] Remove `MovableModelState::cylinder_spin`.
- [x] Remove `MovableMusicBoxModel::set_cylinder_spin`.
- [x] Remove or repurpose `MovableMusicBoxModel::advance`.
- [x] Remove `degrees_per_second` from the model spec parser if no other path
  uses it.
- [x] Stop emitting `degrees_per_second` from the Python model probe spec
  writer.
- [x] Remove `degrees_per_second` from `assets/models/converted/spec.toml`.
- [x] Update model tests so cylinder angle changes only through
  `set_cylinder_degrees`.

Completion criteria:

- [x] `rg "cylinder_spin|set_cylinder_spin|Cylinder spin" src crates py
  assets/models/converted/spec.toml` returns no runtime/spec-writer references.
- [x] Playing to the end leaves the cylinder at the playback-synchronized final
  angle instead of continuing to roll.
- [x] `cargo fmt --all --check`, `cargo test --workspace`, and
  `git diff --check` pass.

### Phase 1: Identify Winding Key Mesh and Axis

Purpose: extend the existing model-probe workflow so the app knows which source
mesh is the winding key and what axis it rotates around.

Investigation inputs:

- Existing model probe outputs under `target/model-probe.json`,
  `target/model-probe.md`, and `target/model-probe-debug.png`.
- Existing source model and converted GLB.
- Current model spec at `assets/models/converted/spec.toml`.

Checklist:

- [x] Inspect model probe mesh bounds and candidate hardware meshes near the
  side/back of the box.
- [x] Add winding-key classification to the Python probe.
- [x] Estimate winding key pivot and rotation axis from candidate mesh geometry.
- [x] Add a `[winding_key]` section to the generated spec draft.
- [x] Extend `airlet-model` parsing with a winding-key part spec.
- [x] Add tests proving the winding-key spec parses and exposes rig-space pose
  data.
- [x] Add debug report fields listing winding key meshes, pivot, axis, and
  bounds.

Proposed spec shape:

```toml
[winding_key]
meshes = [...]
pivot = [x, y, z]
axis = [x, y, z]
rest_degrees = 0.0
pressed_degrees = ...
```

Completion criteria:

- [x] The app can identify and group the winding-key mesh separately from the
  static body.
- [x] The model-probe report exposes winding-key mesh and axis data.
- [x] A screenshot can visually confirm the selected winding key candidate.

Validation notes:

- `uv run --project py airlet-probe-model` now reports `Mesh.018` as the
  winding-key candidate, with pivot `[-0.583744, -1.040804, -1.352274]` and
  horizontal outward axis `[0.72398, 0.0, -0.689821]`.
- `target/model-spec.toml` and `assets/models/converted/spec.toml` now include
  `[winding_key]` with `meshes = [18]`.
- `airlet-model` parses `winding_key` and maps mesh 18 to
  `MeshGroup::WindingKey`; the app attaches that group under a dedicated
  `WindingKeyPivot` entity for interaction and rotation.
- `target/winding-hover.png` and `target/winding-pressed.png` visually confirm
  the selected key mesh, hover material, and pressed rotation state.

### Phase 2: Render Winding Key as an Interactive Target

Purpose: make the winding key discoverable and clickable in the Bevy app.

Bevy implementation notes:

- Prefer Bevy 0.19 built-in picking/interaction features if available in the
  current dependency set.
- If built-in picking is not sufficient, implement a narrow raycast against the
  winding-key entity bounds first; do not add a large dependency before
  checking what Bevy already provides.
- Hover feedback should be visual and local to the key: outline, highlight
  material, or a subtle rim effect.

Checklist:

- [x] Group winding-key meshes under a dedicated entity.
- [x] Add an interaction component/resource for hover and pressed state.
- [x] Implement pointer hover detection for the winding-key target.
- [x] Add hover visual feedback.
- [x] Implement pointer press/release detection.
- [x] Keep camera orbit controls from fighting with winding-key press gestures.
- [x] Add debug state fields for hover, pressed, and current winding angle.

Completion criteria:

- [x] Hovering the key produces a visible outline/highlight.
- [x] Pressing the key does not accidentally start playback; release state is
  consumed separately by the wound-start system.
- [x] Existing playback and rig controls still work.

### Phase 3: Accumulate Wind Meter While Pressed

Purpose: transform pointer press/drag or press-hold into stored winding energy.

Behavior model:

- Pressing the winding key enters winding mode.
- While pressed, user input rotates the key or accumulates wind amount.
- The wind meter clamps to a configured maximum.
- Releasing the key commits the current wound amount to playback start logic.

Checklist:

- [x] Add `WindingState` resource with at least:
  - `hovered`;
  - `pressed`;
  - `wind_amount`;
  - `max_wind_amount`;
  - `key_degrees`;
  - `pending_release_start`.
- [x] Add visual key rotation driven by `key_degrees`.
- [x] Add UI/debug output for wind meter amount.
- [x] Decide whether winding input is press-hold, drag distance, or angular drag.
- [x] Add tests for wind meter clamp and release transition.

Completion criteria:

- [x] Pressing/holding or dragging the key visibly increases wind amount.
- [x] The wind meter clamps deterministically.
- [x] Releasing the key triggers a structured event/resource state, not direct
  ad hoc playback side effects.

### Phase 4: Start Auto Playback on Release

Purpose: connect winding release to playback and cylinder motion.

Rules:

- Playback starts only after winding key release.
- Initial playback duration or number of periods should be determined by the
  accumulated wind amount.
- Cylinder angle should still be derived from musical/cylinder phase, not from
  an independent spin integrator.
- Reset and seek debug actions must clear or explicitly set winding state.

Checklist:

- [x] Add a playback command for wound-start or map winding release to the
  existing start command with explicit state.
- [x] Convert wind amount into available playback time or cycle count.
- [x] Stop playback when wound energy is exhausted.
- [x] Reset wind state on manual reset.
- [x] Replace legacy debug transport controls with winding-first actions whose
  behavior is defined by the twin state machine.

Completion criteria:

- [x] Winding and release starts playback without using the old cylinder spin
  path.
- [x] Cylinder phase remains synchronized to timeline ticks.
- [x] Playback stops when wind is exhausted or the performance policy says it
  should stop.

### Phase 5: Support Overlapped Loop Playback

Purpose: match a physical cylinder cycle when the rendered audio has a tail
that extends beyond the cylinder period.

Model:

- Define a mechanical cycle duration from timeline/cylinder ticks:
  `cycle_seconds = ticks_per_turn_to_seconds`.
- Define an audio render duration that may be longer than the mechanical cycle
  because of release tails.
- On each cycle boundary, start the next rendered cycle while allowing the
  previous tail to continue.
- Cylinder phase uses mechanical cycle time, not audio buffer duration.

Checklist:

- [x] Add explicit mechanical cycle duration helper derived from
  `MechanismResource.ticks_per_turn`.
- [x] Separate `audio_duration_seconds` from `mechanical_cycle_seconds`.
- [x] Add a scheduler that can layer overlapping `rodio::Player` or mixer
  sources at cycle boundaries.
- [x] Decide how wind amount maps to number of cycle starts.
- [x] Keep waveform/timeline UI clear about mechanical cycle versus audio tail.
- [x] Add tests for cycle boundary scheduling and overlap start times.

Completion criteria:

- [x] The next cycle can start before the previous audio tail has fully ended.
- [x] Cylinder phase wraps at the mechanical cycle boundary.
- [x] Long release tails remain audible without delaying the next cylinder
  cycle.
- [x] The default Air intro still starts at the same musical/cylinder phase.

### Phase 6: Validation and Documentation

Checklist:

- [x] Screenshot validate winding-key hover.
- [x] Screenshot validate winding-key pressed/rotated state.
- [x] Runtime validate wind meter debug output.
- [x] Runtime validate release-start playback.
- [x] Runtime validate at least two overlapped cycles.
- [x] Update `docs/mcp.md` if debug actions change.
- [x] Update `AGENTS.md` with winding-specific ownership rules if this work is
  split across agents.

Completion criteria:

- [x] `cargo fmt --all --check` passes.
- [x] `cargo test --workspace` passes.
- [x] `git diff --check` passes.
- [x] Visual validation artifacts are recorded in this roadmap.

Validation notes:

- `target/winding-hover.png` confirms the winding-key hover highlight with the
  key mesh selected separately from the body.
- `target/winding-pressed.png` confirms pressed/rotated key state and the wind
  meter UI.
- Runtime debug validation with `set_winding` and `dump_state` confirmed a
  two-cycle wound start before the tail-rest update:
  `mechanical_cycle_seconds = 15.5`, `mechanical_end_seconds = 31.0`,
  `active_cycle_count = 1`, and
  `remaining_cycle_starts = 1` immediately after release.
- A later runtime dump after the 15.5s mechanical boundary confirmed overlap:
  `active_cycle_count = 2` while one cycle start remained queued.

## Digital Twin Winding Roadmap

This roadmap supersedes the first winding prototype above. The prototype proved
that hover, material swapping, screenshots, and overlapped audio scheduling can
work, but it made two incorrect architectural assumptions:

- It treated the winding key as a single handle mesh. A digital twin must rotate
  the complete crank assembly: crank shaft, bent crank arm, and crank head.
- It treated winding as a release trigger. A digital twin needs persistent
  spring state: pressing winds the key backward and stores energy; releasing
  lets the key unwind forward from its current angle while the stored energy
  drives cylinder phase and audio scheduling.

Target dependency direction:

```text
user input -> spring/key state -> mechanical cylinder phase -> audio scheduling
                                      |
                                      +-> visual key/cylinder/comb rig
```

The top-level app should not manually keep audio playback, cylinder rotation,
and crank rotation in sync. The app should update one mechanical twin state,
then derive visuals and audio events from that state.

### Phase 0: Correct The Roadmap Contract

Purpose: preserve the completed prototype evidence while making clear that it
is not the final winding architecture.

Checklist:

- [x] Record the prototype mismatch in this roadmap.
- [x] Define the new truth source as a mechanical twin state, not playback
  elapsed time.
- [x] Define completion criteria around complete crank assembly motion and
  persistent spring energy.

Completion criteria:

- [x] New implementation work follows this section instead of the earlier
  trigger-based winding roadmap.

### Phase 1: Re-Probe Complete Crank Mesh And Axis

Purpose: identify the whole crank assembly and derive its axis from frame
fit/contact locations rather than from the handle center.

Expected movable assembly:

- crank shaft at the frame;
- bent crank arm;
- crank head / grip;
- any visible coaxial part that must rotate with the crank.

Checklist:

- [x] Use material/color similarity as a first-class clue for crank membership.
- [x] Treat frame-side fit/contact meshes as axis evidence, not automatically
  as moving crank meshes.
- [x] Extend `airlet-probe-model` so `winding_key_estimate.meshes` can contain
  multiple closed-model meshes.
- [x] Detect crank-related candidates near the side/back of the closed box,
  including handle, arm, and shaft-like meshes.
- [x] Estimate the rotation axis from two frame-side fit/contact positions when
  available.
- [x] Use a shaft/cylindrical candidate PCA axis only as a fallback.
- [x] Emit report details for crank meshes, fit points, pivot, axis, and
  fallback reason.
- [x] Update `assets/models/converted/spec.toml` with the complete crank mesh
  list and corrected pivot/axis.
- [x] Update `airlet-model` tests so all crank meshes map to
  `MeshGroup::WindingKey`.

Completion criteria:

- [x] The crank mesh set includes the grey crank head, bent arm, and shaft, and
  excludes the yellow gear/front latch and fixed frame brackets.
- [x] The visual crank pivot rotates the shaft, bent arm, and crank head
  together.
- [x] The reported axis is tied to frame-side fit/contact geometry or explicitly
  documents the fallback.

### Phase 2: Introduce Mechanical Twin State

Purpose: make the digital twin the single source of truth for spring, key,
cylinder, and cycle-boundary state.

State shape:

```rust
MusicBoxTwinState {
    mode: Idle | Winding | Releasing | Exhausted,
    spring_energy,
    max_spring_energy,
    key_degrees,
    cylinder_degrees,
    mechanical_seconds,
    next_cycle_index,
}
```

Rules:

- Pressing the key enters `Winding`.
- While pressed, the crank turns backward and `spring_energy` increases.
- Releasing enters `Releasing`.
- While releasing, the crank turns forward from the current angle and
  `spring_energy` decreases.
- Cylinder phase advances only while spring energy is releasing.
- Exhausted spring energy stops key and cylinder motion.

Checklist:

- [x] Add a twin-state resource and mode enum.
- [x] Move winding accumulation from one-shot `pending_release_cycles` into
  continuous spring energy.
- [x] Add pure tests for wind, release, exhaustion, and phase wrap.
- [x] Keep manual `Start/Pause/Reset/Seek` semantics explicit while the twin
  state exists.

Completion criteria:

- [x] A unit test proves press-hold winds backward and release unwinds forward.
- [x] A unit test proves cylinder phase advances from twin state, not audio
  player elapsed time.

### Phase 3: Transparent Twin-Driven Audio Scheduling

Purpose: make audio follow mechanical phase crossings instead of precomputed
cycle counts.

Checklist:

- [x] Remove the wound-start cycle-count trigger path.
- [x] Replace the legacy playback-owned state with `AudioOutputState` for audio
  output/mixer state only.
- [x] Add an audio scheduler that starts a rendered cycle when twin mechanical
  phase crosses a cycle boundary.
- [x] Allow previous audio tails to continue naturally after the next cycle
  starts.
- [x] Stop scheduling new cycles when spring energy is exhausted.
- [x] Add tests for boundary crossing and overlap trigger decisions.

Completion criteria:

- [x] Audio cycles are scheduled by twin phase crossings.
- [x] Audio tails do not delay mechanical phase or the next cycle start.
- [x] No caller needs to pass a wound cycle count to playback.

### Phase 4: Drive Visual Rig From Twin State

Purpose: derive visible crank and cylinder transforms from one mechanical twin
state.

Checklist:

- [x] Rotate `WindingKeyPivot` from twin `key_degrees`.
- [x] Rotate `CylinderPivot` from twin `cylinder_degrees`.
- [x] Remove duplicated crank/cylinder angle state from interaction and
  playback paths where possible.
- [x] Keep hover/highlight independent from mechanical state.
- [x] Update debug JSON to expose twin mode, spring energy, key angle,
  cylinder angle, and cycle index.

Completion criteria:

- [x] Holding the key visibly rotates the complete crank backward.
- [x] Releasing visibly rotates the crank forward from its current angle.
- [x] Cylinder rotation continues only while spring energy remains.

### Phase 5: Bundle/Plugin Boundary

Purpose: make the external app wiring transparent while still using Bevy
idioms.

Design decision:

- Use entity bundles/components for rig parts such as crank, cylinder, and
  pivots.
- Use a `MusicBoxTwinPlugin` plus resources/systems for transparent linkage
  across input, mechanics, audio, debug, UI, and visuals.
- Do not put cross-system state synchronization into a plain Bevy `Bundle`;
  bundles should only describe entity composition.

Checklist:

- [x] Add or document a `MusicBoxTwinPlugin` boundary.
- [x] Keep app-level `run()` scheduling thin.
- [x] Keep entity bundles limited to spawn-time composition.
- [x] Update `AGENTS.md` with this ownership boundary.

Completion criteria:

- [x] App code wires the twin plugin/systems without manually synchronizing
  crank, cylinder, and audio.

### Phase 6: Validation

Checklist:

- [x] Run `uv run --project py airlet-probe-model`.
- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo test --workspace`.
- [x] Run `git diff --check`.
- [x] Screenshot validate full crank hover.
- [x] Screenshot validate full crank winding backward.
- [x] Runtime debug validate release unwinds forward and advances cylinder.
- [x] Runtime debug validate overlapped audio starts from phase crossing.

Completion criteria:

- [x] All phases in this section are complete.
- [x] No `pending_release_cycles` / wound cycle-count trigger remains in the
  runtime path.

Validation notes:

- `uv run --project py airlet-probe-model` reports moving crank meshes
  `[18, 19, 20]`, all with material color `[107.0, 107.0, 107.0, 255.0]`.
- Fit/contact meshes `Mesh.021`, `Mesh.022`, `Mesh.024`, and `Mesh.025`
  determine the axis but are not automatically treated as moving crank meshes.
- Corrected winding axis is `[0.866024, -0.0, -0.500003]` through fit points
  `[[-0.796114, -0.95897, -1.246202], [-0.946434, -0.95897, -1.159415]]`.
- `target/twin-crank-hover.png` confirms the front latch/yellow gear and fixed
  frame brackets are not part of the moving crank group.
- `target/twin-crank-winding.png` confirms the crank head, arm, and shaft rotate
  together from `MusicBoxTwinState.key_degrees`.
- Runtime debug validation confirmed release enters `Releasing`, advances
  `mechanical_seconds` and `cylinder_degrees`, and schedules overlapped audio
  when the twin phase crosses a cycle boundary.

Follow-up corrections:

- `WindingKeyPart` hover detection now projects each crank mesh AABB instead of
  testing only the entity origin or pivot point. This is required because the
  crank head, arm, and shaft do not share a useful screen-space origin.
- Front latch upper meshes `[10, 12, 13, 14, 16, 17]` are part of the lid rig;
  lower latch meshes `[11, 15]` remain on the static body.
- `target/lid-front-latch-open.png` confirms the upper latch follows the open
  lid.
- `set_winding` is useful for debug state validation, but real hover visuals
  must be validated with the mouse because the interaction system overwrites
  forced hover state on the next frame.

## Digital Twin State Machine Hardening Roadmap

This section supersedes the partial `MusicBoxTwinState` implementation above.
The previous implementation linked spring, key, cylinder, and audio through
public fields plus helper functions. That left digital-twin invariants to the
system schedule and allowed toy-like illegal behavior such as winding while
audio scheduling could still be active, or continuing to rotate the crank after
the spring reached capacity.

Target architecture:

```text
Winding input -> MusicBoxTwinState methods -> TwinEvent audio commands
                                      |
                                      +-> visual crank and cylinder angles
```

Hard invariants:

- Pressing the winding key enters a single `Winding` state.
- While `Winding`, audio scheduling is muted and mechanical cylinder phase does
  not advance.
- While `Winding`, spring energy increases and the crank rotates backward only
  until `max_spring_energy` is reached.
- Once full, continued press keeps the state pressed/full but crank velocity is
  zero and energy remains clamped.
- Releasing from `Winding` enters `Playing` only if stored spring energy is
  above the release threshold.
- `Playing` unwinds the crank forward from the current crank angle, consumes
  spring energy, advances cylinder phase, and emits audio-cycle events only on
  mechanical phase crossings.
- Exhaustion enters `Exhausted`/`Idle` with no more crank, cylinder, or audio
  scheduling motion.

### Phase 7: Replace Field Coupling With Twin Methods

Checklist:

- [x] Keep `MusicBoxTwinState` as the Bevy resource but move transitions behind
  methods such as `begin_winding`, `release_winding`, `tick`, and `reset`.
- [x] Replace `Releasing` with a clearer `Playing` mechanical state.
- [x] Keep spring, crank, cylinder, and audio-cycle bookkeeping in the twin
  resource; `WindingState` is only input/highlight/UI mirror state.
- [x] Make debug setters call twin methods or explicit test-only/debug mutation
  methods instead of assigning mode fields in multiple places.

Completion criteria:

- [x] Unit tests cover the legal state transitions and reject the illegal
  behavior called out above.

### Phase 8: Enforce Winding And Full-Spring Invariants

Checklist:

- [x] Winding clears queued twin audio events and schedules no new ones.
- [x] Winding does not advance `mechanical_seconds` or `cylinder_degrees`.
- [x] Full spring clamps energy and stops further backward crank rotation while
  the mouse remains pressed.
- [x] Releasing starts forward crank motion from the exact current crank angle.

Completion criteria:

- [x] Tests prove no audio event is emitted while winding.
- [x] Tests prove full spring prevents additional reverse crank motion.
- [x] Tests prove release begins from the already-wound crank angle.

### Phase 9: Adopt Bevy Picking For Crank Hover

Checklist:

- [x] Register Bevy picking plugins and the mesh-picking backend.
- [x] Mark the exhibit camera for mesh picking.
- [x] Add `Pickable` and `Hovered` components to every winding-key mesh part.
- [x] Replace custom screen-space projection hover code with Bevy `Hovered`
  state.
- [x] Keep highlight independent from mechanical state.

Completion criteria:

- [x] Hover path targets crank head, bent arm, and shaft through Bevy picking.
- [x] The previous custom hit-test constants and helpers are removed.

### Phase 10: Validation

Checklist:

- [x] Run `cargo fmt --all --check`.
- [x] Run `cargo test --workspace`.
- [x] Run `git diff --check`.
- [x] Runtime/debug validate: pressing the key does not start playback.
- [x] Runtime/debug validate: holding at full spring stops further key reverse
  rotation.
- [x] Runtime/screenshot validate: the Bevy picking runtime still renders the
  crank assembly.

Completion criteria:

- [x] All phases in this hardening section are complete.

Validation notes:

- `cargo fmt --all --check`, `cargo test --workspace`, and `git diff --check`
  pass after the hardening changes.
- Debug runtime validation with `set_winding pressed=true` showed
  `active_cycle_count=0`, `mechanical_seconds=0`, and `pending_audio_cycles=0`
  while winding.
- Holding beyond full spring clamped `spring_energy=1.0` and kept
  `key_degrees=-857.143` for the next sampled second.
- Releasing entered `Playing`; after one second `active_cycle_count=1`,
  `mechanical_seconds=0.9735848`, and `key_degrees` moved forward from
  `-857.143` to `-822.094`.
- Loop hardening update: the current mechanism cycle is `17.5s` (`33600`
  ticks), comb animation uses cycle-local ticks, and releasing after exhaustion
  queues audio from the current cycle phase instead of always restarting sample
  zero.
- `target/twin-picking-runtime.png` confirms the picking-enabled runtime still
  renders the complete crank assembly in place. Automated OS-level mouse hover
  could not be driven in this session because `xdotool` could not discover the
  Bevy window under the current compositor; hover behavior now depends on Bevy's
  `Pickable`/`Hovered` path instead of custom screen-space projection.

## Twin Playback Kernel And Crank Axis Roadmap

Purpose: finish the architectural correction surfaced after the first state
machine pass.

Problems:

- The playback module still owns a legacy direct-start transport path with its
  own `elapsed_seconds` / `is_playing` state. That conflicts with the twin
  design, where spring, cylinder phase, and cycle crossings are the only
  mechanical truth.
- The crank axis uses the midpoint of two frame-side fit/contact positions as
  pivot. That gives a reasonable assembly direction, but the rotating crank
  shaft can still orbit slightly if that midpoint is not on the shaft center
  line.

Target architecture:

```text
twin mechanical phase -> audio cycle event -> AudioOutputState active players
```

Checklist:

- [x] Rename/reduce playback state to an audio output resource: device,
  rendered audio, active players, and last error only.
- [x] Remove direct `start_playback`, `seek_playback`, `elapsed_seconds`, and
  `is_playing` ownership from the playback module.
- [x] Make UI/debug playback status derive from twin state plus active audio
  players.
- [x] Make comb animation and cylinder visuals read twin mechanical time rather
  than playback time.
- [x] Keep manual `Pause`/`Reset` as commands that stop audio and reset/pause
  the twin, not as commands that advance an independent clock.
- [x] Refit winding-key pivot to the crank shaft center line; use frame fit
  points for direction/constraint, not as the shaft-center pivot.
- [x] Regenerate `assets/models/converted/spec.toml` from the corrected probe.
- [x] Add or update tests for audio-output-only playback state and the corrected
  winding axis metadata.

Validation:

- [x] `uv run --project py airlet-probe-model`
- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`
- [x] `git diff --check`
- [x] Runtime/debug smoke test that winding starts audio only through twin
  cycle events.

Validation notes:

- Playback state is now `AudioOutputState`; it owns rodio device, rendered
  audio, active cycle players, and last error, but no mechanical clock.
- Direct `play` has been removed. The current explicit action is `full_wind`,
  which fully winds the spring and releases into playback.
- While winding, runtime debug showed `active_cycle_count=0`,
  `mechanical_seconds=0`, and spring/key state advancing inside the twin.
- On release, twin entered `Playing`, emitted one pending cycle event, and the
  next sample showed `active_cycle_count=1` with `mechanical_seconds=0.6722738`.
- Corrected winding pivot is `[-0.877912, -0.97183, -1.197975]`, sourced from
  `Mesh.020` point-cloud center. Frame fit points still define the axis
  direction `[0.866024, -0.0, -0.500003]`.
- `[lid].meshes` now matches the intended upper front latch membership:
  `[0, 10, 12, 13, 14, 16, 17]`.

## Debug Action Schema Roadmap

Purpose: make Airlet debug actions maintainable as a long-lived protocol. A new
action should be defined in Rust once, with MCP tool names, parameter schemas,
CLI discoverability, and documentation generated from that protocol description
instead of copied across layers.

Problems:

- The old debug action vocabulary was duplicated in Rust `DebugAction`, Python
  MCP tools, Python CLI shorthands, and `docs/mcp.md`.
- Removing deprecated controls such as direct play, cylinder seek, and manual
  cylinder rotation required coordinated edits in several files.
- MCP parameter shapes could drift from Rust serde input shapes.

Target architecture:

```text
Rust DebugAction + ActionSpec registry
            |
            +-> debug endpoint deserialization and handler
            +-> describe_actions JSON schema endpoint
            +-> Python MCP dynamic tool registration
            +-> CLI discovery and documentation
```

Design rules:

- Rust owns the action protocol.
- Python MCP does not hand-write per-action wrappers.
- MCP can still expose individual action-named tools when the app is running.
- A generic `call_action` fallback remains available for development and for
  startup cases where the app is not yet reachable.
- Deprecated actions are removed, not kept as compatibility aliases.

Checklist:

- [x] Move action protocol metadata next to `DebugAction`.
- [x] Add serializable `ActionCatalog`, `ActionSpec`, and `ActionParameterSpec`
  types.
- [x] Add `describe_actions` to the Rust debug endpoint.
- [x] Add tests that the catalog includes every supported public action and no
  deprecated actions.
- [x] Change Python MCP to dynamically register tools from `describe_actions`.
- [x] Keep generic `describe_actions` and `call_action` MCP tools as stable
  fallbacks.
- [x] Change Python CLI shorthands to derive from the action catalog when the
  endpoint is reachable.
- [x] Update MCP docs to describe the schema-driven workflow instead of listing
  duplicated action definitions.
- [x] Validate Rust tests, Python compile checks, and whitespace checks.

Completion criteria:

- [x] Adding/removing a debug action no longer requires editing Python MCP tool
  wrappers.
- [x] MCP parameter schemas are generated from the Rust action catalog.
- [x] Removed controls (`play`, `stop`, `set_cylinder`, `seek_tick`) are absent
  from Rust action specs, Python MCP tools, CLI shorthands, and docs.
- [x] `cargo fmt --all --check`, `cargo test --workspace`,
  `uv run --project py python -m compileall py/airlet_audio_lab`, and
  `git diff --check` pass.

Validation notes:

- Runtime `describe_actions` returned catalog version `1` with actions:
  `describe_actions`, `dump_state`, `dump_mechanism`, `set_camera`, `set_ui`,
  `set_light`, `set_lid`, `set_winding`, `full_wind`, `pause`, `reset`, and
  `screenshot`.
- Removed actions `play`, `stop`, `set_cylinder`, and `seek_tick` were absent
  from the runtime catalog and dynamically registered MCP tool names.
- Dynamic MCP registration produced tools:
  `call_action`, `describe_actions`, `dump_mechanism`, `dump_state`,
  `full_wind`, `pause`, `reset`, `screenshot`, `set_camera`, `set_lid`,
  `set_light`, `set_ui`, and `set_winding`.
- CLI shorthand validation through the catalog showed `full_wind -> Playing`,
  `pause -> Paused`, second `pause -> Playing`, and `reset -> Idle`.

## AAA Lighting Implementation Roadmap

Purpose: turn the current visible Bevy exhibit into a photoreal product-style
music-box presentation while preserving the digital-twin mechanical behavior.
Research source and target feature stack are recorded in
`docs/aaa-lighting-research.md`.

Ownership lanes:

- Roadmap/integration/validation: `docs/roadmap.md`, screenshot evidence,
  validation command transcript.
- Material probe: `py/airlet_audio_lab/probe_model.py`, generated material
  reports under `target/`.
- Runtime lighting/materials: `src/scene.rs`, `src/model_view.rs`,
  `src/mechanism_view.rs`, `src/controls.rs`, and any dedicated lighting
  module.
- Debug/screenshot controls: `src/debug.rs`, `src/screenshot.rs`,
  `py/airlet_audio_lab/debug_client.py`, and MCP docs if the protocol changes.

Quality gates:

- Do not regress winding, twin, playback, comb animation, or schema-driven MCP
  actions.
- Keep imported asset files and `target/` outputs out of source changes unless
  explicitly promoted.
- Validate visual work with screenshots, not compilation alone.
- Keep lighting/material parameters centralized enough that presets can evolve
  without scattered magic numbers.

### Phase 1: Material And Lighting Audit

Checklist:

- [x] Generate a current mesh/material report for the closed model.
- [x] Include mesh index/name/group/material base color/metallic/roughness
  where available.
- [x] Capture baseline screenshots for product, crank, comb, cylinder, and lid
  views.
- [x] Define target material classes for wood/body, brass/gold metal, grey
  metal, dark cavity, platform, and accent parts.

Completion criteria:

- [x] `target/lighting-material-report.json` exists.
- [x] `target/lighting-material-report.md` exists.
- [x] Baseline screenshot files exist under `target/lighting/`.

### Phase 2: Centralized PBR Material Overrides

Checklist:

- [x] Introduce a centralized lighting/material config in the app layer.
- [x] Override imported materials by mesh group or semantic role.
- [x] Keep winding hover material visually distinct without removing the crank
  head or confusing the yellow front latch.
- [x] Preserve procedural cylinder, tooth, and comb material assignment.

Completion criteria:

- [x] Runtime model materials are assigned from the centralized config.
- [x] Metal/wood/platform classes have explicit PBR parameters.
- [x] Existing winding/twin/playback tests still pass.

### Phase 3: Studio Lighting Rig

Checklist:

- [x] Replace the one-off light setup with named key/fill/rim/accent/spot
  lights.
- [x] Use warm key light and neutral/cool rim separation.
- [x] Keep user/debug spotlight controls functional.
- [x] Centralize light defaults and camera-presentation defaults.

Completion criteria:

- [x] Scene spawns named light entities for the studio rig.
- [x] Debug state still reports current controllable spotlight values.
- [x] Product-view screenshot is visibly lit and nonblack.

### Phase 4: Reflection And Environment

Checklist:

- [x] Add an HDR/environment-lighting or Bevy-supported equivalent strategy.
- [x] Tune metal roughness/reflection readability for cylinder, comb, crank,
  pins, and latch.
- [x] Add a restrained readable background/plinth treatment.

Completion criteria:

- [x] Metal surfaces show distinct specular response in screenshots.
- [x] Background remains subordinate to the music box.

### Phase 5: Contact Detail

Checklist:

- [x] Tune shadow-map sizes, spot/directional shadow settings, and bias where
  available.
- [x] Strengthen contact shadows for pins, teeth, comb tines, crank shaft,
  hinge/latch, lid seam, and platform contact.
- [x] Add AO/cavity approximation where feasible without custom render-pipeline
  churn.

Completion criteria:

- [x] Close-up screenshots show small parts grounded rather than floating.
- [x] Shadows are not crushed into unreadable black.

### Phase 6: Camera And Post

Checklist:

- [x] Add deterministic presentation camera presets.
- [x] Enable/tune HDR camera behavior, tone mapping, exposure, and subtle bloom
  where supported.
- [x] Add optional macro close-up presentation without hiding functional
  mechanical detail.

Completion criteria:

- [x] Product, crank, comb, cylinder, and lid screenshot recipes are available.
- [x] Default interactive view remains readable.

### Phase 7: Reference And Regression

Checklist:

- [x] Add repeatable screenshot generation for the lighting preset set.
- [x] Check screenshots for size and brightness range.
- [x] Record screenshot paths and validation outcomes in this roadmap.

Completion criteria:

- [x] `target/lighting/` contains current validation screenshots.
- [x] Screenshot statistics are captured in the validation notes.

### Phase 8: Full Quality Gate

Checklist:

- [x] `cargo fmt --all --check`
- [x] `cargo test --workspace`
- [x] `uv run --project py python -m compileall py/airlet_audio_lab`
- [x] `uv run --project py airlet-probe-model`
- [x] `git diff --check`
- [x] Verify removed controls remain absent from action catalog and docs.

Completion criteria:

- [x] All validation commands pass.
- [x] Roadmap checkboxes above reflect completed implementation and evidence.

Validation notes:

- Material audit command: `uv run --project py airlet-probe-model`.
- Material audit outputs: `target/lighting-material-report.json` and
  `target/lighting-material-report.md`; report covers 75 meshes and 15 material
  summaries from the current GLB/spec pair.
- Runtime lighting implementation: `src/lighting.rs` owns the centralized
  `ExhibitLightingConfig`; scene/model/mechanism material assignment consumes
  this config.
- Studio rig entities: `Lighting Key`, `Lighting Fill`, `Lighting Rim`,
  `Lighting Accent`, and `Lighting Spot`.
- Camera/post stack: HDR camera, fixed indoor exposure, TonyMcMapface tone
  mapping, subtle bloom, screen-space ambient occlusion, contact shadows, and a
  Bevy solid-color environment map for IBL-style reflection/fill.
- Screen-space reflections were tested and removed from the runtime stack
  because Bevy 0.19 produced a Vulkan validation error when SSR and contact
  shadows were active together. The current reflection path is environment map
  plus explicit PBR metal/roughness tuning so small-part contact shadows stay
  stable.
- Screenshot command:
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60 --screenshot-timeout 45 --warmup-seconds 5`.
- Screenshot outputs: `target/lighting/product.png`,
  `target/lighting/crank.png`, `target/lighting/comb.png`,
  `target/lighting/cylinder.png`, and `target/lighting/lid.png`.
- Screenshot stats output:
  `target/lighting/lighting-screenshot-stats.json`.
- Screenshot luminance ranges:
  product `0.038933..0.953649`, crank `0.038933..0.941539`, comb
  `0.038933..0.96965`, cylinder `0.038933..0.961508`, lid
  `0.038933..0.949995`.
- Validation commands passed: `cargo fmt --all --check`,
  `cargo test --workspace`, `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-probe-model`, and
  `git diff --check`.

## Lighting Runtime Control Roadmap

Purpose: expose the AAA lighting rig as runtime controls without turning
`ExhibitLightingConfig` into mutable UI state. Config remains the source for
defaults and bounds; `ExhibitControls` owns the current interactive values.

Checklist:

- [x] Add current key/fill/rim/accent/ambient/environment intensity fields to
  `ExhibitControls`.
- [x] Add egui sliders for studio, ambient, and IBL intensity controls.
- [x] Add marker components for key/fill/rim/accent lights and update all light
  resources/entities from controls.
- [x] Extend `set_light` debug action and action schema with the new optional
  intensity parameters.
- [x] Extend `dump_state.light` so MCP/debug clients can inspect the complete
  rig state.
- [x] Validate with Rust tests and runtime debug smoke tests.

Validation notes:

- `describe_actions` reports `set_light` parameters:
  `yaw`, `pitch`, `inner_angle`, `outer_angle`, `intensity`, `key`, `fill`,
  `rim`, `accent`, `ambient`, and `environment`.
- Runtime `set_light` with `key=1200`, `fill=750`, `rim=1800`, `accent=450`,
  `ambient=24`, `environment=130`, and `intensity=900000` returned those values
  through `dump_state.light`.
- Validation passed: `cargo fmt --all --check`, `cargo test --workspace`,
  `uv run --project py python -m compileall py/airlet_audio_lab`, and runtime
  debug endpoint smoke checks.

## High-Contrast Lighting Mood Pass

Purpose: tune the default AAA lighting preset toward a stronger dark-scene,
spotlit mood inspired by the more realistic/high-contrast intent of
Complementary Unbound-style Minecraft shader presets.

Checklist:

- [x] Keep the black surrounding field and central spotlight as the main visual
  signal.
- [x] Lower ambient, fill, and environment/IBL intensity so shadow areas stay
  atmospheric.
- [x] Preserve rim/accent highlights for brass and silhouette separation.
- [x] Darken the plinth material enough that the stage does not read as a flat
  white studio sweep.
- [x] Update lighting screenshot recipes to use the stronger concentrated
  spotlight preset.
- [x] Validate with current lighting screenshots and brightness statistics.

Validation notes:

- `target/lighting/product.png` after the mood pass keeps a black background,
  central spotlit model, stronger cast shadow, and localized brass highlights.
- Product screenshot luminance moved from the previous roughly `0.038933..
  0.949429` with mean `0.340205` to `0.038933..0.938734` with mean `0.2348`,
  confirming darker midtones while retaining bright highlights.
- Lighting screenshot command:
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60 --screenshot-timeout 45 --warmup-seconds 5`.

## Lid Latch Inspection And Night Spotlight Follow-Up

Purpose: inspect the front latch with per-mesh render evidence before changing
rig membership, and keep the spotlight hard-edged by setting initial inner angle
equal to outer angle.

Checklist:

- [x] Re-run the model probe against the paired open/closed GLB.
- [x] Add a closed-model mesh contact sheet with one highlighted mesh per cell.
- [x] Add a front-latch-only contact sheet for meshes `10..17`.
- [x] Restore the lid pivot and axis to the paired-geometry probe result after
  the attempted `Mesh.001`/rear-pivot correction proved wrong.
- [x] Keep the front upper latch meshes `1`, `2`, `4`, `5`, `6`, `7`, `10`,
  `12`, `13`, `14`, `16`, and `17` in the lid group; keep lower latch meshes
  `11` and `15` fixed on the body.
- [x] Allow an obtuse maximum lid presentation angle while preserving the
  measured pivot and axis from the paired open/closed geometry.
- [x] Set default spotlight inner angle equal to outer angle and remove the
  runtime clamp that forced a soft `outer - 0.02` edge.
- [x] Clear old `target/lighting` screenshots and regenerate the current
  product/crank/comb/cylinder/lid validation set.

Validation notes:

- Mesh inspection outputs: `target/model-closed-mesh-contact-sheet.png` and
  `target/model-front-latch-contact-sheet.png`.
- Restored lid pivot: `[-0.95529, -0.938679, -1.266284]`.
- Restored lid axis: `[0.865899, -0.0, -0.500218]`.
- Product maximum lid angle: `-110.0` degrees.
- Lid meshes: `[0, 1, 2, 4, 5, 6, 7, 10, 12, 13, 14, 16, 17]`.
- Hinge meshes remain empty for this asset.
- Current screenshot recipes use equal inner/outer spotlight angles:
  product `0.13/0.13`, crank `0.12/0.12`, comb `0.13/0.13`,
  cylinder `0.13/0.13`, and lid `0.14/0.14`.
- Validation passed: `uv run --project py airlet-probe-model`,
  `cargo fmt --all --check`, `cargo check --workspace`,
  `cargo test --workspace`, `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-lighting-shots --launch
  --startup-timeout 60 --screenshot-timeout 45 --warmup-seconds 5`, and
  `git diff --check`.

## Procedural Texture Assignment Roadmap

Purpose: give the current model a usable texture pass without baking static
materials back into the GLB while mesh ownership and rigging are still being
validated.

Checklist:

- [x] Generate project-owned procedural PBR texture maps for lacquered wood,
  brass, steel, dark metal, and stage material.
- [x] Add a repeatable Python texture generator under `py/airlet_audio_lab`.
- [x] Assign generated textures in Bevy by semantic mesh/material class.
- [x] Keep dynamic state materials such as winding hover in Bevy rather than
  writing them into the model asset.
- [x] Regenerate current lighting screenshots after texture assignment.
- [x] Validate with Rust checks/tests, Python compile checks, and `git diff
  --check`.

Validation notes:

- Added `py/airlet_audio_lab/generate_textures.py` and the
  `airlet-generate-textures` CLI, then generated 18 PNG maps under
  `assets/textures/procedural`: base color, ORM, and normal maps for
  long-grain lacquered wood, end-grain lacquered wood, aged brass, polished
  steel, dark metal, and dark stage.
- The first wood pass exposed that GLB UV islands could rotate a single wood
  texture into the wrong direction on some faces. This was replaced by the
  geometry-aware remapping tracked in the Wood Grain Direction Roadmap below.
- `src/lighting.rs` still owns runtime texture-class material application for
  procedural mechanism/platform pieces and dynamic hover state. Static imported
  model material assignment has moved to the Python baked GLB path below.
- Refreshed lighting screenshots with `uv run --project py
  airlet-lighting-shots --launch --startup-timeout 60 --screenshot-timeout 45
  --warmup-seconds 5`. Current screenshot luminance means are product 0.0531,
  crank 0.0516, comb 0.0710, cylinder 0.0902, and lid 0.0632, preserving the
  dark high-contrast exhibit mood while showing texture detail.
- Validation passed: `uv run --project py airlet-generate-textures`,
  `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace`, and
  `uv run --project py python -m compileall py/airlet_audio_lab`.

## Python Material Baking Roadmap

Purpose: move wood-grain face splitting, UV generation, and static material
assignment out of Rust and into a repeatable Python asset-baking step. Rust
should load and render the baked model, while keeping only runtime/dynamic
visual state such as winding hover.

Checklist:

- [x] Add a Python material-baking CLI that reads the source GLB plus
  `assets/models/converted/spec.toml`.
- [x] Generate procedural wood/metal/stage texture maps as part of baking so
  the baked asset has all referenced texture files.
- [x] Split wood meshes in Python by triangle face orientation, generate
  model-basis UVs, and assign long-grain or end-grain materials.
- [x] Preserve non-wood mesh geometry, node transforms, mesh names, and material
  intent for brass, steel, dark metal, and dynamic winding-key parts.
- [x] Export a temporary baked GLB under `assets/generated/`.
- [x] Add an optional baked model path to the model spec and make Rust prefer
  that baked GLB when present.
- [x] Remove Rust runtime wood mesh splitting/UV generation so `model_view.rs`
  returns to rendering and rig assembly.
- [x] Regenerate screenshots from the baked asset and verify closed/open wood
  grain direction.
- [x] Run Python, Rust, screenshot, and diff validation.

Validation notes:

- Added `py/airlet_audio_lab/bake_materials.py` and registered the
  `airlet-bake-materials` CLI. The baker reads
  `assets/models/converted/spec.toml`, regenerates procedural texture maps,
  and writes `assets/generated/music_box_material_baked.glb` plus
  `assets/generated/music_box_material_baked.json`.
- The baked GLB preserves the source mesh/node counts: 75 meshes and 79 nodes.
  Wood meshes `0` and `8` stay at their original mesh indices but now contain
  two primitives each, with long-grain and end-grain material assignment.
- `assets/models/converted/spec.toml` now declares
  `baked_gltf = "generated/music_box_material_baked.glb"`, and `src/scene.rs`
  prefers that baked path while retaining fallback to the source `gltf`.
- `src/model_view.rs` no longer performs runtime wood mesh splitting, UV
  generation, or tangent generation. It preserves GLTF material textures and
  applies only runtime lighting material tuning plus dynamic winding hover.
- Refreshed screenshots with `uv run --project py airlet-lighting-shots
  --launch --startup-timeout 60 --screenshot-timeout 45 --warmup-seconds 5`.
  Current screenshot luminance means are product 0.0534, crank 0.0619, comb
  0.0951, cylinder 0.1027, and lid 0.0719.
- Validation passed: `uv run --project py airlet-bake-materials`,
  `uv run --project py python -m compileall py/airlet_audio_lab`,
  `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace`, and
  the screenshot capture command above.

## Wood Material Simulation Calibration

Purpose: improve the baked wood material so it reads as sampled from a larger
wood volume rather than as a repeated procedural decal.

Checklist:

- [x] Change end-grain texture generation to represent a local window from a
  much larger annual-ring scale, with only a small visible arc span.
- [x] Add deterministic per-mesh UV windows in the Python baker so lid and body
  sample different regions of the wood field instead of looking like stretched
  copies.
- [x] Break up long-grain regularity with uneven latewood bands, variable fiber
  density, pores, and low-frequency grain drift while preserving the width-axis
  direction.
- [x] Regenerate the baked GLB and screenshots.
- [x] Validate Python compile, Rust checks/tests, screenshot capture, and diff
  hygiene.

Validation notes:

- Updated `py/airlet_audio_lab/generate_textures.py` so longitudinal wood
  better matches the reference photo: warmer red-brown lacquer color, lower
  contrast, finer pores/fibers, and weaker normal relief.
- Updated end-grain generation to sample a far larger annual-ring field, so
  visible rings are local arcs rather than full target-like rings.
- Added deterministic per-mesh UV windows in `py/airlet_audio_lab/bake_materials.py`.
  Lid/body wood meshes now sample different subregions of the same procedural
  wood field while preserving repeatable builds.
- Rebuilt `assets/generated/music_box_material_baked.glb` with
  `uv run --project py airlet-bake-materials` and refreshed screenshots with
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60
  --screenshot-timeout 45 --warmup-seconds 5`.
- Current screenshot luminance means are product 0.0538, crank 0.0632, comb
  0.0944, cylinder 0.1009, and lid 0.0724.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo check --workspace`, and
  `cargo test --workspace`.

## Wood Fiber Detail Calibration

Purpose: reduce visible wave density on cut faces and make longitudinal wood
detail less periodic by adding natural elongated inclusions aligned with the
grain.

Checklist:

- [x] Stretch the longitudinal scale of the fiber/band fields to reduce dense
  wave artifacts on visible cuts.
- [x] Replace overly regular color-block variation with lower-frequency,
  less periodic modulation.
- [x] Add narrow grain-aligned impurities and vertical elliptical bubble/cloud
  inclusions to the longitudinal wood field.
- [x] Rebuild the baked GLB and refresh screenshots.
- [x] Validate Python compile, Rust checks/tests, screenshot capture, and diff
  hygiene.

Validation notes:

- Removed the remaining periodic `sin(y * frequency)` sources for long-grain
  fibers, hair fibers, and pore bands. Longitudinal detail now comes from
  anisotropic, grain-aligned stochastic fiber fields instead of regular
  stripes.
- Added non-periodic micro-fibers plus grain-aligned impurities and elongated
  bubble/cloud inclusions. Normal relief was restored from the same
  non-periodic height field so the wood surface keeps fiber texture without
  reintroducing visible cycles.
- Fixed a baking artifact where split wood triangles used averaged/model-space
  normals, which could expose diagonal triangle seams. The baker now writes
  original local vertex normals and generates tangents from the local triangle
  geometry.
- Rebuilt `assets/generated/music_box_material_baked.glb` with
  `uv run --project py airlet-bake-materials` and refreshed screenshots with
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60
  --screenshot-timeout 45 --warmup-seconds 5`.
- Current screenshot luminance means are product 0.0532, crank 0.0616, comb
  0.0945, cylinder 0.1004, and lid 0.0711.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo check --workspace`, and
  `cargo test --workspace`.

## Wood Aging And Short-Fiber Calibration

Purpose: make the longitudinal wood surface less smooth without returning to
periodic bands. The long-grain material should use larger warm color regions,
short overlapping fiber relief, and a small amount of aged longitudinal
cracking.

Checklist:

- [x] Add yellow/honey alternate tones to the long-grain color palette.
- [x] Enlarge the long-grain color-band scale so color regions read as broader
  board variation instead of tight stripes.
- [x] Replace most full-length normal relief with overlapping short,
  grain-aligned fiber patches.
- [x] Add sparse, deeper longitudinal aged crack streaks to base color,
  roughness, and normal height.
- [x] Rebuild the baked GLB and refresh lighting screenshots.
- [x] Validate Python compile, Rust checks/tests, screenshot capture, and diff
  hygiene.

Validation notes:

- Long-grain base color now mixes in a lower-frequency honey/yellow palette and
  uses broader board-tone modulation so the visible color regions are larger.
- Longitudinal normal relief now comes primarily from short overlapping
  grain-aligned fiber patches, with reduced contribution from longer fibers.
- Sparse aged crack streaks darken base color, raise local roughness, and cut
  into the height map for slight old-surface relief.
- Rebuilt `assets/generated/music_box_material_baked.glb` with
  `uv run --project py airlet-bake-materials` and refreshed screenshots with
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60
  --screenshot-timeout 45 --warmup-seconds 5`.
- Current screenshot luminance means are product 0.0581, crank 0.0659, comb
  0.1029, cylinder 0.1086, and lid 0.0749.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo check --workspace`, and
  `cargo test --workspace`.

## Extreme Fine-Fiber Calibration

Purpose: push the long-grain normal detail toward very fine, dense,
same-direction short fibers. The base color should keep board-scale variation,
while the normal map carries a much denser woven fiber surface.

Checklist:

- [x] Increase short-fiber density substantially without adding periodic bands.
- [x] Make individual fiber patches finer and shorter.
- [x] Increase normal relief from fine fibers while keeping base-color noise
  controlled.
- [x] Rebuild the baked GLB and refresh lighting screenshots.
- [x] Validate Python compile, Rust checks/tests, screenshot capture, and diff
  hygiene.

Validation notes:

- Increased short-fiber patches from 1250 to 3600 and reduced their length and
  width ranges, keeping the field stochastic and non-periodic.
- Reduced base-color contribution from fiber fields while raising normal-height
  contribution and normal strength, so the dense fiber effect lives primarily in
  surface relief.
- Rebuilt `assets/generated/music_box_material_baked.glb` with
  `uv run --project py airlet-bake-materials` and refreshed screenshots with
  `uv run --project py airlet-lighting-shots --launch --startup-timeout 60
  --screenshot-timeout 45 --warmup-seconds 5`.
- Current screenshot luminance means are product 0.0583, crank 0.0663, comb
  0.1022, cylinder 0.1070, and lid 0.1416.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo check --workspace`, and
  `cargo test --workspace`.

## Extreme Tangential Fiber Calibration

Purpose: keep the current longitudinal fiber reach, but push the tangential
spacing and width much finer. The result should read as many more parallel,
same-direction fibers packed across the wood surface rather than shorter
segments.

Checklist:

- [ ] Preserve the longitudinal short-fiber length range.
- [ ] Increase tangential fiber density with a higher patch count.
- [ ] Make tangential fiber width substantially finer.
- [ ] Rebuild the baked GLB and refresh lighting screenshots.
- [ ] Validate Python compile, Rust checks/tests, screenshot capture, and diff
  hygiene.

## Wood Preset Research And Selection Gallery

Purpose: stop overfitting one procedural wood recipe and generate several
visually distinct, research-informed wood candidates for selection. This pass
does not choose a production material automatically; it creates a comparison
gallery under `target/wood-presets/`.

Research notes:

- Realistic procedural wood should be treated as a structured material, not a
  single noisy stripe texture. Useful layers include growth rings, color
  variation, pores, rays, growth distortion, and fiber-direction-driven
  specular or normal detail.
- Solid/volumetric wood texturing is preferable for future production because
  it evaluates material from 3D position and avoids UV-island discontinuities
  and side/cut-face disagreement.
- Noise should distort ring and grain coordinates instead of simply drawing
  perfect waves; otherwise the result becomes regular and artificial.
- Very high-frequency procedural detail needs band-limiting or enough texture
  resolution, otherwise moire and aliasing can look like cross-direction
  fibers.
- Research references consulted during this pass:
  - Cornell solid wood structure/texture work:
    `https://www.cs.cornell.edu/projects/wood/`
  - Inigo Quilez procedural material notes:
    `https://iquilezles.org/articles/`
  - Blender procedural texture practice for rings/noise/wood-like materials:
    `https://docs.blender.org/manual/en/latest/render/shader_nodes/textures/wave.html`

Preset candidates to generate:

- [x] `mahogany_lacquer`: warm red-brown furniture wood with broad board color
  variation and subtle pores.
- [x] `walnut_oil`: darker chocolate walnut with smoky long-grain clouds and
  sparse open pores.
- [x] `cherry_aged`: orange-red cherry with fine pores, mild darkening, and
  restrained aging.
- [x] `oak_quarter_sawn`: amber oak with stronger ray fleck hints and porous
  longitudinal grain.
- [x] `maple_satin`: lighter maple with low pore contrast and dense fine
  fibers.
- [x] `rosewood_gloss`: higher-contrast dark/red bands with glossy finish and
  deeper streaks.
- [x] `teak_worn`: yellow-brown worn teak with rougher surface and softened
  cracks.

Checklist:

- [x] Add a Python preset-gallery generator that writes base, height, normal,
  roughness, and preview images.
- [x] Generate at least five preset candidates; target is seven.
- [x] Write a contact sheet and preset manifest for selection.
- [x] Keep generated candidate images under `target/wood-presets/` instead of
  replacing the production baked GLB.
- [x] Validate the Python module with `uv run --project py python -m
  compileall py/airlet_audio_lab`.
- [x] Schedule automatic shutdown after all generation and validation are
  complete.

Generated selection artifacts:

- Contact sheet: `target/wood-presets/wood_preset_contact_sheet.png`.
- Manifest: `target/wood-presets/wood_preset_manifest.md` and
  `target/wood-presets/wood_preset_manifest.json`.
- Each preset directory contains `base`, `height`, `normal`, `roughness`, and
  `preview` PNG outputs.
- These are candidate images only; no production baked model/material was
  replaced during this gallery pass.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab` and a smoke generation run with
  `uv run --project py airlet-wood-preset-gallery --size 512 --out-dir
  target/wood-presets-smoke`.
- Automatic shutdown scheduled for `2026-07-01 01:05:04 CST` with
  `shutdown -h +1`.

## Longitudinal Wood Gallery Rebuild

Purpose: discard the previous preset-gallery direction and rebuild the wood
candidate generator specifically for longitudinal-cut wood. The previous
gallery overemphasized rings, cracks, and blotches, and did not produce a
convincing long-grain fiber surface.

Target material features:

- Fibers run along the horizontal/longitudinal direction.
- Tangential variation is dense and fine, but the texture should read as long
  interwoven grain rather than short scratches.
- Board-scale color variation should be slow and longitudinal, not regular
  bands perpendicular to the grain.
- Pores and vessels should be elongated along the grain.
- Normal/height should prioritize fine long-grain relief and subtle silk-like
  highlights.
- Cut-face/end-grain rings are out of scope for this gallery.

Checklist:

- [x] Replace the old gallery generator with a longitudinal-cut-only texture
  model.
- [x] Generate at least five selectable longitudinal presets; target remains
  seven.
- [x] Write base, height, normal, roughness, preview, manifest, and contact
  sheet outputs under `target/wood-presets/`.
- [x] Inspect the generated contact sheet for obvious non-longitudinal artifacts.
- [x] Validate Python compilation and diff hygiene.
- [x] Schedule automatic shutdown after completion.

Validation notes:

- Replaced `py/airlet_audio_lab/wood_preset_gallery.py` with a longitudinal
  generator based on anisotropic Gaussian fields: large smoothing along the
  grain direction, dense variation across the tangent direction, and flow-field
  warping to avoid perfectly straight scan-line bands.
- The gallery now produces seven longitudinal presets:
  `mahogany_long_lacquer`, `walnut_long_oil`, `cherry_long_aged`,
  `oak_long_porcellous`, `maple_long_satin`, `rosewood_long_gloss`, and
  `teak_long_worn`.
- Output directory `target/wood-presets/` is cleared before generation so stale
  candidate directories do not survive between runs.
- Generated selection artifacts:
  `target/wood-presets/wood_preset_contact_sheet.png`,
  `target/wood-presets/wood_preset_manifest.md`, and
  `target/wood-presets/wood_preset_manifest.json`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab` and `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`.
- Automatic shutdown scheduled for `2026-07-01 03:21:11 CST` with
  `shutdown -h +1`.

## Reference Longitudinal Wood Alignment

Purpose: align the procedural longitudinal wood candidates with the downloaded
reference image at `docs/imgs/wood.jpg`. The target is a light yellow-brown
longitudinal cut surface with dense, discontinuous fibers and restrained
surface relief.

Reference-derived target traits:

- Base color should be pale honey/yellow-brown rather than dark red/brown.
- The visible realism should come primarily from base-color fibers, not an
  aggressive normal map.
- Fine dark fibers should be dense, thin, grain-aligned, and discontinuous.
- Medium fibers should appear in local bundles with slight drift and thickness
  variation.
- Large board color variation should be broad and low contrast.
- Deep pores/checks should be sparse, very thin, and elongated along the grain.
- Avoid full-width scan-line stripes and perpendicular ring bands on this
  longitudinal surface.

Checklist:

- [x] Add a reference-oriented `light_oak_reference` preset.
- [x] Rework the generator so base color carries dense discontinuous fibers.
- [x] Keep normal relief subtle and derived from the same fine-fiber structure.
- [x] Regenerate the contact sheet and per-preset maps under
  `target/wood-presets/`.
- [x] Inspect output against `docs/imgs/wood.jpg` for obvious direction,
  color, and stripe artifacts.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Validation notes:

- Added `light_oak_reference` as the first candidate in the preset gallery.
- Added finite, discontinuous longitudinal streak synthesis for dark fibers,
  medium fibers, and pale threads. These line fields are horizontally aligned,
  lightly waved, and finite-length, avoiding the earlier full-width scan-line
  look.
- Reduced reliance on normal relief. The reference-aligned candidate now gets
  most visible fiber detail from base color, with normal height derived from the
  same fibers.
- Generated final candidate outputs with
  `uv run --project py airlet-wood-preset-gallery --size 768`.
- Contact sheet: `target/wood-presets/wood_preset_contact_sheet.png`.
- Reference color mean from `docs/imgs/wood.jpg`: RGB approximately
  `[0.651, 0.491, 0.275]`; generated `light_oak_reference` base color mean:
  RGB approximately `[0.675, 0.485, 0.254]`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Shared 3D Wood Volume Alignment

Purpose: make longitudinal and cross-cut wood candidates come from the same
procedural wood volume. The longitudinal board should show earlywood/latewood
variation that corresponds to the same growth-ring radius field visible on the
cross-cut preview.

Target behavior:

- Define a model-local 3D wood coordinate system in the Python material
  generator: length axis, radial axis, and tangential axis.
- Use one tree-center/radius field for both cross-cut rings and longitudinal
  board bands.
- Generate a longitudinal preview by sampling a thin board window through that
  volume.
- Generate a cross-cut preview by sampling the perpendicular section of the
  same volume.
- Keep longitudinal fibers dense and discontinuous, but modulate their color
  and relief by the shared earlywood/latewood ring field.
- Keep generated gallery artifacts under `target/wood-presets/`; do not replace
  the production baked GLB in this pass.

Checklist:

- [x] Extend the wood preset generator with shared 3D wood-volume parameters.
- [x] Generate both longitudinal and cross-cut previews for each preset.
- [x] Add a contact sheet that shows longitudinal/cross-cut pairs.
- [x] Make `light_oak_reference` use the shared ring field while preserving the
  downloaded reference's pale yellow, discontinuous-fiber appearance.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Validation notes:

- Added `WoodVolume` parameters to `py/airlet_audio_lab/wood_preset_gallery.py`.
  Each preset now owns a tree-center/radius setup, ring frequency, phase,
  jitter, and sampled board window.
- Longitudinal `base` generation now uses `_longitudinal_ring_field`, derived
  from the same radius/ring response used by `_crosscut_preview`.
- Each preset now emits `*_crosscut.png` next to `base`, `height`, `normal`,
  `roughness`, and `preview`.
- The contact sheet now displays paired rows: longitudinal preview followed by
  the cross-cut preview for the same preset. Output path:
  `target/wood-presets/wood_preset_contact_sheet.png`.
- The cross-cut preview intentionally shows a low-curvature local window of a
  larger tree radius, so it reads as 1-2 broad ring layers instead of a full
  bullseye.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Crosscut Ring Sharpness And Longitudinal Cut Modes

Purpose: improve the shared wood-volume gallery after visual review. The
cross-cut preview currently reads too blurred, and the longitudinal preview
only covers one kind of board slice. Real boards may be closer to radial cut or
tangential/flat cut, and these should produce visibly different longitudinal
grain.

Target behavior:

- Cross-cut rings should have clearer earlywood/latewood boundaries instead of
  broad blurry bands.
- Cross-cut previews should include fine pores and medullary/ray-like radial
  detail so they read as cut wood rather than blurred color stripes.
- Longitudinal outputs should include a radial/quarter-cut variant and a
  tangential/flat-cut variant derived from the same wood volume.
- Radial longitudinal cuts should show straighter, more parallel ring bands.
- Tangential longitudinal cuts should show more arched/cathedral-like ring
  movement while preserving fine lengthwise fibers.
- Contact sheets and manifests should expose the cut mode clearly.

Checklist:

- [x] Add explicit longitudinal cut modes to the Python wood-volume generator.
- [x] Emit radial and tangential longitudinal previews/maps for each preset.
- [x] Sharpen cross-cut ring response and add pore/ray detail.
- [x] Regenerate `target/wood-presets/` contact sheet and manifests.
- [x] Inspect generated pairs for radial/tangential distinction and cross-cut
  clarity.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Validation notes:

- Added a `mode` parameter to longitudinal ring sampling. `radial` mode samples
  across radius for straighter quarter-cut-like bands; `tangential` mode samples
  near a fixed radius with a crown curve for flat-cut/cathedral-like movement.
- Each preset now emits tangential outputs in addition to the existing radial
  outputs: `*_tangential_base.png`, `*_tangential_height.png`,
  `*_tangential_normal.png`, `*_tangential_roughness.png`, and
  `*_tangential_preview.png`.
- The contact sheet now uses three rows per preset group: radial, tangential,
  and cross-cut.
- Cross-cut rings were sharpened with narrower latewood lines, explicit ring
  edge shading, stronger pore visibility, and stronger medullary/ray-like
  detail.
- Regenerated `target/wood-presets/wood_preset_contact_sheet.png` and
  `target/wood-presets/wood_preset_manifest.md`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Crosscut Wood Anatomy Correction

Purpose: fix the cross-cut preview after visual review. The current cross-cut
output reads like smooth three-color bands instead of wood because ring bands
are too broad and soft, while anatomical details are too weak.

Target behavior:

- Cross-cut color should be lower-contrast and less saturated in large bands.
- Annual rings should be readable through thin latewood boundaries, not through
  huge smooth blocks.
- Add visible vessel/duct pores as small dark dots and short pores distributed
  by ring phase.
- Add radial medullary/ray-like lines that cut across rings.
- Add fine cross-grain noise so the surface reads as cut wood fiber, not
  airbrushed color.
- Preserve existing radial and tangential longitudinal outputs.

Checklist:

- [x] Rework `_crosscut_preview` to emphasize wood anatomy instead of broad
  color bands.
- [x] Add ring-phase-aware pore and ray detail.
- [x] Emit `*_crosscut_height.png`, `*_crosscut_normal.png`, and
  `*_crosscut_roughness.png`.
- [x] Regenerate `target/wood-presets/` and inspect cross-cut outputs.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Implementation notes:

- Cross-cut output now reduces broad ring-band contrast and moves the material
  read into thin latewood boundaries, ring-edge darkening, fine end-grain cell
  walls, ring-phase-weighted pores, and radial medullary/ray-like highlights.
- Cross-cut maps are emitted alongside the base texture:
  `*_crosscut_height.png`, `*_crosscut_normal.png`, and
  `*_crosscut_roughness.png`.
- Regenerated `target/wood-presets/` at 768px and inspected
  `light_oak_reference_crosscut.png`, `oak_long_porcellous_crosscut.png`, and
  `oak_long_porcellous_crosscut_normal.png`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab` and `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`.

## Reference Crosscut Texture Calibration

Purpose: align cross-cut/end-grain candidates with the supplied reference image
at `docs/imgs/cross.jpeg`. The previous cross-cut pass improved fine detail,
but still has abrupt broad color blocks, blurred ring boundaries, and weak
surface roughness.

Reference-derived target traits:

- Replace smooth broad color bands with smaller irregular board-color patches.
- Make dark annual-ring/crack boundaries narrower, harder, and higher contrast.
- Add rough end-grain relief visible in height, normal, and roughness maps.
- Add sparse dark mineral/check cracks that follow ring curvature but break up
  irregularly.
- Give annual-ring bands random but continuous color offsets so visible bands
  do not share identical periodic color distribution and do not change color as
  hard steps.
- Allow the displayed cross-cut and longitudinal cuts to be perpendicular
  oblique slices of the same wood volume instead of axis-perfect textbook
  cross/longitudinal cuts.
- Increase visible ring coverage and make ring curvature more distorted, with
  stronger shape drift across bands.
- Represent longitudinal fiber growth as a gently curved spatial flow whose
  main direction is lengthwise but whose radial/tangential position drifts.
- Preserve radial and tangential longitudinal outputs.

Checklist:

- [x] Add hard-edged cross-cut boundary/crack fields.
- [x] Add irregular patch-color variation without large soft blocks.
- [x] Add continuous per-ring color offsets for non-periodic annual-band color.
- [x] Add oblique cross-cut sampling with stronger nonlinear ring distortion.
- [x] Increase visible ring coverage while preserving broad, nonuniform bands.
- [x] Add curved fiber-flow warping to longitudinal fibers.
- [x] Increase end-grain height and roughness contrast.
- [x] Regenerate `target/wood-presets/` and inspect reference-facing outputs.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Implementation notes:

- Cross-cut sampling now uses an oblique slice through the shared wood volume
  with rotation, shear, bowing, saddle distortion, and low-frequency drift. The
  resulting rings cover more bands on screen and are no longer axis-perfect
  horizontal contours.
- Annual-band color variation now samples continuous random control points with
  interpolation over ring position. This keeps adjacent band peak/valley color
  differences without introducing instant color steps at band boundaries.
- Longitudinal sampling now includes a curved fiber-flow displacement so fibers
  remain primarily lengthwise but drift through radial/tangential coordinates.
- Regenerated `target/wood-presets/` at 768px and inspected
  `oak_long_porcellous_crosscut.png` plus the corresponding normal map.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Longitudinal Spatial Slice Correction

Purpose: fix the longitudinal wood candidates after identifying two issues:
annual-ring color on longitudinal cuts still changed too abruptly, and the
tangential longitudinal variant mixed spatial ring sampling with screen-space
hardcoded horizontal fiber generation.

Target behavior:

- Longitudinal annual-ring color should use continuous random control points
  over ring position, matching the corrected cross-cut behavior.
- Radial and tangential longitudinal variants should both be sampled from an
  explicit length/radial/tangent slice through the same wood volume.
- Fiber growth should remain primarily lengthwise while drifting through
  radial/tangential space via a curved fiber-flow field.
- Tangential longitudinal cuts should not show large screen-space side blocks
  caused by a hardcoded x-axis crown term.
- Existing output filenames and manifest shape should remain stable.

Checklist:

- [x] Add a `LongitudinalSlice` data structure for length/radial/tangent/radius
  sampling.
- [x] Replace direct longitudinal base-color dependence on `_ring_response`
  with continuous annual color offsets plus only mild ring-response modulation.
- [x] Drive longitudinal fiber warping from the slice's curved fiber-flow field.
- [x] Reduce tangential crown artifacts and make tangential variation come from
  oblique slice geometry plus continuous flow.
- [x] Regenerate `target/wood-presets/` and inspect the tangential longitudinal
  output.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Implementation notes:

- `py/airlet_audio_lab/wood_preset_gallery.py` now constructs a
  `LongitudinalSlice` before longitudinal material synthesis. The slice owns
  length, radial, tangent, radius, ring response, continuous annual color, and
  fiber-flow displacement.
- Longitudinal color now uses `_longitudinal_annual_color`, which interpolates
  random ring-position control points instead of making hard per-ring steps.
- The tangential cut crown term was reduced and mixed with oblique/flow terms,
  removing the previous large left/right block artifact.
- Regenerated `target/wood-presets/` at 768px and inspected
  `oak_long_porcellous_tangential_base.png`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Shared Annual Ring Color Accumulation

Purpose: make annual-ring color evolve like growth history rather than local
independent noise. The cumulative ring color trend must affect radial
longitudinal, tangential longitudinal, and cross-cut outputs through one shared
profile for each preset.

Target behavior:

- Annual-ring color includes a low-frequency cumulative random-walk trend.
- Adjacent bands remain smooth because sampling interpolates over continuous
  ring position.
- Distant bands can drift warmer/cooler or lighter/darker because the random
  walk accumulates over ring index.
- Longitudinal and cross-cut renders for the same preset sample the same
  annual-ring color profile instead of generating separate random profiles.
- Existing generated filenames and manifest layout remain stable.

Checklist:

- [x] Add an `AnnualRingColorProfile` generated once per preset.
- [x] Thread the shared profile through radial longitudinal, tangential
  longitudinal, and cross-cut rendering.
- [x] Replace separate longitudinal/cross-cut random offset generation with
  shared profile sampling.
- [x] Regenerate `target/wood-presets/` and inspect longitudinal/cross-cut
  consistency.
- [x] Validate Python compilation, smoke generation, and diff hygiene.

Implementation notes:

- `AnnualRingColorProfile` is created once per preset in `_render_preset`.
  Radial longitudinal, tangential longitudinal, and cross-cut rendering all
  receive the same profile.
- The profile combines a smoothed random walk, local ring variation, and a slow
  sinusoidal arc. Sampling uses interpolation over continuous ring position, so
  adjacent bands remain smooth while distant bands accumulate a visible color
  drift.
- `_longitudinal_annual_color` and `_crosscut_ring_color_offsets` now sample the
  shared profile instead of generating separate independent ring-color offsets.
- Regenerated `target/wood-presets/` at 768px and inspected
  `oak_long_porcellous_crosscut.png` and
  `oak_long_porcellous_tangential_base.png`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-wood-preset-gallery
  --size 512 --out-dir target/wood-presets-smoke`, and `git diff --check`.

## Walnut Three-Axis Production Wood Material

Purpose: promote the selected `walnut_long_oil` wood preset from gallery output
into the production baked model material. The model has three useful orthogonal
wood-face directions, so radial longitudinal, tangential longitudinal, and
cross-cut maps should all be used instead of collapsing production wood into
one long-grain material plus one end-grain material.

Target behavior:

- `walnut_long_oil` is the selected production wood preset.
- Production texture generation emits three walnut wood PBR material stems:
  radial, tangential, and cross.
- The baked GLB has three wood materials and assigns wood triangles by dominant
  model-space face normal.
- Existing metal/stage texture generation remains intact.
- Baked model path and manifest shape remain compatible with the Rust app.

Checklist:

- [x] Generate production walnut radial/tangential/cross base, normal, and ORM
  maps from the preset-gallery renderer.
- [x] Preserve compatibility aliases for old long/end wood texture stems if
  useful for existing tooling.
- [x] Update the material baker to append three wood materials.
- [x] Split wood triangles into radial, tangential, and cross groups by
  dominant basis-axis normal and assign matching UV projections.
- [x] Rebuild `assets/generated/music_box_material_baked.glb` and inspect the
  bake report for three wood split counts.
- [x] Validate Python compilation, texture generation, baking, screenshot
  rendering, and diff hygiene.

Implementation notes:

- `py/airlet_audio_lab/generate_textures.py` now selects `walnut_long_oil` and
  emits `walnut_wood_radial_*`, `walnut_wood_tangential_*`, and
  `walnut_wood_cross_*` PBR texture maps. Legacy `lacquered_wood_*` and
  `lacquered_wood_end_*` aliases are still emitted for compatibility.
- `py/airlet_audio_lab/bake_materials.py` now appends three walnut wood
  materials and classifies wood triangles by dominant model-space basis normal:
  front-facing faces use radial, up-facing faces use tangential, and
  right-facing cut faces use cross.
- Rebuilt `assets/generated/music_box_material_baked.glb`. The bake report
  splits mesh `0` into radial `8`, tangential `10`, cross `8` triangles and
  mesh `8` into radial `267`, tangential `236`, cross `373` triangles.
- Screenshot validation passed with
  `AIRLET_SCREENSHOT=target/walnut-production-screenshot.png cargo run --bin
  airlet`; the rendered screenshot is nonblank with mean luminance `0.0647`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-generate-textures --out-dir
  target/walnut-production-textures --size 256`, `uv run --project py
  airlet-bake-materials`, `cargo check --workspace`, and `git diff --check`.

## Rounded Wood Shell Volume Bake

Purpose: round the exterior wooden shell edges and keep walnut wood texture
continuous enough for the rounded geometry. The rounded shell should be an
intermediate generated source model; Rust should still load the final baked GLB
through the existing `baked_gltf` spec path.

Target behavior:

- Blender bevels only the wooden shell meshes before material baking.
- The bevel is small and segmented, improving external box edges without
  moving mechanism parts or changing the app rig API.
- The material baker consumes the rounded intermediate GLB and writes the
  existing `assets/generated/music_box_material_baked.glb`.
- Wood faces on the rounded shell still use walnut radial/tangential/cross
  material assignment based on model-space normals.
- Screenshot validation confirms the rounded baked model loads and renders.

Checklist:

- [x] Add a Blender-driven rounded-shell generator for wood meshes.
- [x] Integrate rounded-shell generation into `airlet-bake-materials`.
- [x] Make wood mesh detection robust enough for the Blender-exported
  intermediate source.
- [x] Rebuild generated textures, rounded intermediate GLB, and final baked GLB.
- [x] Inspect bake report for rounded-source triangle counts and three walnut
  wood material classes.
- [x] Validate Python compilation, bake, Rust check/test, screenshot render, and
  diff hygiene.

Implementation notes:

- Added `py/airlet_audio_lab/round_wood_shell.py`, a Blender-driven generator
  that imports the source GLB, bevels wood meshes `Mesh` and `Mesh.008`, applies
  weighted normals, and exports `assets/generated/music_box_rounded_shell.glb`.
- `airlet-bake-materials` now runs the rounded-shell generator by default before
  baking. `--skip-rounding` remains available for debugging.
- `py/airlet_audio_lab/bake_materials.py` now identifies wood meshes by both
  stable mesh indices and stable mesh names, which keeps the baker robust
  against Blender-exported intermediate files.
- Rounded source geometry keeps the original 75 mesh / 79 node structure.
  `Mesh.008` increases from 876 source triangles to 3192 rounded-source
  triangles; the final baked mesh preserves 3173 non-degenerate triangles across
  the three walnut wood materials.
- Final bake report uses `assets/generated/music_box_rounded_shell.glb` as the
  source and splits mesh `8` into cross `969`, radial `943`, and tangential
  `1261` triangles.
- Screenshot validation passed with
  `AIRLET_SCREENSHOT=target/rounded-walnut-shell.png cargo run --bin airlet`;
  the rendered screenshot is nonblank with mean luminance `0.0647`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-bake-materials`,
  `cargo check --workspace`, `cargo test --workspace`, and `git diff --check`.

## Rounded Walnut Scale Calibration

Purpose: apply the selected physical and material scale changes on top of the
rounded walnut production bake.

Target behavior:

- Wooden shell bevel radius is `0.003` model units, treated as 3mm in the
  current model pipeline.
- Bevel angle threshold is increased from the initial rounded-shell default so
  only stronger exterior edges are rounded.
- Longitudinal walnut texture scale is reduced to `0.5` on the model, meaning
  radial/tangential UV span doubles and the visible fiber texture becomes
  denser.
- Annual-ring scale is increased by `2x`, meaning procedural ring frequency is
  halved.
- The final baked GLB and screenshot validation use the calibrated rounded
  shell.

Checklist:

- [x] Set default bevel width to `0.003` and increase bevel angle.
- [x] Thread bevel angle through `airlet-bake-materials` into the Blender
  rounded-shell generator.
- [x] Double radial/tangential walnut UV span in the baker.
- [x] Halve annual-ring frequency in the shared wood-volume generator.
- [x] Rebuild textures, rounded shell, and final baked GLB.
- [x] Validate Python compilation, bake report, screenshot render, Rust check,
  and diff hygiene.

Implementation notes:

- `airlet-bake-materials` now defaults to `--bevel-width 0.003` and
  `--bevel-angle-degrees 45.0`. The angle parameter is passed through to the
  Blender rounded-shell generator.
- `round_wood_shell.py` has matching standalone defaults: width `0.003`,
  segments `5`, and angle `45.0`.
- Radial and tangential walnut UV spans are multiplied by `2.0`, making the
  longitudinal texture appear twice as dense on the model.
- `wood_preset_gallery.py` defines `ANNUAL_RING_SCALE = 2.0` and halves the
  procedural ring frequency, doubling annual-ring scale.
- Rebuilt `assets/generated/music_box_rounded_shell.glb` and
  `assets/generated/music_box_material_baked.glb`. The rounded source keeps
  `75` meshes and `79` nodes, with total triangles increasing from `116999` to
  `117130`.
- Final bake report uses the rounded source and splits wood mesh `0` into
  cross `8`, radial `8`, tangential `10`; wood mesh `8` into cross `377`,
  radial `273`, tangential `353`.
- Screenshot validation passed with
  `AIRLET_SCREENSHOT=target/rounded-3mm-walnut-scale.png cargo run --bin
  airlet`; the rendered screenshot is nonblank with mean luminance `0.0652`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-generate-textures --out-dir
  target/rounded-scale-textures --size 256`, `uv run --project py
  airlet-bake-materials`, `cargo check --workspace`, and `git diff --check`.

## Rounded Shell Bevel Effectiveness Fix

Purpose: fix the rounded-shell pass after visual review showed the exterior
wooden box still reads as sharp-edged. The generated triangle counts confirm
the bevel barely affected the body mesh, which indicates the imported wood mesh
has split vertices/edges that prevent an angle-limited bevel from seeing the
actual exterior corners.

Target behavior:

- Weld wood shell vertices by a tiny distance before applying the bevel.
- Keep the requested `0.003` bevel width and increased angle threshold.
- Ensure rounded source triangle counts visibly increase on the exterior shell
  instead of only adding a handful of triangles.
- Rebuild the rounded source and final baked walnut GLB.
- Validate with screenshot and bake report.

Checklist:

- [x] Add pre-bevel merge-by-distance to the Blender rounded-shell generator.
- [x] Expose merge distance through `airlet-bake-materials`.
- [x] Rebuild the rounded shell and confirm the wood body triangle count
  increases materially.
- [x] Rebuild the final baked GLB and screenshot-validate the rounded shell.
- [x] Validate Python compilation, Rust check, and diff hygiene.

Implementation notes:

- `round_wood_shell.py` now deselects other objects, selects the active wood
  mesh, enters edit mode, selects all vertices, and runs merge-by-distance
  before applying the angle-limited bevel. This fixes the previous failure mode
  where split imported vertices prevented Blender from seeing most exterior
  corner edges.
- `airlet-bake-materials` exposes `--bevel-merge-distance` and passes it to the
  Blender generator. The default is `1.0e-5`.
- The merge step reports removing `30` duplicate vertices from `Mesh` and `332`
  from `Mesh.008`.
- Rounded-source geometry now changes materially: `Mesh` increases from `26`
  to `694` triangles, and `Mesh.008` increases from `876` to `1328` triangles.
  This replaces the ineffective earlier pass where `Mesh.008` only reached
  about `1007` triangles.
- Final baked GLB splits wood mesh `0` into cross `93`, radial `90`,
  tangential `511`; wood mesh `8` into cross `439`, radial `380`, tangential
  `506`.
- Screenshot validation passed with
  `AIRLET_SCREENSHOT=target/rounded-3mm-walnut-merge-fix.png cargo run --bin
  airlet`; the rendered screenshot is nonblank with mean luminance `0.0652`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `uv run --project py airlet-bake-materials`,
  `cargo check --workspace`, and `git diff --check`.

## Strict Side-View Rounded Edge Acceptance

Purpose: complete the rounded-shell work using a stricter visual acceptance
method. The previous product-view screenshot is not sufficient to judge exterior
edge rounding, so the calibrated shell must be rebuilt with a larger bevel and
validated from strict side views.

Target behavior:

- Rounded wood shell uses bevel width `0.005` model units, treated as 5mm.
- Bevel angle threshold is requested as `90.0` degrees. The imported body mesh
  has non-orthogonal triangulated exterior edges, so the generator maps this
  visual right-angle request to a `30.0` degree Blender mesh threshold after
  vertex welding.
- Bevel width is converted through each Blender object's world scale before the
  modifier is applied. The imported wood shell objects use scale `0.01`, so a
  requested `0.005` model-space radius becomes `0.5` in mesh-local bevel width.
- The rounded shell rebuild materially increases wood-shell triangle counts.
- Acceptance screenshots use strict side camera views rather than the default
  product view.
- The final baked GLB remains loadable by the Rust app.

Checklist:

- [x] Set default bevel width to `0.005` and bevel angle to `90.0`.
- [x] Rebuild rounded shell and final baked GLB.
- [x] Capture strict side-view screenshots for rounded-edge inspection.
- [x] Inspect side-view screenshots before marking complete.
- [x] Validate Python compilation, Rust check, screenshot render, and diff
  hygiene.

Completion notes:

- `round_wood_shell.py` now welds wood shell vertices before beveling, converts
  requested model-space bevel width through the object's world scale, and prints
  the effective local bevel width. For the current source model, both `Mesh` and
  `Mesh.008` report object scale `(0.01, 0.01, 0.01)` and local bevel width
  `0.5` for the requested 5mm radius.
- `airlet-bake-materials` rebuilt
  `assets/generated/music_box_rounded_shell.glb` and
  `assets/generated/music_box_material_baked.glb`.
- Geometry check:
  source `Mesh`/`Mesh.008` are `26` and `876` faces; rounded shell is `694` and
  `3676` faces; final baked GLB keeps `694` and `3664` faces across split
  walnut material primitives.
- Final strict side-view screenshots:
  `target/rounded-strict-side-right-final.png`,
  `target/rounded-strict-side-left-final.png`,
  `target/rounded-strict-side-right-close-final.png`, and
  `target/rounded-strict-side-left-close-final.png`.
- Edge-inspection crops from those strict side views:
  `target/rounded-strict-side-right-top-corners-x4.png` and
  `target/rounded-strict-side-left-top-corners-x4.png`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo fmt --all`, `cargo check --workspace`,
  `cargo test --workspace`, and `git diff --check`.

## 14cm Physical Scale And Front Wood Continuity

Purpose: calibrate the app to a plausible real music-box size and fix the
visible front-face texture discontinuity reported on the right edge of the box
body.

Target behavior:

- Treat `1.0` Bevy unit as `1m`.
- Closed music-box width is `0.14m`. The current aligned source width is
  `0.66793925`, so app `MODEL_SCALE` should be approximately `0.2095999`.
- The previously requested 5mm exterior roundover remains 5mm in final
  displayed physical size, not 5mm in unscaled model space.
- Default camera, platform, and lights frame the smaller 14cm object directly.
- The wood body and lid share a coherent projection window so the right side of
  the front face does not show a narrow unrelated texture patch.
- Validation uses a front screenshot and strict side/close screenshots after
  rebaking.

Checklist:

- [x] Set the app model scale to the 14cm target width.
- [x] Rescale default camera, platform, and light setup for the smaller object.
- [x] Convert physical 5mm bevel radius into model-space bake width.
- [x] Use a shared wood projection window for the exterior shell meshes.
- [x] Rebuild generated textures, rounded shell, and baked GLB.
- [x] Screenshot-validate front texture continuity and side roundover.
- [x] Run Python compile, Rust fmt/check/test, and diff hygiene.

Completion notes:

- App `MODEL_SCALE` is `0.2095999`, giving the aligned closed model an overall
  displayed width of approximately `14.0cm`.
- The 5mm physical roundover is baked as `0.023854973` aligned model units,
  which becomes `0.005m` after app scaling. The clean aligned base has object
  scale `(1, 1, 1)`, so Blender reports the same value as the local bevel width.
- Default camera radius, camera target, platform size, light distances, shadow
  bias, contact shadows, and SSAO object thickness were rescaled for the
  smaller tabletop object.
- The wood material baker now computes one global wood axis range and one shared
  shell texture window for all exterior wood meshes, so body/lid mesh boundaries
  no longer choose unrelated texture windows.
- Longitudinal wood UV windows are constrained inside the source texture instead
  of crossing the texture wrap boundary; this removes the narrow full-height
  discontinuity on the box front.
- Screenshot evidence:
  `target/scale14-fixed-front-yaw0-lit.png`,
  `target/scale14-fixed-front-yaw0-lit-crop-enhanced.png`,
  `target/scale14-fixed-side-right-lit.png`, and
  `target/scale14-fixed-side-right-lit-crop-enhanced.png`. The later aligned
  clean base rebuild adds `target/aligned-base-default-v2.png`,
  `target/aligned-base-front-diagnostic.png`, and
  `target/aligned-base-side-diagnostic.png`.

### Body Roundover Follow-Up

Observation: after the 14cm scale calibration, the lid visibly has roundover,
but the body front/side outer silhouette still reads too sharp. Geometry counts
show `Mesh.008` is processed by the bevel pass, but its visible outer bounds do
not show the same clear inset as the lid mesh.

Failure note: an attempted AABB exterior-edge bevel selected `226` body edges
and expanded `Mesh.008` bounds from roughly `62 x 19 x 58` to `101 x 37 x 100`
mesh-local units. That generated geometry was the cause of abnormal lighting
and close-camera distortion. The path was removed and the baked GLB was
regenerated from the stable angle-limited bevel path.

Status: superseded by the aligned clean base model rebuild below. The original
wood shell meshes are open triangle shells with hundreds of boundary edges, so
continuing to tune angle-limited bevels or manual Blender bevels is not a
reliable production path.

Checklist:

- [ ] Verify `Mesh.008` body exterior edges are selected by the Blender bevel,
  not only internal/hole edges.
- [ ] Adjust the body roundover generation so body top, bottom, and vertical
  exterior edges visibly round at the same 5mm physical radius as the lid.
- [ ] Rebuild rounded shell and final baked GLB.
- [ ] Capture close front/side screenshots focused on body corners.
- [ ] Run Python compile, Rust fmt/check/test, and diff hygiene.

### Aligned Clean Base Model Rebuild

Purpose: replace the brittle original wood-shell triangle geometry with a clean
aligned base model that can serve as the future modeling source. The source GLB
contains a slanted closed model plus an open reference state; the app has been
correcting that with runtime/spec basis transforms. The rebuilt base should be
directly aligned in Airlet coordinates, keep the useful mechanism meshes, and
use procedurally rebuilt wood body/lid geometry with real rounded exterior
edges.

Target behavior:

- Export a new generated GLB whose closed model is already aligned to the
  Airlet rig frame.
- Preserve app-visible mechanism meshes from the closed model, excluding the
  hidden raw cylinder/comb meshes that are replaced procedurally at runtime.
- Rebuild wood `Mesh` and `Mesh.008` as clean box-shell proxy meshes with
  rounded exterior corners instead of trying to bevel open imported shells.
- Keep the visible wooden body/lid proportions and placement close to the
  measured source model.
- Bake the existing walnut/brass/steel material workflow onto the rebuilt base.
- Record mesh-health evidence: watertight/open-boundary counts, bounds, and
  screenshots or side-view artifacts sufficient to verify that roundover exists.

Checklist:

- [x] Add a Python generator for an aligned clean base GLB.
- [x] Extract app-visible closed-model meshes through the current spec and
  basis, not through the raw dual-state GLB scene.
- [x] Replace wood meshes with clean rounded proxy geometry in the aligned base.
- [x] Update the material baker to use the aligned clean base as its source.
- [x] Regenerate textures, clean base, rounded/material-baked GLB, and reports.
- [x] Validate the rebuilt wood meshes with topology/bounds diagnostics.
- [x] Capture strict front/side screenshots that show body and lid roundover.
- [x] Run Python compile, Rust fmt/check/test, and diff hygiene.

Completion notes:

- Added `py/airlet_audio_lab/build_aligned_base_model.py` and the
  `airlet-build-aligned-base` entry point. The generator reads
  `assets/models/converted/source_spec.toml` as the original slanted-model
  truth source and writes `assets/generated/music_box_aligned_base.glb`.
- `assets/models/converted/spec.toml` now points to the generated aligned base,
  uses identity basis vectors, and stores lid/cylinder/winding/comb axes in
  aligned coordinates.
- The material baker automatically refreshes the aligned base before rounding
  and baking. Non-wood primitives receive generated UV/tangent attributes so
  Bevy no longer emits missing-UV tangent warnings for the baked GLB.
- Mesh-order validation: the aligned/rounded/baked GLBs contain 37 meshes in
  the expected `Mesh` through `Mesh.036` order, preserving the existing rig
  index contract.
- Topology validation after welding split render vertices:
  `assets/generated/music_box_rounded_shell.glb` and
  `assets/generated/music_box_material_baked.glb` both report `Mesh` and
  `Mesh.008` as watertight, winding-consistent, with `0` boundary edges and
  `0` non-manifold edges.
- Screenshot evidence: `target/aligned-base-default-v2.png`,
  `target/aligned-base-front-close.png`,
  `target/aligned-base-side-close.png`,
  `target/aligned-base-front-diagnostic.png`, and
  `target/aligned-base-side-diagnostic.png`.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo fmt --all`, `cargo check --workspace`,
  `cargo test --workspace`, `AIRLET_SCREENSHOT=target/aligned-base-default-v2.png
  cargo run`, and `git diff --check`.

## Wood Grain Direction Roadmap

Purpose: make wood grain follow the actual music-box geometry instead of the
imported GLB UV islands, so visible long-grain fibers run along the box width
and cut faces use end-grain/ring-like texture.

Status: superseded by the Python Material Baking Roadmap. The behavior remains,
but the implementation now lives in the Python asset baker rather than Rust
runtime rendering code.

Checklist:

- [x] Add separate procedural texture sets for long-grain wood and end-grain
  wood, both with matching normal maps.
- [x] Initially validated triangle face orientation splitting in Rust runtime.
- [x] Generate UVs from model-space basis vectors instead of GLB UVs.
- [x] Assign long-grain material to faces that extend along box width and
  end-grain material to width-axis cut faces.
- [x] Preserve lid/static rig hierarchy and winding/key hover behavior.
- [x] Regenerate screenshots and validate texture direction on closed/open
  views.
- [x] Run Rust, Python, and diff validation.

Validation notes:

- `py/airlet_audio_lab/generate_textures.py` now emits separate
  `lacquered_wood_*` long-grain maps and `lacquered_wood_end_*` end-grain maps.
  Both texture sets include normal maps derived from their own height fields.
- `src/model_view.rs` now detects wood material class, reads the imported
  primitive mesh, duplicates triangles into long-grain and end-grain mesh
  batches, and generates UVs from `spec.basis.right/up/front` instead of using
  GLB UVs. Long-grain faces use the box width axis for U; width-axis cut faces
  use front/up projection and the end-grain material.
- Refreshed screenshots with `uv run --project py airlet-lighting-shots
  --launch --startup-timeout 60 --screenshot-timeout 45 --warmup-seconds 5`.
  `target/lighting/product.png` and `target/lighting/lid.png` show long-grain
  wood running horizontally along the box width, while cut/inset faces no longer
  display GLB-UV-induced vertical fiber stripes.
- Validation passed: `uv run --project py airlet-generate-textures`,
  `cargo fmt --all`, `cargo check --workspace`, `cargo test --workspace`, and
  `uv run --project py python -m compileall py/airlet_audio_lab`.

### Aligned Base Regression Fix

Purpose: fix the first clean-base rebuild regressions found after visual review.
The aligned base must preserve the original model's part registration and lid
shape while still replacing the brittle wood shell topology with bevelable clean
geometry.

Observed regressions:

- The rebuilt wood shell appears rotated/misaligned relative to the internal
  mechanism parts; lid opening axis is visibly wrong.
- The lid was over-simplified to a plain cuboid, losing the inner recessed tray
  shape.
- Lid top-to-side and side-to-side roundovers do not read as consistent.
- The body top closing plane was incorrectly rounded; lid/body contact faces
  should remain sharp and flat.

Checklist:

- [x] Diagnose generated GLB node transforms and raw mesh coordinates for wood
  and non-wood parts, without relying on viewer-specific scene interpretation.
- [x] Rework aligned-base generation so every kept mesh shares one coordinate
  contract and the runtime spec pivots match that contract.
- [x] Rebuild the lid as a clean recessed-lid proxy instead of a plain cuboid.
- [x] Restrict bevels to exterior visible edges only; keep body/lid closing
  contact planes sharp and flat.
- [x] Regenerate aligned base, rounded shell, baked GLB, and reports.
- [x] Validate registration, topology, lid pivot, screenshots, and command
  gates.

Completion notes:

- Raw generated GLB inspection showed all kept meshes now share identity node
  transforms in aligned Airlet coordinates.
- `Mesh` lid generation now keeps a recessed tray structure instead of a plain
  cuboid.
- Lid axis is snapped to the aligned width axis `[1, 0, 0]` to match the clean
  orthogonal shell and avoid visible hinge skew.
- Screenshots used for validation:
  `target/aligned-regression-closed-front.png`,
  `target/aligned-regression-open-front.png`, and
  `target/aligned-regression-open-side.png`.

### Manual Wood Shell Bevel Handoff

Purpose: stop applying automatic bevels to the clean wood-shell proxy and hand
the bevel operation to Blender manual editing. The asset pipeline should still
produce an aligned, clean, bevelable base model and should be able to bake from a
manual rounded GLB without overwriting it.

Checklist:

- [x] Add a material-bake path that accepts a manually rounded source GLB and
  skips the automatic Blender bevel script.
- [x] Generate manual Blender handoff GLBs from the aligned clean base: one full
  app-visible context model and one wood-shell-only model.
- [x] Document the exact Blender input/output paths and the constraints for
  preserving mesh names, mesh order, scale, and contact faces.
- [x] Validate that the non-rounded clean base remains aligned and topologically
  clean.
- [x] Run Python compile, Rust check/test as needed, and diff hygiene.

Completion notes:

- Status: superseded by the Blender-native handoff below. The GLB handoff files
  were removed because GLB triangulation creates coplanar internal edges that
  make manual beveling unstable.
- Full Blender handoff input:
  `target/manual-roundover/music_box_aligned_clean_base_manual_bevel_input.glb`.
- Wood-shell-only reference:
  `target/manual-roundover/music_box_aligned_clean_wood_shell_manual_bevel_reference.glb`.
- Manual export target:
  `assets/generated/music_box_manual_rounded_shell.glb`.
- Handoff instructions:
  `target/manual-roundover/README.md`.
- Material bake command for the manual output:
  `uv run --project py airlet-bake-materials --manual-rounded-source
  assets/generated/music_box_manual_rounded_shell.glb`.
- Smoke validation passed with the clean-base handoff input as a stand-in
  manual source:
  `uv run --project py airlet-bake-materials --manual-rounded-source
  target/manual-roundover/music_box_aligned_clean_base_manual_bevel_input.glb
  --output target/manual-roundover/manual-source-smoke-baked.glb
  --skip-textures`.

### Blender-Native Wood Bevel Handoff

Purpose: fix manual bevel instability caused by GLB triangulation. The clean
wood shell may be watertight, but GLB stores faces as triangles, so large
coplanar panels import into Blender with extra internal edges. Manual beveling
should use a Blender-native handoff where wood-shell panels are authored as
quads/ngons and only semantic edges exist.

Checklist:

- [x] Add a Blender handoff builder that imports the aligned clean base context
  and replaces `Mesh`/`Mesh.008` with Blender-native quad/ngon wood meshes.
- [x] Preserve mesh object names, aligned coordinates, and non-wood context.
- [x] Export a `.blend` handoff for manual bevel work, plus optional GLB/OBJ
  reference if useful.
- [x] Validate that wood shell objects have no coplanar internal triangle edges
  on large panels.
- [x] Update the manual handoff README with the new preferred input file and
  export target.

Completion notes:

- Added `py/airlet_audio_lab/build_manual_bevel_handoff.py`.
- Preferred manual edit file:
  `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- The `.blend` file keeps object and mesh data names as `Mesh` and `Mesh.008`.
- Validation from Blender:
  `Mesh` has 48 vertices, 46 quad faces, and 0 triangles;
  `Mesh.008` has 76 vertices, 74 quad faces, and 0 triangles after adding the
  crank-side clearance opening.
- Removed the older GLB handoff files from `target/manual-roundover` to avoid
  opening the triangulated source by mistake.

### Clean Base Shell Registration Fix

Purpose: fix the clean wood-shell base before manual bevel handoff. The handoff
model must preserve shell/interior registration, lid recess orientation, and the
body crank clearance opening so Blender manual beveling starts from a correct
base.

Observed regressions:

- Lid recessed feature is oriented incorrectly.
- Interior mechanism and wood shell have a visible residual yaw/angle mismatch.
- Body shell lacks the side opening/clearance for the winding crank.

Checklist:

- [x] Measure source wood shell, lid, and crank/internal part axes in the aligned
  coordinate frame to identify the residual yaw mismatch.
- [x] Rebuild shell proxy from an oriented footprint instead of a plain aligned
  AABB, or rotate preserved internal parts consistently if that is the correct
  registration fix.
- [x] Reorient the lid recessed feature to match the original lid inner panel.
- [x] Add a body side clearance opening around the winding crank side.
- [x] Regenerate aligned base and manual Blender handoff files.
- [x] Validate topology, mesh order, registration screenshots, and command gates.

Completion notes:

- Measured the residual internal yaw in aligned coordinates from the cylinder
  and crank axes. The clean shell remains the coordinate reference; only
  non-wood interior meshes and matching non-lid spec axes/pivots are corrected
  into the shell frame.
- Runtime spec now snaps `cylinder.axis` to `[0, 0, 1]` and `winding_key.axis`
  to approximately `[1, 0, 0]`, while `lid.axis` stays `[1, 0, 0]`.
- `Mesh.008` body proxy now has a right-side crank clearance opening around the
  winding-key pivot.
- Regenerated `assets/generated/music_box_aligned_base.glb`,
  `target/manual-roundover/music_box_aligned_clean_base_manual_bevel_input.glb`,
  and
  `target/manual-roundover/music_box_aligned_clean_wood_shell_manual_bevel_reference.glb`.
- Handoff validation: 37 meshes remain in `Mesh` through `Mesh.036` order;
  `Mesh` and `Mesh.008` are watertight with `0` boundary edges and `0`
  non-manifold edges.
- Validation passed: `uv run --project py python -m compileall
  py/airlet_audio_lab`, `cargo fmt --all`, `cargo check --workspace`,
  `cargo test --workspace`, manual-source bake smoke, and `git diff --check`.

### Blender Handoff Z-Up Shell Fix

Purpose: fix the manual bevel `.blend` handoff after verifying that Blender
imports the aligned GLB as `Z-up`, while the handoff proxy builder was still
carving lid/body features as if `Y` were vertical.

Checklist:

- [x] Update the Blender handoff shell proxy builder so `Mesh`/lid uses `Z` as
  thickness/vertical and cuts its inner recess on the correct underside face.
- [x] Update the `Mesh.008`/body proxy so the top tray opens upward in Blender
  `Z-up` space and the side crank clearance opens on the `X+` side wall.
- [x] Regenerate `target/manual-roundover/music_box_manual_bevel_handoff.blend`
  from `assets/generated/music_box_aligned_base.glb`.
- [x] Validate the generated `.blend` by inspecting face distributions: lid
  recess must be on the lid underside, body tray must be open upward, and body
  must have a visible `X+` crank entry opening.
- [x] Run Python compile validation and `git diff --check` for the touched
  files.

Completion notes:

- `py/airlet_audio_lab/build_manual_bevel_handoff.py` now converts runtime spec
  points to Blender import coordinates with `(x, -z, y)`.
- The handoff shell proxy treats `Z` as the Blender vertical/thickness axis:
  `Mesh`/lid recess opens on `z0`, and `Mesh.008`/body tray opens on `z1`.
- The body crank entry is cut from the `X+` side wall around the converted
  winding-key pivot.
- Regenerated `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- Blender validation passed with face-count evidence:
  lid `z0/z1 = 8/9`, body `z0/z1 = 15/11`, body `x0/x1 = 20/16`.
- Validation passed: Blender geometry assertions, `uv run --project py python
  -m compileall py/airlet_audio_lab`, and `git diff --check`.

### Blender Handoff Origin Centering

Purpose: make the manual bevel `.blend` easier to edit by placing the full
music-box assembly on the Blender origin ground plane, while preserving a
reliable path back to the runtime-aligned model coordinates during material
baking.

Checklist:

- [x] Place the handoff `.blend` by bottom-face center rather than by a corner,
  so the music box sits on `Z=0` while remaining centered in `X/Y`.
- [x] Record the applied handoff offset in the `.blend` for inspection.
- [x] Make manual-source material baking realign an exported hand-edited GLB to
  the current aligned base using preserved non-wood mesh positions.
- [x] Regenerate the handoff `.blend` and validate its world bounds are centered
  near the origin.
- [x] Validate that the manual bake path restores runtime-aligned bounds after
  consuming the centered handoff export.
- [x] Run Python compile validation and `git diff --check`.

Completion notes:

- Regenerated `target/manual-roundover/music_box_manual_bevel_handoff.blend`
  with bottom-face center at the Blender origin. Validated world bounds:
  min `[-0.330155, -0.218635, 0.0]`, max `[0.330155, 0.218635, 0.275097]`.
- The scene records `airlet_handoff_center_offset` as
  `(0.014676, -1.381368, 1.129592)`.
- Manual-source bake now aligns exported handoff GLBs back to the current
  runtime-aligned base by comparing preserved non-wood mesh bounds.
- Smoke export from the centered `.blend` produced centered GLB bounds, and
  `airlet-bake-materials --manual-rounded-source` restored baked bounds to
  match `assets/generated/music_box_aligned_base.glb`.

### Blender Handoff Round Crank Opening

Purpose: replace the oversized rectangular crank clearance in the manual bevel
handoff with a restrained round side-wall entry matching the original model
intent.

Checklist:

- [x] Replace the body proxy's rectangular voxel crank cut with a circular or
  near-circular `X+` side opening centered on the winding-key pivot.
- [x] Preserve the body top tray, bottom-face origin placement, and non-wood
  alignment context.
- [x] Regenerate `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- [x] Validate that the crank opening is round, smaller than the old rectangular
  cut, and still pierces the side wall.
- [x] Update the handoff README and run Python compile plus `git diff --check`.

Completion notes:

- Replaced the hand-written radial side-wall topology with a Blender boolean
  cylinder cut. The base side wall stays large and simple; only the local round
  crank entry is retopologized.
- Regenerated `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- `Mesh.008` validation after boolean: 112 vertices, 80 polygons, 0 boundary
  edges, 0 non-manifold edges.

### Blender Handoff Shell Size Source Correction

Purpose: keep the manual bevel base dimensions tied to the original wood shell
meshes. Hardware such as hinges, front locks, and the crank may validate
clearance and openings, but must not decide the lid/body outer dimensions.

Checklist:

- [x] Remove the hardware-driven front/back shell shrink from the Blender
  handoff builder.
- [x] Use only the original wood mesh bounds for simplified lid/body shell
  dimensions.
- [x] Regenerate the handoff `.blend` with original wood-shell dimensions.
- [x] Preserve the bottom-face-center origin placement and round crank entry.
- [x] Validate with geometry probes that the regenerated shell bounds match the
  wood mesh source bounds.
- [x] Run Python compile validation and `git diff --check`.

Completion notes:

- The previous hardware-clearance shrink was removed. It was conceptually wrong
  because hinges/front locks are not valid sources for wood-shell dimensions.
- `py/airlet_audio_lab/build_manual_bevel_handoff.py` now uses the imported
  wood mesh bounds directly for `Mesh` and `Mesh.008`.
- Hardware remains useful only for validating fit and for locating functional
  openings such as the crank hole.

### Blender Handoff Oriented Wood Bounds

Purpose: replace axis-aligned wood shell sizing with an oriented bound computed
from each original wood mesh. The shell size source remains the wood mesh
itself, but residual in-plane rotation is now handled explicitly instead of
assuming the imported mesh is perfectly axis-aligned.

Checklist:

- [x] Compute a horizontal OBB from each wood mesh's local vertex cloud using
  its own principal axes in the Blender `X/Y` plane, with `Z` min/max kept from
  the mesh.
- [x] Generate lid/body proxy vertices in OBB-local coordinates, then transform
  them back into Blender object-local coordinates.
- [x] Place the round crank opening using the OBB's local `+X` side rather than
  the world AABB side.
- [x] Regenerate `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- [x] Validate that restored wood proxy bounds match the OBB-derived dimensions,
  topology remains manifold, and bottom-face center placement still holds.
- [x] Run Python compile validation, `git diff --check`, and commit the fix.

Completion notes:

- `py/airlet_audio_lab/build_manual_bevel_handoff.py` now computes
  `OrientedShellBounds` from each source wood mesh before replacement.
- The generated `Mesh` and `Mesh.008` objects store
  `airlet_source_bounds_kind = horizontal_obb` plus OBB origin, axes, min/max,
  and dimensions for later inspection.
- Validation from the regenerated `.blend`: `Mesh` source OBB dimensions are
  `[0.525021, 0.429683, 0.083748]` at `0.0` degrees; `Mesh.008` source OBB
  dimensions are `[0.525811, 0.430691, 0.190378]` at `-0.132117` degrees.
- Topology validation remains clean: both wood proxies have `0` boundary edges
  and `0` overused/non-manifold edges, and the full assembly bottom center is
  still at the Blender origin.

### Blender Handoff Original Wood Geometry Source

Purpose: fix the handoff sizing chain so wood dimensions and oriented bounds are
computed from the original converted wood meshes, not from the already
simplified `assets/generated/music_box_aligned_base.glb` proxy meshes.

Checklist:

- [x] Add original source GLB/spec inputs to the handoff builder.
- [x] Reuse the aligned-base basis transform so source wood mesh points are
  measured in the same aligned coordinate frame as the runtime model.
- [x] Replace `Mesh` and `Mesh.008` with original source wood geometry
  transformed into the aligned Blender frame, rather than regenerating them from
  any bounding box.
- [x] Store source-derived OBB metadata on the wood objects for inspection, but
  do not use the OBB to fill missing shell geometry.
- [x] Keep non-wood context loaded from the aligned base so runtime mesh order
  and current spec remain unchanged.
- [x] Regenerate the manual bevel `.blend` and validate source-derived OBB
  dimensions differ from or explain the previous proxy-derived values.
- [x] Run Python compile validation, `git diff --check`, and commit the fix.

Completion notes:

- `py/airlet_audio_lab/build_manual_bevel_handoff.py` now imports
  `assets/models/converted/music_box.glb` first, reads source `Mesh` and
  `Mesh.008`, transforms their vertices using `source_spec.toml` basis into the
  same Blender handoff frame, then loads the aligned base for all non-wood
  context.
- Handoff wood geometry is no longer a generated box proxy. Current regenerated
  geometry: `Mesh` has 46 vertices and 26 source faces; `Mesh.008` has 772
  vertices and 876 source faces.
- Source-derived OBB metadata: `Mesh` dimensions
  `[0.524995, 0.429649, 0.083748]` at `-0.004241` degrees; `Mesh.008`
  dimensions `[0.59205, 0.525857, 0.190378]` at `13.648449` degrees.
- The generated `.blend` keeps the full assembly bottom center at the Blender
  origin and has no overused/non-manifold wood edges.

### Blender Handoff Wood Angle Snap

Purpose: preserve original wood geometry dimensions while snapping the source
wood shell's horizontal OBB axes to the aligned handoff coordinate frame, so the
shell no longer carries source residual yaw relative to the mechanism context.

Status: superseded by the aligned-base single-truth handoff below. The OBB
snapping approach added an unnecessary independent wood coordinate path.

Checklist:

- [x] Rotate source wood vertices from their source OBB frame into the handoff
  canonical `X/Y` frame while preserving OBB dimensions and local details.
- [x] Regenerate the manual bevel `.blend`.
- [x] Validate that `Mesh` and `Mesh.008` source OBB metadata angles are near
  `0` degrees after snapping, with expected source dimensions preserved.
- [x] Validate bottom-face-center origin placement and run Python compile plus
  `git diff --check`.

Completion notes:

- Source wood vertices are transformed through their source OBB local frame and
  reconstructed on canonical handoff `X/Y` axes.
- Regenerated `.blend` validation: `Mesh` dimensions
  `[0.524995, 0.429649, 0.083748]` at `0.0` degrees; `Mesh.008` dimensions
  `[0.59205, 0.525857, 0.190378]` at `0.0` degrees.
- The full assembly bottom center remains at the Blender origin, and wood mesh
  edges have no overused/non-manifold edges.

### Blender Handoff Shared Wood Edge Frame

Purpose: fix the remaining manual handoff mismatch where the lid orientation is
correct but the body still appears yawed. The body mesh is asymmetric because of
its tray, side opening, and internal cutouts, so its own PCA/OBB axis is not a
safe source of exterior shell direction. The handoff should extract a shared
horizontal frame from long, near-horizontal wood shell edges instead of using
PCA or preserving the imported object transforms.

Status: superseded by the aligned-base single-truth handoff below. Even edge
frame extraction still kept a second wood-specific alignment path and was not
acceptable as the handoff truth source.

Checklist:

- [x] Load original source lid/body wood vertices before replacement and keep
  original polygon topology.
- [x] Derive the shared handoff frame from source wood shell long edges, then
  project lid and body through that same frame instead of through body-local PCA.
- [x] Reset replaced wood object transforms so already-aligned handoff vertices
  are not rotated a second time by stale imported object matrices.
- [x] Regenerate `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- [x] Validate that lid/body metadata uses canonical handoff axes, the full
  assembly remains bottom-centered at origin, and Python/diff checks pass.

Completion notes:

- The corrected body top diagnostic
  `target/manual-roundover/handoff_body_top.png` shows body, crank, cylinder,
  and comb sharing the same horizontal frame.
- Blender validation reports wood object matrices as identity plus placement
  translation, bottom center `[0.0, 0.0, 0.0]`, and no overused wood edges.
- Long wood shell edge angles after correction are near `0` degrees: `Mesh`
  reports repeated `-0.003` degree long edges; `Mesh.008` reports main edges
  around `-0.075` to `0.07` degrees while preserving diagonal cut/opening edges.

### Blender Handoff Aligned-Base Single Truth

Purpose: remove the handoff builder's remaining custom wood-coordinate
heuristics. The old direct aligned export already had correct direction and
position because every part came from the same source transform chain. The
manual bevel handoff should therefore treat `music_box_aligned_base.glb` as the
single spatial truth and should not re-import source wood, run PCA/OBB, extract
edge frames, or apply any independent wood alignment.

Checklist:

- [x] Remove source-wood replacement and all PCA/edge-frame snapping from the
  Blender handoff builder.
- [x] Import `assets/generated/music_box_aligned_base.glb`, bake wood object
  transforms into mesh vertices only to make Blender editing stable, and keep
  all parts in the same aligned-base coordinate frame.
- [x] Regenerate `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- [x] Validate body/lid/mechanism top-view alignment, bottom-center origin
  placement, Python compile, and `git diff --check`.

Completion notes:

- `py/airlet_audio_lab/build_manual_bevel_handoff.py` now imports only
  `assets/generated/music_box_aligned_base.glb`; `--source`, `--source-spec`,
  source OBB/PCA, and wood edge-frame extraction were removed.
- Wood objects are converted to world-space mesh vertices with identity object
  transforms before the shared scene centering pass. This keeps Blender editing
  stable without introducing an independent wood coordinate system.
- Regenerated handoff output:
  `target/manual-roundover/music_box_manual_bevel_handoff.blend`.
- Diagnostic top views:
  `target/manual-roundover/handoff_aligned_body_top.png` and
  `target/manual-roundover/handoff_aligned_full_top.png`.
- Validation reports bottom center `[0.0, 0.0, 0.0]`; wood meshes have
  `aligned_base_world_space` metadata and no boundary or overused edges.

### Manual Handoff Before/After Blend Pair

Purpose: provide two Blender files for direct manual comparison: one generated
from the original aligned source wood before clean shell reconstruction, and
one generated from the rebuilt clean shell. This removes ambiguity when checking
whether a lid/body offset or shape issue comes from the original aligned asset
or from the reconstruction step.

Checklist:

- [x] Add a repeatable `--preserve-source-wood` export mode to the aligned base
  builder.
- [x] Generate an unrebuilt aligned input GLB under `target/manual-roundover/`.
- [x] Generate `music_box_unrebuilt_aligned_handoff.blend`.
- [x] Generate `music_box_rebuilt_aligned_handoff.blend`.
- [x] Validate both blends have bottom-center origin placement and record wood
  mesh topology counts.

Completion notes:

- Unrebuilt comparison input:
  `target/manual-roundover/music_box_unrebuilt_aligned_handoff_input.glb`.
- Unrebuilt Blender file:
  `target/manual-roundover/music_box_unrebuilt_aligned_handoff.blend`.
- Rebuilt Blender file:
  `target/manual-roundover/music_box_rebuilt_aligned_handoff.blend`.
- Unrebuilt topology: `Mesh` has `16` vertices / `26` polygons with `4`
  boundary edges; `Mesh.008` has `440` vertices / `876` polygons with no
  boundary or overused edges.
- Rebuilt topology: `Mesh` has `48` vertices / `92` polygons; `Mesh.008` has
  `114` vertices / `224` polygons; both rebuilt wood meshes have no boundary or
  overused edges.
- Both blends validate bottom center at `[0.0, 0.0, 0.0]`.
