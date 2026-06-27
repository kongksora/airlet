use std::f32::consts::{FRAC_PI_2, PI};

use airlet::{audio::RenderedAudio, defaults};
use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    window::WindowResolution,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer};

const MUSIC_BOX_SCENE: &str = "models/converted/music_box.glb";
const EXHIBIT_TARGET: Vec3 = Vec3::new(0.0, 0.9, 0.0);

pub fn run() {
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
        .insert_resource(ClearColor(Color::srgb(0.035, 0.035, 0.04)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.72, 0.68, 0.62),
            brightness: 90.0,
            ..default()
        })
        .init_resource::<ExhibitControls>()
        .init_resource::<PlaybackState>()
        .add_systems(Startup, (setup_scene, setup_audio))
        .add_systems(
            Update,
            (
                orbit_camera,
                apply_camera_transform,
                apply_spotlight_controls,
                apply_playback_controls,
            ),
        )
        .add_systems(EguiPrimaryContextPass, control_panel)
        .run();
}

#[derive(Resource)]
pub struct ExhibitControls {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub light_yaw: f32,
    pub light_pitch: f32,
    pub light_distance: f32,
    pub spot_inner_angle: f32,
    pub spot_outer_angle: f32,
    pub spot_intensity: f32,
    pub volume: f32,
    pub playback: PlaybackCommand,
}

impl Default for ExhibitControls {
    fn default() -> Self {
        Self {
            yaw: -0.62,
            pitch: 0.34,
            radius: 7.0,
            light_yaw: -0.45,
            light_pitch: 1.05,
            light_distance: 5.2,
            spot_inner_angle: 0.38,
            spot_outer_angle: 0.72,
            spot_intensity: 85_000.0,
            volume: 0.75,
            playback: PlaybackCommand::Idle,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackCommand {
    Idle,
    Start,
    Stop,
}

#[derive(Resource, Default)]
pub struct PlaybackState {
    pub device: Option<MixerDeviceSink>,
    pub player: Option<Player>,
    pub audio: Option<RenderedAudio>,
    pub is_playing: bool,
    pub last_error: Option<String>,
}

#[derive(Component)]
struct ExhibitCamera;

#[derive(Component)]
struct ExhibitSpotlight;

fn setup_audio(mut playback: ResMut<PlaybackState>) {
    match DeviceSinkBuilder::open_default_sink() {
        Ok(mut device) => {
            device.log_on_drop(false);
            let sample_rate = device.config().sample_rate();
            playback.audio = Some(defaults::air_intro_audio(sample_rate));
            playback.device = Some(device);
        }
        Err(err) => {
            playback.last_error = Some(format!("audio device error: {err}"));
        }
    }
}

fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    controls: Res<ExhibitControls>,
) {
    commands.spawn((
        Name::new("Music Box Model"),
        WorldAssetRoot(asset_server.load(GltfAssetLabel::Scene(0).from_asset(MUSIC_BOX_SCENE))),
        Transform::from_scale(Vec3::splat(1.0)),
    ));

    commands.spawn((
        Name::new("Exhibit Platform"),
        Mesh3d(meshes.add(Cylinder::new(3.6, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.45, 0.42, 0.36),
            perceptual_roughness: 0.82,
            metallic: 0.05,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.16, 0.0),
    ));

    commands.spawn((
        Name::new("Stage Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.11, 0.105, 0.095),
            perceptual_roughness: 0.95,
            cull_mode: None,
            ..default()
        })),
    ));

    commands.spawn((
        Name::new("Fill Light"),
        PointLight {
            intensity: 2_200.0,
            range: 9.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-3.6, 3.2, 3.8),
    ));

    commands.spawn((
        Name::new("Spotlight"),
        ExhibitSpotlight,
        SpotLight {
            intensity: controls.spot_intensity,
            inner_angle: controls.spot_inner_angle,
            outer_angle: controls.spot_outer_angle,
            range: 12.0,
            shadow_maps_enabled: true,
            ..default()
        },
        spotlight_transform(&controls),
    ));

    commands.spawn((
        Name::new("Exhibit Camera"),
        ExhibitCamera,
        Camera3d::default(),
        camera_transform(&controls),
    ));
}

fn orbit_camera(
    mut controls: ResMut<ExhibitControls>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if mouse_buttons.pressed(MouseButton::Right) {
        controls.yaw -= mouse_motion.delta.x * 0.006;
        controls.pitch = (controls.pitch + mouse_motion.delta.y * 0.004).clamp(0.08, 1.25);
    }

    if mouse_scroll.delta.y != 0.0 {
        controls.radius = (controls.radius - mouse_scroll.delta.y * 0.35).clamp(3.4, 12.0);
    }
}

fn apply_camera_transform(
    controls: Res<ExhibitControls>,
    mut camera: Query<&mut Transform, With<ExhibitCamera>>,
) {
    if !controls.is_changed() {
        return;
    }

    for mut transform in &mut camera {
        *transform = camera_transform(&controls);
    }
}

fn apply_spotlight_controls(
    controls: Res<ExhibitControls>,
    mut lights: Query<(&mut SpotLight, &mut Transform), With<ExhibitSpotlight>>,
) {
    if !controls.is_changed() {
        return;
    }

    for (mut light, mut transform) in &mut lights {
        light.intensity = controls.spot_intensity;
        light.inner_angle = controls
            .spot_inner_angle
            .min(controls.spot_outer_angle - 0.02);
        light.outer_angle = controls.spot_outer_angle;
        *transform = spotlight_transform(&controls);
    }
}

fn apply_playback_controls(
    mut controls: ResMut<ExhibitControls>,
    mut playback: ResMut<PlaybackState>,
) {
    let volume = controls.volume;
    if let Some(player) = &playback.player {
        player.set_volume(volume);
        if player.empty() {
            playback.player = None;
            playback.is_playing = false;
        }
    }

    match controls.playback {
        PlaybackCommand::Idle => {}
        PlaybackCommand::Start => {
            start_playback(&mut playback, volume);
            controls.playback = PlaybackCommand::Idle;
        }
        PlaybackCommand::Stop => {
            stop_playback(&mut playback);
            controls.playback = PlaybackCommand::Idle;
        }
    }
}

fn control_panel(
    mut contexts: EguiContexts,
    mut controls: ResMut<ExhibitControls>,
    playback: Res<PlaybackState>,
) -> Result {
    egui::Window::new("Airlet Control")
        .default_width(280.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Performance");
            ui.horizontal(|ui| {
                if ui.button("Start").clicked() {
                    controls.playback = PlaybackCommand::Start;
                }
                if ui.button("Stop").clicked() {
                    controls.playback = PlaybackCommand::Stop;
                }
            });
            ui.add(egui::Slider::new(&mut controls.volume, 0.0..=1.5).text("Volume"));
            ui.label(if playback.is_playing {
                "Status: Playing"
            } else {
                "Status: Stopped"
            });
            if let Some(error) = &playback.last_error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }

            ui.separator();
            ui.heading("Spotlight");
            ui.add(
                egui::Slider::new(&mut controls.spot_outer_angle, 0.22..=1.35).text("Outer angle"),
            );
            controls.spot_inner_angle = controls
                .spot_inner_angle
                .clamp(0.08, controls.spot_outer_angle - 0.02);
            ui.add(
                egui::Slider::new(&mut controls.spot_inner_angle, 0.08..=1.0).text("Inner angle"),
            );
            ui.add(
                egui::Slider::new(&mut controls.spot_intensity, 5_000.0..=180_000.0)
                    .text("Intensity"),
            );
            ui.add(egui::Slider::new(&mut controls.light_yaw, -PI..=PI).text("Light yaw"));
            ui.add(egui::Slider::new(&mut controls.light_pitch, 0.25..=1.45).text("Light pitch"));

            ui.separator();
            ui.heading("Camera");
            ui.add(egui::Slider::new(&mut controls.yaw, -PI..=PI).text("Yaw"));
            ui.add(egui::Slider::new(&mut controls.pitch, 0.08..=1.25).text("Pitch"));
            ui.add(egui::Slider::new(&mut controls.radius, 3.4..=12.0).text("Distance"));
        });
    Ok(())
}

fn start_playback(playback: &mut PlaybackState, volume: f32) {
    let Some(device) = playback.device.as_ref() else {
        playback.last_error = Some("audio device is unavailable".to_string());
        playback.is_playing = false;
        return;
    };
    let Some(audio) = playback.audio.as_ref() else {
        playback.last_error = Some("default performance audio is unavailable".to_string());
        playback.is_playing = false;
        return;
    };

    if let Some(player) = playback.player.take() {
        player.stop();
    }

    let player = Player::connect_new(device.mixer());
    player.set_volume(volume);
    player.append(SamplesBuffer::new(
        audio.channels(),
        audio.sample_rate(),
        audio.samples().to_vec(),
    ));
    playback.player = Some(player);
    playback.is_playing = true;
    playback.last_error = None;
}

fn stop_playback(playback: &mut PlaybackState) {
    if let Some(player) = playback.player.take() {
        player.stop();
    }
    playback.is_playing = false;
}

fn camera_transform(controls: &ExhibitControls) -> Transform {
    let horizontal = controls.radius * controls.pitch.cos();
    let position = Vec3::new(
        horizontal * controls.yaw.sin(),
        controls.radius * controls.pitch.sin(),
        horizontal * controls.yaw.cos(),
    ) + EXHIBIT_TARGET;
    Transform::from_translation(position).looking_at(EXHIBIT_TARGET, Vec3::Y)
}

fn spotlight_transform(controls: &ExhibitControls) -> Transform {
    let pitch = controls.light_pitch.clamp(0.1, FRAC_PI_2 - 0.02);
    let horizontal = controls.light_distance * pitch.cos();
    let position = Vec3::new(
        horizontal * controls.light_yaw.sin(),
        controls.light_distance * pitch.sin(),
        horizontal * controls.light_yaw.cos(),
    ) + EXHIBIT_TARGET;
    Transform::from_translation(position).looking_at(EXHIBIT_TARGET, Vec3::Y)
}
