use std::time::Duration;

use crate::{model::MusicBoxModel, performance::ModelPreset, preset::PresetLibrary};

#[derive(Debug, Clone)]
pub struct ModelPlaybackConfig {
    pub note_tail: Duration,
    pub note_gain: f32,
    pub final_tail: Duration,
    pub model: MusicBoxModel,
    pub seed: u64,
}

impl ModelPlaybackConfig {
    pub fn air_dry() -> Self {
        Self {
            note_tail: Duration::from_millis(1200),
            note_gain: 0.17,
            final_tail: Duration::from_millis(800),
            model: PresetLibrary::bundled()
                .load_model(ModelPreset::ADry)
                .expect("bundled a-dry preset must be valid"),
            seed: 42,
        }
    }
}
