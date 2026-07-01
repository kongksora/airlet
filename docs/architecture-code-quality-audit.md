# Airlet Architecture Status

Updated: 2026-06-28

This document replaces the earlier architecture audit. The original audit was
useful when the Bevy app lived mostly in `src/lib.rs`, but that report is now
historical: the app has been split into focused modules, playback state has
been reduced to audio output, and debug/MCP actions are schema-driven.

## Current Shape

- `src/main.rs` remains the product entry point.
- `src/lib.rs` is a thin Bevy app assembly layer: plugin setup, resource
  initialization, and system scheduling.
- `crates/airlet` is the Bevy-free backend for score, timeline, rendering,
  presets, synthesis, notation, and mechanism hints.
- `crates/airlet-model` is the Bevy-free model specification and rig-pose
  layer.
- `py/airlet_audio_lab` contains asset/audio probe tools and the MCP adapter.
- `docs/roadmap.md` is the implementation evidence ledger.

## Current App Modules

- `controls.rs`: egui control panel, volume/rate controls, full-wind and
  pause/continue/reset commands.
- `playback.rs`: `AudioOutputState`, rendered audio buffer, active rodio
  players, duration/sample helpers.
- `twin.rs`: mechanical digital-twin state machine for spring energy, crank
  angle, cylinder phase, pause/resume, and audio-cycle events.
- `winding.rs`: Bevy picking hover/press input and winding-key visual state.
- `debug.rs`: local JSON endpoint, `DebugAction`, Rust-owned `ActionCatalog`,
  and schema-driven action metadata.
- `model_view.rs`: GLB/model loading, spec-driven model grouping, and rig
  transforms.
- `mechanism_view.rs`: procedural teeth/comb/cylinder visuals and comb mesh
  deformation.
- `comb_animation.rs`: comb timing/event derivation and tine deflection
  sampling.
- `scene.rs`: camera, lighting, platform, and orbit controls.
- `screenshot.rs`: automated screenshot capture and exit.
- `visual_config.rs`: tunable visual mechanism constants.

## Protocol Status

The debug protocol is now Rust-owned:

- `describe_actions` returns the action catalog.
- `ActionSpec` entries define tool name, description, and parameter schemas.
- Python MCP dynamically registers tools from `describe_actions`.
- `call_action` remains as a stable fallback.
- Removed direct transport controls are not exposed: `play`, `stop`,
  `set_cylinder`, and `seek_tick`.

## Current Controls

Product controls:

- `Full Wind`: fully winds the spring once and releases into playback.
- `Pause` / `Continue`: toggles the twin state while preserving mechanical
  phase.
- `Reset`: stops audio and resets the twin.
- `Volume` and `Rate`: affect active and future audio output.
- `Lid t`, camera, and spotlight controls remain presentation controls.

Debug-only controls:

- `set_winding` can force hover/press/energy/key-angle state for validation.
- `set_ui`, `set_camera`, `set_light`, `set_lid`, and `screenshot` support
  inspection and screenshot workflows.

## Validation Baseline

The current clean validation set is:

```bash
cargo fmt --all --check
cargo test --workspace
uv run --project py python -m compileall py/airlet_audio_lab
uv run --project py airlet-probe-model
git diff --check
```

Recent validation status:

- Rust tests pass across backend, app, and model crates.
- Python package compile check passes.
- Model probe writes `target/model-probe.json`, `target/model-probe.md`,
  `target/model-spec.toml`, and `target/model-probe-debug.png`.
- `docs/roadmap.md` contains no open checkbox items.
