# Asset Generation

Airlet keeps source assets and generation code in Git. Large runtime bake
outputs are intentionally ignored and should be regenerated locally after clone.

## Tracked Inputs

- `assets/models/converted/music_box.glb`
  - Original converted music-box model.
- `assets/models/converted/source_spec.toml`
  - Probe-derived source-space model metadata.
- `assets/models/converted/spec.toml`
  - Runtime model spec and generated output paths.
- `assets/generated/music_box_manual_rounded_shell.glb`
  - Hand-edited rounded-shell source. Treat this as an input asset, not a
    disposable bake output.
- `py/airlet_audio_lab/*.py`
  - Procedural texture, aligned-base, material-bake, and validation tools.

## Ignored Outputs

These files are generated and not tracked:

- `assets/generated/music_box_aligned_base.glb`
- `assets/generated/music_box_aligned_base.json`
- `assets/generated/music_box_material_baked.glb`
- `assets/generated/music_box_material_baked.json`
- `assets/textures/procedural/*.png`

## Regenerate Runtime Assets

From the repository root:

```bash
uv run --project py airlet-bake-materials \
  --manual-rounded-source assets/generated/music_box_manual_rounded_shell.glb
```

This command regenerates procedural PBR textures, rebuilds the aligned-base
intermediate, and writes the final material-baked GLB referenced by
`assets/models/converted/spec.toml`.

After generation, the app can be run normally:

```bash
cargo run
```

## Validation

Use the usual project checks after changing the generation pipeline:

```bash
uv run --project py python -m compileall py/airlet_audio_lab
cargo fmt --all
cargo check --workspace
cargo test --workspace
git diff --check
```
