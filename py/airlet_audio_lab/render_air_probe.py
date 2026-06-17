from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np

from .audio_io import write_wav
from .modal import peak_normalize, read_partials, retune_partials, synthesize_note
from .music import air_intro_events, midi_to_freq


def render_air_probe(
    partials_path: Path,
    sample_rate: int,
    note_tail: float,
    final_tail: float,
    note_gain: float,
    attack: float,
    decay: float,
    limit: int,
    seed: int,
    reference_freq: float | None,
) -> np.ndarray:
    source_partials = read_partials(partials_path, limit)
    if not source_partials:
        raise ValueError(f"no partials found in {partials_path}")

    events = air_intro_events()
    song_seconds = sum(event.millis for event in events) / 1000.0
    total_samples = int(np.ceil((song_seconds + note_tail + final_tail) * sample_rate))
    output = np.zeros(total_samples, dtype=np.float64)
    cursor = 0
    rng = np.random.default_rng(seed)

    for event in events:
        if not event.is_rest:
            target = midi_to_freq(event.midi_note)
            partials = retune_partials(source_partials, target, reference_freq)
            note = synthesize_note(partials, sample_rate, note_tail, attack, decay, rng)
            end = min(output.size, cursor + note.size)
            output[cursor:end] += note[: end - cursor] * note_gain
        cursor += int(round(event.millis / 1000.0 * sample_rate))

    return peak_normalize(output, 0.95)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("partials", type=Path)
    parser.add_argument("--out", type=Path, default=Path("py/out/air_probes/air_probe.wav"))
    parser.add_argument("--sample-rate", type=int, default=48_000)
    parser.add_argument("--note-tail", type=float, default=6.0)
    parser.add_argument("--final-tail", type=float, default=3.0)
    parser.add_argument("--note-gain", type=float, default=0.18)
    parser.add_argument("--attack", type=float, default=0.004)
    parser.add_argument("--decay", type=float, default=1.5)
    parser.add_argument("--limit", type=int, default=12)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--reference-freq", type=float, default=None)
    args = parser.parse_args()

    samples = render_air_probe(
        args.partials,
        args.sample_rate,
        args.note_tail,
        args.final_tail,
        args.note_gain,
        args.attack,
        args.decay,
        args.limit,
        args.seed,
        args.reference_freq,
    )
    write_wav(args.out, samples, args.sample_rate)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
