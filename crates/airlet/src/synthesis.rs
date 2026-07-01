use std::{num::NonZero, time::Duration};

use rand::{Rng, RngExt, SeedableRng, rngs::StdRng};

use crate::model::MusicBoxModel;

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
