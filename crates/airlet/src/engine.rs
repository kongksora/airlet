use std::{num::NonZero, time::Duration};

use rand::{SeedableRng, rngs::StdRng};

use crate::{
    BoxTine, ModalTine, TineParams,
    audio::RenderedAudio,
    midi_to_freq,
    performance::{ModelPreset, PerformancePlan},
    render_config::ModelPlaybackConfig,
    score::{Timeline, TimelineEvent},
};

#[derive(Debug, Clone)]
pub struct Engine {
    sample_rate: NonZero<u32>,
    seed: u64,
}

impl Engine {
    pub fn new(sample_rate: NonZero<u32>) -> Self {
        Self {
            sample_rate,
            seed: 0xA17E_7001,
        }
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = seed;
        self
    }

    pub fn render(&self, plan: &PerformancePlan) -> RenderedAudio {
        RenderedAudio::mono(self.sample_rate, self.render_samples(plan))
    }

    pub fn render_samples(&self, plan: &PerformancePlan) -> Vec<f32> {
        let score = plan.composed_score();
        let timeline = score.expand();
        match plan.model_preset {
            ModelPreset::Legacy => self.render_legacy_timeline(
                &timeline,
                plan.legacy_playback().note_tail,
                plan.legacy_playback().note_gain,
                plan.legacy_playback().final_tail,
                plan.legacy_tine(),
            ),
            ModelPreset::ADry => self.render_model_timeline(&timeline, &plan.model_playback()),
        }
    }

    pub fn source(&self, plan: &PerformancePlan) -> std::vec::IntoIter<f32> {
        self.render_samples(plan).into_iter()
    }

    fn render_legacy_timeline(
        &self,
        timeline: &Timeline,
        note_tail: Duration,
        note_gain: f32,
        final_tail: Duration,
        tine: TineParams,
    ) -> Vec<f32> {
        let mut output = timeline_output_buffer(timeline, self.sample_rate, note_tail, final_tail);
        let shift_ticks = timeline_start_shift_ticks(timeline);
        let mut rng = StdRng::seed_from_u64(self.seed);

        for event in playable_events(timeline) {
            let cursor = timeline_sample_offset(event, timeline, shift_ticks, self.sample_rate);
            let freq = midi_to_freq(event.midi_note);
            let tine =
                BoxTine::new_with_rng(freq, self.sample_rate, note_tail, tine.clone(), &mut rng);
            for (offset, sample) in tine.enumerate() {
                if let Some(out) = output.get_mut(cursor + offset) {
                    *out += sample * note_gain * event.velocity;
                }
            }
        }

        output
    }

    fn render_model_timeline(&self, timeline: &Timeline, config: &ModelPlaybackConfig) -> Vec<f32> {
        let mut output = timeline_output_buffer(
            timeline,
            self.sample_rate,
            config.note_tail,
            config.final_tail,
        );
        let shift_ticks = timeline_start_shift_ticks(timeline);
        let mut rng = StdRng::seed_from_u64(config.seed);

        for event in playable_events(timeline) {
            let cursor = timeline_sample_offset(event, timeline, shift_ticks, self.sample_rate);
            let freq = midi_to_freq(event.midi_note);
            let tine = ModalTine::new_with_rng(
                freq,
                self.sample_rate,
                config.note_tail,
                config.model.clone(),
                &mut rng,
            );
            for (offset, sample) in tine.enumerate() {
                if let Some(out) = output.get_mut(cursor + offset) {
                    *out += sample * config.note_gain * event.velocity;
                }
            }
        }

        output
    }
}

fn playable_events(timeline: &Timeline) -> impl Iterator<Item = &TimelineEvent> {
    timeline.events.iter().filter(|event| event.midi_note > 0)
}

fn timeline_start_shift_ticks(timeline: &Timeline) -> i64 {
    timeline
        .events
        .iter()
        .map(|event| event.onset.0)
        .min()
        .unwrap_or(0)
        .min(0)
        .abs()
}

fn timeline_output_buffer(
    timeline: &Timeline,
    sample_rate: NonZero<u32>,
    note_tail: Duration,
    final_tail: Duration,
) -> Vec<f32> {
    let shift_ticks = timeline_start_shift_ticks(timeline);
    let end_ticks = timeline
        .events
        .iter()
        .map(|event| event.onset.0 + event.duration.ticks() + shift_ticks)
        .max()
        .unwrap_or(0);
    let end_millis = timeline.tempo.ticks_to_millis(end_ticks);
    let total_duration = Duration::from_millis(end_millis) + note_tail + final_tail;
    let total_samples = (total_duration.as_secs_f64() * sample_rate.get() as f64).ceil() as usize;
    vec![0.0; total_samples]
}

fn timeline_sample_offset(
    event: &TimelineEvent,
    timeline: &Timeline,
    shift_ticks: i64,
    sample_rate: NonZero<u32>,
) -> usize {
    let shifted_ticks = event.onset.0 + shift_ticks;
    let millis = timeline.tempo.ticks_to_millis(shifted_ticks);
    ((millis as f64 / 1000.0) * sample_rate.get() as f64).round() as usize
}
