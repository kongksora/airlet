from __future__ import annotations

import argparse
import csv
import json
from dataclasses import asdict, dataclass
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np
from scipy import signal

from .audio_io import audio_files, load_mono


@dataclass
class AudioSummary:
    path: str
    sample_rate: int
    samples: int
    duration_seconds: float
    peak: float
    rms: float
    estimated_fundamental_hz: float | None
    attack_seconds: float | None
    decay_t60_seconds: float | None


def rms_envelope(samples: np.ndarray, sample_rate: int, window_ms: float = 10.0) -> tuple[np.ndarray, np.ndarray]:
    window = max(1, int(sample_rate * window_ms / 1000.0))
    squared = samples * samples
    kernel = np.ones(window, dtype=np.float64) / window
    envelope = np.sqrt(np.convolve(squared, kernel, mode="same"))
    times = np.arange(envelope.size) / sample_rate
    return times, envelope


def estimate_attack_decay(times: np.ndarray, envelope: np.ndarray) -> tuple[float | None, float | None]:
    if envelope.size == 0:
        return None, None
    peak = float(np.max(envelope))
    if peak <= 0.0:
        return None, None

    peak_index = int(np.argmax(envelope))
    threshold_90 = peak * 0.9
    attack_candidates = np.flatnonzero(envelope[: peak_index + 1] >= threshold_90)
    attack = float(times[int(attack_candidates[0])]) if attack_candidates.size else None

    t60_level = peak * 0.001
    decay_candidates = np.flatnonzero(envelope[peak_index:] <= t60_level)
    decay = None
    if decay_candidates.size:
        decay = float(times[peak_index + int(decay_candidates[0])] - times[peak_index])
    return attack, decay


def estimate_fundamental(samples: np.ndarray, sample_rate: int) -> float | None:
    if samples.size < sample_rate // 20:
        return None
    segment = trim_to_active(samples, sample_rate)
    max_len = min(segment.size, int(sample_rate * 1.5))
    segment = segment[:max_len]
    if segment.size < 2048:
        return None

    segment = segment - np.mean(segment)
    spectrum = np.fft.rfft(segment * signal.windows.hann(segment.size))
    freqs = np.fft.rfftfreq(segment.size, 1.0 / sample_rate)
    mags = np.abs(spectrum)
    mask = (freqs >= 40.0) & (freqs <= 5000.0)
    if not np.any(mask):
        return None

    idx = int(np.argmax(mags[mask]))
    return float(freqs[mask][idx])


def partials(samples: np.ndarray, sample_rate: int, count: int = 16) -> list[dict[str, float]]:
    segment = trim_to_active(samples, sample_rate)
    max_len = min(segment.size, int(sample_rate * 2.0))
    segment = segment[:max_len]
    if segment.size < 2048:
        return []

    windowed = segment * signal.windows.hann(segment.size)
    spectrum = np.fft.rfft(windowed)
    freqs = np.fft.rfftfreq(segment.size, 1.0 / sample_rate)
    mags = np.abs(spectrum)
    min_distance = max(1, int(40.0 / (sample_rate / segment.size)))
    peaks, props = signal.find_peaks(mags, distance=min_distance, prominence=np.max(mags) * 0.002)
    if peaks.size == 0:
        return []

    order = np.argsort(props["prominences"])[::-1][:count]
    selected = sorted(peaks[order], key=lambda peak: freqs[peak])
    max_mag = max(float(np.max(mags[selected])), 1e-12)
    return [
        {
            "frequency_hz": float(freqs[peak]),
            "relative_amplitude": float(mags[peak] / max_mag),
            "db": float(20.0 * np.log10(max(mags[peak] / max_mag, 1e-12))),
        }
        for peak in selected
    ]


def trim_to_active(samples: np.ndarray, sample_rate: int) -> np.ndarray:
    _times, envelope = rms_envelope(samples, sample_rate)
    peak = float(np.max(envelope)) if envelope.size else 0.0
    if peak <= 0.0:
        return samples
    active = np.flatnonzero(envelope >= peak * 0.02)
    if active.size == 0:
        return samples
    start = max(0, int(active[0]) - sample_rate // 50)
    end = min(samples.size, int(active[-1]) + sample_rate // 10)
    return samples[start:end]


def write_plots(name: str, samples: np.ndarray, sample_rate: int, out_dir: Path) -> None:
    times, envelope = rms_envelope(samples, sample_rate)
    fig, ax = plt.subplots(figsize=(10, 3))
    ax.plot(np.arange(samples.size) / sample_rate, samples, linewidth=0.4, alpha=0.45)
    ax.plot(times, envelope, linewidth=1.0)
    ax.set_title(f"{name} waveform + RMS envelope")
    ax.set_xlabel("seconds")
    ax.set_ylabel("amplitude")
    fig.tight_layout()
    fig.savefig(out_dir / "envelope.png", dpi=160)
    plt.close(fig)

    fig, ax = plt.subplots(figsize=(10, 4))
    ax.specgram(samples, NFFT=4096, Fs=sample_rate, noverlap=3072, cmap="magma")
    ax.set_ylim(0, min(12_000, sample_rate / 2))
    ax.set_title(f"{name} spectrogram")
    ax.set_xlabel("seconds")
    ax.set_ylabel("Hz")
    fig.tight_layout()
    fig.savefig(out_dir / "spectrogram.png", dpi=160)
    plt.close(fig)


def analyze_file(path: Path, output_root: Path) -> AudioSummary:
    samples, sample_rate = load_mono(path)
    out_dir = output_root / path.stem
    out_dir.mkdir(parents=True, exist_ok=True)

    times, envelope = rms_envelope(samples, sample_rate)
    attack, decay = estimate_attack_decay(times, envelope)
    f0 = estimate_fundamental(samples, sample_rate)
    peaks = partials(samples, sample_rate)

    summary = AudioSummary(
        path=str(path),
        sample_rate=sample_rate,
        samples=int(samples.size),
        duration_seconds=float(samples.size / sample_rate),
        peak=float(np.max(np.abs(samples))) if samples.size else 0.0,
        rms=float(np.sqrt(np.mean(samples * samples))) if samples.size else 0.0,
        estimated_fundamental_hz=f0,
        attack_seconds=attack,
        decay_t60_seconds=decay,
    )

    with (out_dir / "summary.json").open("w", encoding="utf-8") as f:
        json.dump(asdict(summary), f, indent=2, ensure_ascii=False)

    with (out_dir / "partials.csv").open("w", encoding="utf-8", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=["frequency_hz", "relative_amplitude", "db"])
        writer.writeheader()
        writer.writerows(peaks)

    write_plots(path.stem, samples, sample_rate, out_dir)
    return summary


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--references", type=Path, default=Path("references"))
    parser.add_argument("--out", type=Path, default=Path("py/out/analysis"))
    args = parser.parse_args()

    files = audio_files(args.references)
    if not files:
        raise SystemExit(f"no supported audio files under {args.references}")

    summaries = [analyze_file(path, args.out) for path in files]
    args.out.mkdir(parents=True, exist_ok=True)
    with (args.out / "index.json").open("w", encoding="utf-8") as f:
        json.dump([asdict(summary) for summary in summaries], f, indent=2, ensure_ascii=False)

    for summary in summaries:
        f0 = "n/a" if summary.estimated_fundamental_hz is None else f"{summary.estimated_fundamental_hz:.1f} Hz"
        print(f"{summary.path}: {summary.duration_seconds:.2f}s, {summary.sample_rate} Hz, f0={f0}")


if __name__ == "__main__":
    main()
