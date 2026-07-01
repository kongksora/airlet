from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import matplotlib.image as mpimg
import numpy as np

from airlet_audio_lab.debug_client import DEFAULT_ADDR, send_action


DEFAULT_OUT_DIR = Path("target/lighting")


@dataclass(frozen=True)
class LightingShot:
    name: str
    yaw: float
    pitch: float
    radius: float
    target: tuple[float, float, float]
    lid_t: float
    light_yaw: float
    light_pitch: float
    inner_angle: float
    outer_angle: float
    intensity: float


SHOTS = [
    LightingShot(
        name="product",
        yaw=0.48,
        pitch=0.36,
        radius=4.2,
        target=(0.0, 0.6, 0.0),
        lid_t=0.0,
        light_yaw=-0.52,
        light_pitch=1.08,
        inner_angle=0.13,
        outer_angle=0.13,
        intensity=1_120_000.0,
    ),
    LightingShot(
        name="crank",
        yaw=-0.82,
        pitch=0.26,
        radius=2.6,
        target=(-0.55, 0.52, -0.22),
        lid_t=0.0,
        light_yaw=-0.38,
        light_pitch=0.95,
        inner_angle=0.12,
        outer_angle=0.12,
        intensity=1_050_000.0,
    ),
    LightingShot(
        name="comb",
        yaw=0.22,
        pitch=0.24,
        radius=2.1,
        target=(0.02, 0.58, -0.28),
        lid_t=1.0,
        light_yaw=-0.7,
        light_pitch=1.08,
        inner_angle=0.13,
        outer_angle=0.13,
        intensity=1_020_000.0,
    ),
    LightingShot(
        name="cylinder",
        yaw=0.10,
        pitch=0.28,
        radius=2.35,
        target=(0.0, 0.55, -0.15),
        lid_t=1.0,
        light_yaw=-0.6,
        light_pitch=1.0,
        inner_angle=0.13,
        outer_angle=0.13,
        intensity=1_060_000.0,
    ),
    LightingShot(
        name="lid",
        yaw=0.74,
        pitch=0.42,
        radius=3.2,
        target=(-0.08, 0.72, -0.06),
        lid_t=1.0,
        light_yaw=-0.34,
        light_pitch=1.1,
        inner_angle=0.14,
        outer_angle=0.14,
        intensity=1_080_000.0,
    ),
]


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Capture repeatable Airlet AAA lighting screenshot recipes."
    )
    parser.add_argument("--addr", default=DEFAULT_ADDR)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument(
        "--launch",
        action="store_true",
        help="Launch `cargo run --bin airlet` with AIRLET_DEBUG=1 before capture.",
    )
    parser.add_argument(
        "--startup-timeout",
        type=float,
        default=30.0,
        help="Seconds to wait for the debug endpoint when --launch is used.",
    )
    parser.add_argument(
        "--screenshot-timeout",
        type=float,
        default=15.0,
        help="Seconds to wait for each screenshot file.",
    )
    parser.add_argument(
        "--warmup-seconds",
        type=float,
        default=3.0,
        help="Seconds to wait after the endpoint is reachable before the first shot.",
    )
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    process: subprocess.Popen[str] | None = None
    try:
        if args.launch:
            process = _launch_app(args.addr)
        _wait_for_endpoint(args.addr, args.startup_timeout)
        send_action({"action": "set_ui", "visible": False}, args.addr)
        time.sleep(args.warmup_seconds)
        results = [_capture_shot(args.addr, args.out_dir, shot, args.screenshot_timeout) for shot in SHOTS]
        stats_path = args.out_dir / "lighting-screenshot-stats.json"
        stats_path.write_text(json.dumps(results, indent=2), encoding="utf-8")
        print(f"wrote {stats_path}")
    finally:
        if process is not None:
            process.terminate()
            try:
                process.wait(timeout=5.0)
            except subprocess.TimeoutExpired:
                process.kill()


def _launch_app(addr: str) -> subprocess.Popen[str]:
    env = os.environ.copy()
    env["AIRLET_DEBUG"] = "1"
    env.setdefault("RUST_LOG", "warn")
    host, _, port = addr.rpartition(":")
    if host and port:
        env["AIRLET_DEBUG_BIND"] = addr
    return subprocess.Popen(
        ["cargo", "run", "--bin", "airlet"],
        env=env,
        text=True,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )


def _wait_for_endpoint(addr: str, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        try:
            response = send_action({"action": "dump_state"}, addr)
        except OSError:
            time.sleep(0.25)
            continue
        if response.get("ok"):
            return
        time.sleep(0.25)
    raise SystemExit(f"Airlet debug endpoint did not become ready at {addr}")


def _capture_shot(
    addr: str, out_dir: Path, shot: LightingShot, screenshot_timeout: float
) -> dict[str, Any]:
    send_action(
        {
            "action": "set_camera",
            "yaw": shot.yaw,
            "pitch": shot.pitch,
            "radius": shot.radius,
            "target": list(shot.target),
        },
        addr,
    )
    send_action({"action": "set_lid", "t": shot.lid_t}, addr)
    send_action(
        {
            "action": "set_light",
            "yaw": shot.light_yaw,
            "pitch": shot.light_pitch,
            "inner_angle": shot.inner_angle,
            "outer_angle": shot.outer_angle,
            "intensity": shot.intensity,
        },
        addr,
    )
    _wait_for_endpoint(addr, 2.0)
    time.sleep(0.5)
    path = out_dir / f"{shot.name}.png"
    if path.exists():
        path.unlink()
    response = send_action({"action": "screenshot", "path": str(path)}, addr)
    if not response.get("ok"):
        raise RuntimeError(f"failed to request screenshot {shot.name}: {response}")
    _wait_for_file(path, screenshot_timeout)
    stats = _image_stats(path)
    return {
        "shot": asdict(shot),
        "path": str(path),
        "bytes": path.stat().st_size,
        "image": stats,
    }


def _wait_for_file(path: Path, timeout: float) -> None:
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_size > 0:
            return
        time.sleep(0.25)
    raise TimeoutError(f"screenshot was not written: {path}")


def _image_stats(path: Path) -> dict[str, Any]:
    image = np.asarray(mpimg.imread(path), dtype=np.float32)
    if image.ndim == 2:
        luminance = image
    else:
        rgb = image[..., :3]
        luminance = rgb[..., 0] * 0.2126 + rgb[..., 1] * 0.7152 + rgb[..., 2] * 0.0722
    return {
        "shape": list(image.shape),
        "luminance_min": round(float(luminance.min()), 6),
        "luminance_max": round(float(luminance.max()), 6),
        "luminance_mean": round(float(luminance.mean()), 6),
    }


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        sys.exit(130)
