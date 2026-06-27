use std::{fmt, fs, path::Path};

use crate::{model::MusicBoxModel, performance::ModelPreset};

#[derive(Debug)]
pub enum PresetLibraryError {
    UnsupportedBundledPreset(ModelPreset),
    Io(std::io::Error),
    Model(crate::model::PresetError),
}

impl fmt::Display for PresetLibraryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedBundledPreset(preset) => {
                write!(f, "unsupported bundled preset: {preset:?}")
            }
            Self::Io(err) => write!(f, "preset io error: {err}"),
            Self::Model(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for PresetLibraryError {}

impl From<std::io::Error> for PresetLibraryError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<crate::model::PresetError> for PresetLibraryError {
    fn from(value: crate::model::PresetError) -> Self {
        Self::Model(value)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct PresetLibrary;

impl PresetLibrary {
    pub fn bundled() -> Self {
        Self
    }

    pub fn load_model(&self, preset: ModelPreset) -> Result<MusicBoxModel, PresetLibraryError> {
        match preset {
            ModelPreset::ADry => MusicBoxModel::from_json_str(MusicBoxModel::a_dry_json())
                .map_err(PresetLibraryError::from),
            ModelPreset::Legacy => Err(PresetLibraryError::UnsupportedBundledPreset(preset)),
        }
    }

    pub fn load_model_from_path(
        path: impl AsRef<Path>,
    ) -> Result<MusicBoxModel, PresetLibraryError> {
        let input = fs::read_to_string(path)?;
        MusicBoxModel::from_json_str(&input).map_err(PresetLibraryError::from)
    }
}
