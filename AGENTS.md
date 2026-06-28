# Airlet Agent Notes

## Project Direction

- Airlet is ultimately a 3D music-box performance app.
- The top-level app is the Bevy exhibit/performance surface.
- The backend crate under `crates/airlet` owns score, timeline, audio, presets,
  mechanism hints, and other Bevy-free core logic.
- Preserve the current Air track and audible behavior when doing structural
  refactors unless the user explicitly asks to retune or replace the sound.

## Roadmap Discipline

- Before every large task, write or update `docs/roadmap.md` first.
- Treat roadmap checklists as completion targets, not loose notes.
- Keep roadmap entries concrete enough to validate with files, generated
  artifacts, screenshots, tests, or command output.
- Mark checklist items complete only after the relevant implementation or
  validation has actually happened.

## Entry Points And Crate Shape

- `src/main.rs` is the only top-level product entry point.
- Do not add extra top-level binaries such as `play_*`, `render_*`, or
  `export_*` for normal workflows.
- Prefer crate APIs and thin app wiring over new bins.
- Keep `crates/airlet` free of Bevy, egui, windowing, rendering, and app UI
  dependencies.

## Python And Asset Experiments

- You may use the Python environment for fast experiments, including audio,
  model, and asset experiments, instead of forcing every validation path through
  Rust.
- Use `uv` and the existing `py/` project for repeatable Python tools.
- Generated probe outputs should go under `target/` or `py/out/` unless there is
  a reason to promote them to tracked source artifacts.
- For model work, prefer measurable probes first: bounds, centers, cluster
  assignments, screenshots, and debug images before committing to a rigging
  implementation.

## 3D App Validation

- For Bevy visual work, validate with screenshots, not just compilation.
- The app supports `AIRLET_SCREENSHOT=<path> cargo run` for primary-window
  screenshot capture and automatic exit.
- Use image statistics such as `identify` or `magick` to catch black or
  near-black frames, then inspect the screenshot visually.
- For GLB loading in Bevy 0.19, load scenes through
  `WorldAssetRoot(GltfAssetLabel::Scene(0).from_asset(...))`.
- Do not assume downloaded model axes or bounds are correct; measure them from
  the asset before placing or animating model parts.

## Assets And Generated Files

- Treat `assets/` contents as user-provided or third-party assets unless told
  otherwise.
- Do not commit downloaded third-party model files by default.
- It is fine to read and probe those assets locally.
- Do not commit `target/` outputs.

## Verification

- For Rust changes, run `cargo fmt --all`, `cargo check --workspace`, and
  `cargo test --workspace` when feasible.
- Run `git diff --check` before committing.
- For Python tool changes, run the relevant `uv run --project py ...` command
  and refresh `py/uv.lock` when dependencies change.
- Keep commits scoped to the completed task and leave unrelated untracked assets
  or user changes alone.
