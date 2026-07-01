use bevy::{picking::hover::Hovered, prelude::*};

use crate::model_view::ModelResource;
use crate::scene::WindingKeyPivot;
use crate::twin::MusicBoxTwinState;

#[derive(Resource, Debug, Clone)]
pub struct WindingState {
    pub hovered: bool,
    pub pressed: bool,
    pub wind_amount: f32,
    pub max_wind_amount: f32,
    pub key_degrees: f32,
    pub last_released_wind_amount: f32,
    pub last_started_cycles: u32,
}

impl Default for WindingState {
    fn default() -> Self {
        Self {
            hovered: false,
            pressed: false,
            wind_amount: 0.0,
            max_wind_amount: 1.0,
            key_degrees: 0.0,
            last_released_wind_amount: 0.0,
            last_started_cycles: 0,
        }
    }
}

impl WindingState {
    pub fn clear_active_wind(&mut self) {
        self.pressed = false;
        self.wind_amount = 0.0;
    }
}

#[derive(Component)]
pub struct WindingKeyPart {
    pub normal_material: Handle<StandardMaterial>,
    pub hover_material: Handle<StandardMaterial>,
}

pub fn update_winding_interaction(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    targets: Query<&Hovered, With<WindingKeyPart>>,
    mut winding: ResMut<WindingState>,
) {
    let hovered = targets.iter().any(Hovered::get);
    winding.hovered = hovered || winding.pressed;

    if mouse_buttons.just_pressed(MouseButton::Left) && hovered {
        winding.pressed = true;
        winding.last_started_cycles = 0;
    }

    if winding.pressed && mouse_buttons.just_released(MouseButton::Left) {
        winding.pressed = false;
        winding.last_released_wind_amount = winding.wind_amount;
        winding.last_started_cycles = u32::from(winding.wind_amount > 0.02);
    }
}

pub fn apply_winding_visuals(
    winding: Res<WindingState>,
    twin: Res<MusicBoxTwinState>,
    model: Res<ModelResource>,
    mut pivots: Query<&mut Transform, With<WindingKeyPivot>>,
    mut parts: Query<(&WindingKeyPart, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    if let Some(pose) = model.model.winding_key_pose() {
        let axis = crate::vec3(pose.axis).normalize_or(Vec3::X);
        for mut transform in &mut pivots {
            transform.rotation = Quat::from_axis_angle(axis, twin.key_degrees.to_radians());
        }
    }

    for (part, mut material) in &mut parts {
        material.0 = if winding.hovered || winding.pressed {
            part.hover_material.clone()
        } else {
            part.normal_material.clone()
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_active_wind_resets_pending_release_state() {
        let mut winding = WindingState {
            pressed: true,
            wind_amount: 0.5,
            key_degrees: 180.0,
            ..default()
        };

        winding.clear_active_wind();

        assert!(!winding.pressed);
        assert_eq!(winding.wind_amount, 0.0);
    }
}
