use std::{
    num::NonZero,
    time::{Duration, Instant},
};

use rand::{SeedableRng, rngs::StdRng};

use crate::{BoxTine, ModalTine, TineParams, midi_to_freq, songs};

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
pub struct Score {
    pub title: &'static str,
    pub events: Vec<NoteEvent>,
}

impl Score {
    pub fn new(title: &'static str, events: Vec<NoteEvent>) -> Self {
        Self { title, events }
    }

    pub fn duration(&self) -> Duration {
        Duration::from_millis(self.events.iter().map(|event| event.millis).sum())
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

#[derive(Debug, Clone)]
pub struct Performance {
    pub score: Score,
    pub playback: PlaybackConfig,
    pub tine: TineParams,
}

impl Performance {
    pub fn new(score: Score, playback: PlaybackConfig, tine: TineParams) -> Self {
        Self {
            score,
            playback,
            tine,
        }
    }

    pub fn air_intro_legacy() -> Self {
        Self::new(
            songs::air::intro(),
            PlaybackConfig::default(),
            TineParams::legacy(),
        )
    }

    pub fn render_config(&self, sample_rate: NonZero<u32>, seed: u64) -> RenderConfig {
        RenderConfig {
            sample_rate,
            playback: self.playback.clone(),
            tine: self.tine.clone(),
            seed,
        }
    }

    pub fn render(&self, sample_rate: NonZero<u32>, seed: u64) -> Vec<f32> {
        render_score(&self.score, &self.render_config(sample_rate, seed))
    }

    pub fn play_realtime<S: TineSink>(&self, sample_rate: NonZero<u32>, sink: &mut S) {
        play_performance_realtime(self, sample_rate, sink);
    }
}

pub trait TineSink {
    fn add_tine(&mut self, tine: BoxTine, gain: f32);
}

pub fn play_performance_realtime<S: TineSink>(
    performance: &Performance,
    sample_rate: NonZero<u32>,
    sink: &mut S,
) {
    play_events_realtime_with_tine(
        &performance.score.events,
        sample_rate,
        sink,
        &performance.playback,
        &performance.tine,
    );
}

pub fn play_events_realtime<S: TineSink>(
    events: &[NoteEvent],
    sample_rate: NonZero<u32>,
    sink: &mut S,
    config: &PlaybackConfig,
) {
    play_events_realtime_with_tine(events, sample_rate, sink, config, &TineParams::legacy());
}

pub fn play_events_realtime_with_tine<S: TineSink>(
    events: &[NoteEvent],
    sample_rate: NonZero<u32>,
    sink: &mut S,
    config: &PlaybackConfig,
    tine_params: &TineParams,
) {
    let begin = Instant::now();
    let mut total_millis: i64 = 0;

    for event in events {
        if !event.is_rest() {
            let freq = midi_to_freq(event.midi_note);
            let note =
                BoxTine::new_with_params(freq, sample_rate, config.note_tail, tine_params.clone());
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

#[derive(Debug, Clone)]
pub struct RenderConfig {
    pub sample_rate: NonZero<u32>,
    pub playback: PlaybackConfig,
    pub tine: TineParams,
    pub seed: u64,
}

impl RenderConfig {
    pub fn new(sample_rate: NonZero<u32>) -> Self {
        Self {
            sample_rate,
            playback: PlaybackConfig::default(),
            tine: TineParams::legacy(),
            seed: 0xA17E_7001,
        }
    }
}

pub fn render_score(score: &Score, config: &RenderConfig) -> Vec<f32> {
    render_events(&score.events, config)
}

pub fn render_events(events: &[NoteEvent], config: &RenderConfig) -> Vec<f32> {
    let sample_rate = config.sample_rate.get() as f64;
    let song_millis: u64 = events.iter().map(|event| event.millis).sum();
    let total_duration =
        Duration::from_millis(song_millis) + config.playback.note_tail + config.playback.final_tail;
    let total_samples = (total_duration.as_secs_f64() * sample_rate).ceil() as usize;
    let mut output = vec![0.0; total_samples];
    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut cursor_samples = 0usize;

    for event in events {
        if !event.is_rest() {
            let freq = midi_to_freq(event.midi_note);
            let tine = BoxTine::new_with_rng(
                freq,
                config.sample_rate,
                config.playback.note_tail,
                config.tine.clone(),
                &mut rng,
            );

            for (offset, sample) in tine.enumerate() {
                if let Some(out) = output.get_mut(cursor_samples + offset) {
                    *out += sample * config.playback.note_gain;
                }
            }
        }

        cursor_samples += millis_to_samples(event.millis, config.sample_rate);
    }

    output
}

pub fn render_events_with_model(
    events: &[NoteEvent],
    config: &ModelPlaybackConfig,
    sample_rate: NonZero<u32>,
) -> Vec<f32> {
    let sample_rate_f64 = sample_rate.get() as f64;
    let song_millis: u64 = events.iter().map(|event| event.millis).sum();
    let total_duration = Duration::from_millis(song_millis) + config.note_tail + config.final_tail;
    let total_samples = (total_duration.as_secs_f64() * sample_rate_f64).ceil() as usize;
    let mut output = vec![0.0; total_samples];
    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut cursor_samples = 0usize;

    for event in events {
        if !event.is_rest() {
            let freq = midi_to_freq(event.midi_note);
            let tine = ModalTine::new_with_rng(
                freq,
                sample_rate,
                config.note_tail,
                config.model.clone(),
                &mut rng,
            );

            for (offset, sample) in tine.enumerate() {
                if let Some(out) = output.get_mut(cursor_samples + offset) {
                    *out += sample * config.note_gain;
                }
            }
        }

        cursor_samples += millis_to_samples(event.millis, sample_rate);
    }

    output
}

pub fn render_air_intro_a_dry(sample_rate: NonZero<u32>) -> Vec<f32> {
    render_events_with_model(
        &songs::air::intro().events,
        &ModelPlaybackConfig::air_dry(),
        sample_rate,
    )
}

pub use crate::render_config::ModelPlaybackConfig;

fn millis_to_samples(millis: u64, sample_rate: NonZero<u32>) -> usize {
    ((millis as f64 / 1000.0) * sample_rate.get() as f64).round() as usize
}
