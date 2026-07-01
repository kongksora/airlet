from __future__ import annotations

import argparse
import json
import socket
import sys
from typing import Any


DEFAULT_ADDR = "127.0.0.1:4777"


def main() -> None:
    parser = argparse.ArgumentParser(description="Send one Airlet debug JSON action.")
    parser.add_argument("action", help="JSON action object or a shorthand action name")
    parser.add_argument("--addr", default=DEFAULT_ADDR)
    args = parser.parse_args()

    request = _parse_action(args.action, args.addr)
    parsed = send_action(request, args.addr)
    json.dump(parsed, sys.stdout, indent=2, ensure_ascii=False)
    sys.stdout.write("\n")
    if not parsed.get("ok", False):
        raise SystemExit(1)


def send_action(action: dict[str, Any], addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    host, port = _parse_addr(addr)
    with socket.create_connection((host, port), timeout=5.0) as stream:
        payload = json.dumps(action, separators=(",", ":")) + "\n"
        stream.sendall(payload.encode("utf-8"))
        response = _read_line(stream)
    parsed = json.loads(response)
    if not isinstance(parsed, dict):
        raise RuntimeError("debug endpoint returned a non-object response")
    return parsed


def _parse_action(action: str, addr: str) -> dict[str, Any]:
    alias = {
        "state": "dump_state",
        "mechanism": "dump_mechanism",
    }.get(action, action)
    if _catalog_contains_action(alias, addr):
        return {"action": alias}
    try:
        parsed = json.loads(action)
    except json.JSONDecodeError as exc:
        raise SystemExit(f"invalid action JSON: {exc}") from exc
    if not isinstance(parsed, dict):
        raise SystemExit("debug action must be a JSON object")
    return parsed


def _catalog_contains_action(action: str, addr: str) -> bool:
    try:
        catalog = send_action({"action": "describe_actions"}, addr)
    except OSError:
        return False
    except TimeoutError:
        return False
    except RuntimeError:
        return False
    if not catalog.get("ok"):
        return False
    data = catalog.get("data")
    if not isinstance(data, dict):
        return False
    actions = data.get("actions")
    if not isinstance(actions, list):
        return False
    return any(
        isinstance(item, dict) and item.get("name") == action
        for item in actions
    )


def _parse_addr(addr: str) -> tuple[str, int]:
    host, sep, port = addr.rpartition(":")
    if not sep:
        raise SystemExit("address must be host:port")
    return host, int(port)


def _read_line(stream: socket.socket) -> str:
    chunks = []
    while True:
        chunk = stream.recv(4096)
        if not chunk:
            break
        chunks.append(chunk)
        if b"\n" in chunk:
            break
    return b"".join(chunks).splitlines()[0].decode("utf-8")


if __name__ == "__main__":
    main()
