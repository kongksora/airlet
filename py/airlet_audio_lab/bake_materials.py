from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Iterable

import numpy as np
import tomllib
from pygltflib import (
    ARRAY_BUFFER,
    ELEMENT_ARRAY_BUFFER,
    FLOAT,
    UNSIGNED_INT,
    Accessor,
    Attributes,
    BufferView,
    GLTF2,
    Image,
    Material,
    NormalMaterialTexture,
    PbrMetallicRoughness,
    Primitive,
    Texture,
    TextureInfo,
)

from airlet_audio_lab.generate_textures import DEFAULT_OUT_DIR, main as generate_textures_main


DEFAULT_SPEC = Path("assets/models/converted/spec.toml")
DEFAULT_OUTPUT = Path("assets/generated/music_box_material_baked.glb")
DEFAULT_ALIGNED_BASE_SOURCE = Path("assets/generated/music_box_aligned_base.glb")
DEFAULT_ROUNDED_SOURCE = Path("assets/generated/music_box_rounded_shell.glb")
DEFAULT_MANUAL_ROUNDED_SOURCE = Path("assets/generated/music_box_manual_rounded_shell.glb")
BUILD_ALIGNED_BASE_SCRIPT = Path("py/airlet_audio_lab/build_aligned_base_model.py")
ROUND_WOOD_SCRIPT = Path("py/airlet_audio_lab/round_wood_shell.py")
BEVEL_WIDTH_METERS = 0.005
BEVEL_ANGLE_DEGREES = 90.0
BEVEL_MERGE_DISTANCE = 1.0e-5
WOOD_UV_DENSITY = 1.0
TARGET_DISPLAY_WIDTH_METERS = 0.14
SOURCE_ALIGNED_WIDTH = 0.66793925
APP_MODEL_SCALE = TARGET_DISPLAY_WIDTH_METERS / SOURCE_ALIGNED_WIDTH
BEVEL_WIDTH_MODEL_UNITS = BEVEL_WIDTH_METERS / APP_MODEL_SCALE

WOOD_MESHES = {0, 8}
WOOD_MESH_NAMES = {"Mesh", "Mesh.008"}
BRASS_MESHES = {1, 2, 4, 5, 6, 7, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 27, 30}
STEEL_MESHES = {21, 22, 24, 25, 31, 32, 33, 34, 35}
DARK_METAL_MESHES = {3, 9, 28, 29, 36}


def main() -> None:
    parser = argparse.ArgumentParser(description="Bake Airlet model materials into a temporary GLB.")
    parser.add_argument("--spec", type=Path, default=DEFAULT_SPEC)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--rounded-source", type=Path, default=DEFAULT_ROUNDED_SOURCE)
    parser.add_argument(
        "--manual-rounded-source",
        type=Path,
        default=None,
        help=(
            "Use a manually rounded full GLB as the material-bake source. "
            "This skips the automatic Blender bevel pass."
        ),
    )
    parser.add_argument("--textures", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--bevel-width", type=float, default=BEVEL_WIDTH_MODEL_UNITS)
    parser.add_argument("--bevel-angle-degrees", type=float, default=BEVEL_ANGLE_DEGREES)
    parser.add_argument("--bevel-merge-distance", type=float, default=BEVEL_MERGE_DISTANCE)
    parser.add_argument("--bevel-segments", type=int, default=5)
    parser.add_argument("--skip-rounding", action="store_true")
    parser.add_argument("--skip-textures", action="store_true")
    args = parser.parse_args()

    if not args.skip_textures:
        generate_textures_main_with_args(args.textures)

    spec = tomllib.loads(args.spec.read_text(encoding="utf-8"))
    source = Path("assets") / spec["asset"]["gltf"]
    if source == DEFAULT_ALIGNED_BASE_SOURCE:
        build_aligned_base(args.spec, source, args.bevel_width)
        spec = tomllib.loads(args.spec.read_text(encoding="utf-8"))
        source = Path("assets") / spec["asset"]["gltf"]
    manual_alignment_source: Path | None = None
    if args.manual_rounded_source is not None:
        if not args.manual_rounded_source.exists():
            raise SystemExit(f"manual rounded source does not exist: {args.manual_rounded_source}")
        manual_alignment_source = source
        source = args.manual_rounded_source
    elif not args.skip_rounding:
        round_wood_shell(
            source,
            args.rounded_source,
            args.bevel_width,
            args.bevel_segments,
            args.bevel_angle_degrees,
            args.bevel_merge_distance,
        )
        source = args.rounded_source
    output = args.output
    output.parent.mkdir(parents=True, exist_ok=True)

    baker = GltfMaterialBaker(source, output, args.textures, spec, align_to=manual_alignment_source)
    report = baker.bake()
    report_path = output.with_suffix(".json")
    report_path.write_text(json.dumps(report, indent=2, sort_keys=True), encoding="utf-8")
    print(f"wrote {output}")
    print(f"wrote {report_path}")


def generate_textures_main_with_args(out_dir: Path) -> None:
    import sys

    old_argv = sys.argv
    try:
        sys.argv = ["airlet-generate-textures", "--out-dir", str(out_dir)]
        generate_textures_main()
    finally:
        sys.argv = old_argv


def round_wood_shell(
    source: Path,
    output: Path,
    width: float,
    segments: int,
    angle_degrees: float,
    merge_distance: float,
) -> None:
    blender = shutil.which("blender")
    if blender is None:
        raise RuntimeError("Blender is required for rounded wood shell baking but was not found on PATH")
    output.parent.mkdir(parents=True, exist_ok=True)
    command = [
        blender,
        "--background",
        "--python",
        str(ROUND_WOOD_SCRIPT),
        "--",
        "--input",
        str(source),
        "--output",
        str(output),
        "--width",
        str(width),
        "--segments",
        str(segments),
        "--angle-degrees",
        str(angle_degrees),
        "--merge-distance",
        str(merge_distance),
    ]
    subprocess.run(command, check=True)


def build_aligned_base(spec: Path, output: Path, bevel_width: float) -> None:
    output.parent.mkdir(parents=True, exist_ok=True)
    report = output.with_suffix(".json")
    command = [
        sys.executable,
        str(BUILD_ALIGNED_BASE_SCRIPT),
        "--spec",
        str(spec),
        "--output",
        str(output),
        "--report",
        str(report),
        "--wood-bevel-width",
        str(bevel_width),
        "--write-spec",
    ]
    subprocess.run(command, check=True)


@dataclass(frozen=True)
class Basis:
    right: np.ndarray
    up: np.ndarray
    front: np.ndarray

    @classmethod
    def from_spec(cls, spec: dict) -> "Basis":
        basis = spec.get("basis", {})
        return cls(
            right=normalized(np.array(basis.get("right", [1.0, 0.0, 0.0]), dtype=np.float32)),
            up=normalized(np.array(basis.get("up", [0.0, 1.0, 0.0]), dtype=np.float32)),
            front=normalized(np.array(basis.get("front", [0.0, 0.0, 1.0]), dtype=np.float32)),
        )


@dataclass
class BakedPrimitive:
    positions: list[list[float]]
    normals: list[list[float]]
    uvs: list[list[float]]
    tangents: list[list[float]]
    indices: list[int]
    material: int

    @property
    def triangle_count(self) -> int:
        return len(self.indices) // 3


class GltfMaterialBaker:
    def __init__(
        self,
        source: Path,
        output: Path,
        textures: Path,
        spec: dict,
        align_to: Path | None = None,
    ) -> None:
        self.source = source
        self.output = output
        self.textures = textures
        self.spec = spec
        self.gltf = GLTF2().load(source)
        self.blob = bytearray(self.gltf.binary_blob() or b"")
        self.manual_alignment_delta: list[float] | None = None
        if align_to is not None:
            self.manual_alignment_delta = self._align_mesh_nodes_to_reference(align_to)
        self.basis = Basis.from_spec(spec)
        self.mesh_to_node = self._mesh_node_transforms()
        self.wood_axis_ranges = self._wood_axis_ranges()
        self.wood_window = WoodWindow.shared_shell()

    def bake(self) -> dict:
        materials = self._append_baked_materials()
        report = {
            "source": str(self.source),
            "output": str(self.output),
            "wood_meshes": sorted(WOOD_MESHES),
            "wood_mesh_names": sorted(WOOD_MESH_NAMES),
            "manual_alignment_delta": self.manual_alignment_delta,
            "split_meshes": {},
            "winding_fixes": {},
            "material_assignments": {},
        }
        for mesh_index, mesh in enumerate(self.gltf.meshes):
            if is_wood_mesh(mesh_index, mesh.name):
                old_primitives = list(mesh.primitives)
                new_primitives: list[Primitive] = []
                split_counts = {"radial": 0, "tangential": 0, "cross": 0}
                for primitive in old_primitives:
                    split = self._split_wood_primitive(mesh_index, primitive, materials)
                    new_primitives.extend(split)
                    for baked in split:
                        if baked.material == materials["walnut_wood_radial"]:
                            split_counts["radial"] += self.gltf.accessors[baked.indices].count // 3
                        elif baked.material == materials["walnut_wood_tangential"]:
                            split_counts["tangential"] += self.gltf.accessors[baked.indices].count // 3
                        elif baked.material == materials["walnut_wood_cross"]:
                            split_counts["cross"] += self.gltf.accessors[baked.indices].count // 3
                if new_primitives:
                    signed_volume = self._mesh_signed_volume(new_primitives)
                    if signed_volume < -1.0e-7:
                        for primitive in new_primitives:
                            self._flip_primitive_winding(primitive)
                        report["winding_fixes"][str(mesh_index)] = {
                            "mesh": mesh.name,
                            "signed_volume_before": signed_volume,
                            "signed_volume_after": self._mesh_signed_volume(new_primitives),
                        }
                    mesh.primitives = new_primitives
                    report["split_meshes"][str(mesh_index)] = split_counts
            else:
                material_class = material_class_for_mesh(mesh_index)
                if material_class is None:
                    continue
                for primitive in mesh.primitives:
                    self._ensure_surface_attributes(mesh_index, primitive)
                    primitive.material = materials[material_class]
                report["material_assignments"][str(mesh_index)] = material_class

        self.gltf.set_binary_blob(bytes(self.blob))
        self.gltf.buffers[0].byteLength = len(self.blob)
        self.gltf.save_binary(self.output)
        return report

    def _append_baked_materials(self) -> dict[str, int]:
        result = {}
        for name, pbr in {
            "walnut_wood_radial": (0.0, 0.46, 0.48),
            "walnut_wood_tangential": (0.0, 0.48, 0.46),
            "walnut_wood_cross": (0.0, 0.34, 0.40),
            "aged_brass": (0.96, 0.24, 0.86),
            "polished_steel": (0.98, 0.12, 0.86),
            "dark_metal": (0.82, 0.54, 0.45),
        }.items():
            base = self._append_texture(f"{name}_base.png")
            orm = self._append_texture(f"{name}_orm.png")
            normal = self._append_texture(f"{name}_normal.png")
            metallic, roughness, _reflectance = pbr
            material = Material(
                name=f"Airlet {name}",
                pbrMetallicRoughness=PbrMetallicRoughness(
                    baseColorFactor=[1.0, 1.0, 1.0, 1.0],
                    metallicFactor=metallic,
                    roughnessFactor=roughness,
                    baseColorTexture=TextureInfo(index=base, texCoord=0),
                    metallicRoughnessTexture=TextureInfo(index=orm, texCoord=0),
                ),
                normalTexture=NormalMaterialTexture(index=normal, texCoord=0, scale=1.0),
                doubleSided=True,
            )
            self.gltf.materials.append(material)
            result[name] = len(self.gltf.materials) - 1
        return result

    def _append_texture(self, file_name: str) -> int:
        uri = relative_uri(self.output.parent, self.textures / file_name)
        self.gltf.images.append(Image(uri=uri, mimeType="image/png", name=file_name))
        image_index = len(self.gltf.images) - 1
        self.gltf.textures.append(Texture(source=image_index, name=file_name))
        return len(self.gltf.textures) - 1

    def _split_wood_primitive(
        self, mesh_index: int, primitive: Primitive, materials: dict[str, int]
    ) -> list[Primitive]:
        positions = self._read_accessor_vec(primitive.attributes.POSITION, 3)
        indices = self._read_indices(primitive.indices, len(positions))
        normals = self._read_accessor_vec_or_none(primitive.attributes.NORMAL, 3)
        if normals is None:
            normals = generated_vertex_normals(positions, indices)
        transform = self.mesh_to_node.get(mesh_index, np.identity(4, dtype=np.float32))
        model_positions = transform_points(transform, positions)
        projection = WoodProjection(
            self.basis,
            model_positions,
            self.wood_window,
            axis_ranges=self.wood_axis_ranges,
        )
        radial = BakedPrimitive([], [], [], [], [], materials["walnut_wood_radial"])
        tangential = BakedPrimitive([], [], [], [], [], materials["walnut_wood_tangential"])
        cross = BakedPrimitive([], [], [], [], [], materials["walnut_wood_cross"])

        for a, b, c in batched(indices, 3):
            local_tri = np.array([positions[a], positions[b], positions[c]], dtype=np.float32)
            local_normals = np.array([normals[a], normals[b], normals[c]], dtype=np.float32)
            model_tri = np.array([model_positions[a], model_positions[b], model_positions[c]], dtype=np.float32)
            face_normal = normalized(np.cross(model_tri[1] - model_tri[0], model_tri[2] - model_tri[0]))
            if not np.any(face_normal):
                continue
            wood_class = projection.wood_class(face_normal)
            target = {"radial": radial, "tangential": tangential, "cross": cross}[wood_class]
            uvs = [projection.uv_for(point, wood_class) for point in model_tri]
            append_triangle(target, local_tri, local_normals, uvs)

        result = []
        for baked in (radial, tangential, cross):
            if baked.indices:
                result.append(self._write_primitive(baked))
        return result

    def _write_primitive(self, primitive: BakedPrimitive) -> Primitive:
        position_accessor = self._append_accessor(
            np.array(primitive.positions, dtype=np.float32), "VEC3", ARRAY_BUFFER
        )
        normal_accessor = self._append_accessor(
            np.array(primitive.normals, dtype=np.float32), "VEC3", ARRAY_BUFFER
        )
        uv_accessor = self._append_accessor(
            np.array(primitive.uvs, dtype=np.float32), "VEC2", ARRAY_BUFFER
        )
        tangent_accessor = self._append_accessor(
            np.array(primitive.tangents, dtype=np.float32), "VEC4", ARRAY_BUFFER
        )
        index_accessor = self._append_accessor(
            np.array(primitive.indices, dtype=np.uint32), "SCALAR", ELEMENT_ARRAY_BUFFER
        )
        return Primitive(
            attributes=Attributes(
                POSITION=position_accessor,
                NORMAL=normal_accessor,
                TEXCOORD_0=uv_accessor,
                TANGENT=tangent_accessor,
            ),
            indices=index_accessor,
            mode=4,
            material=primitive.material,
        )

    def _ensure_surface_attributes(self, mesh_index: int, primitive: Primitive) -> None:
        if (
            primitive.attributes.NORMAL is not None
            and primitive.attributes.TEXCOORD_0 is not None
            and primitive.attributes.TANGENT is not None
        ):
            return
        positions = self._read_accessor_vec(primitive.attributes.POSITION, 3)
        indices = self._read_indices(primitive.indices, len(positions))
        normals = self._read_accessor_vec_or_none(primitive.attributes.NORMAL, 3)
        if normals is None:
            normals = generated_vertex_normals(positions, indices)
        transform = self.mesh_to_node.get(mesh_index, np.identity(4, dtype=np.float32))
        model_positions = transform_points(transform, positions)
        ranges = WoodAxisRanges.from_points(self.basis, model_positions)
        uvs = np.array(
            [
                [
                    ranges.right.normalize(float(point @ self.basis.right)),
                    ranges.front.normalize(float(point @ self.basis.front)),
                ]
                for point in model_positions
            ],
            dtype=np.float32,
        )
        tangents = np.array([fallback_tangent(normal) for normal in normals], dtype=np.float32)
        primitive.attributes.NORMAL = self._append_accessor(normals, "VEC3", ARRAY_BUFFER)
        primitive.attributes.TEXCOORD_0 = self._append_accessor(uvs, "VEC2", ARRAY_BUFFER)
        primitive.attributes.TANGENT = self._append_accessor(tangents, "VEC4", ARRAY_BUFFER)

    def _mesh_signed_volume(self, primitives: list[Primitive]) -> float:
        signed_volume = 0.0
        for primitive in primitives:
            positions = self._read_accessor_vec(primitive.attributes.POSITION, 3)
            indices = self._read_indices(primitive.indices, len(positions))
            for a, b, c in batched(indices, 3):
                signed_volume += float(np.dot(positions[a], np.cross(positions[b], positions[c])) / 6.0)
        return signed_volume

    def _flip_primitive_winding(self, primitive: Primitive) -> None:
        positions = self._read_accessor_vec(primitive.attributes.POSITION, 3)
        normals = self._read_accessor_vec(primitive.attributes.NORMAL, 3)
        uvs = self._read_accessor_vec(primitive.attributes.TEXCOORD_0, 2)
        tangents = self._read_accessor_vec(primitive.attributes.TANGENT, 4)
        indices = np.array(self._read_indices(primitive.indices, len(positions)), dtype=np.uint32).reshape(-1, 3)
        indices[:, [1, 2]] = indices[:, [2, 1]]
        normals = -normals
        tangents[:, :3] = -tangents[:, :3]
        tangents[:, 3] = -tangents[:, 3]
        primitive.attributes.NORMAL = self._append_accessor(normals.astype(np.float32), "VEC3", ARRAY_BUFFER)
        primitive.attributes.TEXCOORD_0 = self._append_accessor(uvs.astype(np.float32), "VEC2", ARRAY_BUFFER)
        primitive.attributes.TANGENT = self._append_accessor(tangents.astype(np.float32), "VEC4", ARRAY_BUFFER)
        primitive.indices = self._append_accessor(indices.reshape(-1).astype(np.uint32), "SCALAR", ELEMENT_ARRAY_BUFFER)

    def _append_accessor(self, data: np.ndarray, accessor_type: str, target: int) -> int:
        component_type = FLOAT if data.dtype == np.float32 else UNSIGNED_INT
        payload = data.tobytes()
        self._align_blob(4)
        byte_offset = len(self.blob)
        self.blob.extend(payload)
        buffer_view = BufferView(
            buffer=0,
            byteOffset=byte_offset,
            byteLength=len(payload),
            target=target,
        )
        self.gltf.bufferViews.append(buffer_view)
        buffer_view_index = len(self.gltf.bufferViews) - 1
        kwargs = {}
        if data.dtype == np.float32 and accessor_type != "SCALAR":
            kwargs["min"] = data.min(axis=0).astype(float).tolist()
            kwargs["max"] = data.max(axis=0).astype(float).tolist()
        accessor = Accessor(
            bufferView=buffer_view_index,
            byteOffset=0,
            componentType=component_type,
            count=len(data),
            type=accessor_type,
            **kwargs,
        )
        self.gltf.accessors.append(accessor)
        return len(self.gltf.accessors) - 1

    def _align_blob(self, alignment: int) -> None:
        padding = (-len(self.blob)) % alignment
        if padding:
            self.blob.extend(b"\x00" * padding)

    def _read_accessor_vec(self, accessor_index: int, width: int) -> np.ndarray:
        accessor = self.gltf.accessors[accessor_index]
        if accessor.componentType != FLOAT or accessor.type != f"VEC{width}":
            raise ValueError(f"unsupported accessor {accessor_index}: {accessor}")
        data = self._accessor_bytes(accessor)
        return np.frombuffer(data, dtype=np.float32).reshape(accessor.count, width).copy()

    def _read_accessor_vec_or_none(self, accessor_index: int | None, width: int) -> np.ndarray | None:
        if accessor_index is None:
            return None
        return self._read_accessor_vec(accessor_index, width)

    def _read_indices(self, accessor_index: int | None, vertex_count: int) -> list[int]:
        if accessor_index is None:
            return list(range(vertex_count))
        accessor = self.gltf.accessors[accessor_index]
        data = self._accessor_bytes(accessor)
        if accessor.componentType == 5123:
            return np.frombuffer(data, dtype=np.uint16).astype(np.uint32).tolist()
        if accessor.componentType == 5125:
            return np.frombuffer(data, dtype=np.uint32).tolist()
        raise ValueError(f"unsupported index accessor {accessor_index}: {accessor.componentType}")

    def _accessor_bytes(self, accessor: Accessor) -> bytes:
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

    def _mesh_node_transforms(self) -> dict[int, np.ndarray]:
        result = {}
        for node in self.gltf.nodes:
            if node.mesh is None or node.mesh in result:
                continue
            result[node.mesh] = node_matrix(node)
        return result

    def _align_mesh_nodes_to_reference(self, reference: Path) -> list[float]:
        reference_gltf = GLTF2().load(reference)
        reference_blob = bytearray(reference_gltf.binary_blob() or b"")
        source_min, source_max = gltf_world_bounds(self.gltf, self.blob, include_mesh=include_alignment_mesh)
        reference_min, reference_max = gltf_world_bounds(
            reference_gltf,
            reference_blob,
            include_mesh=include_alignment_mesh,
        )
        source_center = (source_min + source_max) * 0.5
        reference_center = (reference_min + reference_max) * 0.5
        delta = reference_center - source_center
        if float(np.linalg.norm(delta)) < 1.0e-7:
            return [0.0, 0.0, 0.0]
        for node in self.gltf.nodes:
            if node.mesh is not None:
                translate_node(node, delta)
        return delta.astype(float).tolist()

    def _wood_axis_ranges(self) -> "WoodAxisRanges":
        wood_points = []
        for mesh_index, mesh in enumerate(self.gltf.meshes):
            if not is_wood_mesh(mesh_index, mesh.name):
                continue
            transform = self.mesh_to_node.get(mesh_index, np.identity(4, dtype=np.float32))
            for primitive in mesh.primitives:
                positions = self._read_accessor_vec(primitive.attributes.POSITION, 3)
                wood_points.append(transform_points(transform, positions))
        if not wood_points:
            raise ValueError("no wood mesh points found for material baking")
        return WoodAxisRanges.from_points(self.basis, np.concatenate(wood_points, axis=0))


@dataclass(frozen=True)
class WoodWindow:
    radial_offset: tuple[float, float]
    radial_scale: tuple[float, float]
    tangential_offset: tuple[float, float]
    tangential_scale: tuple[float, float]
    cross_offset: tuple[float, float]
    cross_scale: tuple[float, float]

    @classmethod
    def for_mesh(cls, mesh_index: int) -> "WoodWindow":
        rng = np.random.default_rng(1_304_071 + mesh_index * 9_973)
        return cls.from_rng(rng)

    @classmethod
    def shared_shell(cls) -> "WoodWindow":
        rng = np.random.default_rng(1_304_071)
        return cls.from_rng(rng)

    @classmethod
    def from_rng(cls, rng: np.random.Generator) -> "WoodWindow":
        return cls(
            radial_offset=(float(rng.uniform(0.10, 0.18)), float(rng.uniform(0.12, 0.22))),
            radial_scale=(
                float(rng.uniform(0.58, 0.70) * WOOD_UV_DENSITY),
                float(rng.uniform(0.30, 0.42) * WOOD_UV_DENSITY),
            ),
            tangential_offset=(float(rng.uniform(0.10, 0.18)), float(rng.uniform(0.12, 0.22))),
            tangential_scale=(
                float(rng.uniform(0.58, 0.70) * WOOD_UV_DENSITY),
                float(rng.uniform(0.30, 0.42) * WOOD_UV_DENSITY),
            ),
            cross_offset=(float(rng.uniform(0.05, 0.52)), float(rng.uniform(0.05, 0.48))),
            cross_scale=(
                float(rng.uniform(0.30, 0.44) * WOOD_UV_DENSITY),
                float(rng.uniform(0.30, 0.46) * WOOD_UV_DENSITY),
            ),
        )

    def radial_uv(self, uv: list[float]) -> list[float]:
        return [
            self.radial_offset[0] + uv[0] * self.radial_scale[0],
            self.radial_offset[1] + uv[1] * self.radial_scale[1],
        ]

    def tangential_uv(self, uv: list[float]) -> list[float]:
        return [
            self.tangential_offset[0] + uv[0] * self.tangential_scale[0],
            self.tangential_offset[1] + uv[1] * self.tangential_scale[1],
        ]

    def cross_uv(self, uv: list[float]) -> list[float]:
        return [
            self.cross_offset[0] + uv[0] * self.cross_scale[0],
            self.cross_offset[1] + uv[1] * self.cross_scale[1],
        ]


class WoodProjection:
    def __init__(
        self,
        basis: Basis,
        points: np.ndarray,
        window: WoodWindow,
        axis_ranges: "WoodAxisRanges | None" = None,
    ) -> None:
        self.basis = basis
        self.window = window
        ranges = axis_ranges or WoodAxisRanges.from_points(basis, points)
        self.right = ranges.right
        self.up = ranges.up
        self.front = ranges.front

    def wood_class(self, normal: np.ndarray) -> str:
        right = abs(float(normal @ self.basis.right))
        up = abs(float(normal @ self.basis.up))
        front = abs(float(normal @ self.basis.front))
        if right > max(up, front):
            return "cross"
        if up > front:
            return "tangential"
        return "radial"

    def uv_for(self, point: np.ndarray, wood_class: str) -> list[float]:
        if wood_class == "cross":
            return self.window.cross_uv([
                self.front.normalize(float(point @ self.basis.front)),
                self.up.normalize(float(point @ self.basis.up)),
            ])
        if wood_class == "tangential":
            return self.window.tangential_uv([
                self.right.normalize(float(point @ self.basis.right)),
                self.front.normalize(float(point @ self.basis.front)),
            ])
        return self.window.radial_uv([
            self.right.normalize(float(point @ self.basis.right)),
            self.up.normalize(float(point @ self.basis.up)),
        ])


class AxisRange:
    def __init__(self, values: np.ndarray) -> None:
        self.min = float(values.min())
        self.max = float(values.max())

    def normalize(self, value: float) -> float:
        span = self.max - self.min
        if abs(span) < 1e-8:
            return 0.5
        return max(0.0, min(1.0, (value - self.min) / span))


@dataclass(frozen=True)
class WoodAxisRanges:
    right: AxisRange
    up: AxisRange
    front: AxisRange

    @classmethod
    def from_points(cls, basis: Basis, points: np.ndarray) -> "WoodAxisRanges":
        return cls(
            right=AxisRange(points @ basis.right),
            up=AxisRange(points @ basis.up),
            front=AxisRange(points @ basis.front),
        )


def append_triangle(
    primitive: BakedPrimitive,
    positions: np.ndarray,
    normals: np.ndarray,
    uvs: list[list[float]],
) -> None:
    start = len(primitive.positions)
    normals = np.array([normalized(normal) for normal in normals], dtype=np.float32)
    shading_normal = normalized(normals.mean(axis=0))
    if not np.any(shading_normal):
        shading_normal = normalized(np.cross(positions[1] - positions[0], positions[2] - positions[0]))
    tangent = triangle_tangent(positions, np.array(uvs, dtype=np.float32), shading_normal)
    for index in range(3):
        primitive.positions.append(positions[index].astype(float).tolist())
        primitive.normals.append(normals[index].astype(float).tolist())
        primitive.uvs.append([float(uvs[index][0]), float(uvs[index][1])])
        primitive.tangents.append(tangent)
        primitive.indices.append(start + index)


def triangle_tangent(positions: np.ndarray, uvs: np.ndarray, normal: np.ndarray) -> list[float]:
    edge1 = positions[1] - positions[0]
    edge2 = positions[2] - positions[0]
    duv1 = uvs[1] - uvs[0]
    duv2 = uvs[2] - uvs[0]
    denom = duv1[0] * duv2[1] - duv2[0] * duv1[1]
    if abs(float(denom)) < 1e-8:
        tangent = np.array([1.0, 0.0, 0.0], dtype=np.float32)
    else:
        tangent = (edge1 * duv2[1] - edge2 * duv1[1]) / denom
    tangent = normalized(tangent - normal * float(tangent @ normal))
    if not np.any(tangent):
        tangent = np.array([1.0, 0.0, 0.0], dtype=np.float32)
    bitangent = np.cross(normal, tangent)
    handedness = 1.0 if float(np.cross(normal, tangent) @ bitangent) >= 0.0 else -1.0
    return [float(tangent[0]), float(tangent[1]), float(tangent[2]), handedness]


def fallback_tangent(normal: np.ndarray) -> list[float]:
    normal = normalized(normal)
    tangent = np.array([1.0, 0.0, 0.0], dtype=np.float32)
    if abs(float(tangent @ normal)) > 0.9:
        tangent = np.array([0.0, 0.0, 1.0], dtype=np.float32)
    tangent = normalized(tangent - normal * float(tangent @ normal))
    if not np.any(tangent):
        tangent = np.array([1.0, 0.0, 0.0], dtype=np.float32)
    return [float(tangent[0]), float(tangent[1]), float(tangent[2]), 1.0]


def generated_vertex_normals(positions: np.ndarray, indices: list[int]) -> np.ndarray:
    normals = np.zeros_like(positions, dtype=np.float32)
    for a, b, c in batched(indices, 3):
        face_normal = normalized(np.cross(positions[b] - positions[a], positions[c] - positions[a]))
        if not np.any(face_normal):
            continue
        normals[a] += face_normal
        normals[b] += face_normal
        normals[c] += face_normal
    for index, normal in enumerate(normals):
        normalized_normal = normalized(normal)
        normals[index] = normalized_normal if np.any(normalized_normal) else np.array([0.0, 1.0, 0.0])
    return normals


def material_class_for_mesh(mesh_index: int) -> str | None:
    if mesh_index in BRASS_MESHES:
        return "aged_brass"
    if mesh_index in STEEL_MESHES:
        return "polished_steel"
    if mesh_index in DARK_METAL_MESHES:
        return "dark_metal"
    return None


def is_wood_mesh(mesh_index: int, mesh_name: str | None) -> bool:
    return mesh_index in WOOD_MESHES or mesh_name in WOOD_MESH_NAMES


def include_alignment_mesh(mesh_index: int, mesh_name: str | None) -> bool:
    return not is_wood_mesh(mesh_index, mesh_name)


def accessor_item_size(accessor: Accessor) -> int:
    component_size = {FLOAT: 4, UNSIGNED_INT: 4, 5123: 2}[accessor.componentType]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[accessor.type]
    return component_size * width


def normalized(vector: np.ndarray) -> np.ndarray:
    length = float(np.linalg.norm(vector))
    if length < 1e-12:
        return np.zeros_like(vector, dtype=np.float32)
    return (vector / length).astype(np.float32)


def transform_points(matrix: np.ndarray, points: np.ndarray) -> np.ndarray:
    padded = np.concatenate([points, np.ones((len(points), 1), dtype=np.float32)], axis=1)
    return (padded @ matrix.T)[:, :3]


def transform_vectors(matrix: np.ndarray, vectors: np.ndarray) -> np.ndarray:
    padded = np.concatenate([vectors, np.zeros((len(vectors), 1), dtype=np.float32)], axis=1)
    transformed = (padded @ matrix.T)[:, :3]
    lengths = np.linalg.norm(transformed, axis=1, keepdims=True)
    return np.divide(transformed, np.maximum(lengths, 1e-8)).astype(np.float32)


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


def translate_node(node, delta: np.ndarray) -> None:
    if node.matrix:
        matrix = np.array(node.matrix, dtype=np.float32).reshape(4, 4).T
        matrix[:3, 3] += delta.astype(np.float32)
        node.matrix = matrix.T.reshape(-1).astype(float).tolist()
        return
    translation = np.array(node.translation or [0.0, 0.0, 0.0], dtype=np.float32)
    node.translation = (translation + delta.astype(np.float32)).astype(float).tolist()


def gltf_world_bounds(
    gltf: GLTF2,
    blob: bytes | bytearray,
    *,
    include_mesh,
) -> tuple[np.ndarray, np.ndarray]:
    points: list[np.ndarray] = []
    for node in gltf.nodes:
        if node.mesh is None:
            continue
        mesh = gltf.meshes[node.mesh]
        if not include_mesh(node.mesh, mesh.name):
            continue
        transform = node_matrix(node)
        for primitive in mesh.primitives:
            positions = gltf_accessor_vec(gltf, blob, primitive.attributes.POSITION, 3)
            points.append(transform_points(transform, positions))
    if not points:
        raise ValueError("no mesh points found for GLB alignment")
    merged = np.concatenate(points, axis=0)
    return merged.min(axis=0), merged.max(axis=0)


def gltf_accessor_vec(
    gltf: GLTF2,
    blob: bytes | bytearray,
    accessor_index: int,
    width: int,
) -> np.ndarray:
    accessor = gltf.accessors[accessor_index]
    if accessor.componentType != FLOAT or accessor.type != f"VEC{width}":
        raise ValueError(f"unsupported accessor {accessor_index}: {accessor}")
    view = gltf.bufferViews[accessor.bufferView]
    offset = (view.byteOffset or 0) + (accessor.byteOffset or 0)
    item_size = accessor_item_size(accessor)
    if view.byteStride:
        compact = bytearray()
        for index in range(accessor.count):
            start = offset + index * view.byteStride
            compact.extend(blob[start : start + item_size])
        data = bytes(compact)
    else:
        length = accessor.count * item_size
        data = bytes(blob[offset : offset + length])
    return np.frombuffer(data, dtype=np.float32).reshape(accessor.count, width).copy()


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


def relative_uri(from_dir: Path, target: Path) -> str:
    return Path(os.path.relpath(target.resolve(), from_dir.resolve())).as_posix()


def batched(values: Iterable[int], size: int) -> Iterable[tuple[int, ...]]:
    batch = []
    for value in values:
        batch.append(value)
        if len(batch) == size:
            yield tuple(batch)
            batch.clear()


if __name__ == "__main__":
    main()
