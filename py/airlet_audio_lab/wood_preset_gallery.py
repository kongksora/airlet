from __future__ import annotations

import argparse
import json
import shutil
from dataclasses import asdict, dataclass
from pathlib import Path

import matplotlib.image as mpimg
import matplotlib.pyplot as plt
import numpy as np
from scipy.ndimage import gaussian_filter, map_coordinates


DEFAULT_OUT_DIR = Path("target/wood-presets")
TEXTURE_SIZE = 768
ANNUAL_RING_SCALE = 4.0


@dataclass(frozen=True)
class LongitudinalPreset:
    name: str
    description: str
    dark: tuple[float, float, float]
    mid: tuple[float, float, float]
    light: tuple[float, float, float]
    honey: tuple[float, float, float]
    board_contrast: float
    fiber_contrast: float
    pore_strength: float
    silk_strength: float
    crack_strength: float
    roughness: float
    gloss: float


@dataclass(frozen=True)
class WoodVolume:
    center_radial: float
    center_tangent: float
    radius_scale: float
    ring_frequency: float
    ring_phase: float
    ring_jitter: float
    window_radial: tuple[float, float]
    window_tangent: tuple[float, float]


@dataclass(frozen=True)
class LongitudinalRender:
    base: np.ndarray
    height: np.ndarray
    normal: np.ndarray
    roughness: np.ndarray
    preview: np.ndarray


@dataclass(frozen=True)
class CrosscutRender:
    base: np.ndarray
    height: np.ndarray
    normal: np.ndarray
    roughness: np.ndarray
    preview: np.ndarray


@dataclass(frozen=True)
class LongitudinalSlice:
    length: np.ndarray
    radial: np.ndarray
    tangent: np.ndarray
    radius: np.ndarray
    rings: np.ndarray
    annual_color: np.ndarray
    fiber_flow_pixels: np.ndarray


@dataclass(frozen=True)
class AnnualRingColorProfile:
    knots: np.ndarray
    values: np.ndarray


PRESETS = [
    LongitudinalPreset(
        name="light_oak_reference",
        description="Reference-aligned pale yellow longitudinal wood with dense discontinuous fibers.",
        dark=(0.52, 0.330, 0.150),
        mid=(0.82, 0.620, 0.340),
        light=(1.00, 0.840, 0.570),
        honey=(1.00, 0.905, 0.650),
        board_contrast=0.30,
        fiber_contrast=0.50,
        pore_strength=0.22,
        silk_strength=0.20,
        crack_strength=0.10,
        roughness=0.58,
        gloss=0.22,
    ),
    LongitudinalPreset(
        name="mahogany_long_lacquer",
        description="Warm red-brown longitudinal mahogany with subtle ribbon grain and polished lacquer.",
        dark=(0.15, 0.044, 0.020),
        mid=(0.47, 0.150, 0.060),
        light=(0.76, 0.325, 0.135),
        honey=(0.92, 0.530, 0.200),
        board_contrast=0.40,
        fiber_contrast=0.52,
        pore_strength=0.18,
        silk_strength=0.42,
        crack_strength=0.10,
        roughness=0.38,
        gloss=0.72,
    ),
    LongitudinalPreset(
        name="walnut_long_oil",
        description="Dark oiled walnut with smoky longitudinal clouds and fine open pores.",
        dark=(0.070, 0.034, 0.020),
        mid=(0.270, 0.135, 0.065),
        light=(0.510, 0.305, 0.145),
        honey=(0.690, 0.455, 0.230),
        board_contrast=0.58,
        fiber_contrast=0.48,
        pore_strength=0.30,
        silk_strength=0.26,
        crack_strength=0.12,
        roughness=0.47,
        gloss=0.45,
    ),
    LongitudinalPreset(
        name="cherry_long_aged",
        description="Aged cherry long grain with orange-red warmth and restrained fine pores.",
        dark=(0.17, 0.058, 0.026),
        mid=(0.56, 0.205, 0.072),
        light=(0.88, 0.430, 0.165),
        honey=(1.00, 0.610, 0.255),
        board_contrast=0.34,
        fiber_contrast=0.42,
        pore_strength=0.14,
        silk_strength=0.35,
        crack_strength=0.08,
        roughness=0.43,
        gloss=0.55,
    ),
    LongitudinalPreset(
        name="oak_long_porcellous",
        description="Amber oak longitudinal surface with porous vessels and subdued medullary shimmer.",
        dark=(0.24, 0.120, 0.045),
        mid=(0.62, 0.375, 0.130),
        light=(0.92, 0.690, 0.320),
        honey=(1.00, 0.840, 0.470),
        board_contrast=0.46,
        fiber_contrast=0.34,
        pore_strength=0.52,
        silk_strength=0.22,
        crack_strength=0.08,
        roughness=0.57,
        gloss=0.26,
    ),
    LongitudinalPreset(
        name="maple_long_satin",
        description="Light satin maple with dense pale longitudinal fibers and minimal pore contrast.",
        dark=(0.43, 0.280, 0.135),
        mid=(0.78, 0.600, 0.330),
        light=(1.00, 0.875, 0.580),
        honey=(1.00, 0.940, 0.700),
        board_contrast=0.22,
        fiber_contrast=0.36,
        pore_strength=0.06,
        silk_strength=0.34,
        crack_strength=0.03,
        roughness=0.50,
        gloss=0.36,
    ),
    LongitudinalPreset(
        name="rosewood_long_gloss",
        description="Glossy rosewood with dark red longitudinal bands and deep fine vessels.",
        dark=(0.050, 0.018, 0.014),
        mid=(0.310, 0.070, 0.052),
        light=(0.700, 0.210, 0.120),
        honey=(0.960, 0.430, 0.210),
        board_contrast=0.72,
        fiber_contrast=0.58,
        pore_strength=0.34,
        silk_strength=0.46,
        crack_strength=0.18,
        roughness=0.34,
        gloss=0.84,
    ),
    LongitudinalPreset(
        name="teak_long_worn",
        description="Worn teak with yellow-brown long grain, oily warmth, and light longitudinal checking.",
        dark=(0.22, 0.115, 0.040),
        mid=(0.58, 0.360, 0.135),
        light=(0.87, 0.640, 0.285),
        honey=(1.00, 0.780, 0.380),
        board_contrast=0.50,
        fiber_contrast=0.46,
        pore_strength=0.26,
        silk_strength=0.24,
        crack_strength=0.20,
        roughness=0.62,
        gloss=0.25,
    ),
]


def main() -> None:
    parser = argparse.ArgumentParser(description="Generate longitudinal-cut wood preset images.")
    parser.add_argument("--out-dir", type=Path, default=DEFAULT_OUT_DIR)
    parser.add_argument("--size", type=int, default=TEXTURE_SIZE)
    args = parser.parse_args()

    if args.out_dir.exists():
        shutil.rmtree(args.out_dir)
    args.out_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    previews = []
    for index, preset in enumerate(PRESETS):
        rng = np.random.default_rng(20260702 + index * 131)
        volume = _wood_volume_for(index, preset)
        maps = _render_preset(args.size, preset, volume, rng)
        preset_dir = args.out_dir / preset.name
        preset_dir.mkdir(parents=True, exist_ok=True)
        for suffix, image in maps.items():
            output = image
            if suffix.endswith("height") or suffix.endswith("roughness"):
                output = np.repeat(image[..., None], 3, axis=-1)
            mpimg.imsave(preset_dir / f"{preset.name}_{suffix}.png", np.clip(output, 0.0, 1.0))
        previews.append((preset, maps["preview"], maps["tangential_preview"], maps["crosscut"]))
        manifest.append(
            {
                **asdict(preset),
                "files": {
                    key: str(preset_dir / f"{preset.name}_{key}.png")
                    for key in [
                        "base",
                        "height",
                        "normal",
                        "roughness",
                        "preview",
                        "tangential_base",
                        "tangential_height",
                        "tangential_normal",
                        "tangential_roughness",
                        "tangential_preview",
                        "crosscut_base",
                        "crosscut",
                        "crosscut_height",
                        "crosscut_normal",
                        "crosscut_roughness",
                    ]
                },
                "wood_volume": asdict(volume),
            }
        )
        print(f"wrote {preset_dir}")

    _write_contact_sheet(args.out_dir / "wood_preset_contact_sheet.png", previews)
    (args.out_dir / "wood_preset_manifest.json").write_text(
        json.dumps(manifest, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
    )
    _write_markdown(args.out_dir / "wood_preset_manifest.md", manifest)
    print(f"wrote {args.out_dir / 'wood_preset_contact_sheet.png'}")


def _render_preset(
    size: int, preset: LongitudinalPreset, volume: WoodVolume, rng: np.random.Generator
) -> dict[str, np.ndarray]:
    ring_profile = _annual_ring_color_profile(volume, rng)
    radial = _render_longitudinal(size, preset, volume, rng, ring_profile, mode="radial")
    tangential = _render_longitudinal(size, preset, volume, rng, ring_profile, mode="tangential")
    crosscut = _render_crosscut(size, preset, volume, rng, ring_profile)
    return {
        "base": _rgba(radial.base),
        "height": _normalize(radial.height),
        "normal": radial.normal,
        "roughness": radial.roughness,
        "preview": _rgba(radial.preview),
        "tangential_base": _rgba(tangential.base),
        "tangential_height": _normalize(tangential.height),
        "tangential_normal": tangential.normal,
        "tangential_roughness": tangential.roughness,
        "tangential_preview": _rgba(tangential.preview),
        "crosscut_base": _rgba(crosscut.base),
        "crosscut": _rgba(crosscut.preview),
        "crosscut_height": _normalize(crosscut.height),
        "crosscut_normal": crosscut.normal,
        "crosscut_roughness": crosscut.roughness,
    }


def _render_longitudinal(
    size: int,
    preset: LongitudinalPreset,
    volume: WoodVolume,
    rng: np.random.Generator,
    ring_profile: AnnualRingColorProfile,
    *,
    mode: str,
) -> LongitudinalRender:
    x, y = _grid(size)
    wood_slice = _longitudinal_slice_coordinates(x, y, volume, rng, ring_profile, mode=mode)
    ring_long = wood_slice.rings
    annual_color = wood_slice.annual_color
    board = _longitudinal_noise(size, rng, y_sigma=34.0, x_sigma=190.0)
    ribbon = _longitudinal_noise(size, rng, y_sigma=9.0, x_sigma=155.0)
    fiber_bundles = _longitudinal_noise(size, rng, y_sigma=2.4, x_sigma=78.0)
    fine_fibers = _longitudinal_noise(size, rng, y_sigma=0.48, x_sigma=42.0)
    hair_fibers = _longitudinal_noise(size, rng, y_sigma=0.22, x_sigma=22.0)
    flow = _flow_displacement(size, rng, amount=14.0) + wood_slice.fiber_flow_pixels
    ribbon = _warp_field(ribbon, flow * 0.65)
    fiber_bundles = _warp_field(fiber_bundles, flow)
    fine_fibers = _warp_field(fine_fibers, flow * 1.25)
    hair_fibers = _warp_field(hair_fibers, flow * 1.35)
    local_fibers = _elongated_impulse_field(size, rng, density=0.00065, y_sigma=0.28, x_sigma=64.0)
    bright_threads = _elongated_impulse_field(size, rng, density=0.00045, y_sigma=0.20, x_sigma=42.0)
    local_fibers = _warp_field(local_fibers, flow * 1.10)
    bright_threads = _warp_field(bright_threads, flow * 1.20)
    dark_streaks = _finite_longitudinal_streaks(
        size,
        rng,
        count=int(size * (0.42 + preset.fiber_contrast * 0.92)),
        length=(0.055, 0.42),
        width=(0.24, 0.78),
        strength=(0.12, 0.58),
    )
    mid_streaks = _finite_longitudinal_streaks(
        size,
        rng,
        count=int(size * (0.28 + preset.fiber_contrast * 0.62)),
        length=(0.10, 0.58),
        width=(0.55, 1.35),
        strength=(0.08, 0.36),
    )
    pale_threads = _finite_longitudinal_streaks(
        size,
        rng,
        count=int(size * (0.22 + preset.silk_strength * 0.80)),
        length=(0.06, 0.34),
        width=(0.35, 0.95),
        strength=(0.12, 0.45),
    )
    dark_streaks = _warp_field(dark_streaks, flow * 0.70)
    mid_streaks = _warp_field(mid_streaks, flow * 0.80)
    pale_threads = _warp_field(pale_threads, flow * 0.75)
    silk = _silk_ribbon(size, rng)
    pores = _longitudinal_pores(size, rng, preset.pore_strength)
    checks = _longitudinal_checks(size, rng, preset.crack_strength)

    slow_value = 0.47 + annual_color * (0.52 + preset.board_contrast * 0.30)
    slow_value += (ring_long - 0.5) * preset.board_contrast * 0.24
    slow_value += (board - 0.5) * preset.board_contrast * 0.24
    slow_value += (ribbon - 0.5) * preset.board_contrast * 0.24
    fiber_value = (fiber_bundles - 0.5) * 0.22 + (fine_fibers - 0.5) * 0.17
    fiber_value += (hair_fibers - 0.5) * 0.10
    fiber_value += local_fibers * 0.30 + bright_threads * 0.22 + mid_streaks * 0.12
    fiber_value += pale_threads * 0.10
    value = slow_value + fiber_value * preset.fiber_contrast + silk * preset.silk_strength * 0.10
    value -= pores * (0.16 + preset.pore_strength * 0.20)
    value -= checks * (0.20 + preset.crack_strength * 0.16)
    value -= dark_streaks * (0.16 + preset.fiber_contrast * 0.12)
    value = np.clip(value, 0.0, 1.0)

    base = _palette(value, preset)
    honey_mix = np.clip((annual_color[..., None] + 0.30) * 0.25 + (board[..., None] - 0.62) * 0.16, 0.0, 0.30)
    base = _mix(base, np.array(preset.honey), honey_mix)
    base += np.clip(annual_color[..., None], 0.0, 1.0) * np.array([0.078, 0.040, 0.012])
    base -= np.clip(-annual_color[..., None], 0.0, 1.0) * np.array([0.062, 0.056, 0.048])
    base *= 1.0 - pores[..., None] * (0.12 + preset.pore_strength * 0.18)
    base *= 1.0 - checks[..., None] * 0.18
    base *= 1.0 - dark_streaks[..., None] * (0.12 + preset.fiber_contrast * 0.12)
    base *= 1.0 - mid_streaks[..., None] * 0.055
    base += silk[..., None] * preset.silk_strength * np.array([0.030, 0.024, 0.014])
    base += bright_threads[..., None] * preset.fiber_contrast * np.array([0.055, 0.040, 0.020])
    base += pale_threads[..., None] * preset.fiber_contrast * np.array([0.075, 0.055, 0.026])
    base = np.clip(base, 0.0, 1.0)

    roughness = preset.roughness + pores * 0.16 + checks * 0.18
    roughness -= silk * preset.gloss * 0.10
    roughness += (_longitudinal_noise(size, rng, y_sigma=1.1, x_sigma=26.0) - 0.5) * 0.045
    roughness = np.clip(roughness, 0.18, 0.88)

    height = (ring_long - 0.5) * 0.006 * preset.board_contrast
    height += annual_color * 0.010 * preset.board_contrast
    height += (fiber_bundles - 0.5) * 0.020
    height += (fine_fibers - 0.5) * 0.085 * preset.fiber_contrast
    height += (hair_fibers - 0.5) * 0.070 * preset.fiber_contrast
    height += local_fibers * 0.125 * preset.fiber_contrast
    height += bright_threads * 0.095 * preset.fiber_contrast
    height += pale_threads * 0.045 * preset.fiber_contrast
    height += silk * 0.028 * preset.silk_strength
    height -= dark_streaks * 0.070 * preset.fiber_contrast
    height -= mid_streaks * 0.026 * preset.fiber_contrast
    height -= pores * (0.080 + preset.pore_strength * 0.035)
    height -= checks * (0.085 + preset.crack_strength * 0.040)
    normal = _normal_from_height(height, strength=7.2)
    preview = _preview(base, normal[..., :3], roughness, preset.gloss)
    return LongitudinalRender(
        base=base,
        height=height,
        normal=normal,
        roughness=roughness,
        preview=preview,
    )


def _longitudinal_noise(
    size: int, rng: np.random.Generator, *, y_sigma: float, x_sigma: float
) -> np.ndarray:
    noise = rng.normal(0.0, 1.0, size=(size, size))
    field = gaussian_filter(noise, sigma=(y_sigma, x_sigma), mode="reflect")
    return _normalize(field)


def _fiber_curve_displacement(size: int, rng: np.random.Generator) -> np.ndarray:
    base = rng.normal(0.0, 1.0, size=(size, size))
    long_curve = gaussian_filter(base, sigma=(42.0, 180.0), mode="reflect")
    local_curve = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(12.0, 70.0), mode="reflect")
    return (_normalize(long_curve) - 0.5) * 1.35 + (_normalize(local_curve) - 0.5) * 0.48


def _wood_volume_for(index: int, preset: LongitudinalPreset) -> WoodVolume:
    ring_frequency = (3.15 + preset.board_contrast * 2.35) / ANNUAL_RING_SCALE
    radius_scale = 0.78 + preset.board_contrast * 0.30
    return WoodVolume(
        center_radial=-1.85 - index * 0.17,
        center_tangent=0.18 + ((index % 3) - 1) * 0.09,
        radius_scale=radius_scale,
        ring_frequency=ring_frequency,
        ring_phase=index * 0.37,
        ring_jitter=0.030 + preset.board_contrast * 0.035,
        window_radial=(0.12, 0.96),
        window_tangent=(-0.32, 0.32),
    )


def _annual_ring_color_profile(volume: WoodVolume, rng: np.random.Generator) -> AnnualRingColorProfile:
    min_pos = -4.0
    max_pos = 12.0
    knots = np.arange(min_pos, max_pos + 1.0)
    annual_steps = rng.normal(0.0, 0.125, size=knots.shape)
    cumulative = np.cumsum(annual_steps)
    cumulative -= np.mean(cumulative)
    cumulative = gaussian_filter(cumulative, sigma=1.35, mode="nearest")
    local = gaussian_filter(rng.normal(0.0, 0.12, size=knots.shape), sigma=0.90, mode="nearest")
    slow_arc = np.sin((knots * 0.22 + volume.ring_phase) * np.pi * 2.0) * 0.080
    values = cumulative * 1.45 + local * 0.55 + slow_arc
    values = np.clip(values, -0.72, 0.72)
    return AnnualRingColorProfile(knots=knots, values=values)


def _sample_annual_ring_profile(
    ring_position: np.ndarray, ring_profile: AnnualRingColorProfile
) -> np.ndarray:
    return np.interp(
        ring_position.ravel(),
        ring_profile.knots,
        ring_profile.values,
        left=float(ring_profile.values[0]),
        right=float(ring_profile.values[-1]),
    ).reshape(ring_position.shape)


def _crosscut_slice_coordinates(
    x: np.ndarray, y: np.ndarray, volume: WoodVolume, rng: np.random.Generator
) -> tuple[np.ndarray, np.ndarray]:
    centered_x = x - 0.5
    centered_y = y - 0.5
    angle = 0.18 + np.sin(volume.ring_phase * 1.7) * 0.10
    ca = np.cos(angle)
    sa = np.sin(angle)
    rotated_x = centered_x * ca - centered_y * sa
    rotated_y = centered_x * sa + centered_y * ca
    radial_span = (volume.window_radial[1] - volume.window_radial[0]) * 1.55
    tangent_span = (volume.window_tangent[1] - volume.window_tangent[0]) * 1.35
    radial_mid = sum(volume.window_radial) * 0.5 + 0.06 * np.sin(volume.ring_phase)
    tangent_mid = sum(volume.window_tangent) * 0.5 + 0.05 * np.cos(volume.ring_phase * 1.3)
    shear = 0.20 * np.sin(volume.ring_phase * 0.9 + 0.4)
    bow = (rotated_x**2 - 0.083) * (0.40 + volume.ring_jitter * 3.0)
    saddle = rotated_x * rotated_y * (0.34 + volume.ring_jitter * 2.0)
    distortion_a = gaussian_filter(rng.normal(0.0, 1.0, size=x.shape), sigma=(18.0, 34.0), mode="reflect")
    distortion_b = gaussian_filter(rng.normal(0.0, 1.0, size=x.shape), sigma=(8.0, 19.0), mode="reflect")
    distortion_a = _normalize(distortion_a) - 0.5
    distortion_b = _normalize(distortion_b) - 0.5
    drift_gain = 0.45 + np.clip(np.abs(rotated_y) * 1.55 + np.abs(rotated_x) * 0.55, 0.0, 1.1)
    radial = radial_mid + rotated_y * radial_span + rotated_x * radial_span * shear
    radial += bow * volume.ring_jitter * 7.5 + saddle * volume.ring_jitter * 4.5
    radial += (distortion_a * 0.13 + distortion_b * 0.055) * drift_gain
    tangent = tangent_mid + rotated_x * tangent_span + rotated_y * tangent_span * 0.18
    tangent += distortion_a * 0.055 + distortion_b * 0.040
    return radial, tangent


def _longitudinal_slice_coordinates(
    x: np.ndarray,
    y: np.ndarray,
    volume: WoodVolume,
    rng: np.random.Generator,
    ring_profile: AnnualRingColorProfile,
    *,
    mode: str,
) -> LongitudinalSlice:
    length = x
    cross = y - 0.5
    fiber_curve = _fiber_curve_displacement(x.shape[0], rng)
    length_curve = gaussian_filter(
        rng.normal(0.0, 1.0, size=x.shape), sigma=(56.0, 190.0), mode="reflect"
    )
    length_curve = _normalize(length_curve) - 0.5
    oblique = 0.10 * np.sin(volume.ring_phase * 1.4 + 0.2)
    if mode == "radial":
        radial_mid = sum(volume.window_radial) * 0.5
        radial_span = (volume.window_radial[1] - volume.window_radial[0]) * 1.08
        tangent_mid = sum(volume.window_tangent) * 0.5
        radial = radial_mid + cross * radial_span
        radial += (x - 0.5) * radial_span * oblique
        radial += fiber_curve * volume.ring_jitter * 1.20 + length_curve * volume.ring_jitter * 0.95
        tangent = tangent_mid + cross * 0.035 + fiber_curve * 0.024
    elif mode == "tangential":
        tangent_mid = sum(volume.window_tangent) * 0.5
        tangent_span = (volume.window_tangent[1] - volume.window_tangent[0]) * 1.12
        radial_mid = sum(volume.window_radial) * 0.5
        tangent = tangent_mid + cross * tangent_span
        tangent += (x - 0.5) * tangent_span * oblique * 0.85
        crown = ((x - 0.5) ** 2 - 0.083) * (0.16 + volume.ring_jitter * 1.3)
        lateral_roll = np.sin((x * 1.10 + cross * 0.55 + volume.ring_phase) * np.pi * 2.0) * 0.026
        radial = radial_mid + crown + lateral_roll
        radial += cross * volume.ring_jitter * 2.10
        radial += fiber_curve * volume.ring_jitter * 1.05 + length_curve * volume.ring_jitter * 1.15
    else:
        raise ValueError(f"unknown longitudinal cut mode: {mode}")
    radius = _wood_radius(radial, tangent, volume)
    rings = _ring_response(radius, volume)
    annual_color = _longitudinal_annual_color(radius, length, volume, rng, ring_profile)
    fiber_flow_pixels = (fiber_curve + length_curve * 0.65) * x.shape[0] * 0.018
    return LongitudinalSlice(
        length=length,
        radial=radial,
        tangent=tangent,
        radius=radius,
        rings=rings,
        annual_color=annual_color,
        fiber_flow_pixels=fiber_flow_pixels,
    )


def _longitudinal_annual_color(
    radius: np.ndarray,
    length: np.ndarray,
    volume: WoodVolume,
    rng: np.random.Generator,
    ring_profile: AnnualRingColorProfile,
) -> np.ndarray:
    ring_position = radius * volume.ring_frequency + volume.ring_phase / (np.pi * 2.0)
    smooth_ring = _sample_annual_ring_profile(ring_position, ring_profile)
    cycle = ring_position % 1.0
    annual_gradient = np.cos((cycle - 0.22) * np.pi * 2.0) * 0.065
    length_drift = gaussian_filter(
        rng.normal(0.0, 1.0, size=radius.shape), sigma=(30.0, 150.0), mode="reflect"
    )
    length_drift = (_normalize(length_drift) - 0.5) * 0.16
    slow_length = np.sin((length * 1.15 + volume.ring_phase * 0.21) * np.pi * 2.0) * 0.025
    return np.clip(smooth_ring + annual_gradient + length_drift + slow_length, -0.42, 0.42)


def _longitudinal_ring_field(
    x: np.ndarray, y: np.ndarray, volume: WoodVolume, rng: np.random.Generator, *, mode: str
) -> np.ndarray:
    drift = _longitudinal_noise(x.shape[0], rng, y_sigma=20.0, x_sigma=180.0) - 0.5
    fiber_curve = _fiber_curve_displacement(x.shape[0], rng)
    if mode == "radial":
        radial = volume.window_radial[0] + y * (volume.window_radial[1] - volume.window_radial[0])
        tangent_mid = sum(volume.window_tangent) * 0.5
        tangent = tangent_mid + (x - 0.5) * 0.045
        radial = radial + drift * volume.ring_jitter + fiber_curve * volume.ring_jitter * 1.25
        tangent = tangent + fiber_curve * 0.030
    elif mode == "tangential":
        radial_mid = sum(volume.window_radial) * 0.5
        tangent = volume.window_tangent[0] + y * (volume.window_tangent[1] - volume.window_tangent[0])
        crown = ((x - 0.5) ** 2) * (0.42 + volume.ring_jitter * 4.0)
        undulation = np.sin((x * 2.2 + volume.ring_phase) * np.pi) * 0.035
        radial = radial_mid + crown + undulation + drift * volume.ring_jitter * 0.75
        radial = radial + fiber_curve * volume.ring_jitter * 1.10
        tangent = tangent + fiber_curve * 0.035
    else:
        raise ValueError(f"unknown longitudinal cut mode: {mode}")
    radius = _wood_radius(radial, tangent, volume)
    return _ring_response(radius, volume)


def _render_crosscut(
    size: int,
    preset: LongitudinalPreset,
    volume: WoodVolume,
    rng: np.random.Generator,
    ring_profile: AnnualRingColorProfile,
) -> CrosscutRender:
    x, y = _grid(size)
    radial, tangent = _crosscut_slice_coordinates(x, y, volume, rng)
    wobble = _crosscut_wobble(size, rng) * volume.ring_jitter
    radius = _wood_radius(radial + wobble, tangent, volume)
    rings = _ring_response(radius, volume)
    latewood_line = _latewood_line(radius, volume)
    cycle = _ring_cycle(radius, volume)
    pores = _crosscut_pores(size, rng, preset.pore_strength, cycle)
    edge = _ring_edge(radius, volume)
    fine_cut = _crosscut_fine_grain(size, rng)
    ray_detail = _crosscut_rays(radial, tangent, radius, volume)
    cells = _crosscut_cells(size, rng)
    cell_walls = _crosscut_cell_walls(size, rng)
    patch = _crosscut_patch_variation(size, rng)
    ring_color = _crosscut_ring_color_offsets(radius, volume, rng, ring_profile)
    boundaries = _crosscut_broken_boundaries(size, rng, latewood_line, edge)
    checks = _crosscut_checks(size, rng, cycle)
    grit = _crosscut_rough_grit(size, rng)
    value = 0.52 + (rings - 0.5) * preset.board_contrast * 0.028
    value += ring_color * (0.205 + preset.board_contrast * 0.055)
    value += patch * (0.125 + preset.board_contrast * 0.038)
    value += fine_cut * 0.046 + (cells - 0.5) * 0.046 + cell_walls * 0.078
    value -= boundaries * (0.150 + preset.board_contrast * 0.045)
    value -= checks * (0.150 + preset.crack_strength * 0.240)
    base = _palette(np.clip(value, 0.0, 1.0), preset)
    warm_stain = np.clip(patch[..., None] * 0.35 + checks[..., None] * 0.18, 0.0, 0.35)
    base = _mix(base, np.array(preset.honey), np.clip((ring_color[..., None] + 0.36) * 0.060, 0.0, 0.075))
    base = _mix(base, np.array(preset.mid) * np.array([1.12, 0.86, 0.68]), warm_stain)
    base += np.clip(ring_color[..., None], 0.0, 1.0) * np.array([0.082, 0.038, 0.010])
    base -= np.clip(-ring_color[..., None], 0.0, 1.0) * np.array([0.066, 0.058, 0.050])
    base += np.clip(patch[..., None], 0.0, 1.0) * np.array([0.045, 0.020, 0.004])
    base -= np.clip(-patch[..., None], 0.0, 1.0) * np.array([0.035, 0.030, 0.026])
    base += ray_detail[..., None] * (0.50 + preset.silk_strength) * np.array([0.120, 0.095, 0.050])
    base *= 1.0 - cells[..., None] * np.array([0.042, 0.046, 0.052])
    base *= 1.0 - np.clip(-cell_walls[..., None], 0.0, 1.0) * np.array([0.18, 0.16, 0.13])
    base += np.clip(cell_walls[..., None], 0.0, 1.0) * np.array([0.030, 0.024, 0.012])
    base *= 1.0 - boundaries[..., None] * np.array([0.34, 0.37, 0.42])
    base *= 1.0 - checks[..., None] * np.array([0.58, 0.62, 0.66])
    base *= 1.0 - pores[..., None] * (0.78 + preset.pore_strength * 0.44)
    base += grit[..., None] * np.array([0.022, 0.016, 0.008])
    base = np.clip(base, 0.0, 1.0)
    roughness = preset.roughness * 0.74 + pores * 0.12 + boundaries * 0.055 + checks * 0.075
    roughness += cells * 0.032 + np.abs(grit) * 0.055
    roughness += np.abs(cell_walls) * 0.032
    roughness -= ray_detail * 0.035
    roughness = np.clip(roughness, 0.20, 0.68)
    height = (
        (rings - 0.5) * 0.005 * preset.board_contrast
        + boundaries * 0.020
        + ray_detail * 0.012
        + cells * 0.010
        + cell_walls * 0.014
        + grit * 0.017
        - pores * (0.048 + preset.pore_strength * 0.030)
        - checks * (0.028 + preset.crack_strength * 0.020)
    )
    normal = _normal_from_height(height, strength=3.2)
    preview = _preview(base, normal[..., :3], roughness, preset.gloss)
    return CrosscutRender(
        base=base,
        height=height,
        normal=normal,
        roughness=roughness,
        preview=preview,
    )


def _wood_radius(
    radial: np.ndarray | float, tangent: np.ndarray | float, volume: WoodVolume
) -> np.ndarray:
    return np.sqrt(
        ((radial - volume.center_radial) * volume.radius_scale) ** 2
        + (tangent - volume.center_tangent) ** 2
    )


def _ring_response(radius: np.ndarray, volume: WoodVolume) -> np.ndarray:
    phase = radius * volume.ring_frequency * np.pi * 2.0 + volume.ring_phase
    broad = np.sin(phase) * 0.5 + 0.5
    latewood = np.clip((broad - 0.48) * 3.8, 0.0, 1.0) ** 2.6
    earlywood = 1.0 - np.clip((0.46 - broad) * 2.6, 0.0, 1.0)
    return np.clip(earlywood * 0.30 + latewood * 0.70, 0.0, 1.0)


def _latewood_line(radius: np.ndarray, volume: WoodVolume) -> np.ndarray:
    cycle = _ring_cycle(radius, volume)
    distance = np.minimum(np.abs(cycle - 0.78), np.abs(cycle - 1.78))
    return np.exp(-((distance / 0.013) ** 2))


def _ring_edge(radius: np.ndarray, volume: WoodVolume) -> np.ndarray:
    cycle = _ring_cycle(radius, volume)
    distance = np.minimum(np.abs(cycle - 0.50), np.abs(cycle - 1.50))
    return np.exp(-((distance / 0.022) ** 2))


def _ring_cycle(radius: np.ndarray, volume: WoodVolume) -> np.ndarray:
    return (radius * volume.ring_frequency + volume.ring_phase / (np.pi * 2.0)) % 1.0


def _crosscut_wobble(size: int, rng: np.random.Generator) -> np.ndarray:
    field = rng.normal(0.0, 1.0, size=(size, size))
    return _normalize(gaussian_filter(field, sigma=(18.0, 18.0), mode="reflect")) - 0.5


def _crosscut_pores(
    size: int, rng: np.random.Generator, strength: float, cycle: np.ndarray
) -> np.ndarray:
    if strength <= 0.0:
        return np.zeros((size, size), dtype=np.float32)
    impulses = np.zeros((size, size), dtype=np.float32)
    earlywood_weight = np.clip((0.38 - cycle) / 0.38, 0.0, 1.0)
    count = max(160, int(size * size * (0.00064 + strength * 0.00120)))
    ys = rng.integers(0, size, count)
    xs = rng.integers(0, size, count)
    impulses[ys, xs] = rng.uniform(0.35, 1.0, count) * (0.35 + earlywood_weight[ys, xs] * 1.4)
    pores = gaussian_filter(impulses, sigma=(0.42, 0.62), mode="reflect")
    pores = _normalize(pores)
    return np.clip((pores - 0.34) * 5.8, 0.0, 1.0) * strength


def _crosscut_fine_grain(size: int, rng: np.random.Generator) -> np.ndarray:
    speckle = rng.normal(0.0, 1.0, size=(size, size))
    short = gaussian_filter(speckle, sigma=(0.65, 0.65), mode="reflect")
    cloudy = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(3.5, 3.5), mode="reflect")
    return (_normalize(short) - 0.5) * 0.7 + (_normalize(cloudy) - 0.5) * 0.3


def _crosscut_cells(size: int, rng: np.random.Generator) -> np.ndarray:
    fine = rng.normal(0.0, 1.0, size=(size, size))
    cellular = gaussian_filter(fine, sigma=(0.30, 0.30), mode="reflect")
    cellular = np.abs(_normalize(cellular) - 0.5) * 2.0
    short_cloud = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(1.4, 1.4), mode="reflect")
    return np.clip(cellular * 0.78 + _normalize(short_cloud) * 0.22, 0.0, 1.0)


def _crosscut_cell_walls(size: int, rng: np.random.Generator) -> np.ndarray:
    noise = rng.normal(0.0, 1.0, size=(size, size))
    micro = gaussian_filter(noise, sigma=(0.26, 0.26), mode="reflect")
    soft = gaussian_filter(noise, sigma=(1.10, 1.10), mode="reflect")
    highpass = _normalize(micro - soft) - 0.5
    walls = np.sign(highpass) * (np.abs(highpass) ** 0.52)
    return np.clip(walls * 1.45, -1.0, 1.0)


def _crosscut_patch_variation(size: int, rng: np.random.Generator) -> np.ndarray:
    medium = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(7.0, 15.0), mode="reflect")
    small = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(2.2, 5.0), mode="reflect")
    large = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(28.0, 46.0), mode="reflect")
    patch = (_normalize(medium) - 0.5) * 0.72
    patch += (_normalize(small) - 0.5) * 0.36
    patch -= (_normalize(large) - 0.5) * 0.20
    return np.clip(patch, -0.55, 0.55)


def _crosscut_ring_color_offsets(
    radius: np.ndarray,
    volume: WoodVolume,
    rng: np.random.Generator,
    ring_profile: AnnualRingColorProfile,
) -> np.ndarray:
    ring_position = radius * volume.ring_frequency + volume.ring_phase / (np.pi * 2.0)
    smooth_ring = _sample_annual_ring_profile(ring_position, ring_profile)
    cycle = ring_position % 1.0
    annual_gradient = np.cos((cycle - 0.18) * np.pi * 2.0) * 0.085
    local_drift = gaussian_filter(
        rng.normal(0.0, 1.0, size=radius.shape), sigma=(18.0, 32.0), mode="reflect"
    )
    local_drift = (_normalize(local_drift) - 0.5) * 0.20
    return np.clip(smooth_ring + annual_gradient + local_drift, -0.48, 0.48)


def _crosscut_broken_boundaries(
    size: int, rng: np.random.Generator, latewood_line: np.ndarray, edge: np.ndarray
) -> np.ndarray:
    line = np.maximum(latewood_line, edge * 0.82)
    broken = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(1.0, 7.0), mode="reflect")
    broken = np.clip((_normalize(broken) - 0.35) * 1.75, 0.0, 1.0)
    serration = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(0.35, 1.4), mode="reflect")
    serration = np.clip((_normalize(serration) - 0.42) * 2.2, 0.0, 1.0)
    hard = np.clip((line - 0.16) * 2.8, 0.0, 1.0) ** 0.62
    return np.clip(hard * (0.30 + broken * 0.56 + serration * 0.24), 0.0, 1.0)


def _crosscut_checks(size: int, rng: np.random.Generator, cycle: np.ndarray) -> np.ndarray:
    impulses = np.zeros((size, size), dtype=np.float32)
    count = max(48, int(size * size * 0.00016))
    ys = rng.integers(0, size, count)
    xs = rng.integers(0, size, count)
    ring_weight = 0.35 + np.clip(np.abs(cycle[ys, xs] - 0.62) * 2.6, 0.0, 1.0)
    impulses[ys, xs] = rng.uniform(0.45, 1.0, count) * ring_weight
    cracks = gaussian_filter(impulses, sigma=(0.34, 13.0), mode="reflect")
    cracks += gaussian_filter(impulses, sigma=(0.80, 4.0), mode="reflect") * 0.36
    cracks = _normalize(cracks)
    broken_mask = gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(2.0, 12.0), mode="reflect")
    broken_mask = np.clip((_normalize(broken_mask) - 0.36) * 1.8, 0.0, 1.0)
    return np.clip((cracks - 0.50) * 4.4, 0.0, 1.0) * broken_mask


def _crosscut_rough_grit(size: int, rng: np.random.Generator) -> np.ndarray:
    fine = rng.normal(0.0, 1.0, size=(size, size))
    micro = gaussian_filter(fine, sigma=(0.18, 0.18), mode="reflect")
    soft = gaussian_filter(fine, sigma=(0.95, 0.95), mode="reflect")
    grit = _normalize(micro - soft) - 0.5
    grit += (_normalize(gaussian_filter(rng.normal(0.0, 1.0, size=(size, size)), sigma=(0.55, 1.1), mode="reflect")) - 0.5) * 0.4
    return np.clip(grit, -0.65, 0.65)


def _crosscut_rays(
    radial: np.ndarray, tangent: np.ndarray, radius: np.ndarray, volume: WoodVolume
) -> np.ndarray:
    angle = np.arctan2(tangent - volume.center_tangent, radial - volume.center_radial)
    major = np.maximum(0.0, np.cos(angle * 32.0 + radius * 0.7)) ** 28.0
    minor = np.maximum(0.0, np.cos(angle * 76.0 + radius * 1.9)) ** 42.0
    return (major * 0.55 + minor * 0.32) * 0.72


def _flow_displacement(size: int, rng: np.random.Generator, *, amount: float) -> np.ndarray:
    flow = rng.normal(0.0, 1.0, size=(size, size))
    flow = gaussian_filter(flow, sigma=(44.0, 96.0), mode="reflect")
    flow = _normalize(flow) - 0.5
    return flow * amount


def _warp_field(field: np.ndarray, displacement: np.ndarray) -> np.ndarray:
    yy, xx = np.mgrid[0 : field.shape[0], 0 : field.shape[1]]
    coords = np.array([yy + displacement, xx])
    return map_coordinates(field, coords, order=1, mode="reflect")


def _longitudinal_pores(
    size: int, rng: np.random.Generator, strength: float
) -> np.ndarray:
    if strength <= 0.0:
        return np.zeros((size, size), dtype=np.float32)
    impulses = np.zeros((size, size), dtype=np.float32)
    count = max(8, int(size * size * (0.00012 + strength * 0.00022)))
    ys = rng.integers(0, size, size=count)
    xs = rng.integers(0, size, size=count)
    impulses[ys, xs] = rng.uniform(0.35, 1.0, size=count)
    vessels = gaussian_filter(impulses, sigma=(0.85, 24.0 + strength * 30.0), mode="reflect")
    vessels = _normalize(vessels)
    threshold = 0.74 - strength * 0.11
    return np.clip((vessels - threshold) / max(1e-6, 1.0 - threshold), 0.0, 1.0) * strength


def _elongated_impulse_field(
    size: int,
    rng: np.random.Generator,
    *,
    density: float,
    y_sigma: float,
    x_sigma: float,
) -> np.ndarray:
    impulses = np.zeros((size, size), dtype=np.float32)
    count = max(16, int(size * size * density))
    ys = rng.integers(0, size, size=count)
    xs = rng.integers(0, size, size=count)
    impulses[ys, xs] = rng.uniform(0.25, 1.0, size=count)
    field = gaussian_filter(impulses, sigma=(y_sigma, x_sigma), mode="reflect")
    field = _normalize(field)
    return np.clip((field - 0.42) * 1.9, 0.0, 1.0)


def _finite_longitudinal_streaks(
    size: int,
    rng: np.random.Generator,
    *,
    count: int,
    length: tuple[float, float],
    width: tuple[float, float],
    strength: tuple[float, float],
) -> np.ndarray:
    field = np.zeros((size, size), dtype=np.float32)
    for _ in range(count):
        segment = int(rng.uniform(length[0], length[1]) * size)
        if segment <= 1:
            continue
        x0 = rng.integers(-segment // 4, size)
        y0 = rng.uniform(0.0, size - 1)
        slope = rng.normal(0.0, 0.0045)
        wave_amp = rng.uniform(0.0, 0.55)
        wave_phase = rng.uniform(0.0, np.pi * 2.0)
        xs = np.arange(x0, x0 + segment)
        mask = (xs >= 0) & (xs < size)
        xs = xs[mask]
        if xs.size == 0:
            continue
        t = np.linspace(-0.5, 0.5, xs.size)
        ys = y0 + slope * t * size + np.sin(t * np.pi * 2.0 + wave_phase) * wave_amp
        yi = np.clip(np.rint(ys).astype(np.int32), 0, size - 1)
        fade = np.sin(np.linspace(0.0, np.pi, xs.size)) ** rng.uniform(0.35, 0.85)
        value = rng.uniform(strength[0], strength[1]) * fade
        field[yi, xs] = np.maximum(field[yi, xs], value.astype(np.float32))
    sigma_y = rng.uniform(width[0], width[1])
    blurred = gaussian_filter(field, sigma=(sigma_y, 0.65), mode="reflect")
    return np.clip(blurred / max(float(np.percentile(blurred, 99.7)), 1e-6), 0.0, 1.0)


def _longitudinal_checks(
    size: int, rng: np.random.Generator, strength: float
) -> np.ndarray:
    if strength <= 0.0:
        return np.zeros((size, size), dtype=np.float32)
    field = np.zeros((size, size), dtype=np.float32)
    count = max(2, int(20 * strength))
    yy, xx = np.mgrid[0:size, 0:size]
    x = xx / size
    y = yy / size
    for _ in range(count):
        cx = rng.uniform(-0.12, 1.12)
        cy = rng.uniform(0.04, 0.96)
        long_axis = rng.uniform(0.16, 0.62)
        short_axis = rng.uniform(0.0012, 0.0045)
        angle = rng.normal(0.0, 0.010)
        dx = x - cx
        dy = y - cy
        along = dx * np.cos(angle) + dy * np.sin(angle)
        across = -dx * np.sin(angle) + dy * np.cos(angle)
        check = np.exp(-((along / long_axis) ** 2 + (across / short_axis) ** 2))
        field = np.maximum(field, check * rng.uniform(0.22, 0.72))
    return np.clip(field * strength, 0.0, 1.0)


def _silk_ribbon(size: int, rng: np.random.Generator) -> np.ndarray:
    field = _longitudinal_noise(size, rng, y_sigma=4.5, x_sigma=130.0)
    shimmer = _longitudinal_noise(size, rng, y_sigma=1.8, x_sigma=70.0)
    silk = np.clip((field * 0.65 + shimmer * 0.35 - 0.54) * 2.4, 0.0, 1.0)
    return gaussian_filter(silk, sigma=(1.0, 18.0), mode="reflect")


def _palette(value: np.ndarray, preset: LongitudinalPreset) -> np.ndarray:
    dark = np.array(preset.dark)
    mid = np.array(preset.mid)
    light = np.array(preset.light)
    base = _mix(dark, mid, np.clip(value[..., None] * 1.34, 0.0, 1.0))
    return _mix(base, light, np.clip((value[..., None] - 0.52) * 1.18, 0.0, 0.42))


def _normal_from_height(height: np.ndarray, strength: float) -> np.ndarray:
    dy, dx = np.gradient(height)
    normal = np.dstack((-dx * strength, -dy * strength, np.ones_like(height)))
    normal /= np.linalg.norm(normal, axis=-1, keepdims=True).clip(min=1e-6)
    normal = normal * 0.5 + 0.5
    return _rgba(normal)


def _preview(
    base: np.ndarray, normal: np.ndarray, roughness: np.ndarray, gloss: float
) -> np.ndarray:
    n = normal * 2.0 - 1.0
    light = np.array([-0.32, -0.24, 0.92])
    light /= np.linalg.norm(light)
    view = np.array([0.0, 0.0, 1.0])
    ndotl = np.clip(n @ light, 0.0, 1.0)
    half_vec = light + view
    half_vec /= np.linalg.norm(half_vec)
    spec = np.clip(n @ half_vec, 0.0, 1.0) ** (20.0 + gloss * 72.0)
    spec *= (1.0 - roughness) * (0.06 + gloss * 0.34)
    lit = base * (0.38 + ndotl[..., None] * 0.74) + spec[..., None]
    vignette_x, vignette_y = _grid(base.shape[0])
    vignette = 1.0 - ((vignette_x - 0.5) ** 2 + (vignette_y - 0.5) ** 2) * 0.28
    return np.clip(lit * vignette[..., None], 0.0, 1.0)


def _write_contact_sheet(path: Path, previews: list[tuple[LongitudinalPreset, np.ndarray, np.ndarray, np.ndarray]]) -> None:
    columns = 4
    rows = int(np.ceil(len(previews) / columns)) * 3
    fig, axes = plt.subplots(rows, columns, figsize=(columns * 4.2, rows * 3.4), dpi=140)
    axes_array = np.atleast_1d(axes).reshape(rows, columns)
    for ax in axes_array.flat:
        ax.axis("off")
    for index, (preset, radial, tangential, crosscut) in enumerate(previews):
        col = index % columns
        row = (index // columns) * 3
        radial_ax = axes_array[row, col]
        tangential_ax = axes_array[row + 1, col]
        cross_ax = axes_array[row + 2, col]
        radial_ax.imshow(np.clip(radial[..., :3], 0.0, 1.0))
        radial_ax.set_title(f"{preset.name} radial", fontsize=9)
        radial_ax.axis("off")
        tangential_ax.imshow(np.clip(tangential[..., :3], 0.0, 1.0))
        tangential_ax.set_title(f"{preset.name} tangential", fontsize=9)
        tangential_ax.axis("off")
        cross_ax.imshow(np.clip(crosscut[..., :3], 0.0, 1.0))
        cross_ax.set_title(f"{preset.name} cross", fontsize=9)
        cross_ax.axis("off")
    fig.tight_layout(pad=1.2)
    fig.savefig(path)
    plt.close(fig)


def _write_markdown(path: Path, manifest: list[dict[str, object]]) -> None:
    lines = ["# Longitudinal Wood Preset Gallery", ""]
    for item in manifest:
        files = item["files"]
        assert isinstance(files, dict)
        lines.extend(
            [
                f"## {item['name']}",
                "",
                str(item["description"]),
                "",
                f"- preview: `{files['preview']}`",
                f"- base: `{files['base']}`",
                f"- normal: `{files['normal']}`",
                f"- roughness: `{files['roughness']}`",
                f"- height: `{files['height']}`",
                f"- tangential preview: `{files['tangential_preview']}`",
                f"- tangential base: `{files['tangential_base']}`",
                f"- tangential normal: `{files['tangential_normal']}`",
                f"- tangential roughness: `{files['tangential_roughness']}`",
                f"- tangential height: `{files['tangential_height']}`",
                f"- crosscut base: `{files['crosscut_base']}`",
                f"- crosscut: `{files['crosscut']}`",
                f"- crosscut normal: `{files['crosscut_normal']}`",
                f"- crosscut roughness: `{files['crosscut_roughness']}`",
                f"- crosscut height: `{files['crosscut_height']}`",
                "",
            ]
        )
    path.write_text("\n".join(lines), encoding="utf-8")


def _grid(size: int) -> tuple[np.ndarray, np.ndarray]:
    axis = np.linspace(0.0, 1.0, size, endpoint=False)
    return np.meshgrid(axis, axis)


def _mix(a: np.ndarray, b: np.ndarray, t: np.ndarray) -> np.ndarray:
    return a * (1.0 - t) + b * t


def _normalize(value: np.ndarray) -> np.ndarray:
    value = value - value.min()
    peak = value.max()
    return value / peak if peak > 0.0 else value


def _rgba(image: np.ndarray) -> np.ndarray:
    alpha = np.ones((*image.shape[:2], 1), dtype=np.float32)
    return np.concatenate([image.astype(np.float32), alpha], axis=-1)


if __name__ == "__main__":
    main()
