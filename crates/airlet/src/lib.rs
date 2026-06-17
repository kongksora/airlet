use std::{
    num::NonZero,
    time::{Duration, Instant},
};

pub mod songs;

// S(t) = amp * exp(-k*t) * sin(2πf*t + beta*exp(-gamma*t)*sin(2πm*t) + phi0)
#[derive(Debug, Clone)]
pub struct Single {
    amp: f32,
    k: f32,
    freq: f32,
    beta: f32,
    gamma: f32,
    m: f32,
    phi: f32,
}

const K_FACTOR: f32 = 5e-3;
const BETA_FACTOR: f32 = 80.0;
const GAMMA_FACTOR: f32 = 5e-2;
const M_FACTOR: f32 = 20.0;

impl Single {
    pub fn new(freq: f32, amp: f32) -> Self {
        Self {
            freq,
            amp,
            k: freq * K_FACTOR,
            beta: BETA_FACTOR / freq,
            gamma: freq * GAMMA_FACTOR,
            m: freq * M_FACTOR,
            phi: rand::random::<f32>() * 2.0 * std::f32::consts::PI,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoxTine {
    modes: Vec<Single>,
    sample_rate: NonZero<u32>,
    duration_samples: usize,
    current_sample: usize,
}

impl BoxTine {
    pub fn new(freq: f32, sample_rate: NonZero<u32>, duration: Duration) -> Self {
        let mut modes = Vec::new();

        let freq = freq * (1.0 + rand::random::<f32>() * 0.001 - 0.005);
        let partials = [
            1.0, // 基频（最强）
            2.99, 5.01, 7.02, 10.03, 12.04, 15.05, 17.06, 20.07,
        ];

        for part in &partials {
            modes.push(Single::new(freq * part, 1.0 / part.powi(2)));
        }

        Self {
            modes,
            sample_rate,
            duration_samples: (sample_rate.get() as f64 * duration.as_secs_f64()) as usize,
            current_sample: 0,
        }
    }

    pub fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }

    pub fn current_span_len(&self) -> usize {
        self.duration_samples - self.current_sample
    }
}

fn smoothstep(t: f32) -> f32 {
    t * t * 3.0 - t * t * t * 2.0
}

impl Iterator for BoxTine {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.current_sample >= self.duration_samples {
            return None;
        }

        let sr = self.sample_rate.get() as f32;
        let t = self.current_sample as f32 / sr;
        let attack = 0.002;
        let mut out = 0.0;

        for m in &mut self.modes {
            let modul = m.beta
                * (-m.gamma * t).exp()
                * (2.0 * std::f32::consts::PI * m.m * t + m.phi).sin();
            let phase = 2.0 * std::f32::consts::PI * m.freq * t + modul + m.phi;
            out += m.amp * (-m.k * t).exp() * phase.sin();
        }

        let out = (out * 1.4).tanh() * 0.4;

        self.current_sample += 1;
        let out = out * smoothstep((t / attack).min(1.0));

        Some(out)
    }
}

pub fn midi_to_freq(midi_note: i32) -> f32 {
    440.0 * 2.0_f32.powf((midi_note as f32 - 69.0) / 12.0)
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum Pitch {
    C,
    CSharp,
    DFlat,
    D,
    DSharp,
    EFlat,
    E,
    F,
    FSharp,
    GFlat,
    G,
    GSharp,
    AFlat,
    A,
    ASharp,
    BFlat,
    B,
}

impl Pitch {
    pub const fn to_midi(&self, octave: i32) -> i32 {
        let base = match self {
            Pitch::C => 0,
            Pitch::CSharp | Pitch::DFlat => 1,
            Pitch::D => 2,
            Pitch::DSharp | Pitch::EFlat => 3,
            Pitch::E => 4,
            Pitch::F => 5,
            Pitch::FSharp | Pitch::GFlat => 6,
            Pitch::G => 7,
            Pitch::GSharp | Pitch::AFlat => 8,
            Pitch::A => 9,
            Pitch::ASharp | Pitch::BFlat => 10,
            Pitch::B => 11,
        };
        base + (octave + 1) * 12
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CypherNotation {
    key: Pitch,
}

impl CypherNotation {
    pub fn new(key: Pitch) -> Self {
        Self { key }
    }

    pub fn midi(&self, note: i32, octave: i32) -> i32 {
        let offset = match note {
            1 => 0,
            2 => 2,
            3 => 4,
            4 => 5,
            5 => 7,
            6 => 9,
            7 => 11,
            _ => panic!("Invalid note: {}", note),
        };
        self.key.to_midi(4 + octave) + offset
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NoteEvent {
    pub midi_note: i32,
    pub millis: u64,
}

impl NoteEvent {
    pub const fn rest(millis: u64) -> Self {
        Self {
            midi_note: 0,
            millis,
        }
    }

    pub const fn new(midi_note: i32, millis: u64) -> Self {
        Self { midi_note, millis }
    }

    pub const fn is_rest(&self) -> bool {
        self.midi_note <= 0
    }
}

#[derive(Debug, Clone)]
pub struct PlaybackConfig {
    pub note_tail: Duration,
    pub note_gain: f32,
    pub final_tail: Duration,
}

impl Default for PlaybackConfig {
    fn default() -> Self {
        Self {
            note_tail: Duration::from_secs(10),
            note_gain: 0.25,
            final_tail: Duration::from_secs(5),
        }
    }
}

pub trait TineSink {
    fn add_tine(&mut self, tine: BoxTine, gain: f32);
}

pub fn play_events_realtime<S: TineSink>(
    events: &[NoteEvent],
    sample_rate: NonZero<u32>,
    sink: &mut S,
    config: &PlaybackConfig,
) {
    let begin = Instant::now();
    let mut total_millis: i64 = 0;

    for event in events {
        if !event.is_rest() {
            let freq = midi_to_freq(event.midi_note);
            let note = BoxTine::new(freq, sample_rate, config.note_tail);
            sink.add_tine(note, config.note_gain);
        }
        total_millis += event.millis as i64;
        let to_sleep_millis = total_millis - begin.elapsed().as_millis() as i64;
        if to_sleep_millis > 0 {
            std::thread::sleep(Duration::from_millis(to_sleep_millis as u64));
        }
    }

    std::thread::sleep(config.final_tail);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_69_is_a4() {
        assert!((midi_to_freq(69) - 440.0).abs() < f32::EPSILON);
    }

    #[test]
    fn air_intro_track_keeps_current_length() {
        assert_eq!(songs::air::intro_melody().len(), 43);
    }
}
