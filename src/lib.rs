pub mod comb_animation;
pub mod controls;
pub mod debug;
pub mod lid;
pub mod lighting;
pub mod mechanical_audio;
pub mod mechanism_view;
pub mod model_view;
pub mod outline;
pub mod playback;
pub mod scene;
pub mod screenshot;
pub mod twin;
pub mod visual_config;
pub mod winding;

use bevy::{
    picking::prelude::{MeshPickingPlugin, MeshPickingSettings},
    prelude::*,
    window::WindowResolution,
};
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

use controls::ExhibitControls;
use debug::DebugEndpoint;
use lighting::ExhibitLightingConfig;
use model_view::{load_mechanism_layout, load_movable_model};
use playback::AudioOutputState;
use screenshot::ScreenshotState;
use visual_config::MechanismVisualConfig;
use winding::WindingState;

pub fn run() {
    let lighting = ExhibitLightingConfig::default();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Airlet".to_string(),
                resolution: WindowResolution::new(1280, 800),
                prevent_default_event_handling: false,
                ..default()
            }),
            ..default()
        }))
        .add_plugins(EguiPlugin::default())
        .add_plugins(MeshPickingPlugin)
        .add_plugins(twin::MusicBoxTwinPlugin)
        .add_plugins(lid::LidPlugin)
        .insert_resource(MeshPickingSettings {
            require_markers: true,
            ..default()
        })
        .insert_resource(ClearColor(Color::srgb(0.035, 0.034, 0.032)))
        .insert_resource(lighting.directional_shadow_map())
        .insert_resource(lighting.point_shadow_map())
        .insert_resource(lighting.ambient_light())
        .insert_resource(lighting)
        .init_resource::<ExhibitControls>()
        .init_resource::<ScreenshotState>()
        .insert_resource(load_movable_model())
        .insert_resource(load_mechanism_layout())
        .init_resource::<AudioOutputState>()
        .init_resource::<mechanical_audio::MechanicalAudioConfig>()
        .init_resource::<mechanical_audio::MechanicalEventQueue>()
        .init_resource::<mechanical_audio::MechanicalEventState>()
        .init_resource::<mechanical_audio::MechanicalEventStats>()
        .init_resource::<mechanical_audio::MechanicalAuditionQueue>()
        .init_resource::<mechanical_audio::MechanicalAudioState>()
        .init_resource::<WindingState>()
        .insert_resource(DebugEndpoint::start_from_env())
        .init_resource::<MechanismVisualConfig>()
        .add_systems(Startup, (scene::setup_scene, playback::setup_audio))
        .add_systems(
            Update,
            (
                debug::apply_debug_actions,
                scene::orbit_camera,
                scene::apply_camera_transform,
                scene::apply_lighting_controls,
                playback::apply_playback_controls,
                winding::update_winding_interaction,
                outline::toggle_lid_on_click,
                lid::update_lid_state,
                twin::update_music_box_twin,
                mechanical_audio::emit_mechanical_events,
                twin::schedule_twin_audio_cycles,
                mechanical_audio::play_mechanical_audio,
                winding::apply_winding_visuals,
                outline::update_interactive_outlines,
                model_view::apply_rig_controls,
                mechanism_view::animate_comb_tines,
                model_view::spawn_spec_model,
                model_view::report_model_load,
                screenshot::auto_screenshot,
                screenshot::exit_after_screenshot,
            ),
        )
        .add_systems(EguiPrimaryContextPass, controls::control_panel)
        .run();
}

pub(crate) fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::from_array(value)
}
