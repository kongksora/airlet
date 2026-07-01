use airlet::score::PPQ;
use bevy::prelude::*;

/// Visual configuration for the full mechanism (teeth, comb, animation).
#[derive(Resource, Debug, Clone)]
pub struct MechanismVisualConfig {
    pub tooth: ToothVisualConfig,
    pub comb: CombVisualConfig,
    pub animation: CombAnimationConfig,
}

impl Default for MechanismVisualConfig {
    fn default() -> Self {
        Self {
            tooth: ToothVisualConfig::default(),
            comb: CombVisualConfig::default(),
            animation: CombAnimationConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ToothVisualConfig {
    /// Fraction of cylinder length for tooth width.
    pub width_ratio: f32,
    /// Fraction of cylinder radius for tooth total radial height.
    pub height_ratio: f32,
    /// Fraction of measured cylinder/comb clearance for tooth clearance.
    pub default_clearance_ratio: f32,
}

impl Default for ToothVisualConfig {
    fn default() -> Self {
        Self {
            width_ratio: 0.028,
            height_ratio: 0.14,
            default_clearance_ratio: 0.92,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombVisualConfig {
    /// Fallback comb tine length relative to cylinder radius.
    pub tine_length_ratio: f32,
    /// Fallback comb tine width relative to cylinder length.
    pub tine_width_ratio: f32,
    /// Comb tine thickness relative to cylinder radius.
    pub tine_thickness_ratio: f32,
    /// Free (playable) portion of comb tine length.
    pub free_length_ratio: f32,
    /// Tine width as a fraction of track spacing.
    pub tine_width_spacing_ratio: f32,
    /// Usable fraction of cylinder length for comb track placement.
    pub track_usable_length_ratio: f32,
}

impl Default for CombVisualConfig {
    fn default() -> Self {
        Self {
            tine_length_ratio: 1.35,
            tine_width_ratio: 0.035,
            tine_thickness_ratio: 0.025,
            free_length_ratio: 0.72,
            tine_width_spacing_ratio: 0.82,
            track_usable_length_ratio: 0.86,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CombAnimationConfig {
    /// Minimum pluck/contact ticks before note onset.
    pub min_pluck_ticks: i64,
    /// Maximum pluck/contact ticks before note onset.
    pub max_pluck_ticks: i64,
    /// Minimum damped vibration ticks after release.
    pub min_vibration_ticks: i64,
    /// Maximum damped vibration ticks after release.
    pub max_vibration_ticks: i64,
    /// Lift window as fraction of contact window.
    pub lift_window_ratio: f32,
    /// Scale applied to raw deflection before clamping.
    pub deflection_scale: f32,
    /// Minimum deflection angle in radians.
    pub min_deflection_rad: f32,
    /// Maximum deflection angle in radians.
    pub max_deflection_rad: f32,
    /// Ghost smear phase offsets as a fraction of vibration period.
    pub ghost_samples: &'static [f32],
    /// Sign multiplier for cylinder rotation during playback.
    pub cylinder_playback_rotation_sign: f32,
}

impl Default for CombAnimationConfig {
    fn default() -> Self {
        Self {
            min_pluck_ticks: PPQ / 16,
            max_pluck_ticks: PPQ / 3,
            min_vibration_ticks: PPQ / 2,
            max_vibration_ticks: PPQ * 3,
            lift_window_ratio: 0.65,
            deflection_scale: 0.65,
            min_deflection_rad: 0.035,
            max_deflection_rad: 0.18,
            ghost_samples: &[-0.38, -0.18, 0.18, 0.38],
            cylinder_playback_rotation_sign: -1.0,
        }
    }
}
