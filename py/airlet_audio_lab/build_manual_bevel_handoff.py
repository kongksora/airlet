from __future__ import annotations

import argparse
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
    clear_scene(bpy)
    bpy.ops.import_scene.gltf(filepath=str(args.input))

    baked: list[tuple[str, int, int]] = []
    body_obj = None
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH" or obj.data.name not in WOOD_MESH_NAMES:
            continue
        bake_object_mesh_to_world_space(bpy, obj)
        if obj.data.name == "Mesh.008":
            body_obj = obj
            cut_round_crank_opening(bpy, obj, spec)
        record_mesh_metadata(obj)
        baked.append((obj.data.name, len(obj.data.vertices), len(obj.data.polygons)))

    if body_obj is None:
        raise SystemExit("expected to find body mesh Mesh.008")

    baked_names = {name for name, _, _ in baked}
    if baked_names != WOOD_MESH_NAMES:
        raise SystemExit(f"expected to bake {sorted(WOOD_MESH_NAMES)}, baked {sorted(baked_names)}")

    center_offset = place_scene_meshes_on_origin_ground(bpy)
    bpy.context.scene["airlet_handoff_source"] = str(args.input)
    bpy.context.scene["airlet_handoff_center_offset"] = tuple(float(value) for value in center_offset)

    args.output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.wm.save_as_mainfile(filepath=str(args.output))
    print(f"wrote {args.output}")
    print(
        "placed assembly on origin ground by offset "
        f"({center_offset[0]:.6f}, {center_offset[1]:.6f}, {center_offset[2]:.6f})"
    )
    for name, vertex_count, face_count in baked:
        print(f"{name}: {vertex_count} vertices, {face_count} aligned-base faces")


def bake_object_mesh_to_world_space(bpy, obj) -> None:
    from mathutils import Matrix

    old_mesh = obj.data
    mesh_name = old_mesh.name
    old_materials = list(old_mesh.materials)
    vertices = [tuple(obj.matrix_world @ vertex.co) for vertex in old_mesh.vertices]
    faces = [tuple(poly.vertices) for poly in old_mesh.polygons]

    mesh = bpy.data.meshes.new(f"{mesh_name}.handoff")
    mesh.from_pydata(vertices, [], faces)
    mesh.update(calc_edges=True)
    for material in old_materials:
        mesh.materials.append(material)

    obj.data = mesh
    obj.matrix_world = Matrix.Identity(4)
    if old_mesh.users == 0:
        bpy.data.meshes.remove(old_mesh)
    obj.name = mesh_name
    obj.data.name = mesh_name


def record_mesh_metadata(obj) -> None:
    bounds_min, bounds_max = object_bounds(obj)
    obj["airlet_handoff_mesh_source"] = "aligned_base_world_space"
    obj["airlet_handoff_bounds_min"] = tuple(float(value) for value in bounds_min)
    obj["airlet_handoff_bounds_max"] = tuple(float(value) for value in bounds_max)
    obj["airlet_handoff_dimensions"] = tuple(
        float(bounds_max[index] - bounds_min[index]) for index in range(3)
    )


def cut_round_crank_opening(bpy, obj, spec: dict) -> None:
    from mathutils import Matrix

    winding = spec.get("winding_key")
    if not winding:
        return
    bounds_min, bounds_max = object_bounds(obj)
    width = bounds_max[0] - bounds_min[0]
    depth = bounds_max[1] - bounds_min[1]
    height = bounds_max[2] - bounds_min[2]
    wall = min(max(width * 0.13, depth * 0.13), width * 0.28, depth * 0.28)
    radius = min(depth * 0.08, height * 0.18, max(height * 0.12, wall * 0.36))
    pivot = runtime_point_to_blender(tuple(float(value) for value in winding["pivot"]))
    center = (
        bounds_max[0] - wall * 0.5,
        float(min(max(pivot[1], bounds_min[1] + radius), bounds_max[1] - radius)),
        float(min(max(pivot[2], bounds_min[2] + radius), bounds_max[2] - radius)),
    )
    bpy.ops.mesh.primitive_cylinder_add(
        vertices=64,
        radius=radius,
        depth=wall * 3.0,
        end_fill_type="NGON",
        location=center,
        rotation=(0.0, math.pi * 0.5, 0.0),
    )
    cutter = bpy.context.object
    cutter.name = "Airlet round crank opening cutter"
    boolean = obj.modifiers.new(name="Airlet round crank opening", type="BOOLEAN")
    boolean.operation = "DIFFERENCE"
    boolean.object = cutter
    boolean.solver = "EXACT"
    bpy.ops.object.select_all(action="DESELECT")
    obj.select_set(True)
    bpy.context.view_layer.objects.active = obj
    bpy.ops.object.modifier_apply(modifier=boolean.name)
    obj.matrix_world = Matrix.Identity(4)
    bpy.data.objects.remove(cutter, do_unlink=True)
    obj["airlet_crank_opening_kind"] = "blender_boolean_cylinder"
    obj["airlet_crank_opening_center"] = tuple(float(value) for value in center)
    obj["airlet_crank_opening_radius"] = float(radius)


def runtime_point_to_blender(point: tuple[float, float, float]) -> tuple[float, float, float]:
    x, y, z = point
    return (x, -z, y)


def clear_scene(bpy) -> None:
    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    for mesh in list(bpy.data.meshes):
        if mesh.users == 0:
            bpy.data.meshes.remove(mesh)


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


def object_bounds(obj) -> tuple[tuple[float, float, float], tuple[float, float, float]]:
    corners = [obj.matrix_world @ vertex.co for vertex in obj.data.vertices]
    bounds_min = tuple(min(corner[index] for corner in corners) for index in range(3))
    bounds_max = tuple(max(corner[index] for corner in corners) for index in range(3))
    return bounds_min, bounds_max


def _parse_blender_args(parser: argparse.ArgumentParser) -> argparse.Namespace:
    import sys

    args = sys.argv
    if "--" in args:
        args = args[args.index("--") + 1 :]
    else:
        args = []
    return parser.parse_args(args)


if __name__ == "__main__":
    main()
