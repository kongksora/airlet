use airlet::{BoxTine, PlaybackConfig, TineSink, play_events_realtime, songs};
use rodio::Source;
use std::{num::NonZero, time::Duration};

struct RodioSink<'a> {
    mixer: &'a rodio::mixer::Mixer,
}

impl TineSink for RodioSink<'_> {
    fn add_tine(&mut self, tine: BoxTine, gain: f32) {
        self.mixer.add(TineSource(tine).amplify(gain));
    }
}

struct TineSource(BoxTine);

impl Iterator for TineSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

impl Source for TineSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.0.current_span_len())
    }

    fn channels(&self) -> NonZero<u16> {
        NonZero::new(1).unwrap()
    }

    fn sample_rate(&self) -> NonZero<u32> {
        self.0.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
    let sample_rate = stream_handle.config().sample_rate();
    let mut sink = RodioSink {
        mixer: stream_handle.mixer(),
    };

    play_events_realtime(
        &songs::air::intro_melody(),
        sample_rate,
        &mut sink,
        &PlaybackConfig::default(),
    );
    Ok(())
}
