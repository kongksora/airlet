# Airlet MCP Adapter

Airlet exposes runtime control through a local JSON debug endpoint in the Bevy
app and wraps that endpoint with a Python MCP stdio server.

## Start The App

Run the normal product entry:

```sh
AIRLET_DEBUG=1 cargo run --bin airlet
```

With `AIRLET_DEBUG=1`, the debug endpoint listens on `127.0.0.1:4777` by
default.

Environment controls:

- unset `AIRLET_DEBUG`, `AIRLET_DEBUG=0`, or `AIRLET_DEBUG=false` disables the
  endpoint.
- `AIRLET_DEBUG=1` or `AIRLET_DEBUG=true` enables the endpoint.
- `AIRLET_DEBUG_BIND=127.0.0.1:4888` changes the bind address.

## Start The MCP Server

```sh
uv run --project py airlet-mcp
```

The MCP process uses stdio transport. It should be launched by an MCP host, or
run directly only for smoke testing.

## Schema-Driven Tools

The Rust app owns the action protocol. The debug endpoint exposes
`describe_actions`, which returns the action catalog with tool names,
descriptions, and parameter schemas. The Python MCP adapter reads that catalog
at startup and dynamically registers matching MCP tools when the app is
reachable.

Stable fallback tools:

- `describe_actions`: return the Rust-owned action catalog.
- `call_action`: send any JSON action object directly.

When the app is running, the MCP adapter also registers one tool per action in
the catalog, such as `dump_state`, `set_camera`, `full_wind`, `pause`, `reset`,
and `screenshot`. Deprecated controls such as direct `play`, `stop`,
`set_cylinder`, and `seek_tick` are not exposed.

All tools accept an optional `addr` parameter for non-default debug endpoints.

`set_light` covers the full AAA lighting rig. In addition to spotlight yaw,
pitch, cone angles, and intensity, it accepts optional `key`, `fill`, `rim`,
`accent`, `ambient`, and `environment` intensity fields. These parameter names
come from the Rust action catalog, so MCP adapters should not hand-copy them.
