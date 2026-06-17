#[derive(Debug, Clone)]
pub struct MusicBoxModel {
    pub exciter: StrikeParams,
    pub tines: TineBankParams,
    pub body: ResonatorParams,
    pub performance: HumanizationParams,
}

impl MusicBoxModel {
    pub fn modal_a_probe() -> Self {
        Self {
            exciter: StrikeParams {
                attack_seconds: 0.003,
                click_gain: 0.035,
                click_decay_seconds: 0.006,
                noise_gain: 0.003,
            },
            tines: TineBankParams {
                partials: Vec::new(),
                base_decay_seconds: 0.55,
                low_decay_boost: 0.25,
                high_decay_power: 0.85,
                pitch_decay_power: 0.35,
                amplitude_power: 0.85,
                detune_cents: 1.5,
                stretch: 0.0004,
                drive: 1.15,
                output_gain: 0.72,
            },
            body: ResonatorParams::Dry,
            performance: HumanizationParams {
                velocity_jitter: 0.04,
                onset_jitter_seconds: 0.0,
                seed: 42,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrikeParams {
    pub attack_seconds: f32,
    pub click_gain: f32,
    pub click_decay_seconds: f32,
    pub noise_gain: f32,
}

#[derive(Debug, Clone)]
pub struct TineBankParams {
    pub partials: Vec<PartialParams>,
    pub base_decay_seconds: f32,
    pub low_decay_boost: f32,
    pub high_decay_power: f32,
    pub pitch_decay_power: f32,
    pub amplitude_power: f32,
    pub detune_cents: f32,
    pub stretch: f32,
    pub drive: f32,
    pub output_gain: f32,
}

#[derive(Debug, Clone)]
pub struct PartialParams {
    pub frequency_ratio: f32,
    pub amplitude: f32,
    pub decay_scale: f32,
}

#[derive(Debug, Clone)]
pub enum ResonatorParams {
    Dry,
    ImpulseResponse { wet: f32, dry: f32 },
}

#[derive(Debug, Clone)]
pub struct HumanizationParams {
    pub velocity_jitter: f32,
    pub onset_jitter_seconds: f32,
    pub seed: u64,
}
