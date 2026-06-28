from __future__ import annotations

from typing import Any

from mcp.server.fastmcp import FastMCP

from airlet_audio_lab.debug_client import DEFAULT_ADDR, send_action


mcp = FastMCP("airlet")


def _unwrap(response: dict[str, Any]) -> dict[str, Any]:
    if response.get("ok"):
        data = response.get("data")
        if isinstance(data, dict):
            return data
        return {"value": data}
    error = response.get("error") or "airlet debug endpoint returned an error"
    raise RuntimeError(str(error))


def _call(action: dict[str, Any], addr: str) -> dict[str, Any]:
    return _unwrap(send_action(action, addr))


@mcp.tool()
def dump_state(addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Return the current Airlet camera, light, rig, playback, and mechanism state."""
    return _call({"action": "dump_state"}, addr)


@mcp.tool()
def dump_mechanism(addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Return detailed cylinder, comb, tooth, and score-to-geometry mapping data."""
    return _call({"action": "dump_mechanism"}, addr)


@mcp.tool()
def set_camera(
    yaw: float | None = None,
    pitch: float | None = None,
    radius: float | None = None,
    addr: str = DEFAULT_ADDR,
) -> dict[str, Any]:
    """Set orbit camera parameters in radians/meters and return the updated state."""
    return _call(
        {"action": "set_camera", "yaw": yaw, "pitch": pitch, "radius": radius},
        addr,
    )


@mcp.tool()
def set_light(
    yaw: float | None = None,
    pitch: float | None = None,
    inner_angle: float | None = None,
    outer_angle: float | None = None,
    intensity: float | None = None,
    addr: str = DEFAULT_ADDR,
) -> dict[str, Any]:
    """Set spotlight direction, cone angles, and intensity, then return the updated state."""
    return _call(
        {
            "action": "set_light",
            "yaw": yaw,
            "pitch": pitch,
            "inner_angle": inner_angle,
            "outer_angle": outer_angle,
            "intensity": intensity,
        },
        addr,
    )


@mcp.tool()
def set_lid(t: float, addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Set lid open parameter t in [0, 1] and return the updated state."""
    return _call({"action": "set_lid", "t": t}, addr)


@mcp.tool()
def set_cylinder(degrees: float, addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Set cylinder rotation in degrees and return the updated state."""
    return _call({"action": "set_cylinder", "degrees": degrees}, addr)


@mcp.tool()
def seek_tick(tick: int, addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Seek playback and cylinder phase to an absolute score tick."""
    return _call({"action": "seek_tick", "tick": tick}, addr)


@mcp.tool()
def play(addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Start the Airlet performance."""
    return _call({"action": "play"}, addr)


@mcp.tool()
def stop(addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Stop the Airlet performance."""
    return _call({"action": "stop"}, addr)


@mcp.tool()
def screenshot(
    path: str = "target/airlet-mcp-shot.png",
    addr: str = DEFAULT_ADDR,
) -> dict[str, Any]:
    """Capture the primary Bevy window to a PNG path."""
    return _call({"action": "screenshot", "path": path}, addr)


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
