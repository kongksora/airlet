from __future__ import annotations

import csv
from dataclasses import dataclass
from pathlib import Path

import numpy as np


@dataclass(frozen=True)
class Partial:
    frequency_hz: float
    amplitude: float


@dataclass(frozen=True)
class ModalVoiceParams:
    attack: float = 0.003
    base_decay: float = 0.55
    low_decay_boost: float = 0.25
    high_decay_power: float = 0.85
    pitch_decay_power: float = 0.35
    amplitude_power: float = 0.85
    detune_cents: float = 1.5
    stretch: float = 0.0004
    click_gain: float = 0.035
    click_decay: float = 0.006
    noise_gain: float = 0.003
    drive: float = 1.15
    output_gain: float = 0.72


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


def synthesize_modal_note(
    partials: list[Partial],
    sample_rate: int,
    duration: float,
    params: ModalVoiceParams,
    rng: np.random.Generator,
    target_freq: float | None = None,
) -> np.ndarray:
    t = np.arange(int(sample_rate * duration), dtype=np.float64) / sample_rate
    out = np.zeros_like(t)
    if not partials:
        return out

    anchor = max(partials, key=lambda partial: partial.amplitude).frequency_hz
    target = target_freq or anchor
    pitch_decay_scale = max(target / max(anchor, 1.0), 0.25) ** params.pitch_decay_power
    for index, partial in enumerate(partials):
        phase = rng.uniform(0.0, 2.0 * np.pi)
        detune = 2.0 ** (rng.normal(0.0, params.detune_cents) / 1200.0)
        stretched = 1.0 + params.stretch * index * index
        freq = partial.frequency_hz * detune * stretched
        freq_ratio = max(partial.frequency_hz / max(anchor, 1.0), 0.1)
        high_decay_scale = freq_ratio**params.high_decay_power
        low_decay = 1.0 + params.low_decay_boost / max(freq_ratio, 0.25)
        per_partial_decay = params.base_decay * low_decay / max(high_decay_scale * pitch_decay_scale, 1e-6)
        env = np.exp(-t / max(per_partial_decay, 1e-6))
        amp = partial.amplitude**params.amplitude_power
        out += amp * env * np.sin(2.0 * np.pi * freq * t + phase)

    if params.click_gain > 0.0:
        click_env = np.exp(-t / max(params.click_decay, 1e-6))
        click_tone = np.sin(2.0 * np.pi * min(anchor * 7.0, sample_rate * 0.45) * t)
        click_noise = rng.normal(0.0, 1.0, size=t.size)
        out += params.click_gain * click_env * (0.7 * click_tone + 0.3 * click_noise)

    if params.noise_gain > 0.0:
        noise_env = np.exp(-t / max(params.base_decay * 0.35, 1e-6))
        out += params.noise_gain * noise_env * rng.normal(0.0, 1.0, size=t.size)

    attack_env = np.clip(t / max(params.attack, 1e-6), 0.0, 1.0)
    attack_env = attack_env * attack_env * (3.0 - 2.0 * attack_env)
    out *= attack_env
    if params.drive > 0.0:
        out = np.tanh(out * params.drive) / np.tanh(params.drive)
    return out * params.output_gain


def synthesize_note(
    partials: list[Partial],
    sample_rate: int,
    duration: float,
    attack: float,
    decay: float,
    rng: np.random.Generator,
) -> np.ndarray:
    params = ModalVoiceParams(attack=attack, base_decay=decay, click_gain=0.0, noise_gain=0.0)
    return synthesize_modal_note(partials, sample_rate, duration, params, rng)


def peak_normalize(samples: np.ndarray, peak: float) -> np.ndarray:
    current = np.max(np.abs(samples)) if samples.size else 0.0
    if current > 0.0:
        return samples / current * peak
    return samples
