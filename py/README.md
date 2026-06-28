# Airlet Audio Lab

Fast Python tools for reference analysis and synthesis probes.

Current best full-song probe:

```bash
uv run --project py airlet-render-air-probe py/out/analysis/A/partials.csv --preset a-dry --out py/out/air_probes/air_from_A_v3_dry_preset.wav
cargo run --bin render_air -- a-dry py/out/air_probes/air_from_A_rust_a_dry.wav
```

## Commands

```bash
uv run --project py python -m airlet_audio_lab.analyze_reference
uv run --project py python -m airlet_audio_lab.probe_model
uv run --project py python -m airlet_audio_lab.synth_probe py/out/analysis/A/partials.csv
uv run --project py python -m airlet_audio_lab.render_air_probe py/out/analysis/Eb1/partials.csv
```

Equivalent script entrypoints:

```bash
uv run --project py airlet-analyze-reference
uv run --project py airlet-probe-model
uv run --project py airlet-render-air-probe py/out/analysis/A/partials.csv --preset a-dry
uv run --project py airlet-synth-probe py/out/analysis/A/partials.csv
```

Outputs are written under `py/out/`.
Model probe outputs are written under `target/`.
