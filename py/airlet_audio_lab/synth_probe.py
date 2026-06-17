from __future__ import annotations

import argparse
from pathlib import Path

from .audio_io import write_wav
from .modal import peak_normalize, read_partials, synthesize_note
import numpy as np


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

    rng = np.random.default_rng(args.seed)
    samples = synthesize_note(partials, args.sample_rate, args.duration, args.attack, args.decay, rng)
    samples = peak_normalize(samples, 0.9)
    write_wav(args.out, samples, args.sample_rate)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
