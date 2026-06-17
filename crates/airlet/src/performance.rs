use crate::{
    ModelPlaybackConfig, PlaybackConfig, TineParams,
    model::MusicBoxModel,
    score::{ComposedScore, Composition, Tempo},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelPreset {
    Legacy,
    ADry,
}

#[derive(Debug, Clone)]
pub struct OrnamentPolicy {
    pub grace_steals_time: bool,
}

impl Default for OrnamentPolicy {
    fn default() -> Self {
        Self {
            grace_steals_time: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct VelocityPolicy {
    pub base_gain: f32,
}

impl Default for VelocityPolicy {
    fn default() -> Self {
        Self { base_gain: 1.0 }
    }
}

#[derive(Debug, Clone)]
pub struct PerformancePlan {
    pub composition: Composition,
    pub tempo: Tempo,
    pub model_preset: ModelPreset,
    pub ornament_policy: OrnamentPolicy,
    pub velocity_policy: VelocityPolicy,
}

impl PerformancePlan {
    pub fn new(composition: Composition) -> Self {
        Self {
            composition,
            tempo: Tempo::bpm(120.0),
            model_preset: ModelPreset::ADry,
            ornament_policy: OrnamentPolicy::default(),
            velocity_policy: VelocityPolicy::default(),
        }
    }

    pub fn tempo(mut self, tempo: Tempo) -> Self {
        self.tempo = tempo;
        self
    }

    pub fn model(mut self, model_preset: ModelPreset) -> Self {
        self.model_preset = model_preset;
        self
    }

    pub fn composed_score(&self) -> ComposedScore {
        self.composition.clone().with_tempo(self.tempo)
    }

    pub fn legacy_playback(&self) -> PlaybackConfig {
        PlaybackConfig::default()
    }

    pub fn legacy_tine(&self) -> TineParams {
        TineParams::legacy()
    }

    pub fn model_playback(&self) -> ModelPlaybackConfig {
        match self.model_preset {
            ModelPreset::Legacy => ModelPlaybackConfig {
                note_tail: self.legacy_playback().note_tail,
                note_gain: self.legacy_playback().note_gain,
                final_tail: self.legacy_playback().final_tail,
                model: MusicBoxModel::modal_a_dry_probe(),
                seed: 0xA17E_7001,
            },
            ModelPreset::ADry => ModelPlaybackConfig::air_dry(),
        }
    }
}
