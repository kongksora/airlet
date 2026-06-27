use airlet::defaults;
use rodio::buffer::SamplesBuffer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream_handle = rodio::DeviceSinkBuilder::open_default_sink()?;
    let sample_rate = stream_handle.config().sample_rate();
    let audio = defaults::air_intro_audio(sample_rate);
    let duration = audio.duration();

    stream_handle.mixer().add(SamplesBuffer::new(
        audio.channels(),
        audio.sample_rate(),
        audio.into_samples(),
    ));
    std::thread::sleep(duration);
    Ok(())
}
