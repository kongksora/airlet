use std::{num::NonZero, time::Duration};

use airlet::{
    engine::Engine,
    performance::{ModelPreset, PerformancePlan},
    songs,
};
use rodio::buffer::SamplesBuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
    let sample_rate = stream_handle.config().sample_rate();
    let plan = PerformancePlan::new(songs::air::intro_composition())
        .tempo(songs::air::intro_tempo())
        .model(ModelPreset::ADry);
    let samples = Engine::new(sample_rate).render(&plan);
    let duration = Duration::from_secs_f64(samples.len() as f64 / sample_rate.get() as f64);

    stream_handle.mixer().add(SamplesBuffer::new(
        NonZero::new(1).unwrap(),
        sample_rate,
        samples,
    ));
    std::thread::sleep(duration);
    Ok(())
}
