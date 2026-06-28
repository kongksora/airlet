# Airlet MCP Adapter

Airlet exposes runtime control through a local JSON debug endpoint in the Bevy
app and wraps that endpoint with a Python MCP stdio server.

## Start The App

Run the normal product entry:

```sh
cargo run --bin airlet
```

The debug endpoint listens on `127.0.0.1:4777` by default.

Environment controls:

- `AIRLET_DEBUG=0` disables the endpoint.
- `AIRLET_DEBUG_BIND=127.0.0.1:4888` changes the bind address.

## Start The MCP Server

```sh
uv run --project py airlet-mcp
```

The MCP process uses stdio transport. It should be launched by an MCP host, or
run directly only for smoke testing.

## Tools

- `dump_state`: current camera, light, rig, playback, and mechanism summary.
- `dump_mechanism`: cylinder, comb, tooth, and score-to-geometry calibration.
- `set_camera`: set orbit camera yaw, pitch, and radius.
- `set_light`: set spotlight yaw, pitch, cone angles, and intensity.
- `set_lid`: set parameterized lid open amount `t`.
- `set_cylinder`: set cylinder rotation in degrees.
- `seek_tick`: seek playback and cylinder phase to an absolute score tick.
- `play`: start the default performance.
- `stop`: stop playback.
- `screenshot`: save a primary-window screenshot.

All tools accept an optional `addr` parameter for non-default debug endpoints.
