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

## Roadmap Status

- [x] Timeline-driven audio engine
- [x] Unified engine API
- [x] Performance and arrangement layer
- [x] Score DSL expansion
- [x] Mechanism hint export
- [x] Preset serialization
- [x] Golden checks

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
