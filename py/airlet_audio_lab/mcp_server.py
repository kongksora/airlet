from __future__ import annotations

import inspect
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
def describe_actions(addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Return the Rust-owned Airlet debug action catalog."""
    return _call({"action": "describe_actions"}, addr)


@mcp.tool()
def call_action(action: dict[str, Any], addr: str = DEFAULT_ADDR) -> dict[str, Any]:
    """Call any Airlet debug action object against the local debug endpoint."""
    return _call(action, addr)


def _register_dynamic_action_tools() -> None:
    try:
        catalog = describe_actions(DEFAULT_ADDR)
    except Exception:
        return

    for spec in catalog.get("actions", []):
        name = spec.get("name")
        if not isinstance(name, str) or name in {"describe_actions"}:
            continue
        tool = _make_action_tool(spec)
        mcp.add_tool(
            tool,
            name=name,
            title=spec.get("title"),
            description=spec.get("description"),
            structured_output=True,
        )


def _make_action_tool(spec: dict[str, Any]):
    action_name = spec["name"]
    parameters = [
        parameter
        for parameter in spec.get("parameters", [])
        if isinstance(parameter, dict) and isinstance(parameter.get("name"), str)
    ]

    def action_tool(**kwargs: Any) -> dict[str, Any]:
        addr = kwargs.pop("addr", DEFAULT_ADDR)
        action: dict[str, Any] = {"action": action_name}
        for parameter in parameters:
            name = parameter["name"]
            if name in kwargs and kwargs[name] is not None:
                action[name] = kwargs[name]
            elif parameter.get("required"):
                raise ValueError(f"missing required Airlet action parameter: {name}")
        return _call(action, addr)

    action_tool.__name__ = f"airlet_action_{action_name}"
    action_tool.__doc__ = spec.get("description")
    action_tool.__annotations__ = {
        parameter["name"]: _schema_annotation(parameter.get("schema", {}))
        for parameter in parameters
    }
    action_tool.__annotations__["addr"] = str
    action_tool.__annotations__["return"] = dict[str, Any]
    action_tool.__signature__ = _tool_signature(parameters)
    return action_tool


def _tool_signature(parameters: list[dict[str, Any]]) -> inspect.Signature:
    required = [parameter for parameter in parameters if parameter.get("required")]
    optional = [parameter for parameter in parameters if not parameter.get("required")]
    signature_parameters = [
        _inspect_parameter(parameter, required=True) for parameter in required
    ] + [
        _inspect_parameter(parameter, required=False) for parameter in optional
    ]
    signature_parameters.append(
        inspect.Parameter(
            "addr",
            inspect.Parameter.KEYWORD_ONLY,
            default=DEFAULT_ADDR,
            annotation=str,
        )
    )
    return inspect.Signature(
        parameters=signature_parameters,
        return_annotation=dict[str, Any],
    )


def _inspect_parameter(parameter: dict[str, Any], *, required: bool) -> inspect.Parameter:
    name = parameter["name"]
    default = inspect.Parameter.empty if required else None
    return inspect.Parameter(
        name,
        inspect.Parameter.KEYWORD_ONLY,
        default=default,
        annotation=_schema_annotation(parameter.get("schema", {})),
    )


def _schema_annotation(schema: object) -> object:
    if not isinstance(schema, dict):
        return Any
    schema_type = schema.get("type")
    if schema_type == "boolean":
        return bool
    if schema_type == "integer":
        return int
    if schema_type == "number":
        return float
    if schema_type == "string":
        return str
    if schema_type == "array":
        return list[Any]
    if schema_type == "object":
        return dict[str, Any]
    return Any


_register_dynamic_action_tools()


def main() -> None:
    mcp.run()


if __name__ == "__main__":
    main()
