use std::{fs, num::NonZero, path::PathBuf};

use airlet::{Performance, normalize_peak, render_air_intro_a_dry};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let first = args.first().and_then(|arg| arg.to_str());
    let model = match first {
        Some("legacy" | "a-dry") => first.unwrap().to_string(),
        _ => "legacy".to_string(),
    };
    let out_arg_index = if model == "legacy" && first != Some("legacy") {
        0
    } else {
        1
    };
    let out = args
        .get(out_arg_index)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(format!("target/airlet-air-intro-{model}.wav")));

    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }

    let sample_rate = NonZero::new(48_000).unwrap();
    let mut samples = match model.as_str() {
        "legacy" => Performance::air_intro_legacy().render(sample_rate, 0xA17E_7001),
        "a-dry" => render_air_intro_a_dry(sample_rate),
        _ => unreachable!(),
    };
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
