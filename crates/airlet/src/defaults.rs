use std::num::NonZero;

use crate::{
    audio::RenderedAudio,
    engine::Engine,
    performance::{ModelPreset, PerformancePlan},
    songs,
};

pub fn air_intro_plan() -> PerformancePlan {
    PerformancePlan::new(songs::air::intro_composition())
        .tempo(songs::air::intro_tempo())
        .model(ModelPreset::ADry)
}

pub fn air_intro_audio(sample_rate: NonZero<u32>) -> RenderedAudio {
    Engine::new(sample_rate).render(&air_intro_plan())
}
