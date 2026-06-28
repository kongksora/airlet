from __future__ import annotations

import argparse
import json
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import matplotlib.pyplot as plt
import numpy as np
import trimesh
from scipy.spatial import cKDTree


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
    pca_axes: list[list[float]]
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
                pca_axes=mesh["pca_axes"],
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

    lid_probe = _largest_role_mesh(probes, "lid")
    lid = _mesh_by_geometry(meshes, lid_probe.geometry)
    open_lid = _open_lid_reference(meshes, labels, open_label)
    basis = _estimate_basis(lid_probe)
    hinge = _estimate_hinge(
        lid_probe=lid_probe,
        lid_mesh=lid,
        open_lid=open_lid,
        meshes=meshes,
        labels=labels,
        closed_label=closed_label,
        open_label=open_label,
        basis=basis,
    )

    payload = {
        "model": str(args.model),
        "clusters": [asdict(cluster) for cluster in cluster_probes],
        "closed_model": {
            "cluster": "closed",
            "reason": "The closed cluster has the lower vertical max bound.",
        },
        "basis": basis,
        "lid_animation_estimate": hinge,
        "meshes": [asdict(probe) for probe in probes],
    }

    json_path = args.out_dir / "model-probe.json"
    report_path = args.out_dir / "model-probe.md"
    image_path = args.out_dir / "model-probe-debug.png"
    spec_path = args.out_dir / "model-spec.toml"
    json_path.write_text(json.dumps(payload, indent=2), encoding="utf-8")
    report_path.write_text(_render_report(payload), encoding="utf-8")
    spec_path.write_text(_render_spec_draft(payload, meshes), encoding="utf-8")
    _render_debug_image(probes, image_path)

    print(f"wrote {json_path}")
    print(f"wrote {report_path}")
    print(f"wrote {spec_path}")
    print(f"wrote {image_path}")


def _collect_meshes(scene: trimesh.Scene) -> list[dict[str, Any]]:
    meshes = []
    for node in scene.graph.nodes_geometry:
        matrix, geometry_name = scene.graph.get(node)
        geometry = scene.geometry[geometry_name]
        vertices = np.c_[geometry.vertices, np.ones(len(geometry.vertices))]
        points = (vertices @ matrix.T)[:, :3]
        bounds = np.array([points.min(axis=0), points.max(axis=0)])
        pca_axes = _pca_axes(points)
        meshes.append(
            {
                "node": str(node),
                "geometry": str(geometry_name),
                "points": points,
                "pca_axes": pca_axes,
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


def _pca_axes(points: np.ndarray) -> list[list[float]]:
    centered = points - points.mean(axis=0)
    values, vectors = np.linalg.eigh(np.cov(centered.T))
    axes = vectors[:, np.argsort(values)[::-1]].T
    return [_round_vec(axis / np.linalg.norm(axis)) for axis in axes]


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


def _mesh_by_geometry(meshes: list[dict[str, Any]], geometry: str) -> dict[str, Any]:
    for mesh in meshes:
        if mesh["geometry"] == geometry:
            return mesh
    raise RuntimeError(f"no mesh with geometry {geometry}")


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


def _fit_body_alignment(
    meshes: list[dict[str, Any]], labels: np.ndarray, closed_label: int, open_label: int
) -> dict[str, Any]:
    closed = [mesh for mesh, label in zip(meshes, labels) if label == closed_label]
    opened = [mesh for mesh, label in zip(meshes, labels) if label == open_label]
    fits = []
    for closed_mesh in closed:
        closed_volume = float(np.prod(closed_mesh["extent"]))
        if closed_volume < 0.01:
            continue
        for open_mesh in opened:
            if len(closed_mesh["points"]) != len(open_mesh["points"]):
                continue
            extent_error = np.linalg.norm(
                np.sort(closed_mesh["extent"]) - np.sort(open_mesh["extent"])
            )
            if extent_error > 1e-4:
                continue
            rotation, translation, errors = _fit_rigid_transform(
                open_mesh["points"], closed_mesh["points"]
            )
            rms_error = float(np.sqrt(np.mean(errors * errors)))
            fits.append(
                {
                    "closed_mesh": closed_mesh["geometry"],
                    "open_mesh": open_mesh["geometry"],
                    "rotation": rotation,
                    "translation": translation,
                    "rms_error": rms_error,
                    "max_error": float(errors.max()),
                    "volume": closed_volume,
                }
            )
    if not fits:
        raise RuntimeError("could not find a paired closed/open body mesh")
    precise = [fit for fit in fits if fit["rms_error"] < 1e-5 and fit["max_error"] < 1e-4]
    if precise:
        return max(precise, key=lambda fit: fit["volume"])
    return min(fits, key=lambda fit: (fit["rms_error"], -fit["volume"]))


def _fit_rigid_transform(
    source: np.ndarray, target: np.ndarray
) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    source_center = source.mean(axis=0)
    target_center = target.mean(axis=0)
    covariance = (source - source_center).T @ (target - target_center)
    left, _, right_t = np.linalg.svd(covariance)
    rotation = right_t.T @ left.T
    if np.linalg.det(rotation) < 0.0:
        right_t[-1] *= -1.0
        rotation = right_t.T @ left.T
    translation = target_center - rotation @ source_center
    errors = np.linalg.norm(_apply_rigid_transform(source, rotation, translation) - target, axis=1)
    return rotation, translation, errors


def _apply_rigid_transform(
    points: np.ndarray, rotation: np.ndarray, translation: np.ndarray
) -> np.ndarray:
    return points @ rotation.T + translation


def _pca_frame(points: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    center = points.mean(axis=0)
    centered = points - center
    values, vectors = np.linalg.eigh(np.cov(centered.T))
    axes = vectors[:, np.argsort(values)[::-1]].T
    return center, axes


def _hinge_candidate_meshes(
    meshes: list[dict[str, Any]],
    labels: np.ndarray,
    closed_label: int,
    lid_mesh: dict[str, Any],
    basis: dict[str, list[float]],
) -> list[int]:
    front = np.array(basis["front"], dtype=float)
    up = np.array(basis["up"], dtype=float)
    lid_points = lid_mesh["points"]
    lid_front = lid_points @ front
    lid_up = lid_points @ up
    rear_threshold = lid_front.max() - 0.08
    lower_up = lid_up.min() - 0.03
    upper_up = lid_up.min() + 0.09
    candidates = []
    for mesh, label in zip(meshes, labels):
        if label != closed_label or mesh["geometry"] == lid_mesh["geometry"]:
            continue
        index = _mesh_index(mesh["geometry"])
        center_front = float(mesh["center"] @ front)
        center_up = float(mesh["center"] @ up)
        max_extent = float(mesh["extent"].max())
        if center_front >= rear_threshold and lower_up <= center_up <= upper_up and max_extent < 0.09:
            candidates.append((index, center_up, float(mesh["extent"][1])))
    if not candidates:
        return []

    return sorted(index for index, _, _ in candidates)


def _moving_hinge_meshes(
    *,
    meshes: list[dict[str, Any]],
    labels: np.ndarray,
    open_label: int,
    hinge_candidates: list[int],
    rotation: np.ndarray,
    translation: np.ndarray,
    pivot: np.ndarray,
    axis: np.ndarray,
    open_angle_degrees: float,
) -> list[int]:
    open_meshes = [
        {
            **mesh,
            "points": _apply_rigid_transform(mesh["points"], rotation, translation),
        }
        for mesh, label in zip(meshes, labels)
        if label == open_label
    ]
    moving = []
    lid_rotation = _rotation_matrix(axis, open_angle_degrees)
    for index in hinge_candidates:
        name = "Mesh" if index == 0 else f"Mesh.{index:03d}"
        mesh = next((item for item in meshes if item["geometry"] == name), None)
        if mesh is None:
            continue
        fixed_points = mesh["points"]
        moved_points = (fixed_points - pivot) @ lid_rotation.T + pivot
        fixed_error = _best_chamfer(fixed_points, open_meshes)
        moved_error = _best_chamfer(moved_points, open_meshes)
        if moved_error < fixed_error * 0.85:
            moving.append(index)
    return sorted(moving)


def _hinge_axis_from_meshes(
    meshes: list[dict[str, Any]], hinge_meshes: list[int]
) -> np.ndarray | None:
    centers = []
    for index in hinge_meshes:
        name = "Mesh" if index == 0 else f"Mesh.{index:03d}"
        mesh = next((item for item in meshes if item["geometry"] == name), None)
        if mesh is not None:
            centers.append(mesh["center"])
    if len(centers) < 2:
        return None
    _, axes = _pca_frame(np.array(centers))
    axis = axes[0]
    axis[1] = 0.0
    if np.linalg.norm(axis) <= 1e-6:
        return None
    return axis / np.linalg.norm(axis)


def _hinge_pivot(
    meshes: list[dict[str, Any]],
    hinge_meshes: list[int],
    lid_mesh: dict[str, Any],
    hinge_axis: np.ndarray,
    basis: dict[str, list[float]],
) -> np.ndarray:
    centers = []
    for index in hinge_meshes:
        name = "Mesh" if index == 0 else f"Mesh.{index:03d}"
        mesh = next((item for item in meshes if item["geometry"] == name), None)
        if mesh is not None:
            centers.append(mesh["center"])
    if centers:
        return np.array(centers).mean(axis=0)

    front = np.array(basis["front"], dtype=float)
    up = np.array(basis["up"], dtype=float)
    points = lid_mesh["points"]
    rear = points @ front
    lower = points @ up
    edge_points = points[(rear > rear.max() - 0.02) & (lower < lower.min() + 0.03)]
    if len(edge_points) == 0:
        edge_points = points[rear > rear.max() - 0.02]
    pivot = edge_points.mean(axis=0)
    return pivot - hinge_axis * np.dot(pivot, hinge_axis)


def _orient_axis(axis: np.ndarray, preferred: np.ndarray) -> np.ndarray:
    axis = axis / np.linalg.norm(axis)
    if np.dot(axis, preferred) < 0.0:
        axis = -axis
    return axis


def _rotation_matrix(axis: np.ndarray, angle_degrees: float) -> np.ndarray:
    axis = axis / np.linalg.norm(axis)
    angle = np.radians(angle_degrees)
    x, y, z = axis
    c = np.cos(angle)
    s = np.sin(angle)
    one_minus_c = 1.0 - c
    return np.array(
        [
            [
                c + x * x * one_minus_c,
                x * y * one_minus_c - z * s,
                x * z * one_minus_c + y * s,
            ],
            [
                y * x * one_minus_c + z * s,
                c + y * y * one_minus_c,
                y * z * one_minus_c - x * s,
            ],
            [
                z * x * one_minus_c - y * s,
                z * y * one_minus_c + x * s,
                c + z * z * one_minus_c,
            ],
        ]
    )


def _best_chamfer(points: np.ndarray, meshes: list[dict[str, Any]]) -> float:
    return min(_chamfer(points, mesh["points"]) for mesh in meshes)


def _chamfer(source: np.ndarray, target: np.ndarray) -> float:
    source_distances, _ = cKDTree(target).query(source, k=1)
    target_distances, _ = cKDTree(source).query(target, k=1)
    return float((source_distances.mean() + target_distances.mean()) * 0.5)


def _estimate_hinge(
    *,
    lid_probe: MeshProbe,
    lid_mesh: dict[str, Any],
    open_lid: dict[str, Any],
    meshes: list[dict[str, Any]],
    labels: np.ndarray,
    closed_label: int,
    open_label: int,
    basis: dict[str, list[float]],
) -> dict[str, Any]:
    body_fit = _fit_body_alignment(meshes, labels, closed_label, open_label)
    open_points = _apply_rigid_transform(
        open_lid["points"], body_fit["rotation"], body_fit["translation"]
    )
    closed_points = lid_mesh["points"]
    closed_center, closed_axes = _pca_frame(closed_points)
    open_center, open_axes = _pca_frame(open_points)
    closed_normal = closed_axes[2]
    open_normal = open_axes[2]
    if closed_normal[1] < 0.0:
        closed_normal = -closed_normal
    if np.dot(closed_normal, open_normal) < 0.0:
        open_normal = -open_normal

    hinge_candidates = _hinge_candidate_meshes(meshes, labels, closed_label, lid_mesh, basis)
    hinge_axis = _hinge_axis_from_meshes(meshes, hinge_candidates)
    if hinge_axis is None:
        hinge_axis = np.cross(closed_normal, open_normal)
    hinge_axis = _orient_axis(hinge_axis, np.array(basis["right"], dtype=float))

    signed_angle = np.degrees(
        np.arctan2(
            np.dot(hinge_axis, np.cross(closed_normal, open_normal)),
            np.dot(closed_normal, open_normal),
        )
    )
    open_angle = -abs(float(signed_angle))
    pivot = _hinge_pivot(meshes, hinge_candidates, lid_mesh, hinge_axis, basis)
    hinge_meshes = _moving_hinge_meshes(
        meshes=meshes,
        labels=labels,
        open_label=open_label,
        hinge_candidates=hinge_candidates,
        rotation=body_fit["rotation"],
        translation=body_fit["translation"],
        pivot=pivot,
        axis=hinge_axis,
        open_angle_degrees=open_angle,
    )
    return {
        "status": "paired_geometry",
        "body_alignment": {
            "closed_body_mesh": body_fit["closed_mesh"],
            "open_body_mesh": body_fit["open_mesh"],
            "rms_error": round(float(body_fit["rms_error"]), 9),
            "max_error": round(float(body_fit["max_error"]), 9),
        },
        "closed_lid_mesh": lid_probe.geometry,
        "open_reference_lid_mesh": open_lid["geometry"],
        "moving_hinge_meshes": hinge_meshes,
        "hinge_candidate_meshes": hinge_candidates,
        "pivot": _round_vec(pivot),
        "axis": _round_vec(hinge_axis),
        "closed_angle_degrees": 0.0,
        "open_angle_degrees": round(open_angle, 3),
        "note": "Body alignment is Kabsch-fitted from paired body meshes; hinge axis is fitted from hinge hardware when available.",
    }


def _estimate_basis(lid: MeshProbe) -> dict[str, list[float]]:
    lid_axis = _horizontal_axis_from_pca(lid.pca_axes)
    right = lid_axis
    up = np.array([0.0, 1.0, 0.0])
    front = np.cross(up, right)
    right /= np.linalg.norm(right)
    front /= np.linalg.norm(front)
    if front[2] > 0.0:
        front = -front
        right = -right
    right = np.cross(front, up)
    right /= np.linalg.norm(right)
    front = np.cross(up, right)
    front /= np.linalg.norm(front)
    return {
        "right": _round_vec(right),
        "up": _round_vec(up),
        "front": _round_vec(front),
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
            "## Basis",
            "",
            f"- Right: `{payload['basis']['right']}`",
            f"- Up: `{payload['basis']['up']}`",
            f"- Front: `{payload['basis']['front']}`",
            "",
            "## Lid Animation Estimate",
            "",
            f"- Status: `{estimate['status']}`",
            f"- Body pair: `{estimate['body_alignment']['closed_body_mesh']}` / `{estimate['body_alignment']['open_body_mesh']}`",
            f"- Body alignment RMS: `{estimate['body_alignment']['rms_error']}`",
            f"- Closed lid mesh: `{estimate['closed_lid_mesh']}`",
            f"- Open reference lid mesh: `{estimate['open_reference_lid_mesh']}`",
            f"- Moving hinge meshes: `{estimate['moving_hinge_meshes']}`",
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


def _render_spec_draft(payload: dict[str, Any], raw_meshes: list[dict[str, Any]]) -> str:
    closed = [mesh for mesh in payload["meshes"] if mesh["cluster"] == "closed"]
    roles = {
        role: [int(_mesh_index(mesh["geometry"])) for mesh in closed if mesh["role"] == role]
        for role in ["body", "lid", "hinge", "handle", "mechanism"]
    }
    unknown = [int(_mesh_index(mesh["geometry"])) for mesh in closed if mesh["role"] == "unknown"]
    preferred_cylinder = [26, 27, 29, 36]
    cylinder_meshes = [index for index in preferred_cylinder if index in roles["mechanism"]]
    if not cylinder_meshes:
        mechanism_sorted = sorted(roles["mechanism"], key=lambda index: abs(index - 29))
        cylinder_meshes = mechanism_sorted[:4] if len(mechanism_sorted) >= 4 else mechanism_sorted
    estimate = payload["lid_animation_estimate"]
    moving_hinges = [int(index) for index in estimate["moving_hinge_meshes"]]
    lid_meshes = sorted(set(roles["lid"] + moving_hinges))
    closed_cluster = next(cluster for cluster in payload["clusters"] if cluster["name"] == "closed")
    cylinder_geometry = _cylinder_geometry(
        raw_meshes,
        [int(_mesh_index(mesh["geometry"])) for mesh in closed],
        cylinder_meshes,
    )
    comb_geometry = _comb_geometry(
        raw_meshes,
        [int(_mesh_index(mesh["geometry"])) for mesh in closed],
        cylinder_geometry,
    )
    body = sorted(
        set(roles["body"] + roles["handle"] + roles["mechanism"] + unknown)
        - set(cylinder_meshes)
        - set(comb_geometry["meshes"])
        - set(lid_meshes)
    )

    return "\n".join(
        [
            "[asset]",
            f'gltf = "{Path(payload["model"]).relative_to("assets").as_posix()}"',
            "",
            "[basis]",
            f"right = {_toml_list(payload['basis']['right'])}",
            f"up = {_toml_list(payload['basis']['up'])}",
            f"front = {_toml_list(payload['basis']['front'])}",
            "",
            "[closed_model]",
            f"mesh_indices = {_toml_list(sorted(_mesh_index(mesh['geometry']) for mesh in closed))}",
            f"bounds_min = {_toml_list(closed_cluster['bounds'][0])}",
            f"bounds_max = {_toml_list(closed_cluster['bounds'][1])}",
            f"body_meshes = {_toml_list(body)}",
            f"lid_meshes = {_toml_list(lid_meshes)}",
            f"hinge_meshes = {_toml_list(moving_hinges)}",
            f"handle_meshes = {_toml_list(roles['handle'])}",
            f"mechanism_meshes = {_toml_list(roles['mechanism'])}",
            "",
            "[lid]",
            f"meshes = {_toml_list(lid_meshes)}",
            f"pivot = {_toml_list(estimate['pivot'])}",
            f"axis = {_toml_list(estimate['axis'])}",
            f"closed_degrees = {estimate['closed_angle_degrees']}",
            f"open_degrees = {estimate['open_angle_degrees']}",
            "",
            "[cylinder]",
            f"meshes = {_toml_list(cylinder_meshes)}",
            f"pivot = {_toml_list(cylinder_geometry['pivot'])}",
            f"axis = {_toml_list(cylinder_geometry['axis'])}",
            f"radius = {cylinder_geometry['radius']}",
            f"length = {cylinder_geometry['length']}",
            "degrees_per_second = 120.0",
            "",
            "[comb]",
            f"meshes = {_toml_list(comb_geometry['meshes'])}",
            f"radial_direction = {_toml_list(comb_geometry['radial_direction'])}",
            f"axial_min = {comb_geometry['axial_min']}",
            f"axial_max = {comb_geometry['axial_max']}",
            f"tip_radius = {comb_geometry['tip_radius']}",
            f"root_radius = {comb_geometry['root_radius']}",
            f"clearance = {comb_geometry['clearance']}",
            f"tine_length = {comb_geometry['tine_length']}",
            "",
        ]
    )


def _mesh_index(geometry: str) -> int:
    if geometry == "Mesh":
        return 0
    return int(geometry.split(".", 1)[1])


def _horizontal_axis_from_pca(pca_axes: list[list[float]]) -> np.ndarray:
    candidates = []
    for raw_axis in pca_axes:
        axis = np.array(raw_axis, dtype=float)
        axis[1] = 0.0
        if np.linalg.norm(axis) > 1e-6:
            candidates.append(axis / np.linalg.norm(axis))
    if not candidates:
        return np.array([1.0, 0.0, 0.0])
    axis = candidates[0]
    if axis[0] < 0.0:
        axis = -axis
    return axis / np.linalg.norm(axis)


def _cylinder_geometry(
    meshes: list[dict[str, Any]], closed_indices: list[int], cylinder_indices: list[int]
) -> dict[str, Any]:
    cylinder = _cylinder_body_mesh(meshes, cylinder_indices)
    if cylinder is None:
        return {
            "pivot": [0.0, 0.0, 0.0],
            "axis": [1.0, 0.0, 0.0],
            "radius": 0.0,
            "length": 0.0,
        }
    cylinder_center, cylinder_axes = _pca_frame(cylinder["points"])
    cylinder_projection = (cylinder["points"] - cylinder_center) @ cylinder_axes.T
    cylinder_extent = np.ptp(cylinder_projection, axis=0)
    cylinder_axis = _horizontal_axis_from_pca([_round_vec(axis) for axis in cylinder_axes])
    cylinder_axis = _orient_axis(cylinder_axis, np.array([1.0, 0.0, 0.0]))
    rel = cylinder["points"] - cylinder_center
    axial = rel @ cylinder_axis
    radial = np.linalg.norm(rel - np.outer(axial, cylinder_axis), axis=1)
    radius = float(np.median(radial))
    length = float(np.ptp(axial))

    axle_meshes = _coaxial_axle_meshes(meshes, closed_indices, cylinder, cylinder_axis)
    if len(axle_meshes) >= 2:
        centers = np.array([mesh["points"].mean(axis=0) for mesh in axle_meshes])
        projections = centers @ cylinder_axis
        start = centers[int(np.argmin(projections))]
        end = centers[int(np.argmax(projections))]
        axle_axis = end - start
        axle_axis[1] = 0.0
        if np.linalg.norm(axle_axis) > 1e-6:
            axle_axis = _orient_axis(axle_axis, cylinder_axis)
            pivot = start + axle_axis * np.dot(cylinder_center - start, axle_axis)
            return {
                "pivot": _round_vec(pivot),
                "axis": _round_vec(axle_axis),
                "radius": round(radius, 6),
                "length": round(length, 6),
            }

    return {
        "pivot": _round_vec(cylinder_center),
        "axis": _round_vec(cylinder_axis),
        "radius": round(radius, 6),
        "length": round(length, 6),
    }


def _coaxial_axle_meshes(
    meshes: list[dict[str, Any]],
    closed_indices: list[int],
    cylinder: dict[str, Any],
    cylinder_axis: np.ndarray,
) -> list[dict[str, Any]]:
    cylinder_center = cylinder["points"].mean(axis=0)
    candidates = []
    for index in closed_indices:
        mesh = _mesh_by_index(meshes, index)
        if mesh is None or mesh["geometry"] == cylinder["geometry"]:
            continue
        center, axes = _pca_frame(mesh["points"])
        axis = _horizontal_axis_from_pca([_round_vec(axis) for axis in axes])
        alignment = abs(float(np.dot(axis, cylinder_axis)))
        if alignment < 0.98:
            continue
        offset = center - cylinder_center
        radial = offset - cylinder_axis * np.dot(offset, cylinder_axis)
        if np.linalg.norm(radial) > 0.018:
            continue
        candidates.append(mesh)
    return candidates


def _mesh_by_index(meshes: list[dict[str, Any]], index: int) -> dict[str, Any] | None:
    name = "Mesh" if index == 0 else f"Mesh.{index:03d}"
    return next((item for item in meshes if item["geometry"] == name), None)


def _dominant_cylinder_mesh(
    meshes: list[dict[str, Any]], indices: list[int]
) -> dict[str, Any] | None:
    candidates = []
    for index in indices:
        name = "Mesh" if index == 0 else f"Mesh.{index:03d}"
        mesh = next((item for item in meshes if item["geometry"] == name), None)
        if mesh is not None:
            candidates.append(mesh)
    if not candidates:
        return None
    return max(candidates, key=lambda item: np.prod(np.array(item["extent"])))


def _cylinder_body_mesh(
    meshes: list[dict[str, Any]], indices: list[int]
) -> dict[str, Any] | None:
    candidates = []
    for index in indices:
        mesh = _mesh_by_index(meshes, index)
        if mesh is None:
            continue
        center, axes = _pca_frame(mesh["points"])
        axis = _horizontal_axis_from_pca([_round_vec(item) for item in axes])
        rel = mesh["points"] - center
        axial = rel @ axis
        radial = np.linalg.norm(rel - np.outer(axial, axis), axis=1)
        axial_length = float(np.ptp(axial))
        radial_mean = float(radial.mean())
        radial_std = float(radial.std())
        if axial_length < 0.08 or radial_mean <= 1e-6:
            continue
        score = axial_length / max(radial_std, 1e-6)
        candidates.append((score, mesh))
    if candidates:
        return max(candidates, key=lambda item: item[0])[1]
    return _dominant_cylinder_mesh(meshes, indices)


def _comb_geometry(
    meshes: list[dict[str, Any]],
    closed_indices: list[int],
    cylinder_geometry: dict[str, Any],
) -> dict[str, Any]:
    pivot = np.array(cylinder_geometry["pivot"], dtype=float)
    axis = np.array(cylinder_geometry["axis"], dtype=float)
    axis /= np.linalg.norm(axis)
    cylinder_radius = float(cylinder_geometry["radius"])

    preferred_comb = _mesh_by_index(meshes, 23)
    if preferred_comb is not None and 23 in closed_indices:
        comb = preferred_comb
    else:
        comb = _find_comb_mesh(meshes, closed_indices, pivot, axis, cylinder_radius)
    if comb is None:
        return {
            "meshes": [],
            "radial_direction": [0.0, 1.0, 0.0],
            "axial_min": -0.5 * float(cylinder_geometry["length"]),
            "axial_max": 0.5 * float(cylinder_geometry["length"]),
            "tip_radius": cylinder_radius,
            "root_radius": cylinder_radius,
            "clearance": 0.0,
            "tine_length": 0.0,
        }

    points = comb["points"]
    center_offset = comb["center"] - pivot
    radial_direction = center_offset - axis * np.dot(center_offset, axis)
    radial_direction /= np.linalg.norm(radial_direction)
    rel = points - pivot
    axial = rel @ axis
    radial = rel @ radial_direction
    tip_radius = float(radial.min())
    root_radius = float(radial.max())
    return {
        "meshes": [int(_mesh_index(comb["geometry"]))],
        "radial_direction": _round_vec(radial_direction),
        "axial_min": round(float(axial.min()), 6),
        "axial_max": round(float(axial.max()), 6),
        "tip_radius": round(tip_radius, 6),
        "root_radius": round(root_radius, 6),
        "clearance": round(tip_radius - cylinder_radius, 6),
        "tine_length": round(root_radius - tip_radius, 6),
    }


def _find_comb_mesh(
    meshes: list[dict[str, Any]],
    closed_indices: list[int],
    pivot: np.ndarray,
    axis: np.ndarray,
    cylinder_radius: float,
) -> dict[str, Any] | None:
    candidates = []
    for index in closed_indices:
        mesh = _mesh_by_index(meshes, index)
        if mesh is None:
            continue
        rel = mesh["points"] - pivot
        axial = rel @ axis
        center_offset = mesh["center"] - pivot
        radial_direction = center_offset - axis * np.dot(center_offset, axis)
        if np.linalg.norm(radial_direction) < 1e-6:
            continue
        radial_direction /= np.linalg.norm(radial_direction)
        radial = rel @ radial_direction
        axial_length = float(np.ptp(axial))
        radial_span = float(radial.max() - radial.min())
        vertical_thickness = float(mesh["extent"][1])
        tip_clearance = float(radial.min() - cylinder_radius)
        if axial_length < 0.06 or radial_span < 0.05:
            continue
        if vertical_thickness > 0.02:
            continue
        if not (-0.005 <= tip_clearance <= 0.03):
            continue
        score = axial_length + radial_span - vertical_thickness * 5.0 - abs(tip_clearance) * 2.0
        candidates.append((score, mesh))
    if not candidates:
        return None
    return max(candidates, key=lambda item: item[0])[1]


def _toml_list(values: list[Any]) -> str:
    return "[" + ", ".join(str(round(value, 6)) if isinstance(value, float) else str(value) for value in values) + "]"


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
