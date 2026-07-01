pub mod audio;
pub mod compat;
pub mod defaults;
pub mod engine;
pub mod mechanism;
pub mod model;
pub mod notation;
pub mod performance;
pub mod preset;
pub mod render_config;
pub mod score;
pub mod songs;
pub mod synthesis;

// Re-exports for backward compatibility
pub use notation::{CypherNotation, Pitch};
pub use synthesis::{BoxTine, ModalTine, Single, TineParams, midi_to_freq, normalize_peak};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compat::{
        ModelPlaybackConfig, NoteEvent, Performance, PlaybackConfig, RenderConfig, render_events,
        render_events_with_model,
    };
    use std::num::NonZero;
    use std::time::Duration;

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
        use crate::model::MusicBoxModel;

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
