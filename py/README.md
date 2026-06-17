# Airlet Audio Lab

Fast Python tools for reference analysis and synthesis probes.

## Commands

```bash
uv run --project py python -m airlet_audio_lab.analyze_reference
uv run --project py python -m airlet_audio_lab.synth_probe py/out/analysis/A/partials.csv
uv run --project py python -m airlet_audio_lab.render_air_probe py/out/analysis/Eb1/partials.csv
```

Equivalent script entrypoints:

```bash
uv run --project py airlet-analyze-reference
uv run --project py airlet-render-air-probe py/out/analysis/A/partials.csv --preset a-dry
uv run --project py airlet-synth-probe py/out/analysis/A/partials.csv
```

Outputs are written under `py/out/`.
