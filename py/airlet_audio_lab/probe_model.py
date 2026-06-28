from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import trimesh


DEFAULT_MODEL = Path("assets/models/converted/music_box.glb")
DEFAULT_OUT_DIR = Path("target")

ROLE_COLORS = {
    "body": "#8b4a2f",
    "lid": "#d66a3f",
    "hinge": "#d2a936",
    "handle": "#d9d9d9",
    "mechanism": "#6f7f86",
    "unknown": "#7b6fd0",
    "open_reference": "#4f9bd8",
}


@dataclass
class MeshProbe:
    node: str
    geometry: str
    bounds: list[list[float]]
    center: list[float]
    extent: list[float]
    cluster: str
    role: str


@dataclass
class ClusterProbe:
    name: str
    mesh_count: int
    bounds: list[list[float]]
    center: list[float]
    extent: list[float]


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Probe the downloaded music-box GLB and classify model parts."
    )
    parser.add_argument("--model", type=Path, default=DEFAULT_MODEL)
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    scene = trimesh.load(args.model, force="scene")
    meshes = _collect_meshes(scene)
    labels = _cluster_by_x(meshes)
    clusters = _summarize_clusters(meshes, labels)
    closed_label = min(clusters, key=lambda label: clusters[label]["bounds"][1][1])
    open_label = 1 - closed_label

    probes: list[MeshProbe] = []
    for index, mesh in enumerate(meshes):
        cluster_name = "closed" if labels[index] == closed_label else "open_reference"
        role = (
            _classify_closed_mesh(mesh, clusters[closed_label]["bounds"])
            if labels[index] == closed_label
            else "open_reference"
        )
        probes.append(
            MeshProbe(
                node=mesh["node"],
                geometry=mesh["geometry"],
                bounds=_round_array(mesh["bounds"]),
                center=_round_vec(mesh["center"]),
                extent=_round_vec(mesh["extent"]),
                cluster=cluster_name,
                role=role,
            )
        )

    cluster_probes = [
        ClusterProbe(
            name="closed" if label == closed_label else "open_reference",
            mesh_count=int(clusters[label]["mesh_count"]),
            bounds=_round_array(clusters[label]["bounds"]),
            center=_round_vec(clusters[label]["center"]),
            extent=_round_vec(clusters[label]["extent"]),
        )
        for label in sorted(clusters)
    ]
    cluster_probes.sort(key=lambda cluster: cluster.name)

    lid = _largest_role_mesh(probes, "lid")
    open_lid = _open_lid_reference(meshes, labels, open_label)
    hinge = _estimate_hinge(lid, open_lid)

    payload = {
        "model": str(args.model),
        "clusters": [asdict(cluster) for cluster in cluster_probes],
        "closed_model": {
            "cluster": "closed",
            "reason": "The closed cluster has the lower vertical max bound.",
        },
        "lid_animation_estimate": hinge,
        "meshes": [asdict(probe) for probe in probes],
    }

    json_path = args.out_dir / "model-probe.json"
    report_path = args.out_dir / "model-probe.md"
    image_path = args.out_dir / "model-probe-debug.png"
    json_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    report_path.write_text(_render_report(payload), encoding="utf-8")
    _render_debug_image(probes, image_path)

    print(f"wrote {json_path}")
    print(f"wrote {report_path}")
    print(f"wrote {image_path}")


def _collect_meshes(scene: trimesh.Scene) -> list[dict[str, Any]]:
    meshes = []
    for node in scene.graph.nodes_geometry:
        matrix, geometry_name = scene.graph.get(node)
        geometry = scene.geometry[geometry_name]
        vertices = np.c_[geometry.vertices, np.ones(len(geometry.vertices))]
        points = (vertices @ matrix.T)[:, :3]
        bounds = np.array([points.min(axis=0), points.max(axis=0)])
        meshes.append(
            {
                "node": str(node),
                "geometry": str(geometry_name),
                "bounds": bounds,
                "center": bounds.mean(axis=0),
                "extent": bounds[1] - bounds[0],
            }
        )
    return meshes


def _cluster_by_x(meshes: list[dict[str, Any]]) -> np.ndarray:
    xs = np.array([mesh["center"][0] for mesh in meshes])
    centers = np.array([xs.min(), xs.max()])
    labels = np.zeros(len(xs), dtype=int)
    for _ in range(32):
        labels = np.abs(xs[:, None] - centers[None, :]).argmin(axis=1)
        next_centers = np.array([xs[labels == i].mean() for i in range(2)])
        if np.allclose(next_centers, centers):
            break
        centers = next_centers
    return labels


def _summarize_clusters(
    meshes: list[dict[str, Any]], labels: np.ndarray
) -> dict[int, dict[str, Any]]:
    clusters = {}
    for label in sorted(set(labels)):
        group = [mesh for mesh, item_label in zip(meshes, labels) if item_label == label]
        bounds = _combined_bounds([mesh["bounds"] for mesh in group])
        clusters[int(label)] = {
            "mesh_count": len(group),
            "bounds": bounds,
            "center": bounds.mean(axis=0),
            "extent": bounds[1] - bounds[0],
        }
    return clusters


def _classify_closed_mesh(mesh: dict[str, Any], cluster_bounds: np.ndarray) -> str:
    bounds = mesh["bounds"]
    center = mesh["center"]
    extent = mesh["extent"]
    cluster_extent = cluster_bounds[1] - cluster_bounds[0]
    height_mid = cluster_bounds[0][1] + cluster_extent[1] * 0.58
    large_x = extent[0] > cluster_extent[0] * 0.45
    large_z = extent[2] > cluster_extent[2] * 0.45
    flat_y = extent[1] < cluster_extent[1] * 0.40

    if large_x and large_z and flat_y and center[1] >= height_mid:
        return "lid"
    if large_x and large_z:
        return "body"
    if center[0] > cluster_bounds[1][0] - cluster_extent[0] * 0.18:
        return "handle"
    if center[1] >= height_mid and (extent[0] > cluster_extent[0] * 0.20 or extent[2] > 0.08):
        return "hinge"
    if center[1] > cluster_bounds[0][1] + cluster_extent[1] * 0.28:
        return "mechanism"
    return "unknown"


def _largest_role_mesh(probes: list[MeshProbe], role: str) -> MeshProbe:
    matches = [probe for probe in probes if probe.role == role]
    if not matches:
        raise RuntimeError(f"no mesh classified as {role}")
    return max(matches, key=lambda probe: np.prod(np.array(probe.extent)))


def _open_lid_reference(
    meshes: list[dict[str, Any]], labels: np.ndarray, open_label: int
) -> dict[str, Any]:
    group = [mesh for mesh, label in zip(meshes, labels) if label == open_label]
    bounds = _combined_bounds([mesh["bounds"] for mesh in group])
    group_extent = bounds[1] - bounds[0]
    candidates = [
        mesh
        for mesh in group
        if mesh["extent"][0] > group_extent[0] * 0.35
        and mesh["extent"][2] > group_extent[2] * 0.35
        and mesh["center"][1] > bounds[0][1] + group_extent[1] * 0.45
    ]
    if not candidates:
        candidates = group
    return max(
        candidates,
        key=lambda mesh: mesh["extent"][0]
        * mesh["extent"][2]
        * (1.0 + (mesh["center"][1] - bounds[0][1]) / max(group_extent[1], 1e-6)),
    )


def _estimate_hinge(lid: MeshProbe, open_lid: dict[str, Any]) -> dict[str, Any]:
    lid_bounds = np.array(lid.bounds)
    open_bounds = open_lid["bounds"]
    pivot = [
        float((lid_bounds[0][0] + lid_bounds[1][0]) * 0.5),
        float(lid_bounds[1][1]),
        float(lid_bounds[0][2]),
    ]
    closed_depth = float(lid_bounds[1][2] - lid_bounds[0][2])
    open_vertical = float(open_bounds[1][1] - open_bounds[0][1])
    open_angle = float(np.degrees(np.arctan2(open_vertical, max(closed_depth, 1e-6))))
    return {
        "status": "heuristic",
        "closed_lid_mesh": lid.geometry,
        "open_reference_lid_mesh": open_lid["geometry"],
        "pivot": _round_vec(np.array(pivot)),
        "axis": [1.0, 0.0, 0.0],
        "closed_angle_degrees": 0.0,
        "open_angle_degrees": round(max(65.0, min(110.0, open_angle)), 3),
        "note": "Pivot is the closed lid rear/top edge; verify visually before rigging.",
    }


def _render_report(payload: dict[str, Any]) -> str:
    lines = [
        "# Airlet Model Probe",
        "",
        f"- Model: `{payload['model']}`",
        "- Decision: the lower-height spatial cluster is the closed model.",
        "",
        "## Clusters",
        "",
    ]
    for cluster in payload["clusters"]:
        lines.extend(
            [
                f"### {cluster['name']}",
                "",
                f"- Meshes: {cluster['mesh_count']}",
                f"- Bounds: `{cluster['bounds']}`",
                f"- Center: `{cluster['center']}`",
                f"- Extent: `{cluster['extent']}`",
                "",
            ]
        )

    estimate = payload["lid_animation_estimate"]
    lines.extend(
        [
            "## Lid Animation Estimate",
            "",
            f"- Closed lid mesh: `{estimate['closed_lid_mesh']}`",
            f"- Open reference lid mesh: `{estimate['open_reference_lid_mesh']}`",
            f"- Pivot: `{estimate['pivot']}`",
            f"- Axis: `{estimate['axis']}`",
            f"- Open angle: `{estimate['open_angle_degrees']}` degrees",
            f"- Note: {estimate['note']}",
            "",
            "## Closed Mesh Roles",
            "",
            "| role | geometry | center | extent |",
            "| --- | --- | --- | --- |",
        ]
    )
    closed = [mesh for mesh in payload["meshes"] if mesh["cluster"] == "closed"]
    role_order = {"lid": 0, "body": 1, "hinge": 2, "handle": 3, "mechanism": 4, "unknown": 5}
    for mesh in sorted(closed, key=lambda item: (role_order[item["role"]], item["geometry"])):
        lines.append(
            f"| {mesh['role']} | `{mesh['geometry']}` | `{mesh['center']}` | `{mesh['extent']}` |"
        )
    lines.append("")
    return "\n".join(lines)


def _render_debug_image(probes: list[MeshProbe], path: Path) -> None:
    fig = plt.figure(figsize=(12, 8))
    ax = fig.add_subplot(111, projection="3d")
    for probe in probes:
        bounds = np.array(probe.bounds)
        color = ROLE_COLORS[probe.role]
        alpha = 0.18 if probe.cluster == "open_reference" else 0.42
        _draw_box(ax, bounds, color, alpha)
        center = np.array(probe.center)
        ax.scatter(center[0], center[2], center[1], color=color, s=12)
    ax.set_xlabel("X")
    ax.set_ylabel("Z")
    ax.set_zlabel("Y")
    ax.set_title("Airlet model classification: closed parts vs open reference")
    ax.view_init(elev=24, azim=-62)
    ax.set_box_aspect((2.2, 1.2, 0.9))
    handles = [
        plt.Line2D([0], [0], marker="s", color="w", label=role, markerfacecolor=color, markersize=10)
        for role, color in ROLE_COLORS.items()
    ]
    ax.legend(handles=handles, loc="upper left")
    fig.tight_layout()
    fig.savefig(path, dpi=180)
    plt.close(fig)


def _draw_box(ax: Any, bounds: np.ndarray, color: str, alpha: float) -> None:
    x0, y0, z0 = bounds[0]
    x1, y1, z1 = bounds[1]
    corners = np.array(
        [
            [x0, z0, y0],
            [x1, z0, y0],
            [x1, z1, y0],
            [x0, z1, y0],
            [x0, z0, y1],
            [x1, z0, y1],
            [x1, z1, y1],
            [x0, z1, y1],
        ]
    )
    edges = [
        (0, 1),
        (1, 2),
        (2, 3),
        (3, 0),
        (4, 5),
        (5, 6),
        (6, 7),
        (7, 4),
        (0, 4),
        (1, 5),
        (2, 6),
        (3, 7),
    ]
    for start, end in edges:
        points = corners[[start, end]]
        ax.plot(points[:, 0], points[:, 1], points[:, 2], color=color, alpha=alpha)


def _combined_bounds(bounds: list[np.ndarray]) -> np.ndarray:
    return np.array(
        [
            np.min([bound[0] for bound in bounds], axis=0),
            np.max([bound[1] for bound in bounds], axis=0),
        ]
    )


def _round_array(value: np.ndarray) -> list[list[float]]:
    return [[round(float(item), 6) for item in row] for row in value]


def _round_vec(value: np.ndarray) -> list[float]:
    return [round(float(item), 6) for item in value]


if __name__ == "__main__":
    main()
