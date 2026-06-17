use crate::{CypherNotation, NoteEvent, Pitch};

pub const PPQ: i64 = 960;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(pub i64);

impl Tick {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: i64) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dur {
    ticks: i64,
}

impl Dur {
    pub const ZERO: Self = Self { ticks: 0 };
    pub const WHOLE: Self = Self { ticks: PPQ * 4 };
    pub const HALF: Self = Self { ticks: PPQ * 2 };
    pub const QUARTER: Self = Self { ticks: PPQ };
    pub const EIGHTH: Self = Self { ticks: PPQ / 2 };
    pub const SIXTEENTH: Self = Self { ticks: PPQ / 4 };

    pub const fn ticks(self) -> i64 {
        self.ticks
    }

    pub const fn from_ticks(ticks: i64) -> Self {
        Self { ticks }
    }

    pub const fn dotted(self) -> Self {
        Self {
            ticks: self.ticks + self.ticks / 2,
        }
    }

    pub const fn tuplet(self, count: i64, in_space_of: i64) -> Self {
        Self {
            ticks: self.ticks * in_space_of / count,
        }
    }

    pub fn split_even(self, count: usize) -> Vec<Self> {
        assert!(count > 0, "split count must be non-zero");
        assert!(
            self.ticks % count as i64 == 0,
            "duration cannot be evenly split"
        );
        vec![
            Self {
                ticks: self.ticks / count as i64,
            };
            count
        ]
    }

    pub fn pattern(self, weights: impl IntoIterator<Item = i64>) -> Vec<Self> {
        let weights: Vec<_> = weights.into_iter().collect();
        assert!(!weights.is_empty(), "duration pattern must not be empty");
        assert!(
            weights.iter().all(|weight| *weight > 0),
            "duration pattern weights must be positive"
        );
        let total: i64 = weights.iter().sum();
        assert!(
            self.ticks % total == 0,
            "duration cannot be split by this pattern"
        );
        let unit = self.ticks / total;
        weights
            .into_iter()
            .map(|weight| Self {
                ticks: unit * weight,
            })
            .collect()
    }
}

impl std::ops::Add for Dur {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            ticks: self.ticks + rhs.ticks,
        }
    }
}

impl std::ops::Mul<i64> for Dur {
    type Output = Self;

    fn mul(self, rhs: i64) -> Self::Output {
        Self {
            ticks: self.ticks * rhs,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tempo {
    pub quarter_millis: u64,
}

impl Tempo {
    pub const fn from_quarter_millis(quarter_millis: u64) -> Self {
        Self { quarter_millis }
    }

    pub fn bpm(bpm: f64) -> Self {
        assert!(bpm.is_finite() && bpm > 0.0, "bpm must be positive");
        Self {
            quarter_millis: (60_000.0 / bpm).round() as u64,
        }
    }

    pub fn ticks_to_millis(self, ticks: i64) -> u64 {
        ((ticks as i128 * self.quarter_millis as i128) / PPQ as i128) as u64
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Composition {
    pub title: String,
    pub voices: Vec<Voice>,
}

impl Composition {
    pub fn with_tempo(self, tempo: Tempo) -> ComposedScore {
        ComposedScore {
            title: self.title,
            tempo,
            voices: self.voices,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedScore {
    pub title: String,
    pub tempo: Tempo,
    pub voices: Vec<Voice>,
}

impl ComposedScore {
    pub fn expand(&self) -> Timeline {
        let mut events = Vec::new();

        for voice in &self.voices {
            let mut cursor = Tick::ZERO;
            for item in &voice.items {
                match item {
                    ScoreItem::Note(note) => {
                        let grace_total_ticks = note
                            .grace_before
                            .iter()
                            .map(|grace| grace.dur.ticks())
                            .sum::<i64>();
                        let mut grace_cursor = Tick(cursor.0 - grace_total_ticks);
                        for grace in &note.grace_before {
                            events.push(TimelineEvent {
                                onset: grace_cursor,
                                duration: grace.dur,
                                midi_note: grace.midi_note,
                                velocity: grace.velocity,
                                voice: voice.name.clone(),
                                kind: EventKind::Grace,
                                tie: Tie::None,
                                slur: false,
                            });
                            grace_cursor.0 += grace.dur.ticks();
                        }

                        events.push(TimelineEvent {
                            onset: cursor,
                            duration: note.dur,
                            midi_note: note.midi_note,
                            velocity: note.velocity,
                            voice: voice.name.clone(),
                            kind: EventKind::Main,
                            tie: note.tie,
                            slur: note.slur,
                        });
                        cursor.0 += note.dur.ticks();
                    }
                    ScoreItem::Rest(dur) => {
                        cursor.0 += dur.ticks();
                    }
                }
            }
        }

        events.sort_by_key(|event| (event.onset.0, event.voice.clone(), event.midi_note));
        Timeline {
            tempo: self.tempo,
            events,
        }
    }

    pub fn to_note_events(&self) -> Vec<NoteEvent> {
        assert_eq!(
            self.voices.len(),
            1,
            "legacy NoteEvent conversion only supports one voice"
        );
        self.voices[0]
            .items
            .iter()
            .map(|item| match item {
                ScoreItem::Note(note) => {
                    NoteEvent::new(note.midi_note, self.tempo.ticks_to_millis(note.dur.ticks()))
                }
                ScoreItem::Rest(dur) => NoteEvent::rest(self.tempo.ticks_to_millis(dur.ticks())),
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    pub name: String,
    pub items: Vec<ScoreItem>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScoreItem {
    Note(ComposedNote),
    Rest(Dur),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComposedNote {
    pub midi_note: i32,
    pub dur: Dur,
    pub velocity: f32,
    pub grace_before: Vec<GraceNote>,
    pub tie: Tie,
    pub slur: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tie {
    None,
    Start,
    Continue,
    End,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraceNote {
    pub midi_note: i32,
    pub dur: Dur,
    pub velocity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Timeline {
    pub tempo: Tempo,
    pub events: Vec<TimelineEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineEvent {
    pub onset: Tick,
    pub duration: Dur,
    pub midi_note: i32,
    pub velocity: f32,
    pub voice: String,
    pub kind: EventKind,
    pub tie: Tie,
    pub slur: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    Main,
    Grace,
}

#[derive(Debug, Clone)]
pub struct ScoreBuilder {
    title: String,
    notation: CypherNotation,
    voices: Vec<Voice>,
}

impl ScoreBuilder {
    pub fn cypher(title: impl Into<String>, key: Pitch) -> Self {
        Self {
            title: title.into(),
            notation: CypherNotation::new(key),
            voices: Vec::new(),
        }
    }

    pub fn voice(mut self, name: impl Into<String>, build: impl FnOnce(&mut VoiceBuilder)) -> Self {
        let mut voice = VoiceBuilder {
            notation: self.notation,
            items: Vec::new(),
        };
        build(&mut voice);
        self.voices.push(Voice {
            name: name.into(),
            items: voice.items,
        });
        self
    }

    pub fn finish(self) -> Composition {
        Composition {
            title: self.title,
            voices: self.voices,
        }
    }
}

pub struct VoiceBuilder {
    notation: CypherNotation,
    items: Vec<ScoreItem>,
}

impl VoiceBuilder {
    pub fn n(&mut self, note: i32, octave: i32, dur: Dur) -> &mut Self {
        self.items.push(ScoreItem::Note(ComposedNote {
            midi_note: self.notation.midi(note, octave),
            dur,
            velocity: 1.0,
            grace_before: Vec::new(),
            tie: Tie::None,
            slur: false,
        }));
        self
    }

    pub fn rest(&mut self, dur: Dur) -> &mut Self {
        self.items.push(ScoreItem::Rest(dur));
        self
    }

    pub fn grace_before(&mut self, notes: impl IntoIterator<Item = GraceNote>) -> &mut Self {
        let Some(ScoreItem::Note(note)) = self.items.last_mut() else {
            panic!("grace_before must follow a note");
        };
        note.grace_before.extend(notes);
        self
    }

    pub fn velocity(&mut self, velocity: f32) -> &mut Self {
        let Some(ScoreItem::Note(note)) = self.items.last_mut() else {
            panic!("velocity must follow a note");
        };
        note.velocity = velocity;
        self
    }

    pub fn tie(&mut self, tie: Tie) -> &mut Self {
        let Some(ScoreItem::Note(note)) = self.items.last_mut() else {
            panic!("tie must follow a note");
        };
        note.tie = tie;
        self
    }

    pub fn slur(&mut self) -> &mut Self {
        let Some(ScoreItem::Note(note)) = self.items.last_mut() else {
            panic!("slur must follow a note");
        };
        note.slur = true;
        self
    }

    pub fn triplet(&mut self, build: impl FnOnce(&mut TupletBuilder)) -> &mut Self {
        self.tuplet(3, Dur::QUARTER * 2, build)
    }

    pub fn tuplet(
        &mut self,
        count: usize,
        total: Dur,
        build: impl FnOnce(&mut TupletBuilder),
    ) -> &mut Self {
        assert!(count > 0, "tuplet count must be non-zero");
        assert!(
            total.ticks() % count as i64 == 0,
            "tuplet duration cannot be evenly divided"
        );
        let mut tuplet = TupletBuilder {
            notation: self.notation,
            dur: Dur::from_ticks(total.ticks() / count as i64),
            items: Vec::new(),
        };
        build(&mut tuplet);
        self.items.extend(tuplet.items);
        self
    }

    pub fn repeat(&mut self, times: usize, build: impl Fn(&mut VoiceBuilder)) -> &mut Self {
        for _ in 0..times {
            build(self);
        }
        self
    }

    pub fn durs(
        &mut self,
        durs: impl IntoIterator<Item = Dur>,
        mut build: impl FnMut(&mut VoiceBuilder, Dur),
    ) -> &mut Self {
        for dur in durs {
            build(self, dur);
        }
        self
    }
}

pub struct TupletBuilder {
    notation: CypherNotation,
    dur: Dur,
    items: Vec<ScoreItem>,
}

impl TupletBuilder {
    pub fn n(&mut self, note: i32, octave: i32) -> &mut Self {
        self.items.push(ScoreItem::Note(ComposedNote {
            midi_note: self.notation.midi(note, octave),
            dur: self.dur,
            velocity: 1.0,
            grace_before: Vec::new(),
            tie: Tie::None,
            slur: false,
        }));
        self
    }

    pub fn rest(&mut self) -> &mut Self {
        self.items.push(ScoreItem::Rest(self.dur));
        self
    }
}

pub fn g(midi_note: i32, dur: Dur) -> GraceNote {
    GraceNote {
        midi_note,
        dur,
        velocity: 0.55,
    }
}
