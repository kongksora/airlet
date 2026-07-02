from __future__ import annotations

import argparse
import json
import math
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import tomllib
import trimesh
from pygltflib import FLOAT, GLTF2


DEFAULT_SPEC = Path("assets/models/converted/spec.toml")
DEFAULT_SOURCE_SPEC = Path("assets/models/converted/source_spec.toml")
DEFAULT_SOURCE = Path("assets/models/converted/music_box.glb")
DEFAULT_OUTPUT = Path("assets/generated/music_box_aligned_base.glb")
DEFAULT_REPORT = Path("assets/generated/music_box_aligned_base.json")
WOOD_MESHES = {0, 8}


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Build an Airlet-aligned clean base GLB from the converted music-box source."
    )
    parser.add_argument("--spec", type=Path, default=DEFAULT_SPEC)
    parser.add_argument("--source-spec", type=Path, default=DEFAULT_SOURCE_SPEC)
    parser.add_argument("--source", type=Path, default=DEFAULT_SOURCE)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--report", type=Path, default=DEFAULT_REPORT)
    parser.add_argument("--write-spec", action="store_true")
    parser.add_argument(
        "--wood-bevel-width",
        type=float,
        default=0.023854973,
        help="Model-space bevel target used to size clean proxy wall thickness.",
    )
    parser.add_argument(
        "--preserve-source-wood",
        action="store_true",
        help="Keep source wood meshes instead of replacing them with clean rebuilt shell proxies.",
    )
    args = parser.parse_args()

    spec_path = args.source_spec if args.source_spec.exists() else args.spec
    spec = tomllib.loads(spec_path.read_text(encoding="utf-8"))
    basis = Basis.from_spec(spec)
    source = SourceGltf(args.source)
    interior_alignment = InteriorAlignment.from_spec(spec, basis, source)
    winding_pivot = basis.point(spec["winding_key"]["pivot"]) if spec.get("winding_key") else None
    crank_center = interior_alignment.point(winding_pivot) if winding_pivot is not None else None
    mesh_points: dict[int, np.ndarray] = {}
    scene = trimesh.Scene()

    for mesh_index in spec["closed_model"]["mesh_indices"]:
        if mesh_index in WOOD_MESHES and not args.preserve_source_wood:
            mesh = clean_wood_mesh(
                mesh_index,
                source,
                basis,
                interior_alignment,
                args.wood_bevel_width,
                crank_center,
            )
        else:
            mesh = source.aligned_mesh(mesh_index, basis, interior_alignment)
        mesh.metadata["name"] = source.mesh_name(mesh_index)
        scene.add_geometry(mesh, geom_name=source.mesh_name(mesh_index), node_name=source.mesh_name(mesh_index))
        mesh_points[mesh_index] = np.asarray(mesh.vertices, dtype=np.float32)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    scene.export(args.output)

    all_points = np.concatenate(list(mesh_points.values()), axis=0)
    generated_spec = aligned_spec_text(
        spec,
        basis,
        args.output,
        args.source,
        args.wood_bevel_width,
        interior_alignment,
        args.preserve_source_wood,
    )
    if args.write_spec:
        args.spec.write_text(generated_spec, encoding="utf-8")

    report = {
        "source": str(args.source),
        "output": str(args.output),
        "spec": str(args.spec),
        "source_spec": str(spec_path),
        "wrote_spec": args.write_spec,
        "closed_bounds_min": all_points.min(axis=0).astype(float).tolist(),
        "closed_bounds_max": all_points.max(axis=0).astype(float).tolist(),
        "closed_extent": np.ptp(all_points, axis=0).astype(float).tolist(),
        "mesh_count": len(spec["closed_model"]["mesh_indices"]),
        "wood_proxy_meshes": {
            str(index): mesh_health(mesh_points[index], source.mesh_name(index)) for index in sorted(WOOD_MESHES)
        },
        "preserved_source_wood": args.preserve_source_wood,
        "basis": {
            "right": basis.right.astype(float).tolist(),
            "up": basis.up.astype(float).tolist(),
            "front": basis.front.astype(float).tolist(),
        },
    }
    args.report.parent.mkdir(parents=True, exist_ok=True)
    args.report.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    print(f"wrote {args.output}")
    print(f"wrote {args.report}")
    if args.write_spec:
        print(f"updated {args.spec}")


@dataclass(frozen=True)
class Basis:
    right: np.ndarray
    up: np.ndarray
    front: np.ndarray
    matrix: np.ndarray

    @classmethod
    def from_spec(cls, spec: dict) -> "Basis":
        right = normalized(np.array(spec["basis"]["right"], dtype=np.float32))
        up = normalized(np.array(spec["basis"]["up"], dtype=np.float32))
        front = normalized(np.array(spec["basis"]["front"], dtype=np.float32))
        matrix = np.stack([right, up, -front], axis=1).T.astype(np.float32)
        return cls(right=right, up=up, front=front, matrix=matrix)

    def point(self, point: Iterable[float]) -> np.ndarray:
        return self.matrix @ np.array(point, dtype=np.float32)

    def axis(self, axis: Iterable[float]) -> np.ndarray:
        return normalized(self.matrix @ np.array(axis, dtype=np.float32))


@dataclass(frozen=True)
class InteriorAlignment:
    center: np.ndarray
    matrix: np.ndarray

    @classmethod
    def from_spec(cls, spec: dict, basis: Basis, source: "SourceGltf") -> "InteriorAlignment":
        wood_points = np.concatenate(
            [
                np.asarray(source.aligned_mesh(mesh_index, basis).vertices, dtype=np.float32)
                for mesh_index in sorted(WOOD_MESHES)
            ],
            axis=0,
        )
        center = (wood_points.min(axis=0) + wood_points.max(axis=0)) * 0.5
        cylinder_axis = basis.axis(spec["cylinder"]["axis"])
        yaw = math.atan2(float(cylinder_axis[0]), float(cylinder_axis[2]))
        matrix = np.array(
            [
                [math.cos(yaw), 0.0, -math.sin(yaw)],
                [0.0, 1.0, 0.0],
                [math.sin(yaw), 0.0, math.cos(yaw)],
            ],
            dtype=np.float32,
        )
        return cls(center=center.astype(np.float32), matrix=matrix)

    def point(self, point: np.ndarray) -> np.ndarray:
        return self.center + self.matrix @ (point - self.center)

    def points(self, points: np.ndarray) -> np.ndarray:
        return self.center + (points - self.center) @ self.matrix.T

    def axis(self, axis: np.ndarray) -> np.ndarray:
        return normalized(self.matrix @ axis)


class SourceGltf:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.gltf = GLTF2().load(path)
        self.blob = self.gltf.binary_blob() or b""
        self.mesh_nodes = self._mesh_nodes()

    def mesh_name(self, mesh_index: int) -> str:
        name = self.gltf.meshes[mesh_index].name
        return name or f"Mesh.{mesh_index:03d}"

    def aligned_mesh(
        self,
        mesh_index: int,
        basis: Basis,
        interior_alignment: InteriorAlignment | None = None,
    ) -> trimesh.Trimesh:
        vertices: list[np.ndarray] = []
        faces: list[np.ndarray] = []
        offset = 0
        transform = node_matrix(self.mesh_nodes[mesh_index])
        for primitive in self.gltf.meshes[mesh_index].primitives:
            positions = self.read_vec3(primitive.attributes.POSITION)
            positions = transform_points(transform, positions)
            positions = positions @ basis.matrix.T
            if interior_alignment is not None:
                positions = interior_alignment.points(positions)
            indices = self.read_indices(primitive.indices, len(positions)).reshape(-1, 3)
            vertices.append(positions)
            faces.append(indices + offset)
            offset += len(positions)
        mesh = trimesh.Trimesh(
            vertices=np.concatenate(vertices, axis=0),
            faces=np.concatenate(faces, axis=0),
            process=False,
        )
        mesh.merge_vertices(digits_vertex=7)
        mesh.update_faces(mesh.nondegenerate_faces())
        mesh.remove_unreferenced_vertices()
        return mesh

    def read_vec3(self, accessor_index: int) -> np.ndarray:
        accessor = self.gltf.accessors[accessor_index]
        if accessor.componentType != FLOAT or accessor.type != "VEC3":
            raise ValueError(f"unsupported VEC3 accessor {accessor_index}: {accessor}")
        data = self.accessor_bytes(accessor)
        return np.frombuffer(data, dtype=np.float32).reshape(accessor.count, 3).copy()

    def read_indices(self, accessor_index: int | None, vertex_count: int) -> np.ndarray:
        if accessor_index is None:
            return np.arange(vertex_count, dtype=np.uint32)
        accessor = self.gltf.accessors[accessor_index]
        data = self.accessor_bytes(accessor)
        if accessor.componentType == 5123:
            return np.frombuffer(data, dtype=np.uint16).astype(np.uint32)
        if accessor.componentType == 5125:
            return np.frombuffer(data, dtype=np.uint32).copy()
        raise ValueError(f"unsupported index accessor {accessor_index}: {accessor.componentType}")

    def accessor_bytes(self, accessor) -> bytes:
        view = self.gltf.bufferViews[accessor.bufferView]
        offset = (view.byteOffset or 0) + (accessor.byteOffset or 0)
        item_size = accessor_item_size(accessor)
        length = accessor.count * item_size
        if view.byteStride:
            compact = bytearray()
            for index in range(accessor.count):
                start = offset + index * view.byteStride
                compact.extend(self.blob[start : start + item_size])
            return bytes(compact)
        return bytes(self.blob[offset : offset + length])

    def _mesh_nodes(self) -> dict[int, object]:
        result = {}
        for node in self.gltf.nodes:
            if node.mesh is not None and node.mesh not in result:
                result[node.mesh] = node
        return result


def clean_wood_mesh(
    mesh_index: int,
    source: SourceGltf,
    basis: Basis,
    interior_alignment: InteriorAlignment,
    bevel_width: float,
    crank_center: np.ndarray | None,
) -> trimesh.Trimesh:
    measured = source.aligned_mesh(mesh_index, basis, interior_alignment)
    bounds_min, bounds_max = measured.bounds
    if mesh_index == 0:
        return lid_proxy(bounds_min, bounds_max, bevel_width, source.mesh_name(mesh_index))
    return body_proxy(bounds_min, bounds_max, bevel_width, source.mesh_name(mesh_index), crank_center)


def lid_proxy(bounds_min: np.ndarray, bounds_max: np.ndarray, bevel_width: float, name: str) -> trimesh.Trimesh:
    x0, y0, z0 = bounds_min
    x1, y1, z1 = bounds_max
    width = x1 - x0
    depth = z1 - z0
    height = y1 - y0
    rim = max(bevel_width * 2.4, min(width, depth) * 0.12)
    rim = min(rim, width * 0.25, depth * 0.25)
    recess = min(max(bevel_width * 1.2, height * 0.28), height * 0.58)
    x = [x0, x0 + rim, x1 - rim, x1]
    y = [y0, y0 + recess, y1]
    z = [z0, z0 + rim, z1 - rim, z1]
    solid = {
        (ix, iy, iz)
        for ix in range(3)
        for iy in range(2)
        for iz in range(3)
        if iy == 1 or ix != 1 or iz != 1
    }
    return voxel_boundary_mesh(x, y, z, solid, name)


def body_proxy(
    bounds_min: np.ndarray,
    bounds_max: np.ndarray,
    bevel_width: float,
    name: str,
    crank_center: np.ndarray | None,
) -> trimesh.Trimesh:
    x0, y0, z0 = bounds_min
    x1, y1, z1 = bounds_max
    width = x1 - x0
    depth = z1 - z0
    height = y1 - y0
    wall = max(bevel_width * 2.8, min(width, depth) * 0.13)
    wall = min(wall, width * 0.28, depth * 0.28)
    floor = min(max(bevel_width * 1.8, height * 0.24), height * 0.42)
    xi0, xi1 = x0 + wall, x1 - wall
    zi0, zi1 = z0 + wall, z1 - wall
    yi = y0 + floor
    y_top_split = max(yi, y1 - bevel_width * 1.35)
    x = sorted_unique([x0, xi0, xi1, x1])
    y = sorted_unique([y0, yi, y_top_split, y1])
    z = sorted_unique([z0, zi0, zi1, z1])
    solid = {
        (ix, iy, iz)
        for ix in range(len(x) - 1)
        for iy in range(len(y) - 1)
        for iz in range(len(z) - 1)
        if (
            y_cell_center(y, iy) < yi
            or x_cell_center(x, ix) < xi0
            or x_cell_center(x, ix) > xi1
            or z_cell_center(z, iz) < zi0
            or z_cell_center(z, iz) > zi1
        )
    }
    return voxel_boundary_mesh(x, y, z, solid, name)


def sorted_unique(values: list[float]) -> list[float]:
    result: list[float] = []
    for value in sorted(float(item) for item in values):
        if not result or abs(value - result[-1]) > 1.0e-6:
            result.append(value)
    return result


def x_cell_center(values: list[float], index: int) -> float:
    return (values[index] + values[index + 1]) * 0.5


def y_cell_center(values: list[float], index: int) -> float:
    return (values[index] + values[index + 1]) * 0.5


def z_cell_center(values: list[float], index: int) -> float:
    return (values[index] + values[index + 1]) * 0.5


def voxel_boundary_mesh(
    x: list[float],
    y: list[float],
    z: list[float],
    solid: set[tuple[int, int, int]],
    name: str,
) -> trimesh.Trimesh:
    vertices: list[tuple[float, float, float]] = []
    faces: list[tuple[int, int, int]] = []
    vertex_index: dict[tuple[float, float, float], int] = {}

    def vertex(coord: tuple[float, float, float]) -> int:
        if coord not in vertex_index:
            vertex_index[coord] = len(vertices)
            vertices.append(coord)
        return vertex_index[coord]

    def add_quad(coords: list[tuple[float, float, float]]) -> None:
        a, b, c, d = [vertex(coord) for coord in coords]
        faces.extend([(a, b, c), (a, c, d)])

    directions = [
        ((-1, 0, 0), lambda ix, iy, iz: [(x[ix], y[iy], z[iz]), (x[ix], y[iy + 1], z[iz]), (x[ix], y[iy + 1], z[iz + 1]), (x[ix], y[iy], z[iz + 1])]),
        ((1, 0, 0), lambda ix, iy, iz: [(x[ix + 1], y[iy], z[iz + 1]), (x[ix + 1], y[iy + 1], z[iz + 1]), (x[ix + 1], y[iy + 1], z[iz]), (x[ix + 1], y[iy], z[iz])]),
        ((0, -1, 0), lambda ix, iy, iz: [(x[ix], y[iy], z[iz]), (x[ix], y[iy], z[iz + 1]), (x[ix + 1], y[iy], z[iz + 1]), (x[ix + 1], y[iy], z[iz])]),
        ((0, 1, 0), lambda ix, iy, iz: [(x[ix], y[iy + 1], z[iz + 1]), (x[ix], y[iy + 1], z[iz]), (x[ix + 1], y[iy + 1], z[iz]), (x[ix + 1], y[iy + 1], z[iz + 1])]),
        ((0, 0, -1), lambda ix, iy, iz: [(x[ix + 1], y[iy], z[iz]), (x[ix + 1], y[iy + 1], z[iz]), (x[ix], y[iy + 1], z[iz]), (x[ix], y[iy], z[iz])]),
        ((0, 0, 1), lambda ix, iy, iz: [(x[ix], y[iy], z[iz + 1]), (x[ix], y[iy + 1], z[iz + 1]), (x[ix + 1], y[iy + 1], z[iz + 1]), (x[ix + 1], y[iy], z[iz + 1])]),
    ]
    for ix, iy, iz in sorted(solid):
        for (dx, dy, dz), coords in directions:
            if (ix + dx, iy + dy, iz + dz) not in solid:
                add_quad(coords(ix, iy, iz))

    mesh = trimesh.Trimesh(vertices=np.array(vertices), faces=np.array(faces), process=True)
    mesh.metadata["name"] = name
    mesh.merge_vertices(digits_vertex=7)
    return mesh


def box_from_bounds(bounds_min: np.ndarray, bounds_max: np.ndarray, name: str) -> trimesh.Trimesh:
    mesh = box_from_ranges(bounds_min[0], bounds_max[0], bounds_min[1], bounds_max[1], bounds_min[2], bounds_max[2])
    mesh.metadata["name"] = name
    return mesh


def box_from_ranges(x0: float, x1: float, y0: float, y1: float, z0: float, z1: float) -> trimesh.Trimesh:
    extents = np.array([x1 - x0, y1 - y0, z1 - z0], dtype=np.float32)
    center = np.array([(x0 + x1) * 0.5, (y0 + y1) * 0.5, (z0 + z1) * 0.5], dtype=np.float32)
    transform = np.eye(4, dtype=np.float32)
    transform[:3, 3] = center
    return trimesh.creation.box(extents=extents, transform=transform)


def aligned_spec_text(
    spec: dict,
    basis: Basis,
    aligned_output: Path,
    source_path: Path,
    wood_bevel_width: float,
    interior_alignment: InteriorAlignment,
    preserve_source_wood: bool = False,
) -> str:
    closed = spec["closed_model"]
    source = SourceGltf(source_path)
    winding_pivot = basis.point(spec["winding_key"]["pivot"]) if spec.get("winding_key") else None
    crank_center = interior_alignment.point(winding_pivot) if winding_pivot is not None else None
    points = []
    for mesh_index in closed["mesh_indices"]:
        if mesh_index in WOOD_MESHES and not preserve_source_wood:
            mesh = clean_wood_mesh(
                mesh_index,
                source,
                basis,
                interior_alignment,
                wood_bevel_width,
                crank_center,
            )
        else:
            mesh = source.aligned_mesh(mesh_index, basis, interior_alignment)
        points.append(np.asarray(mesh.vertices, dtype=np.float32))
    all_points = np.concatenate(points, axis=0)
    bounds_min = all_points.min(axis=0)
    bounds_max = all_points.max(axis=0)
    asset_gltf = Path("generated") / aligned_output.name

    lines = [
        "[asset]",
        f'gltf = "{asset_gltf.as_posix()}"',
        'baked_gltf = "generated/music_box_material_baked.glb"',
        "",
        "[basis]",
        "right = [1.0, 0.0, 0.0]",
        "up = [0.0, 1.0, 0.0]",
        "front = [0.0, 0.0, -1.0]",
        "",
        "[closed_model]",
        f"mesh_indices = {int_array(closed['mesh_indices'])}",
        f"bounds_min = {float_array(bounds_min)}",
        f"bounds_max = {float_array(bounds_max)}",
        f"body_meshes = {int_array(closed.get('body_meshes', []))}",
        f"lid_meshes = {int_array(closed.get('lid_meshes', []))}",
        f"hinge_meshes = {int_array(closed.get('hinge_meshes', []))}",
        f"handle_meshes = {int_array(closed.get('handle_meshes', []))}",
        f"mechanism_meshes = {int_array(closed.get('mechanism_meshes', []))}",
        "",
    ]
    lines.extend(rotating_part("lid", spec["lid"], basis, interior_alignment))
    lines.extend(rotating_part("cylinder", spec["cylinder"], basis, interior_alignment))
    if spec.get("winding_key"):
        lines.extend(rotating_part("winding_key", spec["winding_key"], basis, interior_alignment))
    lines.extend(comb_part(spec["comb"], basis, interior_alignment))
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines) + "\n"


def rotating_part(name: str, part: dict, basis: Basis, interior_alignment: InteriorAlignment) -> list[str]:
    base_pivot = basis.point(part["pivot"])
    if name == "lid":
        pivot = base_pivot
        axis = np.array([1.0, 0.0, 0.0], dtype=np.float32)
    else:
        pivot = interior_alignment.point(base_pivot)
        axis = interior_alignment.axis(basis.axis(part["axis"]))
    lines = [
        f"[{name}]",
        f"meshes = {int_array(part.get('meshes', []))}",
        f"pivot = {float_array(pivot)}",
        f"axis = {float_array(axis)}",
    ]
    if "radius" in part:
        lines.append(f"radius = {float(part['radius']):.6g}")
    if "length" in part:
        lines.append(f"length = {float(part['length']):.6g}")
    lines.extend([
        f"closed_degrees = {float(part.get('closed_degrees', 0.0)):.6g}",
        f"open_degrees = {float(part.get('open_degrees', 0.0)):.6g}",
        "",
    ])
    return lines


def comb_part(part: dict, basis: Basis, interior_alignment: InteriorAlignment) -> list[str]:
    return [
        "[comb]",
        f"meshes = {int_array(part.get('meshes', []))}",
        f"radial_direction = {float_array(interior_alignment.axis(basis.axis(part.get('radial_direction', [0.0, 1.0, 0.0]))))}",
        f"axial_min = {float(part.get('axial_min', 0.0)):.6g}",
        f"axial_max = {float(part.get('axial_max', 0.0)):.6g}",
        f"tip_radius = {float(part.get('tip_radius', 0.0)):.6g}",
        f"root_radius = {float(part.get('root_radius', 0.0)):.6g}",
        f"clearance = {float(part.get('clearance', 0.0)):.6g}",
        f"tine_length = {float(part.get('tine_length', 0.0)):.6g}",
        "",
    ]


def mesh_health(points: np.ndarray, name: str) -> dict[str, object]:
    return {
        "name": name,
        "bounds_min": points.min(axis=0).astype(float).tolist(),
        "bounds_max": points.max(axis=0).astype(float).tolist(),
        "extent": np.ptp(points, axis=0).astype(float).tolist(),
        "vertex_count": int(len(points)),
    }


def int_array(values: Iterable[int]) -> str:
    return "[" + ", ".join(str(int(value)) for value in values) + "]"


def float_array(values: Iterable[float]) -> str:
    return "[" + ", ".join(f"{float(value):.6g}" for value in values) + "]"


def accessor_item_size(accessor) -> int:
    component_size = {FLOAT: 4, 5123: 2, 5125: 4}[accessor.componentType]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor.type]
    return component_size * width


def node_matrix(node) -> np.ndarray:
    if node.matrix:
        return np.array(node.matrix, dtype=np.float32).reshape(4, 4).T
    translation = np.array(node.translation or [0.0, 0.0, 0.0], dtype=np.float32)
    rotation = np.array(node.rotation or [0.0, 0.0, 0.0, 1.0], dtype=np.float32)
    scale = np.array(node.scale or [1.0, 1.0, 1.0], dtype=np.float32)
    matrix = quaternion_matrix(rotation)
    matrix[:3, :3] = matrix[:3, :3] @ np.diag(scale)
    matrix[:3, 3] = translation
    return matrix


def quaternion_matrix(quaternion: np.ndarray) -> np.ndarray:
    x, y, z, w = quaternion
    xx, yy, zz = x * x, y * y, z * z
    xy, xz, yz = x * y, x * z, y * z
    wx, wy, wz = w * x, w * y, w * z
    return np.array(
        [
            [1.0 - 2.0 * (yy + zz), 2.0 * (xy - wz), 2.0 * (xz + wy), 0.0],
            [2.0 * (xy + wz), 1.0 - 2.0 * (xx + zz), 2.0 * (yz - wx), 0.0],
            [2.0 * (xz - wy), 2.0 * (yz + wx), 1.0 - 2.0 * (xx + yy), 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
        dtype=np.float32,
    )


def transform_points(matrix: np.ndarray, points: np.ndarray) -> np.ndarray:
    padded = np.concatenate([points, np.ones((len(points), 1), dtype=np.float32)], axis=1)
    return (padded @ matrix.T)[:, :3]


def normalized(vector: np.ndarray) -> np.ndarray:
    length = float(np.linalg.norm(vector))
    if length < 1.0e-12:
        return np.zeros_like(vector, dtype=np.float32)
    return (vector / length).astype(np.float32)


if __name__ == "__main__":
    main()
