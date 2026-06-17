from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True)
class NoteEvent:
    midi_note: int
    millis: int

    @property
    def is_rest(self) -> bool:
        return self.midi_note <= 0


PITCH_OFFSETS = {
    "C": 0,
    "C#": 1,
    "Db": 1,
    "D": 2,
    "D#": 3,
    "Eb": 3,
    "E": 4,
    "F": 5,
    "F#": 6,
    "Gb": 6,
    "G": 7,
    "G#": 8,
    "Ab": 8,
    "A": 9,
    "A#": 10,
    "Bb": 10,
    "B": 11,
}


def midi_to_freq(midi_note: int) -> float:
    return 440.0 * 2.0 ** ((midi_note - 69.0) / 12.0)


def pitch_to_midi(pitch: str, octave: int) -> int:
    return PITCH_OFFSETS[pitch] + (octave + 1) * 12


def cypher_midi(key: str, note: int, octave: int) -> int:
    offsets = {
        1: 0,
        2: 2,
        3: 4,
        4: 5,
        5: 7,
        6: 9,
        7: 11,
    }
    return pitch_to_midi(key, 4 + octave) + offsets[note]


def air_intro_events() -> list[NoteEvent]:
    qn = 500
    en = qn // 2
    sn = qn // 4
    midi = lambda note, oct: cypher_midi("D", note, oct)

    return [
        NoteEvent(midi(6, 0), qn + en),
        NoteEvent(midi(7, 0), en),
        NoteEvent(midi(1, 1), en),
        NoteEvent(midi(5, 1), en),
        NoteEvent(midi(3, 1), qn),
        NoteEvent(midi(3, 1), en),
        NoteEvent(midi(2, 1), sn),
        NoteEvent(midi(3, 1), sn + qn),
        NoteEvent(0, qn),
        NoteEvent(0, qn),
        NoteEvent(midi(2, 1), en),
        NoteEvent(midi(3, 1), en),
        NoteEvent(midi(5, 1), en),
        NoteEvent(midi(1, 1), en),
        NoteEvent(midi(7, 0), en),
        NoteEvent(midi(1, 1), en),
        NoteEvent(midi(7, 0), qn),
        NoteEvent(midi(7, 0), en),
        NoteEvent(midi(6, 0), sn),
        NoteEvent(midi(3, 0), sn + qn),
        NoteEvent(0, qn),
        NoteEvent(0, qn + en),
        NoteEvent(midi(6, 0), qn),
        NoteEvent(midi(7, 0), en),
        NoteEvent(midi(1, 1), en),
        NoteEvent(midi(5, 1), en),
        NoteEvent(midi(3, 1), qn),
        NoteEvent(midi(3, 1), en),
        NoteEvent(midi(2, 1), sn),
        NoteEvent(midi(3, 1), sn + qn),
        NoteEvent(0, qn),
        NoteEvent(0, qn),
        NoteEvent(midi(2, 1), en),
        NoteEvent(midi(3, 1), en),
        NoteEvent(midi(5, 1), en),
        NoteEvent(midi(3, 1), en),
        NoteEvent(midi(5, 1), en),
        NoteEvent(midi(1, 2), en),
        NoteEvent(midi(7, 1), en + sn),
        NoteEvent(midi(6, 1), en + sn),
        NoteEvent(midi(3, 1), qn + en),
        NoteEvent(0, en),
        NoteEvent(midi(2, 1), en),
    ]
