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
- For the architecture refactor, use the "Architecture Refactor Roadmap" section
  in `docs/roadmap.md` as the shared source of truth.

## Multi-Agent Workflow

- Assign each agent an explicit ownership area before editing. Do not have two
  agents edit the same module boundary at the same time.
- Keep behavior-preserving code movement separate from behavior changes. A
  module extraction batch should mostly be moves, import fixes, and visibility
  adjustments.
- Agents may add new module files in parallel after agreeing on names, but
  edits to shared module declarations must be serialized.
- Only one agent should edit `src/lib.rs` at a time while it owns Bevy app
  schedule wiring, module declarations, and cross-module imports.
- Only one agent should edit `crates/airlet/src/lib.rs` at a time while it owns
  public facade exports.
- Only one agent should update roadmap checkbox status for a completed batch,
  and only after implementation plus validation.
- If two agents touch adjacent behavior, merge by preserving the tested behavior
  first, then clean up names/imports in a follow-up patch.
- If a post-merge test fails, check module visibility, moved imports, and
  schedule registration before changing runtime behavior.
- Leave unrelated dirty or untracked files alone. In particular, do not remove
  or commit local third-party model assets unless the task explicitly says so.

Recommended ownership lanes for the current refactor:

- App shell: `src/lib.rs`, app/plugin/schedule wiring, module declarations.
- Playback/debug: `AudioOutputState`, rodio integration, debug TCP endpoint,
  and Rust-owned action schema plumbing.
- Mechanism view: procedural cylinder/tooth/comb geometry, comb mesh ranges,
  comb animation view systems.
- Backend core: `crates/airlet`, `compat`, `engine`, `performance`, synthesis,
  notation, and public exports.
- Docs/validation: `docs/roadmap.md`, audit follow-ups, screenshot notes, and
  verification transcripts.

Winding playback ownership lanes:

- Model/probe: `py/airlet_audio_lab/probe_model.py`,
  `assets/models/converted/spec.toml`, and `crates/airlet-model` winding-key
  spec parsing/grouping.
- Interaction: `src/winding.rs`, winding-key hover/press state, wind meter
  accumulation, and visual key rotation.
- Mechanical twin: `src/twin.rs`, spring state, crank angle, cylinder phase,
  cycle-boundary events, and the `MusicBoxTwinPlugin` boundary.
- Playback scheduling: `src/playback.rs`, audio output state and overlapping
  audio players. Playback should follow twin phase events instead of owning
  mechanical time.
- Runtime controls/debug: `src/controls.rs`, `src/debug.rs`, and `docs/mcp.md`
  fields/actions that expose winding state.
- Validation/docs: `docs/roadmap.md`, screenshots, debug dumps, and command
  transcripts for winding behavior.

Parallel-safe targets after ownership assignment:

- `src/screenshot.rs`
- `src/playback.rs`
- `src/debug.rs`
- `src/comb_animation.rs`
- `src/mechanism_view.rs`
- `src/model_view.rs`
- `src/scene.rs`
- `src/twin.rs`
- `crates/airlet/src/synthesis.rs`
- `crates/airlet/src/notation.rs`

Serial edit zones:

- `src/lib.rs`
- `crates/airlet/src/lib.rs`
- shared visual config structs once introduced;
- roadmap checklist status changes.

## Entry Points And Crate Shape

- `src/main.rs` is the only top-level product entry point.
- Do not add extra top-level binaries such as `play_*`, `render_*`, or
  `export_*` for normal workflows.
- Prefer crate APIs and thin app wiring over new bins.
- Keep `crates/airlet` free of Bevy, egui, windowing, rendering, and app UI
  dependencies.
- Prefer workspace dependency inheritance through `[workspace.dependencies]`.
- Avoid exact dependency versions in individual crates unless a crate genuinely
  needs a specific override.

## Debug Action Protocol

- `src/debug.rs` is the single source of truth for debug actions.
- Add or remove actions by updating `DebugAction` and the adjacent
  `ActionCatalog`/`ActionSpec` registry together.
- Do not hand-write matching Python MCP wrappers for individual actions.
  `py/airlet_audio_lab/mcp_server.py` dynamically registers tools from
  `describe_actions`.
- Keep `describe_actions` and `call_action` as stable MCP fallback tools.
- Removed transport controls must stay removed from the catalog and MCP layer:
  `play`, `stop`, `set_cylinder`, and `seek_tick`.

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
- For photoreal lighting work, update `docs/aaa-lighting-research.md` and
  record screenshot evidence in `docs/roadmap.md`.

## Assets And Generated Files

- Treat `assets/` contents as user-provided or third-party assets unless told
  otherwise.
- Do not commit downloaded third-party model files by default.
- It is fine to read and probe those assets locally.
- Do not commit `target/` outputs.
- Do not commit regenerated runtime bake outputs:
  `assets/generated/music_box_aligned_base.*`,
  `assets/generated/music_box_material_baked.*`, or
  `assets/textures/procedural/*.png`.
- Keep `assets/generated/music_box_manual_rounded_shell.glb` tracked. It is a
  hand-edited source input for the current asset bake, not a disposable output.
- Regenerate runtime assets after clone with
  `uv run --project py airlet-bake-materials --manual-rounded-source assets/generated/music_box_manual_rounded_shell.glb`.

## Verification

- For Rust changes, run `cargo fmt --all`, `cargo check --workspace`, and
  `cargo test --workspace` when feasible.
- Run `git diff --check` before committing.
- For Python tool changes, run the relevant `uv run --project py ...` command
  and refresh `py/uv.lock` when dependencies change.
- Keep commits scoped to the completed task and leave unrelated untracked assets
  or user changes alone.
- For pure documentation changes, at minimum run `git diff --check`; Rust tests
  are not required unless the docs update accompanies code changes.
