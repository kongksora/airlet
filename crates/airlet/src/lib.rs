use std::{num::NonZero, time::Duration};

use rand::{Rng, RngExt, SeedableRng, rngs::StdRng};

pub mod audio;
pub mod compat;
pub mod defaults;
pub mod engine;
pub mod mechanism;
pub mod model;
pub mod performance;
pub mod preset;
pub mod score;
pub mod songs;

use model::MusicBoxModel;

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
        Self::new_with_phase(
            freq,
            amp,
            rand::random::<f32>() * 2.0 * std::f32::consts::PI,
        )
    }

    fn new_with_rng<R: Rng + ?Sized>(freq: f32, amp: f32, rng: &mut R) -> Self {
        Self::new_with_phase(freq, amp, rng.random::<f32>() * 2.0 * std::f32::consts::PI)
    }

    fn new_with_phase(freq: f32, amp: f32, phi: f32) -> Self {
        Self {
            freq,
            amp,
            k: freq * K_FACTOR,
            beta: BETA_FACTOR / freq,
            gamma: freq * GAMMA_FACTOR,
            m: freq * M_FACTOR,
            phi,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TineParams {
    pub partials: Vec<f32>,
    pub attack: f32,
    pub output_drive: f32,
    pub output_gain: f32,
    pub detune_offset: f32,
    pub detune_random_span: f32,
}

impl TineParams {
    pub fn legacy() -> Self {
        Self {
            partials: vec![1.0, 2.99, 5.01, 7.02, 10.03, 12.04, 15.05, 17.06, 20.07],
            attack: 0.002,
            output_drive: 1.4,
            output_gain: 0.4,
            detune_offset: -0.005,
            detune_random_span: 0.001,
        }
    }
}

impl Default for TineParams {
    fn default() -> Self {
        Self::legacy()
    }
}

#[derive(Debug, Clone)]
pub struct BoxTine {
    modes: Vec<Single>,
    sample_rate: NonZero<u32>,
    duration_samples: usize,
    current_sample: usize,
    params: TineParams,
}

impl BoxTine {
    pub fn new(freq: f32, sample_rate: NonZero<u32>, duration: Duration) -> Self {
        Self::new_with_params(freq, sample_rate, duration, TineParams::legacy())
    }

    pub fn new_with_params(
        freq: f32,
        sample_rate: NonZero<u32>,
        duration: Duration,
        params: TineParams,
    ) -> Self {
        let mut rng = rand::rng();
        Self::new_with_rng(freq, sample_rate, duration, params, &mut rng)
    }

    pub fn new_with_seed(
        freq: f32,
        sample_rate: NonZero<u32>,
        duration: Duration,
        params: TineParams,
        seed: u64,
    ) -> Self {
        let mut rng = StdRng::seed_from_u64(seed);
        Self::new_with_rng(freq, sample_rate, duration, params, &mut rng)
    }

    pub fn new_with_rng<R: Rng + ?Sized>(
        freq: f32,
        sample_rate: NonZero<u32>,
        duration: Duration,
        params: TineParams,
        rng: &mut R,
    ) -> Self {
        let mut modes = Vec::new();

        let freq =
            freq * (1.0 + rng.random::<f32>() * params.detune_random_span + params.detune_offset);

        for part in &params.partials {
            modes.push(Single::new_with_rng(freq * part, 1.0 / part.powi(2), rng));
        }

        Self {
            modes,
            sample_rate,
            duration_samples: (sample_rate.get() as f64 * duration.as_secs_f64()) as usize,
            current_sample: 0,
            params,
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
        let mut out = 0.0;

        for m in &mut self.modes {
            let modul = m.beta
                * (-m.gamma * t).exp()
                * (2.0 * std::f32::consts::PI * m.m * t + m.phi).sin();
            let phase = 2.0 * std::f32::consts::PI * m.freq * t + modul + m.phi;
            out += m.amp * (-m.k * t).exp() * phase.sin();
        }

        let out = (out * self.params.output_drive).tanh() * self.params.output_gain;

        self.current_sample += 1;
        let out = out * smoothstep((t / self.params.attack).min(1.0));

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

pub fn normalize_peak(samples: &mut [f32], peak: f32) {
    let max = samples
        .iter()
        .fold(0.0_f32, |max, sample| max.max(sample.abs()));

    if max > peak && max > 0.0 {
        let gain = peak / max;
        for sample in samples {
            *sample *= gain;
        }
    }
}

#[derive(Debug, Clone)]
struct ModalMode {
    freq: f32,
    amp: f32,
    decay: f32,
    phase: f32,
}

#[derive(Debug)]
pub struct ModalTine {
    modes: Vec<ModalMode>,
    model: MusicBoxModel,
    sample_rate: NonZero<u32>,
    duration_samples: usize,
    current_sample: usize,
    click_phase: f32,
    rng: StdRng,
}

impl ModalTine {
    pub fn new_with_rng<R: Rng + ?Sized>(
        freq: f32,
        sample_rate: NonZero<u32>,
        duration: Duration,
        model: MusicBoxModel,
        rng: &mut R,
    ) -> Self {
        let mut modes = Vec::with_capacity(model.tines.partials.len());

        for (index, partial) in model.tines.partials.iter().enumerate() {
            let detune = 2.0_f32.powf(normal01(rng) * model.tines.detune_cents / 1200.0);
            let stretched = 1.0 + model.tines.stretch * index as f32 * index as f32;
            let mode_freq = freq * partial.frequency_ratio * detune * stretched;
            let freq_ratio = partial.frequency_ratio.max(0.1);
            let high_decay_scale = freq_ratio.powf(model.tines.high_decay_power);
            let low_decay = 1.0 + model.tines.low_decay_boost / freq_ratio.max(0.25);
            let decay = model.tines.base_decay_seconds * low_decay / high_decay_scale.max(1e-6)
                * partial.decay_scale;

            modes.push(ModalMode {
                freq: mode_freq,
                amp: partial.amplitude.powf(model.tines.amplitude_power),
                decay,
                phase: rng.random::<f32>() * 2.0 * std::f32::consts::PI,
            });
        }

        Self {
            modes,
            model,
            sample_rate,
            duration_samples: (sample_rate.get() as f64 * duration.as_secs_f64()) as usize,
            current_sample: 0,
            click_phase: rng.random::<f32>() * 2.0 * std::f32::consts::PI,
            rng: StdRng::seed_from_u64(rng.random::<u64>()),
        }
    }
}

impl Iterator for ModalTine {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current_sample >= self.duration_samples {
            return None;
        }

        let sr = self.sample_rate.get() as f32;
        let t = self.current_sample as f32 / sr;
        let mut out = 0.0;

        for mode in &self.modes {
            let env = (-t / mode.decay.max(1e-6)).exp();
            let phase = 2.0 * std::f32::consts::PI * mode.freq * t + mode.phase;
            out += mode.amp * env * phase.sin();
        }

        if self.model.exciter.click_gain > 0.0 {
            let anchor = self
                .modes
                .iter()
                .max_by(|a, b| a.amp.total_cmp(&b.amp))
                .map(|mode| mode.freq)
                .unwrap_or(440.0);
            let click_env = (-t / self.model.exciter.click_decay_seconds.max(1e-6)).exp();
            let click_freq = (anchor * 7.0).min(sr * 0.45);
            let click_tone = (2.0 * std::f32::consts::PI * click_freq * t + self.click_phase).sin();
            let click_noise = normal01(&mut self.rng);
            out +=
                self.model.exciter.click_gain * click_env * (0.7 * click_tone + 0.3 * click_noise);
        }

        if self.model.exciter.noise_gain > 0.0 {
            let noise_env = (-t / (self.model.tines.base_decay_seconds * 0.35).max(1e-6)).exp();
            out += self.model.exciter.noise_gain * noise_env * normal01(&mut self.rng);
        }

        let attack = smoothstep((t / self.model.exciter.attack_seconds.max(1e-6)).min(1.0));
        out *= attack;

        let drive = self.model.tines.drive;
        if drive > 0.0 {
            out = (out * drive).tanh() / drive.tanh();
        }

        self.current_sample += 1;
        Some(out * self.model.tines.output_gain)
    }
}

fn normal01<R: Rng + ?Sized>(rng: &mut R) -> f32 {
    let u1 = rng.random::<f32>().clamp(f32::MIN_POSITIVE, 1.0);
    let u2 = rng.random::<f32>();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f32::consts::PI * u2).cos()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::{
        ModelPlaybackConfig, NoteEvent, Performance, PlaybackConfig, RenderConfig, render_events,
        render_events_with_model,
    };

    #[test]
    fn midi_69_is_a4() {
        assert!((midi_to_freq(69) - 440.0).abs() < f32::EPSILON);
    }

    #[test]
    fn air_intro_track_keeps_current_length() {
        assert_eq!(songs::air::intro().events.len(), 43);
    }

    #[test]
    fn air_intro_builder_preserves_legacy_durations() {
        let events = songs::air::intro_melody();
        assert_eq!(events.len(), 43);
        assert_eq!(
            events[0],
            NoteEvent::new(CypherNotation::new(Pitch::D).midi(6, 0), 750)
        );
        assert_eq!(
            events[7],
            NoteEvent::new(CypherNotation::new(Pitch::D).midi(3, 1), 625)
        );
        assert_eq!(events[8], NoteEvent::rest(500));
        assert_eq!(
            events.last(),
            Some(&NoteEvent::new(
                CypherNotation::new(Pitch::D).midi(2, 1),
                250
            ))
        );
    }

    #[test]
    fn score_builder_expands_grace_notes_and_triplets() {
        use crate::score::{Dur, EventKind, ScoreBuilder, Tempo, g};

        let music = CypherNotation::new(Pitch::D);
        let score = ScoreBuilder::cypher("test", Pitch::D)
            .voice("melody", |v| {
                v.n(1, 0, Dur::QUARTER)
                    .grace_before([g(music.midi(7, -1), Dur::SIXTEENTH)]);
                v.triplet(|t| {
                    t.n(1, 0).n(2, 0).n(3, 0);
                });
            })
            .finish()
            .with_tempo(Tempo::from_quarter_millis(500));
        let timeline = score.expand();

        assert_eq!(Dur::QUARTER.split_even(3), vec![Dur::from_ticks(320); 3]);
        assert_eq!(timeline.events[0].kind, EventKind::Grace);
        assert_eq!(timeline.events[0].onset.0, -Dur::SIXTEENTH.ticks());
        assert_eq!(timeline.events[1].kind, EventKind::Main);
        assert_eq!(timeline.events[2].onset.0, Dur::QUARTER.ticks());
        assert_eq!(timeline.events[2].duration, Dur::QUARTER.tuplet(3, 2));
    }

    #[test]
    fn duration_and_tempo_are_decoupled() {
        use crate::score::{Dur, ScoreBuilder, Tempo};

        let composition = ScoreBuilder::cypher("tempo-free", Pitch::D)
            .voice("melody", |v| {
                v.n(1, 0, Dur::QUARTER);
            })
            .finish();
        let slow = composition
            .clone()
            .with_tempo(Tempo::from_quarter_millis(600));
        let fast = composition.with_tempo(Tempo::from_quarter_millis(300));

        assert_eq!(
            slow.expand().events[0].duration,
            fast.expand().events[0].duration
        );
        assert_eq!(slow.to_note_events()[0].millis, 600);
        assert_eq!(fast.to_note_events()[0].millis, 300);
    }

    #[test]
    fn score_dsl_supports_patterns_velocity_ties_and_repeats() {
        use crate::score::{Dur, EventKind, ScoreBuilder, Tempo, Tie};

        let composition = ScoreBuilder::cypher("dsl", Pitch::D)
            .voice("melody", |v| {
                let pattern = Dur::WHOLE.pattern([1, 1, 2]);
                v.durs(pattern, |v, dur| {
                    v.n(1, 0, dur).velocity(0.8).slur();
                });
                v.tuplet(5, Dur::QUARTER, |t| {
                    t.n(1, 0).n(2, 0).n(3, 0).n(4, 0).n(5, 0);
                });
                v.n(6, 0, Dur::QUARTER).tie(Tie::Start);
                v.repeat(2, |v| {
                    v.rest(Dur::EIGHTH);
                });
            })
            .finish();
        let timeline = composition.with_tempo(Tempo::bpm(120.0)).expand();

        assert_eq!(timeline.events[0].duration, Dur::QUARTER);
        assert_eq!(timeline.events[2].duration, Dur::HALF);
        assert_eq!(timeline.events[0].velocity, 0.8);
        assert!(timeline.events[0].slur);
        assert_eq!(timeline.events[3].duration, Dur::QUARTER.tuplet(5, 1));
        assert_eq!(timeline.events[8].tie, Tie::Start);
        assert_eq!(timeline.events[8].kind, EventKind::Main);
    }

    #[test]
    fn render_is_deterministic_for_same_seed() {
        let sample_rate = NonZero::new(8_000).unwrap();
        let config = RenderConfig {
            sample_rate,
            playback: PlaybackConfig {
                note_tail: Duration::from_millis(100),
                note_gain: 0.25,
                final_tail: Duration::from_millis(10),
            },
            tine: TineParams::legacy(),
            seed: 42,
        };
        let events = [NoteEvent::new(69, 50), NoteEvent::rest(50)];

        let first = render_events(&events, &config);
        let second = render_events(&events, &config);

        assert_eq!(first, second);
        assert!(first.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn performance_render_uses_default_song() {
        let sample_rate = NonZero::new(8_000).unwrap();
        let performance = Performance {
            playback: PlaybackConfig {
                note_tail: Duration::from_millis(20),
                note_gain: 0.25,
                final_tail: Duration::from_millis(5),
            },
            ..Performance::air_intro_legacy()
        };
        let rendered = performance.render(sample_rate, 7);

        assert!(!rendered.is_empty());
        assert!(rendered.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn model_a_dry_render_is_deterministic() {
        let sample_rate = NonZero::new(8_000).unwrap();
        let events = [NoteEvent::new(69, 50), NoteEvent::rest(50)];
        let config = ModelPlaybackConfig {
            note_tail: Duration::from_millis(100),
            final_tail: Duration::from_millis(10),
            ..ModelPlaybackConfig::air_dry()
        };

        let first = render_events_with_model(&events, &config, sample_rate);
        let second = render_events_with_model(&events, &config, sample_rate);

        assert_eq!(first, second);
        assert!(first.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn engine_renders_timeline_models() {
        use crate::{
            engine::Engine,
            performance::{ModelPreset, PerformancePlan},
        };

        let sample_rate = NonZero::new(8_000).unwrap();
        let composition = songs::air::intro_composition();
        let legacy = PerformancePlan::new(composition.clone())
            .tempo(songs::air::intro_tempo())
            .model(ModelPreset::Legacy);
        let dry = PerformancePlan::new(composition)
            .tempo(songs::air::intro_tempo())
            .model(ModelPreset::ADry);
        let engine = Engine::new(sample_rate);

        let legacy_audio = engine.render(&legacy);
        let dry_audio = engine.render(&dry);
        let dry_audio_again = engine.render(&dry);

        assert!(!legacy_audio.samples().is_empty());
        assert!(!dry_audio.samples().is_empty());
        assert_eq!(dry_audio, dry_audio_again);
        assert!(legacy_audio.is_finite());
        assert!(dry_audio.is_finite());
        assert_eq!(dry_audio.sample_rate(), sample_rate);
        assert_eq!(dry_audio.channels().get(), 1);
    }

    #[test]
    fn air_intro_timeline_onsets_are_stable() {
        let timeline = songs::air::intro_score().expand();

        assert_eq!(timeline.events.len(), 36);
        assert_eq!(timeline.events[0].onset.0, 0);
        assert_eq!(timeline.events[0].duration.ticks(), 1440);
        assert_eq!(timeline.events[1].onset.0, 1440);
        assert_eq!(timeline.events[4].onset.0, 2880);
        assert_eq!(timeline.events[7].onset.0, 4560);
        assert_eq!(timeline.events[8].onset.0, 7680);

        let last = timeline.events.last().unwrap();
        assert_eq!(last.onset.0, 29280);
        assert_eq!(last.duration.ticks(), 480);
    }

    #[test]
    fn engine_a_dry_golden_audio_stats() {
        use crate::{
            engine::Engine,
            performance::{ModelPreset, PerformancePlan},
        };

        let sample_rate = NonZero::new(8_000).unwrap();
        let plan = PerformancePlan::new(songs::air::intro_composition())
            .tempo(songs::air::intro_tempo())
            .model(ModelPreset::ADry);
        let first = Engine::new(sample_rate).render(&plan);
        let second = Engine::new(sample_rate).render(&plan);

        assert_eq!(first, second);
        assert_eq!(first.samples().len(), 140_000);
        assert_eq!(first.duration(), Duration::from_millis(17_500));
        assert!(first.is_finite());
        assert!(
            (0.005..0.5).contains(&first.peak()),
            "unexpected peak level: {}",
            first.peak()
        );
        assert!(
            (0.0001..0.08).contains(&first.rms()),
            "unexpected rms level: {}",
            first.rms()
        );
    }

    #[test]
    fn defaults_render_current_air_intro_model() {
        let sample_rate = NonZero::new(8_000).unwrap();
        let plan = defaults::air_intro_plan();
        let audio = defaults::air_intro_audio(sample_rate);

        assert_eq!(plan.model_preset, performance::ModelPreset::ADry);
        assert_eq!(audio.sample_rate(), sample_rate);
        assert_eq!(audio.channels().get(), 1);
        assert_eq!(audio.samples().len(), 140_000);
        assert!(audio.is_finite());
    }

    #[test]
    fn mechanism_planner_exports_tooth_hints() {
        use crate::mechanism::MechanismPlanner;

        let timeline = songs::air::intro_score().expand();
        let hints = MechanismPlanner::default().plan(&timeline);
        let playable = timeline
            .events
            .iter()
            .filter(|event| event.midi_note > 0)
            .count();

        assert_eq!(hints.events.len(), playable);
        assert!(hints.events.iter().all(|hint| hint.angle_rad.is_finite()));
        assert!(
            hints
                .events
                .iter()
                .all(|hint| hint.axial_position.is_finite())
        );
    }

    #[test]
    fn bundled_a_dry_preset_round_trips() {
        let from_json = MusicBoxModel::a_dry_from_json();
        let from_rust = MusicBoxModel::modal_a_dry_probe();

        assert_eq!(
            from_json.tines.partials.len(),
            from_rust.tines.partials.len()
        );
        assert_eq!(from_json.exciter.click_gain, from_rust.exciter.click_gain);

        let json = from_json.to_json_string().unwrap();
        let round_trip = MusicBoxModel::from_json_str(&json).unwrap();

        assert_eq!(
            round_trip.tines.partials.len(),
            from_json.tines.partials.len()
        );
        assert_eq!(round_trip.body, from_json.body);
    }

    #[test]
    fn bundled_preset_library_loads_a_dry() {
        let model = preset::PresetLibrary::bundled()
            .load_model(performance::ModelPreset::ADry)
            .unwrap();

        assert_eq!(model.tines.partials.len(), 8);
        assert_eq!(model.body, model::ResonatorParams::Dry);
    }
}
