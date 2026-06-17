from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path

import numpy as np


@dataclass(frozen=True)
class Partial:
    frequency_hz: float
    amplitude: float


def read_partials(path: Path, limit: int) -> list[Partial]:
    with path.open("r", encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    partials = [
        Partial(float(row["frequency_hz"]), float(row["relative_amplitude"]))
        for row in rows
        if float(row["frequency_hz"]) > 0.0 and float(row["relative_amplitude"]) > 0.0
    ]
    partials.sort(key=lambda item: item.amplitude, reverse=True)
    return sorted(partials[:limit], key=lambda item: item.frequency_hz)


def retune_partials(partials: list[Partial], target_freq: float, reference_freq: float | None = None) -> list[Partial]:
    if not partials:
        return []
    anchor = reference_freq or max(partials, key=lambda partial: partial.amplitude).frequency_hz
    scale = target_freq / anchor
    return [Partial(partial.frequency_hz * scale, partial.amplitude) for partial in partials]


def synthesize_note(
    partials: list[Partial],
    sample_rate: int,
    duration: float,
    attack: float,
    decay: float,
    rng: np.random.Generator,
) -> np.ndarray:
    t = np.arange(int(sample_rate * duration), dtype=np.float64) / sample_rate
    out = np.zeros_like(t)
    if not partials:
        return out

    anchor = max(partials, key=lambda partial: partial.amplitude).frequency_hz
    for partial in partials:
        phase = rng.uniform(0.0, 2.0 * np.pi)
        high_decay_scale = np.sqrt(max(partial.frequency_hz / max(anchor, 1.0), 1.0))
        per_partial_decay = decay / high_decay_scale
        env = np.exp(-t / max(per_partial_decay, 1e-6))
        out += partial.amplitude * env * np.sin(2.0 * np.pi * partial.frequency_hz * t + phase)

    attack_env = np.clip(t / max(attack, 1e-6), 0.0, 1.0)
    attack_env = attack_env * attack_env * (3.0 - 2.0 * attack_env)
    return out * attack_env


def peak_normalize(samples: np.ndarray, peak: float) -> np.ndarray:
    current = np.max(np.abs(samples)) if samples.size else 0.0
    if current > 0.0:
        return samples / current * peak
    return samples
