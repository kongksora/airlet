#[derive(Debug, Clone)]
pub struct MusicBoxModel {
    pub exciter: StrikeParams,
    pub tines: TineBankParams,
    pub body: ResonatorParams,
    pub performance: HumanizationParams,
}

impl MusicBoxModel {
    pub fn modal_a_probe() -> Self {
        Self::modal_a_dry_probe()
    }

    pub fn modal_a_dry_probe() -> Self {
        Self {
            exciter: StrikeParams {
                attack_seconds: 0.003,
                click_gain: 0.035,
                click_decay_seconds: 0.006,
                noise_gain: 0.002,
            },
            tines: TineBankParams {
                partials: vec![
                    PartialParams {
                        frequency_ratio: 0.66438018,
                        amplitude: 0.00601504,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 1.0,
                        amplitude: 1.0,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 1.05251027,
                        amplitude: 0.00321139,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 1.68264716,
                        amplitude: 0.00380713,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 1.99771682,
                        amplitude: 0.00427584,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 2.24657311,
                        amplitude: 0.00255180,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 2.37671131,
                        amplitude: 0.00409024,
                        decay_scale: 1.0,
                    },
                    PartialParams {
                        frequency_ratio: 9.68949859,
                        amplitude: 0.01131735,
                        decay_scale: 1.0,
                    },
                ],
                base_decay_seconds: 0.25,
                low_decay_boost: 0.1,
                high_decay_power: 1.1,
                pitch_decay_power: 0.5,
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
