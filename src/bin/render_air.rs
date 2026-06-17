use std::{fs, num::NonZero, path::PathBuf};

use airlet::{Performance, normalize_peak};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/airlet-air-intro.wav"));

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }

    let sample_rate = NonZero::new(48_000).unwrap();
    let performance = Performance::air_intro_legacy();
    let mut samples = performance.render(sample_rate, 0xA17E_7001);
    normalize_peak(&mut samples, 0.95);

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: sample_rate.get(),
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(&out, spec)?;
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;
        writer.write_sample(value)?;
    }
    writer.finalize()?;

    println!("rendered {}", out.display());
    Ok(())
}
