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
playback start/stop, and structured mechanism-state dumps.

### Airlet Debug Action Surface And MCP Path

MCP should be an adapter over Airlet's own debug/action vocabulary, not the first
place where app state semantics are invented. The first implementation should be
a local-only JSON action endpoint that can be driven by scripts and later wrapped
by an MCP server.

Implementation checklist:

- [x] Add a local debug endpoint bound to `127.0.0.1`.
- [x] Support JSON actions: `dump_state`, `dump_mechanism`,
  `set_camera`, `set_light`, `set_lid`, `set_cylinder`, `seek_tick`,
  `play`, `stop`, and `screenshot`.
- [x] Route debug actions through the same `ExhibitControls`,
  `PlaybackState`, and screenshot resources used by egui.
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

- [x] Derive `ticks_per_turn` from the full timeline end tick.
- [x] Use the derived value in both `MechanismPlanner` and playback phase sync.
- [x] Add a validation report for same-onset groups and same-phase groups.
- [x] Require zero same-phase collisions when the score has no same-onset
  chords.
- [x] Expose the timing validation in `dump_mechanism`.
- [x] Test the default Air intro has no folded phase collisions.
- [x] Validate with the running app through the debug endpoint.

Validation result:

- `ticks_per_turn`: `29760`, equal to `last_onset + last_duration`.
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
- [x] Gate the visual pluck/vibration on tooth contact length so too-short
  teeth do not fake a pluck or auto-vibrate.
- [x] Split the pre-release motion into contact, lift, max-deflection hold, and
  release phases instead of a single ramp.

Validation notes:

- `uv run --project py airlet-comb-motion-sequence` captures a close-up
  sequence plus `sequence.json` metadata and `contact_sheet.png`; the frame
  plan now samples pre-contact, contact start, mid-lift, max deflection,
  release, early vibration, and late decay.
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
