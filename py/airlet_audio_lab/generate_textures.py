from __future__ import annotations

import argparse
from pathlib import Path

import matplotlib.image as mpimg
import numpy as np

from airlet_audio_lab.wood_preset_gallery import (
    PRESETS,
    _render_preset,
    _wood_volume_for,
)


DEFAULT_OUT_DIR = Path("assets/textures/procedural")
TEXTURE_SIZE = 512
PRODUCTION_WOOD_PRESET = "walnut_long_oil"


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate Airlet procedural PBR textures.")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--size", type=int, default=TEXTURE_SIZE)
    args = parser.parse_args()

    args.out_dir.mkdir(parents=True, exist_ok=True)
    rng = np.random.default_rng(42)
    recipes = {
        "aged_brass": _metal(args.size, rng, base=(0.95, 0.62, 0.18), roughness=0.24),
        "polished_steel": _metal(args.size, rng, base=(0.66, 0.68, 0.66), roughness=0.18),
        "dark_metal": _metal(args.size, rng, base=(0.12, 0.12, 0.11), roughness=0.52),
        "dark_stage": _stage(args.size, rng),
    }
    recipes.update(_production_wood(args.size))
    for name, maps in recipes.items():
        for suffix, image in maps.items():
            path = args.out_dir / f"{name}_{suffix}.png"
            mpimg.imsave(path, np.clip(image, 0.0, 1.0))
            print(f"wrote {path}")


def _production_wood(size: int) -> dict[str, dict[str, np.ndarray]]:
    preset_index, preset = next(
        (index, item) for index, item in enumerate(PRESETS) if item.name == PRODUCTION_WOOD_PRESET
    )
    rng = np.random.default_rng(20260702 + preset_index * 131)
    volume = _wood_volume_for(preset_index, preset)
    maps = _render_preset(size, preset, volume, rng)
    result = {
        "walnut_wood_radial": _wood_maps(maps["base"], maps["normal"], maps["roughness"]),
        "walnut_wood_tangential": _wood_maps(
            maps["tangential_base"],
            maps["tangential_normal"],
            maps["tangential_roughness"],
        ),
        "walnut_wood_cross": _wood_maps(
            maps["crosscut_base"],
            maps["crosscut_normal"],
            maps["crosscut_roughness"],
        ),
    }
    # Compatibility aliases for older scripts and stale baked GLBs.
    result["lacquered_wood"] = result["walnut_wood_radial"]
    result["lacquered_wood_end"] = result["walnut_wood_cross"]
    return result


def _wood_maps(base: np.ndarray, normal: np.ndarray, roughness: np.ndarray) -> dict[str, np.ndarray]:
    return {
        "base": _rgb(base[..., :3]),
        "orm": _orm(roughness, metallic=0.0),
        "normal": _rgb(normal[..., :3]),
    }


def _wood_long_grain(size: int, rng: np.random.Generator) -> dict[str, np.ndarray]:
    x, y = _grid(size)
    longitudinal_drift = _fbm(x * 0.72, y * 0.08, rng, 5)
    board_tone = _fbm(x * 0.48, y * 0.055, rng, 5)
    amber_band = _fbm(x * 0.34, y * 0.040, rng, 5)
    color_cloud = _fbm(x * 0.82, y * 0.12, rng, 5)
    latewood = _fbm(x * 1.25, y * 0.10, rng, 5) ** 2.2
    soft_band = _fbm(x * 0.68, y * 0.105, rng, 5)

    fine_fibers = _anisotropic_fiber_noise(x, y, rng, count=220, width=(0.00075, 0.0025), length=(0.045, 0.18))
    hair_fibers = _anisotropic_fiber_noise(x, y, rng, count=520, width=(0.00025, 0.00095), length=(0.018, 0.075))
    micro_fibers = _short_fiber_weave(x, y, rng, count=8200)
    dark_pores = _anisotropic_fiber_noise(x, y, rng, count=90, width=(0.0016, 0.0065), length=(0.035, 0.140))
    pore_noise = _fbm(x * 105.0, y * 18.0, rng, 3)
    dark_pores *= 0.40 + pore_noise * 0.95

    impurities = _elongated_impurities(x, y, rng, count=48)
    bubble_clouds = _elliptical_clouds(x, y, rng, count=22)
    aged_cracks = _aged_longitudinal_cracks(x, y, rng, count=20)

    value = 0.39 + soft_band * 0.026 + latewood * 0.018 + longitudinal_drift * 0.020
    value += (color_cloud - 0.5) * 0.060 + (board_tone - 0.5) * 0.075
    value += fine_fibers * 0.030 + hair_fibers * 0.026 + micro_fibers * 0.014
    value -= dark_pores * 0.105
    value -= impurities * 0.205
    value -= aged_cracks * 0.145
    value += bubble_clouds * 0.070
    base_dark = np.array([0.19, 0.070, 0.032])
    base_mid = np.array([0.46, 0.175, 0.075])
    base_light = np.array([0.66, 0.320, 0.155])
    base_honey = np.array([0.86, 0.500, 0.185])
    base = _mix(base_dark, base_mid, np.clip(value[..., None], 0.0, 1.0))
    base = _mix(base, base_light, np.clip((value[..., None] - 0.55) * 0.95, 0.0, 0.22))
    amber_mix = np.clip((amber_band[..., None] - 0.32) * 0.55, 0.0, 0.30)
    base = _mix(base, base_honey, amber_mix)
    base *= 1.0 - dark_pores[..., None] * 0.115
    base *= 1.0 - aged_cracks[..., None] * 0.155
    warm_lacquer = np.array([1.08, 0.96, 0.86])
    base *= warm_lacquer
    roughness = (
        np.full((size, size), 0.43)
        + (1.0 - latewood) * 0.08
        + pore_noise * 0.055
        + impurities * 0.105
        + bubble_clouds * 0.045
        + aged_cracks * 0.130
    )
    height = (
        latewood * 0.010
        + fine_fibers * 0.014
        + hair_fibers * 0.052
        + micro_fibers * 0.210
        - dark_pores * 0.044
        + pore_noise * 0.010
        - impurities * 0.045
        - aged_cracks * 0.092
        + bubble_clouds * 0.020
    )
    return {
        "base": _rgb(base),
        "orm": _orm(roughness, metallic=0.0),
        "normal": _normal_from_height(height, strength=6.2),
    }


def _wood_end_grain(size: int, rng: np.random.Generator) -> dict[str, np.ndarray]:
    x, y = _grid(size)
    cx = x + 2.65
    cy = y - 0.18
    angle = np.arctan2(cy, cx)
    radius = np.sqrt((cx * 1.0) ** 2 + (cy * 1.0) ** 2)
    warp = _fbm(x * 3.2, y * 3.2, rng, 5)
    radial_wobble = 0.006 * np.sin(angle * 4.0 + warp * 2.0)
    local_radius = radius + radial_wobble + warp * 0.010
    rings = np.sin(local_radius * 9.2 * np.pi) * 0.5 + 0.5
    rings = rings**3.2
    latewood = np.sin(local_radius * 4.8 * np.pi + warp * 0.65) * 0.5 + 0.5
    pores = _fbm(x * 92.0, y * 92.0, rng, 4) ** 3.8
    rays = np.maximum(0.0, np.cos(angle * 40.0 + warp * 2.2)) ** 24.0
    subtle_arc_shadow = np.sin((local_radius * 2.0 + warp * 0.25) * np.pi) * 0.5 + 0.5
    value = 0.29 + latewood * 0.15 + rings * 0.20 + rays * 0.055
    value += subtle_arc_shadow * 0.045
    value -= pores * 0.13
    base_dark = np.array([0.15, 0.042, 0.020])
    base_mid = np.array([0.39, 0.128, 0.055])
    base_light = np.array([0.60, 0.245, 0.110])
    base = _mix(base_dark, base_mid, np.clip(value[..., None], 0.0, 1.0))
    base = _mix(base, base_light, np.clip((value[..., None] - 0.50) * 1.15, 0.0, 0.38))
    roughness = np.full((size, size), 0.56) + pores * 0.12 + (1.0 - latewood) * 0.08
    height = rings * 0.09 + rays * 0.035 - pores * 0.07 + warp * 0.025
    return {
        "base": _rgb(base),
        "orm": _orm(roughness, metallic=0.0),
        "normal": _normal_from_height(height, strength=4.4),
    }


def _ring_grain(
    x: np.ndarray,
    y: np.ndarray,
    *,
    center: tuple[float, float],
    scale: tuple[float, float],
    frequency: float,
) -> np.ndarray:
    sx, sy = scale
    radius = np.sqrt(((x - center[0]) * sx) ** 2 + ((y - center[1]) * sy) ** 2)
    rings = np.sin(radius * frequency * np.pi) * 0.5 + 0.5
    return rings


def _elongated_impurities(
    x: np.ndarray, y: np.ndarray, rng: np.random.Generator, *, count: int
) -> np.ndarray:
    field = np.zeros_like(x)
    for _ in range(count):
        cx = rng.uniform(-0.10, 1.10)
        cy = rng.uniform(0.02, 0.98)
        length = rng.uniform(0.070, 0.24)
        width = rng.uniform(0.0045, 0.0140)
        angle = rng.normal(0.0, 0.020)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        streak = np.exp(-((along / length) ** 2 + (across / width) ** 2))
        streak *= rng.uniform(0.35, 1.0)
        field = np.maximum(field, streak)
    return np.clip(field, 0.0, 1.0)


def _anisotropic_fiber_noise(
    x: np.ndarray,
    y: np.ndarray,
    rng: np.random.Generator,
    *,
    count: int,
    width: tuple[float, float],
    length: tuple[float, float],
) -> np.ndarray:
    field = np.zeros_like(x)
    for _ in range(count):
        cx = rng.uniform(-0.25, 1.25)
        cy = rng.uniform(-0.04, 1.04)
        long_axis = rng.uniform(length[0], length[1])
        short_axis = rng.uniform(width[0], width[1])
        angle = rng.normal(0.0, 0.012)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        strand = np.exp(-((along / long_axis) ** 2 + (across / short_axis) ** 2))
        strand *= rng.uniform(0.25, 1.0)
        field = np.maximum(field, strand)
    return np.clip(field, 0.0, 1.0)


def _short_fiber_weave(
    x: np.ndarray, y: np.ndarray, rng: np.random.Generator, *, count: int
) -> np.ndarray:
    field = np.zeros_like(x)
    for _ in range(count):
        cx = rng.uniform(-0.035, 1.035)
        cy = rng.uniform(-0.015, 1.015)
        long_axis = rng.uniform(0.0035, 0.024)
        short_axis = rng.uniform(0.00012, 0.00062)
        angle = rng.normal(0.0, 0.070)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        core = np.exp(-((along / long_axis) ** 2 + (across / short_axis) ** 2))
        shoulder = np.exp(-((along / (long_axis * 1.22)) ** 2 + (across / (short_axis * 2.3)) ** 2))
        strand = core - shoulder * rng.uniform(0.05, 0.20)
        strand *= rng.uniform(0.08, 0.46)
        field += strand
    return np.clip(field * 0.42, 0.0, 1.0)


def _aged_longitudinal_cracks(
    x: np.ndarray, y: np.ndarray, rng: np.random.Generator, *, count: int
) -> np.ndarray:
    field = np.zeros_like(x)
    for _ in range(count):
        cx = rng.uniform(-0.12, 1.12)
        cy = rng.uniform(0.04, 0.96)
        long_axis = rng.uniform(0.12, 0.46)
        short_axis = rng.uniform(0.0018, 0.0068)
        angle = rng.normal(0.0, 0.018)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        split = np.exp(-((along / long_axis) ** 2 + (across / short_axis) ** 2))
        broken_edges = _fbm((x + cx) * 34.0, (y + cy) * 9.0, rng, 3)
        split *= np.clip(0.45 + broken_edges * 0.90, 0.0, 1.0)
        split *= rng.uniform(0.24, 0.80)
        field = np.maximum(field, split)
    return np.clip(field, 0.0, 1.0)


def _elliptical_clouds(
    x: np.ndarray, y: np.ndarray, rng: np.random.Generator, *, count: int
) -> np.ndarray:
    field = np.zeros_like(x)
    for _ in range(count):
        cx = rng.uniform(-0.05, 1.05)
        cy = rng.uniform(0.05, 0.95)
        long_axis = rng.uniform(0.075, 0.26)
        short_axis = rng.uniform(0.020, 0.070)
        angle = rng.normal(0.0, 0.05)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        cloud = np.exp(-((along / long_axis) ** 2 + (across / short_axis) ** 2))
        cloud *= rng.uniform(0.20, 0.75)
        field += cloud
    return np.clip(field, 0.0, 1.0)


def _metal(
    size: int,
    rng: np.random.Generator,
    *,
    base: tuple[float, float, float],
    roughness: float,
) -> dict[str, np.ndarray]:
    x, y = _grid(size)
    scratches = np.maximum(
        0.0,
        1.0
        - np.abs(np.sin((x * 85.0 + _fbm(x * 7.0, y * 4.0, rng, 3) * 2.0) * np.pi)),
    )
    scratches = scratches**8.0
    cloudy = _fbm(x * 7.0, y * 7.0, rng, 5)
    anisotropic = _fbm(x * 38.0, y * 3.5, rng, 4)
    color = np.array(base) * (0.82 + cloudy[..., None] * 0.28)
    color += scratches[..., None] * 0.10
    roughness_map = roughness + (cloudy - 0.5) * 0.16 + scratches * 0.18
    height = scratches * 0.10 + anisotropic * 0.045 + cloudy * 0.035
    return {
        "base": _rgb(color),
        "orm": _orm(roughness_map, metallic=1.0),
        "normal": _normal_from_height(height, strength=1.7),
    }


def _stage(size: int, rng: np.random.Generator) -> dict[str, np.ndarray]:
    x, y = _grid(size)
    noise = _fbm(x * 10.0, y * 10.0, rng, 5)
    fine = _fbm(x * 48.0, y * 48.0, rng, 3)
    base = np.array([0.20, 0.19, 0.17]) * (0.78 + noise[..., None] * 0.26)
    roughness = 0.82 + fine * 0.10
    height = noise * 0.05 + fine * 0.025
    return {
        "base": _rgb(base),
        "orm": _orm(roughness, metallic=0.0),
        "normal": _normal_from_height(height, strength=1.2),
    }


def _grid(size: int) -> tuple[np.ndarray, np.ndarray]:
    axis = np.linspace(0.0, 1.0, size, endpoint=False)
    return np.meshgrid(axis, axis)


def _fbm(
    x: np.ndarray, y: np.ndarray, rng: np.random.Generator, octaves: int
) -> np.ndarray:
    value = np.zeros_like(x)
    amplitude = 0.5
    frequency = 1.0
    for _ in range(octaves):
        phase = rng.uniform(0.0, np.pi * 2.0, size=4)
        value += amplitude * (
            np.sin(x * frequency + phase[0])
            * np.cos(y * frequency * 1.27 + phase[1])
            + np.sin((x + y) * frequency * 0.71 + phase[2])
            + np.cos((x - y) * frequency * 1.43 + phase[3])
        )
        amplitude *= 0.5
        frequency *= 2.0
    value -= value.min()
    peak = value.max()
    return value / peak if peak > 0.0 else value


def _mix(a: np.ndarray, b: np.ndarray, t: np.ndarray) -> np.ndarray:
    return a * (1.0 - t) + b * t


def _rgb(image: np.ndarray) -> np.ndarray:
    alpha = np.ones((*image.shape[:2], 1), dtype=np.float32)
    return np.concatenate([image.astype(np.float32), alpha], axis=-1)


def _orm(roughness: np.ndarray, metallic: float) -> np.ndarray:
    height, width = roughness.shape
    image = np.ones((height, width, 4), dtype=np.float32)
    image[..., 1] = np.clip(roughness, 0.02, 1.0)
    image[..., 2] = metallic
    return image


def _normal_from_height(height: np.ndarray, strength: float) -> np.ndarray:
    dy, dx = np.gradient(height)
    normal = np.dstack((-dx * strength, -dy * strength, np.ones_like(height)))
    normal /= np.linalg.norm(normal, axis=-1, keepdims=True).clip(min=1e-6)
    normal = normal * 0.5 + 0.5
    return _rgb(normal)


if __name__ == "__main__":
    main()
