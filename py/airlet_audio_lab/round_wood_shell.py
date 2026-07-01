from __future__ import annotations

import argparse
from pathlib import Path


WOOD_MESH_NAMES = {"Mesh", "Mesh.008"}


def main() -> None:
    parser = argparse.ArgumentParser(description="Round Airlet wooden shell meshes with Blender.")
    parser.add_argument("--input", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--width", type=float, default=0.005)
    parser.add_argument("--segments", type=int, default=5)
    parser.add_argument("--angle-degrees", type=float, default=90.0)
    parser.add_argument("--merge-distance", type=float, default=1.0e-5)
    args = _parse_blender_args(parser)

    import bpy

    bpy.ops.object.select_all(action="SELECT")
    bpy.ops.object.delete()
    bpy.ops.import_scene.gltf(filepath=str(args.input))

    rounded = []
    for obj in bpy.context.scene.objects:
        if obj.type != "MESH" or obj.data.name not in WOOD_MESH_NAMES:
            continue
        bpy.ops.object.select_all(action="DESELECT")
        bpy.context.view_layer.objects.active = obj
        obj.select_set(True)
        bpy.ops.object.mode_set(mode="EDIT")
        bpy.ops.mesh.select_all(action="SELECT")
        bpy.ops.mesh.remove_doubles(threshold=args.merge_distance)
        bpy.ops.object.mode_set(mode="OBJECT")
        object_scale = obj.matrix_world.to_3x3().to_scale()
        average_scale = sum(abs(component) for component in object_scale) / 3.0
        if average_scale <= 0:
            raise SystemExit(f"{obj.name} has invalid world scale {tuple(object_scale)}")
        local_width = args.width / average_scale

        selected_edge_count = bevel_selected_exterior_edges(obj, local_width, args.segments)
        weighted = obj.modifiers.new(name="Airlet weighted shell normals", type="WEIGHTED_NORMAL")
        weighted.keep_sharp = True
        bpy.ops.object.modifier_apply(modifier=weighted.name)
        obj.select_set(False)
        rounded.append(
            (
                obj.data.name,
                local_width,
                tuple(object_scale),
                selected_edge_count,
            )
        )

    rounded_names = [name for name, _, _, _ in rounded]
    if sorted(rounded_names) != sorted(WOOD_MESH_NAMES):
        raise SystemExit(
            f"expected wood meshes {sorted(WOOD_MESH_NAMES)}, rounded {sorted(rounded_names)}"
        )

    args.output.parent.mkdir(parents=True, exist_ok=True)
    bpy.ops.export_scene.gltf(
        filepath=str(args.output),
        export_format="GLB",
        export_apply=False,
        export_yup=True,
    )
    print(f"rounded wood meshes: {', '.join(sorted(rounded_names))}")
    print(
        f"bevel width={args.width}, requested angle={args.angle_degrees}, "
        f"selected-edge mode, merge distance={args.merge_distance}"
    )
    for name, local_width, scale, selected_edge_count in rounded:
        scale_text = ", ".join(f"{component:.6g}" for component in scale)
        print(
            f"{name}: local bevel width={local_width:.6g}, object scale=({scale_text}), "
            f"selected exterior bevel edges={selected_edge_count}"
        )
    print(f"wrote {args.output}")


def bevel_selected_exterior_edges(obj, local_width: float, segments: int) -> int:
    import bmesh
    import bpy

    bpy.ops.object.mode_set(mode="EDIT")
    bpy.ops.mesh.select_mode(type="EDGE")
    bpy.ops.mesh.select_all(action="DESELECT")
    mesh = bmesh.from_edit_mesh(obj.data)
    mesh.edges.ensure_lookup_table()
    coords = [vertex.co.copy() for vertex in mesh.verts]
    min_corner = coords[0].copy()
    max_corner = coords[0].copy()
    for coord in coords[1:]:
        min_corner.x = min(min_corner.x, coord.x)
        min_corner.y = min(min_corner.y, coord.y)
        min_corner.z = min(min_corner.z, coord.z)
        max_corner.x = max(max_corner.x, coord.x)
        max_corner.y = max(max_corner.y, coord.y)
        max_corner.z = max(max_corner.z, coord.z)
    threshold = local_width * 0.45
    top_guard = local_width * 0.9
    bottom_guard = local_width * 0.9
    selected = 0
    for edge in mesh.edges:
        midpoint = (edge.verts[0].co + edge.verts[1].co) * 0.5
        near_min_x = abs(midpoint.x - min_corner.x) <= threshold
        near_max_x = abs(midpoint.x - max_corner.x) <= threshold
        near_min_z = abs(midpoint.z - min_corner.z) <= threshold
        near_max_z = abs(midpoint.z - max_corner.z) <= threshold
        near_min_y = abs(midpoint.y - min_corner.y) <= threshold
        near_max_y = abs(midpoint.y - max_corner.y) <= threshold
        exterior_x = near_min_x or near_max_x
        exterior_z = near_min_z or near_max_z
        exterior_side = exterior_x or exterior_z
        exterior_corner = exterior_x and exterior_z
        if obj.data.name == "Mesh.008":
            below_body_contact = midpoint.y < max_corner.y - top_guard
            should_select = (
                (near_min_y and exterior_side)
                or (exterior_corner and below_body_contact)
            )
        else:
            above_lid_contact = midpoint.y > min_corner.y + bottom_guard
            should_select = (
                (near_max_y and exterior_side)
                or (exterior_corner and above_lid_contact)
            )
        edge.select = bool(should_select)
        if edge.select:
            selected += 1
    bmesh.update_edit_mesh(obj.data)
    if selected:
        bpy.ops.mesh.bevel(
            offset=local_width,
            segments=segments,
            profile=0.5,
            affect="EDGES",
            harden_normals=True,
        )
    bpy.ops.object.mode_set(mode="OBJECT")
    return selected


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
