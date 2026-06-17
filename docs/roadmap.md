# Airlet Backend Roadmap

This document keeps the next infrastructure steps explicit so sound-design
iterations do not erase backend/API goals.

## Current Baseline

- `Composition` contains notes and musical durations only.
- `ComposedScore` is created by binding a `Composition` to a `Tempo`.
- `Timeline` can expand absolute musical onsets and durations.
- The current best sound-design target is the `a-dry` modal model, ported from
  the Python probe into Rust and stored as a bundled JSON preset.
- The default app renders through `Engine` into a buffer, then plays that buffer
  with `rodio`.

## Roadmap Status

- [x] Timeline-driven audio engine
- [x] Unified engine API
- [x] Performance and arrangement layer
- [x] Score DSL expansion
- [x] Mechanism hint export
- [x] Preset serialization
- [x] Golden checks

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
