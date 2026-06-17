# Airlet Backend Roadmap

This document keeps the next infrastructure steps explicit so sound-design
iterations do not erase backend/API goals.

## Current Baseline

- `Composition` contains notes and musical durations only.
- `ComposedScore` is created by binding a `Composition` to a `Tempo`.
- `Timeline` can expand absolute musical onsets and durations.
- The current best sound-design target is the `a-dry` modal model, ported from
  the Python probe into Rust.
- The default app still plays the legacy realtime path.

## Important Diagnosis: Direct Playback Timing

The direct playback bin in `src/main.rs` currently runs:

```rust
Performance::air_intro_legacy().play_realtime(sample_rate, &mut sink);
```

That means direct playback is not using the current `a-dry` model yet.

It also does not use the new `Timeline`. It still consumes legacy
`NoteEvent { midi_note, millis }` events and schedules notes by sleeping between
events. This is enough for a demo, but it is not sample-accurate. Timing can feel
loose because:

- `std::thread::sleep` has scheduler jitter.
- `rodio` mixer insertion happens in realtime, not at exact sample offsets.
- old `NoteEvent.millis` represents "advance to next onset", not explicit
  `onset + duration`.
- the direct app uses the legacy sound model, while recent listening decisions
  were made against exported `a-dry` wav files.

So if the direct app feels rhythmically inaccurate, the likely cause is the old
realtime scheduling path, not the score durations themselves. The score layer now
has enough structure to support sample-accurate scheduling, but the audio engine
has not been migrated to consume it.

## Next Infrastructure

### 1. Timeline-Driven Audio Engine

Make both `legacy` and `a-dry` consume `Timeline` instead of legacy
`NoteEvent`.

Target API:

```rust
let composition = songs::air::intro_composition();
let performance = PerformancePlan::new(composition).tempo(songs::air::intro_tempo());
let audio = Engine::new(ModelPreset::ADry).render(&performance);
```

Required behavior:

- use absolute `TimelineEvent.onset`;
- schedule notes at sample offsets, not with sleeps;
- preserve velocity and voice information;
- keep legacy conversion only as compatibility glue.

### 2. Unified Engine API

Replace scattered render/play entry points with one backend surface:

```rust
Engine::new(ModelPreset::Legacy).render(&performance)
Engine::new(ModelPreset::ADry).render(&performance)
Engine::new(ModelPreset::ADry).source(&performance)
```

The app and wav exporter should both use this engine.

### 3. Performance and Arrangement Layer

Add a layer between composition and engine:

```rust
PerformancePlan {
    composition,
    tempo,
    ornament_policy,
    velocity_policy,
    model_preset,
}
```

This layer decides how ornaments steal time, how velocities map to sound/model
parameters, and how mechanical performance should quantize or humanize events.

### 4. Score DSL Expansion

Current DSL supports notes, rests, triplets, and `grace_before`.

Next additions:

- `Dur::pattern([1, 1, 2])`
- tuplets over arbitrary total duration;
- `velocity(...)`;
- `tie(...)` and slur metadata;
- repeat helpers;
- more natural ornament helpers.

### 5. Mechanism Hint Export

Add a planner that consumes `Timeline` and exports JSON:

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

Initial planner assumptions:

- pitch maps to comb/tine track and axial position;
- onset maps to cylinder angle;
- velocity maps to protrusion/depth;
- short-distance repeated notes on the same track emit diagnostics;
- dense adjacent notes at the same angle emit spacing diagnostics.

### 6. Preset Serialization

The current `a-dry` model is hardcoded in Rust and Python. Add TOML/RON/JSON
model presets before doing many more sound iterations.

### 7. Golden Checks

Avoid locking full wav contents, but add tests for:

- output duration;
- no NaN/inf;
- peak and RMS range;
- deterministic seeded render;
- timeline onset correctness.
