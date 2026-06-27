use std::{num::NonZero, time::Duration};

#[derive(Debug, Clone, PartialEq)]
pub struct RenderedAudio {
    sample_rate: NonZero<u32>,
    channels: NonZero<u16>,
    samples: Vec<f32>,
}

impl RenderedAudio {
    pub fn mono(sample_rate: NonZero<u32>, samples: Vec<f32>) -> Self {
        Self {
            sample_rate,
            channels: NonZero::new(1).unwrap(),
            samples,
        }
    }

    pub const fn sample_rate(&self) -> NonZero<u32> {
        self.sample_rate
    }

    pub const fn channels(&self) -> NonZero<u16> {
        self.channels
    }

    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub fn samples_mut(&mut self) -> &mut [f32] {
        &mut self.samples
    }

    pub fn into_samples(self) -> Vec<f32> {
        self.samples
    }

    pub fn duration(&self) -> Duration {
        Duration::from_secs_f64(
            self.samples.len() as f64 / self.sample_rate.get() as f64 / self.channels.get() as f64,
        )
    }

    pub fn peak(&self) -> f32 {
        self.samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0, f32::max)
    }

    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }

        let mean_square = self
            .samples
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>()
            / self.samples.len() as f32;
        mean_square.sqrt()
    }

    pub fn is_finite(&self) -> bool {
        self.samples.iter().all(|sample| sample.is_finite())
    }
}
