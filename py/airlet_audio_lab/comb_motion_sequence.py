from __future__ import annotations

import argparse
import json
import math
import time
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import matplotlib.image as mpimg

from .debug_client import DEFAULT_ADDR, send_action


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Capture a close-up comb tine pluck/release/vibration sequence."
    )
    parser.add_argument("--addr", default=DEFAULT_ADDR)
    parser.add_argument("--event-index", type=int, default=0)
    parser.add_argument("--out-dir", default="target/comb-motion-sequence")
    parser.add_argument("--yaw", type=float, default=-1.15)
    parser.add_argument("--pitch", type=float, default=0.62)
    parser.add_argument("--radius", type=float, default=0.95)
    parser.add_argument("--target", type=float, nargs=3, default=[-0.15, 0.50, 0.02])
    parser.add_argument("--crop", type=int, nargs=4, default=[300, 160, 980, 650])
    parser.add_argument("--show-ui", action="store_true")
    args = parser.parse_args()

    out_dir = Path(args.out_dir)
    out_dir.mkdir(parents=True, exist_ok=True)

    mechanism = send_action({"action": "dump_mechanism"}, args.addr)["data"]
    events = mechanism["comb_animation"]["release_alignment_preview"]
    if not events:
        raise SystemExit("mechanism has no comb animation events")
    event = events[min(max(args.event_index, 0), len(events) - 1)]
    frames = _frame_plan(event)

    send_action({"action": "set_lid", "t": 1.0}, args.addr)
    send_action({"action": "set_ui", "visible": args.show_ui}, args.addr)
    send_action(
        {
            "action": "set_camera",
            "yaw": args.yaw,
            "pitch": args.pitch,
            "radius": args.radius,
            "target": args.target,
        },
        args.addr,
    )
    send_action(
        {
            "action": "set_light",
            "inner_angle": 0.14,
            "outer_angle": 0.26,
            "intensity": 1_900_000.0,
        },
        args.addr,
    )

    captured: list[dict[str, Any]] = []
    for frame in frames:
        tick = int(frame["tick"])
        send_action({"action": "seek_tick", "tick": tick}, args.addr)
        path = out_dir / f"{frame['name']}_tick_{tick}.png"
        if path.exists():
            path.unlink()
        send_action({"action": "screenshot", "path": str(path)}, args.addr)
        _wait_for_file(path)
        captured.append({**frame, "path": str(path)})

    metadata = {
        "event": event,
        "frames": captured,
        "camera": {
            "yaw": args.yaw,
            "pitch": args.pitch,
            "radius": args.radius,
            "target": args.target,
            "show_ui": args.show_ui,
        },
        "crop": args.crop,
    }
    metadata_path = out_dir / "sequence.json"
    metadata_path.write_text(json.dumps(metadata, indent=2), encoding="utf-8")
    sheet_path = out_dir / "contact_sheet.png"
    _write_contact_sheet(captured, sheet_path, args.crop)
    print(json.dumps({"metadata": str(metadata_path), "contact_sheet": str(sheet_path)}, indent=2))


def _frame_plan(event: dict[str, Any]) -> list[dict[str, Any]]:
    contact_start = int(event.get("contact_start_tick", event["pluck_start_tick"]))
    max_deflection_start = int(event.get("max_deflection_start_tick", event["release_tick"]))
    release = int(event["release_tick"])
    lift_window = max(1, max_deflection_start - contact_start)
    vibration_ticks = max(1, int(event["vibration_ticks"]))
    return [
        {"name": "pre_contact", "tick": contact_start - max(1, lift_window // 8)},
        {"name": "contact_start", "tick": contact_start},
        {"name": "mid_lift", "tick": contact_start + lift_window // 2},
        {"name": "max_deflection", "tick": max_deflection_start},
        {"name": "release", "tick": release},
        {"name": "early_vibration", "tick": release + vibration_ticks // 10},
        {"name": "late_decay", "tick": release + vibration_ticks // 2},
    ]


def _wait_for_file(path: Path) -> None:
    deadline = time.monotonic() + 8.0
    while time.monotonic() < deadline:
        if path.exists() and path.stat().st_size > 0:
            return
        time.sleep(0.1)
    raise TimeoutError(f"screenshot was not written: {path}")


def _write_contact_sheet(frames: list[dict[str, Any]], path: Path, crop: list[int]) -> None:
    columns = 3
    rows = math.ceil(len(frames) / columns)
    fig, axes = plt.subplots(rows, columns, figsize=(14, 3.75 * rows), dpi=140)
    x0, y0, x1, y1 = crop
    flat_axes = list(axes.flat) if hasattr(axes, "flat") else [axes]
    for ax, frame in zip(flat_axes, frames, strict=False):
        image = mpimg.imread(frame["path"])
        image = image[y0:y1, x0:x1]
        ax.imshow(image)
        ax.set_title(f"{frame['name']}\ntick {frame['tick']}", fontsize=8)
        ax.axis("off")
    for ax in flat_axes[len(frames) :]:
        ax.axis("off")
    fig.tight_layout(pad=0.8)
    fig.savefig(path)
    plt.close(fig)


if __name__ == "__main__":
    main()
