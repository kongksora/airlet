from __future__ import annotations

from pathlib import Path

import numpy as np
import soundfile as sf


SUPPORTED_AUDIO = {".wav", ".flac", ".aiff", ".aif"}


def load_mono(path: Path) -> tuple[np.ndarray, int]:
    data, sample_rate = sf.read(path, always_2d=True, dtype="float64")
    mono = np.mean(data, axis=1)
    return mono, int(sample_rate)


def write_wav(path: Path, samples: np.ndarray, sample_rate: int) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    sf.write(path, samples.astype(np.float32), sample_rate, subtype="PCM_16")


def audio_files(root: Path) -> list[Path]:
    return sorted(
        path
        for path in root.iterdir()
        if path.is_file() and path.suffix.lower() in SUPPORTED_AUDIO
    )
