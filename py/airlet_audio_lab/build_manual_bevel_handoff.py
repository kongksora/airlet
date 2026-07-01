from __future__ import annotations

import argparse
from dataclasses import dataclass
import math
from pathlib import Path
import tomllib


WOOD_MESH_NAMES = {"Mesh", "Mesh.008"}
DEFAULT_INPUT = Path("assets/generated/music_box_aligned_base.glb")
DEFAULT_OUTPUT = Path("target/manual-roundover/music_box_manual_bevel_handoff.blend")
DEFAULT_SPEC = Path("assets/models/converted/spec.toml")


def main() -> None:
    parser = argparse.ArgumentParser(description="Build a Blender-native manual bevel handoff.")
    parser.add_argument("--input", type=Path, default=DEFAULT_INPUT)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--spec", type=Path, default=DEFAULT_SPEC)
    args = _parse_blender_args(parser)

    import bpy

    spec = tomllib.loads(args.spec.read_text(encoding="utf-8"))
    crank_center = blender_point(tuple(spec["winding_key"]["pivot"])) if spec.get("winding_key") else None

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.ops.import_scene.gltf(filepath=str(args.input))

    replaced: list[tuple[str, int, int]] = []
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH" or obj.data.name not in WOOD_MESH_NAMES:
            continue
        bounds = oriented_shell_bounds(obj)
        old_materials = list(obj.data.materials)
        mesh_name = obj.data.name
        if mesh_name == "Mesh":
            vertices, faces = lid_quad_proxy(bounds)
        else:
            vertices, faces = body_quad_proxy(bounds)
        old_mesh = obj.data
        mesh = bpy.data.meshes.new(f"{mesh_name}.handoff")
        mesh.from_pydata(vertices, [], faces)
        mesh.update(calc_edges=True)
        for material in old_materials:
            mesh.materials.append(material)
        obj.data = mesh
        if old_mesh.users == 0:
            bpy.data.meshes.remove(old_mesh)
        obj.name = mesh_name
        obj.data.name = mesh_name
        if mesh_name == "Mesh.008" and crank_center is not None:
            cut_round_crank_opening(bpy, obj, bounds, crank_center)
        record_shell_bounds_metadata(obj, bounds)
        replaced.append((mesh_name, len(vertices), len(faces)))

    replaced_names = {name for name, _, _ in replaced}
    if replaced_names != WOOD_MESH_NAMES:
        raise SystemExit(f"expected to replace {sorted(WOOD_MESH_NAMES)}, replaced {sorted(replaced_names)}")

    center_offset = place_scene_meshes_on_origin_ground(bpy)
    bpy.context.scene["airlet_handoff_center_offset"] = tuple(float(value) for value in center_offset)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(args.output))
    print(f"wrote {args.output}")
    print(
        "placed assembly on origin ground by offset "
        f"({center_offset[0]:.6f}, {center_offset[1]:.6f}, {center_offset[2]:.6f})"
    )
    for name, vertex_count, face_count in replaced:
        print(f"{name}: {vertex_count} vertices, {face_count} quad faces")


@dataclass(frozen=True)
class OrientedShellBounds:
    origin: tuple[float, float]
    axis_x: tuple[float, float]
    axis_y: tuple[float, float]
    min_corner: tuple[float, float, float]
    max_corner: tuple[float, float, float]

    def point(self, coord: tuple[float, float, float]) -> tuple[float, float, float]:
        x, y, z = coord
        return (
            self.origin[0] + self.axis_x[0] * x + self.axis_y[0] * y,
            self.origin[1] + self.axis_x[1] * x + self.axis_y[1] * y,
            z,
        )

    def local_point(self, coord: tuple[float, float, float]) -> tuple[float, float, float]:
        x, y, z = coord
        dx = x - self.origin[0]
        dy = y - self.origin[1]
        return (
            dx * self.axis_x[0] + dy * self.axis_x[1],
            dx * self.axis_y[0] + dy * self.axis_y[1],
            z,
        )


def oriented_shell_bounds(obj) -> OrientedShellBounds:
    coords = [(vertex.co.x, vertex.co.y, vertex.co.z) for vertex in obj.data.vertices]
    center_x = sum(coord[0] for coord in coords) / len(coords)
    center_y = sum(coord[1] for coord in coords) / len(coords)
    xx = sum((coord[0] - center_x) * (coord[0] - center_x) for coord in coords)
    xy = sum((coord[0] - center_x) * (coord[1] - center_y) for coord in coords)
    yy = sum((coord[1] - center_y) * (coord[1] - center_y) for coord in coords)
    if abs(xy) < 1.0e-10 and xx >= yy:
        axis_x = (1.0, 0.0)
    elif abs(xy) < 1.0e-10:
        axis_x = (0.0, 1.0)
    else:
        angle = 0.5 * math.atan2(2.0 * xy, xx - yy)
        axis_x = (math.cos(angle), math.sin(angle))
    # Keep the first axis pointing roughly toward Blender +X so output is stable.
    if axis_x[0] < 0.0:
        axis_x = (-axis_x[0], -axis_x[1])
    axis_y = (-axis_x[1], axis_x[0])
    origin = (center_x, center_y)
    local = []
    for coord in coords:
        dx = coord[0] - origin[0]
        dy = coord[1] - origin[1]
        local.append((dx * axis_x[0] + dy * axis_x[1], dx * axis_y[0] + dy * axis_y[1], coord[2]))
    min_corner = tuple(min(point[index] for point in local) for index in range(3))
    max_corner = tuple(max(point[index] for point in local) for index in range(3))
    return OrientedShellBounds(origin, axis_x, axis_y, min_corner, max_corner)


def place_scene_meshes_on_origin_ground(bpy) -> tuple[float, float, float]:
    bounds_min, bounds_max = scene_mesh_bounds(bpy)
    offset = (
        -((bounds_min[0] + bounds_max[0]) * 0.5),
        -((bounds_min[1] + bounds_max[1]) * 0.5),
        -bounds_min[2],
    )
    for obj in bpy.context.scene.objects:
        if obj.type == "MESH":
            obj.location.x += offset[0]
            obj.location.y += offset[1]
            obj.location.z += offset[2]
    return offset


def scene_mesh_bounds(bpy) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    mesh_objects = [obj for obj in bpy.context.scene.objects if obj.type == "MESH"]
    if not mesh_objects:
        raise RuntimeError("cannot center handoff: scene has no mesh objects")
    corners = []
    for obj in mesh_objects:
        corners.extend(obj.matrix_world @ vertex.co for vertex in obj.data.vertices)
    bounds_min = tuple(min(corner[index] for corner in corners) for index in range(3))
    bounds_max = tuple(max(corner[index] for corner in corners) for index in range(3))
    return bounds_min, bounds_max


def lid_quad_proxy(
    bounds: OrientedShellBounds,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    x0, y0, z0 = bounds.min_corner
    x1, y1, z1 = bounds.max_corner
    width = x1 - x0
    depth = y1 - y0
    height = z1 - z0
    rim = min(max(min(width, depth) * 0.12, height * 0.36), width * 0.25, depth * 0.25)
    recess = min(max(height * 0.34, 0.0), height * 0.58)
    x = [x0, x0 + rim, x1 - rim, x1]
    y = [y0, y0 + rim, y1 - rim, y1]
    z = [z0, z0 + recess, z1]
    solid = {
        (ix, iy, iz)
        for ix in range(3)
        for iy in range(3)
        for iz in range(2)
        if iz == 1 or ix != 1 or iy != 1
    }
    return transform_vertices(voxel_boundary_quads(x, y, z, solid), bounds)


def body_quad_proxy(
    bounds: OrientedShellBounds,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    x0, y0, z0 = bounds.min_corner
    x1, y1, z1 = bounds.max_corner
    width = x1 - x0
    depth = y1 - y0
    height = z1 - z0
    wall = min(max(min(width, depth) * 0.13, height * 0.32), width * 0.28, depth * 0.28)
    floor = min(max(height * 0.24, 0.0), height * 0.42)
    xi0 = x0 + wall
    xi1 = x1 - wall
    yi0 = y0 + wall
    yi1 = y1 - wall
    zi = z0 + floor

    x = [x0, xi0, xi1, x1]
    y = [y0, yi0, yi1, y1]
    z = [z0, zi, z1]
    solid = {
        (ix, iy, iz)
        for ix in range(3)
        for iy in range(3)
        for iz in range(2)
        if iz == 0 or ix != 1 or iy != 1
    }
    return transform_vertices(voxel_boundary_quads(x, y, z, solid), bounds)


def clamp(value: float, low: float, high: float) -> float:
    return min(max(value, low), high)


def cut_round_crank_opening(
    bpy,
    obj,
    bounds: OrientedShellBounds,
    crank_center: tuple[float, float, float],
) -> None:
    x0, y0, z0 = bounds.min_corner
    x1, y1, z1 = bounds.max_corner
    depth = y1 - y0
    height = z1 - z0
    wall = min(max(min(x1 - x0, depth) * 0.13, height * 0.32), (x1 - x0) * 0.28, depth * 0.28)
    floor = min(max(height * 0.24, 0.0), height * 0.42)
    radius = min(depth * 0.055, height * 0.10, max(height * 0.075, 0.016))
    local_crank = bounds.local_point(crank_center)
    cy = clamp(local_crank[1], y0 + wall + radius * 1.2, y1 - wall - radius * 1.2)
    cz = clamp(local_crank[2], z0 + floor + radius * 1.2, z1 - radius * 1.2)
    length = wall * 2.8
    center = bounds.point((x1 - wall * 0.5, cy, cz))
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=32,
        radius=radius,
        depth=length,
        end_fill_type="NGON",
        location=center,
        rotation=(0.0, 1.5707963267948966, 0.0),
    )
    cutter = bpy.context.object
    cutter.name = "Airlet temporary crank opening cutter"
    boolean = obj.modifiers.new(name="Airlet round crank opening", type="BOOLEAN")
    boolean.operation = "DIFFERENCE"
    boolean.object = cutter
    boolean.solver = "EXACT"
    cutter.rotation_euler.rotate_axis("Z", math.atan2(bounds.axis_x[1], bounds.axis_x[0]))
    bpy.context.view_layer.objects.active = obj
    obj.select_set(True)
    bpy.ops.object.modifier_apply(modifier=boolean.name)
    bpy.data.objects.remove(cutter, do_unlink=True)


def record_shell_bounds_metadata(obj, bounds: OrientedShellBounds) -> None:
    obj["airlet_source_bounds_kind"] = "horizontal_obb"
    obj["airlet_source_obb_origin"] = tuple(float(value) for value in bounds.origin)
    obj["airlet_source_obb_axis_x"] = tuple(float(value) for value in bounds.axis_x)
    obj["airlet_source_obb_axis_y"] = tuple(float(value) for value in bounds.axis_y)
    obj["airlet_source_obb_min"] = tuple(float(value) for value in bounds.min_corner)
    obj["airlet_source_obb_max"] = tuple(float(value) for value in bounds.max_corner)
    obj["airlet_source_obb_dimensions"] = tuple(
        float(bounds.max_corner[index] - bounds.min_corner[index]) for index in range(3)
    )


def blender_point(point: tuple[float, float, float]) -> tuple[float, float, float]:
    x, y, z = point
    return (x, -z, y)


def transform_vertices(
    mesh: tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]],
    bounds: OrientedShellBounds,
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    vertices, faces = mesh
    return [bounds.point(vertex) for vertex in vertices], faces


def voxel_boundary_quads(
    x: list[float],
    y: list[float],
    z: list[float],
    solid: set[tuple[int, int, int]],
) -> tuple[list[tuple[float, float, float]], list[tuple[int, int, int, int]]]:
    vertices: list[tuple[float, float, float]] = []
    faces: list[tuple[int, int, int, int]] = []
    vertex_index: dict[tuple[float, float, float], int] = {}

    def vertex(coord: tuple[float, float, float]) -> int:
        if coord not in vertex_index:
            vertex_index[coord] = len(vertices)
            vertices.append(coord)
        return vertex_index[coord]

    def add_quad(coords: list[tuple[float, float, float]]) -> None:
        faces.append(tuple(vertex(coord) for coord in coords))

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
    return vertices, faces


def _parse_blender_args(parser: argparse.ArgumentParser) -> argparse.Namespace:
    import sys

    argv = sys.argv
    if "--" in argv:
        argv = argv[argv.index("--") + 1 :]
    else:
        argv = argv[1:]
    return parser.parse_args(argv)


if __name__ == "__main__":
    main()
