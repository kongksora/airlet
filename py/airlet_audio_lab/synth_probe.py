from __future__ import annotations

import argparse
import csv
from pathlib import Path

import numpy as np

from .audio_io import write_wav


def read_partials(path: Path, limit: int) -> list[tuple[float, float]]:
    with path.open("r", encoding="utf-8", newline="") as f:
        rows = list(csv.DictReader(f))
    partials = [
        (float(row["frequency_hz"]), float(row["relative_amplitude"]))
        for row in rows
        if float(row["frequency_hz"]) > 0.0 and float(row["relative_amplitude"]) > 0.0
    ]
    partials.sort(key=lambda item: item[1], reverse=True)
    return sorted(partials[:limit], key=lambda item: item[0])


def synthesize(
    partials: list[tuple[float, float]],
    sample_rate: int,
    duration: float,
    attack: float,
    decay: float,
    seed: int,
) -> np.ndarray:
    rng = np.random.default_rng(seed)
    t = np.arange(int(sample_rate * duration), dtype=np.float64) / sample_rate
    out = np.zeros_like(t)
    for freq, amp in partials:
        phase = rng.uniform(0.0, 2.0 * np.pi)
        per_partial_decay = decay / np.sqrt(max(freq / max(partials[0][0], 1.0), 1.0))
        env = np.exp(-t / max(per_partial_decay, 1e-6))
        out += amp * env * np.sin(2.0 * np.pi * freq * t + phase)

    attack_env = np.clip(t / max(attack, 1e-6), 0.0, 1.0)
    attack_env = attack_env * attack_env * (3.0 - 2.0 * attack_env)
    out *= attack_env
    peak = np.max(np.abs(out)) if out.size else 0.0
    if peak > 0.0:
        out = out / peak * 0.9
    return out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("partials", type=Path)
    parser.add_argument("--out", type=Path, default=Path("py/out/probes/probe.wav"))
    parser.add_argument("--sample-rate", type=int, default=48_000)
    parser.add_argument("--duration", type=float, default=4.0)
    parser.add_argument("--attack", type=float, default=0.002)
    parser.add_argument("--decay", type=float, default=1.25)
    parser.add_argument("--limit", type=int, default=12)
    parser.add_argument("--seed", type=int, default=42)
    args = parser.parse_args()

    partials = read_partials(args.partials, args.limit)
    if not partials:
        raise SystemExit(f"no partials found in {args.partials}")

    samples = synthesize(partials, args.sample_rate, args.duration, args.attack, args.decay, args.seed)
    write_wav(args.out, samples, args.sample_rate)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
