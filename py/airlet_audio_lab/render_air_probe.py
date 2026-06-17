from __future__ import annotations

import argparse
from pathlib import Path

import numpy as np

from .audio_io import write_wav
from .modal import (
    ModalVoiceParams,
    peak_normalize,
    read_partials,
    retune_partials,
    synthesize_modal_note,
)
from .music import air_intro_events, midi_to_freq


PRESETS = {
    "default": {
        "note_tail": 6.0,
        "final_tail": 3.0,
        "note_gain": 0.18,
        "limit": 12,
        "voice": ModalVoiceParams(),
    },
    "a-dry": {
        "note_tail": 1.2,
        "final_tail": 0.8,
        "note_gain": 0.17,
        "limit": 8,
        "voice": ModalVoiceParams(
            attack=0.003,
            base_decay=0.25,
            low_decay_boost=0.1,
            high_decay_power=1.1,
            pitch_decay_power=0.5,
            amplitude_power=0.85,
            detune_cents=1.5,
            stretch=0.0004,
            click_gain=0.035,
            click_decay=0.006,
            noise_gain=0.002,
            drive=1.15,
            output_gain=0.72,
        ),
    },
}


def preset_value(args: argparse.Namespace, name: str):
    value = getattr(args, name)
    if value is not None:
        return value
    return PRESETS[args.preset][name]


def preset_voice(args: argparse.Namespace) -> ModalVoiceParams:
    voice = PRESETS[args.preset]["voice"]
    return ModalVoiceParams(
        attack=args.attack if args.attack is not None else voice.attack,
        base_decay=args.decay if args.decay is not None else voice.base_decay,
        low_decay_boost=args.low_decay_boost if args.low_decay_boost is not None else voice.low_decay_boost,
        high_decay_power=args.high_decay_power if args.high_decay_power is not None else voice.high_decay_power,
        pitch_decay_power=args.pitch_decay_power if args.pitch_decay_power is not None else voice.pitch_decay_power,
        amplitude_power=args.amplitude_power if args.amplitude_power is not None else voice.amplitude_power,
        detune_cents=args.detune_cents if args.detune_cents is not None else voice.detune_cents,
        stretch=args.stretch if args.stretch is not None else voice.stretch,
        click_gain=args.click_gain if args.click_gain is not None else voice.click_gain,
        click_decay=args.click_decay if args.click_decay is not None else voice.click_decay,
        noise_gain=args.noise_gain if args.noise_gain is not None else voice.noise_gain,
        drive=args.drive if args.drive is not None else voice.drive,
        output_gain=args.output_gain if args.output_gain is not None else voice.output_gain,
    )


def render_air_probe(
    partials_path: Path,
    sample_rate: int,
    note_tail: float,
    final_tail: float,
    note_gain: float,
    voice: ModalVoiceParams,
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
            note = synthesize_modal_note(partials, sample_rate, note_tail, voice, rng, target)
            end = min(output.size, cursor + note.size)
            output[cursor:end] += note[: end - cursor] * note_gain
        cursor += int(round(event.millis / 1000.0 * sample_rate))

    return peak_normalize(output, 0.95)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("partials", type=Path)
    parser.add_argument("--out", type=Path, default=Path("py/out/air_probes/air_probe.wav"))
    parser.add_argument("--preset", choices=sorted(PRESETS), default="default")
    parser.add_argument("--sample-rate", type=int, default=48_000)
    parser.add_argument("--note-tail", type=float, default=None)
    parser.add_argument("--final-tail", type=float, default=None)
    parser.add_argument("--note-gain", type=float, default=None)
    parser.add_argument("--attack", type=float, default=None)
    parser.add_argument("--decay", type=float, default=None)
    parser.add_argument("--low-decay-boost", type=float, default=None)
    parser.add_argument("--high-decay-power", type=float, default=None)
    parser.add_argument("--pitch-decay-power", type=float, default=None)
    parser.add_argument("--amplitude-power", type=float, default=None)
    parser.add_argument("--detune-cents", type=float, default=None)
    parser.add_argument("--stretch", type=float, default=None)
    parser.add_argument("--click-gain", type=float, default=None)
    parser.add_argument("--click-decay", type=float, default=None)
    parser.add_argument("--noise-gain", type=float, default=None)
    parser.add_argument("--drive", type=float, default=None)
    parser.add_argument("--output-gain", type=float, default=None)
    parser.add_argument("--limit", type=int, default=None)
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--reference-freq", type=float, default=None)
    args = parser.parse_args()

    voice = preset_voice(args)
    samples = render_air_probe(
        args.partials,
        args.sample_rate,
        preset_value(args, "note_tail"),
        preset_value(args, "final_tail"),
        preset_value(args, "note_gain"),
        voice,
        preset_value(args, "limit"),
        args.seed,
        args.reference_freq,
    )
    write_wav(args.out, samples, args.sample_rate)
    print(f"wrote {args.out}")


if __name__ == "__main__":
    main()
