use std::f32::consts::{FRAC_PI_2, PI};

use airlet::{audio::RenderedAudio, defaults};
use airlet_model::{MeshGroup, ModelSpec, MovableMusicBoxModel, PivotPose};
use bevy::{
    gltf::{Gltf, GltfMaterial, GltfMesh, GltfNode},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::WindowResolution,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer};

const DEFAULT_MODEL_SPEC: &str = "assets/models/converted/spec.toml";
const EXHIBIT_TARGET: Vec3 = Vec3::new(0.0, 0.74, 0.0);
const PLATFORM_TOP_Y: f32 = 0.14;
const MODEL_SCALE: f32 = 1.8;

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
        .insert_resource(ClearColor(Color::srgb(0.12, 0.115, 0.105)))
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(1.0, 0.92, 0.78),
            brightness: 650.0,
            ..default()
        })
        .init_resource::<ExhibitControls>()
        .init_resource::<ScreenshotState>()
        .insert_resource(load_movable_model())
        .init_resource::<PlaybackState>()
        .add_systems(Startup, (setup_scene, setup_audio))
        .add_systems(
            Update,
            (
                orbit_camera,
                apply_camera_transform,
                apply_spotlight_controls,
                apply_rig_controls,
                apply_playback_controls,
                spawn_spec_model,
                report_model_load,
                auto_screenshot,
                exit_after_screenshot,
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
    pub lid_t: f32,
    pub cylinder_degrees: f32,
    pub cylinder_spin: bool,
}

impl Default for ExhibitControls {
    fn default() -> Self {
        Self {
            yaw: 0.48,
            pitch: 0.36,
            radius: 4.2,
            light_yaw: -0.45,
            light_pitch: 1.0,
            light_distance: 5.2,
            spot_inner_angle: 0.55,
            spot_outer_angle: 1.1,
            spot_intensity: 650_000.0,
            volume: 0.75,
            playback: PlaybackCommand::Idle,
            lid_t: env_f32("AIRLET_LID_T", 0.0).clamp(0.0, 1.0),
            cylinder_degrees: env_f32("AIRLET_CYLINDER_DEGREES", 0.0),
            cylinder_spin: false,
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

#[derive(Resource)]
pub struct ScreenshotState {
    pub path: Option<String>,
    pub requested: bool,
    pub frames_before_capture: u32,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        Self {
            path: std::env::var("AIRLET_SCREENSHOT").ok(),
            requested: false,
            frames_before_capture: 180,
        }
    }
}

#[derive(Component)]
struct ExhibitCamera;

#[derive(Component)]
struct ExhibitSpotlight;

#[derive(Component)]
struct LidPivot;

#[derive(Component)]
struct CylinderPivot;

#[derive(Resource)]
struct ModelGltfHandle(Handle<Gltf>);

#[derive(Resource)]
struct ModelResource {
    model: MovableMusicBoxModel,
}

#[derive(Resource, Default)]
struct ModelSpawnState {
    spawned: bool,
    logged: bool,
}

struct PendingPrimitive {
    group: MeshGroup,
    name: String,
    transform: Transform,
    mesh: Handle<Mesh>,
    material: Option<Handle<GltfMaterial>>,
}

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
    model: Res<ModelResource>,
) {
    commands.init_resource::<ModelSpawnState>();
    commands.insert_resource(ModelGltfHandle(
        asset_server.load(model.model.spec().asset.gltf.clone()),
    ));

    commands.spawn((
        Name::new("Exhibit Platform"),
        Mesh3d(meshes.add(Cylinder::new(2.15, 0.28))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.86, 0.73, 0.55),
            perceptual_roughness: 0.82,
            metallic: 0.05,
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.16, 0.0),
    ));

    commands.spawn((
        Name::new("Stage Floor"),
        Mesh3d(meshes.add(Plane3d::default().mesh().size(14.0, 14.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.50, 0.45, 0.36),
            perceptual_roughness: 0.95,
            cull_mode: None,
            unlit: true,
            ..default()
        })),
    ));

    commands.spawn((
        Name::new("Key Directional Light"),
        DirectionalLight {
            illuminance: 18_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_xyz(0.0, 4.0, 0.0).looking_at(EXHIBIT_TARGET, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Fill Light"),
        PointLight {
            intensity: 22_000.0,
            range: 16.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.2, 4.4, 4.6),
    ));

    commands.spawn((
        Name::new("Rim Light"),
        PointLight {
            intensity: 14_000.0,
            range: 12.0,
            color: Color::srgb(0.78, 0.88, 1.0),
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(3.2, 3.0, -4.2),
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
                egui::Slider::new(&mut controls.spot_intensity, 5_000.0..=1_200_000.0)
                    .text("Intensity"),
            );
            ui.add(egui::Slider::new(&mut controls.light_yaw, -PI..=PI).text("Light yaw"));
            ui.add(egui::Slider::new(&mut controls.light_pitch, 0.25..=1.45).text("Light pitch"));

            ui.separator();
            ui.heading("Camera");
            ui.add(egui::Slider::new(&mut controls.yaw, -PI..=PI).text("Yaw"));
            ui.add(egui::Slider::new(&mut controls.pitch, 0.08..=1.25).text("Pitch"));
            ui.add(egui::Slider::new(&mut controls.radius, 3.4..=12.0).text("Distance"));

            ui.separator();
            ui.heading("Rig");
            ui.add(egui::Slider::new(&mut controls.lid_t, 0.0..=1.0).text("Lid t"));
            ui.add(
                egui::Slider::new(&mut controls.cylinder_degrees, -720.0..=720.0)
                    .text("Cylinder angle"),
            );
            ui.checkbox(&mut controls.cylinder_spin, "Cylinder spin");
        });
    Ok(())
}

fn spawn_spec_model(
    mut commands: Commands,
    handle: Option<Res<ModelGltfHandle>>,
    model: Res<ModelResource>,
    gltfs: Res<Assets<Gltf>>,
    nodes: Res<Assets<GltfNode>>,
    meshes: Res<Assets<GltfMesh>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut state: ResMut<ModelSpawnState>,
) {
    if state.spawned {
        return;
    }
    let Some(handle) = handle else {
        return;
    };
    let Some(gltf) = gltfs.get(&handle.0) else {
        return;
    };
    if gltf.nodes.iter().any(|node| nodes.get(node).is_none())
        || gltf.meshes.iter().any(|mesh| meshes.get(mesh).is_none())
        || gltf
            .materials
            .iter()
            .any(|material| gltf_materials.get(material).is_none())
    {
        return;
    }

    let closed = model.model.closed_meshes();
    let mut pending = Vec::new();
    for node_handle in &gltf.nodes {
        let Some(node) = nodes.get(node_handle) else {
            continue;
        };
        let Some(mesh_handle) = &node.mesh else {
            continue;
        };
        let Some(mesh) = meshes.get(mesh_handle) else {
            continue;
        };
        if !closed.contains(&mesh.index) {
            continue;
        }
        let group = model.model.group_for_mesh(mesh.index);
        for primitive in &mesh.primitives {
            pending.push(PendingPrimitive {
                group,
                name: format!("{} primitive {}", node.name, primitive.index),
                transform: node.transform,
                mesh: primitive.mesh.clone(),
                material: primitive.material.clone(),
            });
        }
    }

    let root = commands
        .spawn((
            Name::new("Music Box Closed Model"),
            model_transform(&model.model),
            Visibility::Visible,
        ))
        .id();
    let lid_pose = model.model.lid_pose();
    let lid_pivot = commands
        .spawn((
            Name::new("Music Box Lid Pivot"),
            LidPivot,
            Transform::from_translation(vec3(lid_pose.pivot)),
            Visibility::Visible,
        ))
        .id();
    let cylinder_pose = model.model.cylinder_pose();
    let cylinder_pivot = commands
        .spawn((
            Name::new("Music Box Cylinder Pivot"),
            CylinderPivot,
            Transform::from_translation(vec3(cylinder_pose.pivot)),
            Visibility::Visible,
        ))
        .id();
    commands.entity(root).add_child(lid_pivot);
    commands.entity(root).add_child(cylinder_pivot);

    for primitive in pending {
        let parent = match primitive.group {
            MeshGroup::Static | MeshGroup::Comb => root,
            MeshGroup::Lid => lid_pivot,
            MeshGroup::Cylinder => cylinder_pivot,
            MeshGroup::Excluded => continue,
        };
        let mut transform = primitive.transform;
        transform.translation = vec3(
            model
                .model
                .relative_translation(transform.translation.to_array(), primitive.group),
        );
        let mut entity =
            commands.spawn((Name::new(primitive.name), Mesh3d(primitive.mesh), transform));
        if let Some(material) = primitive.material {
            let Some(gltf_material) = gltf_materials.get(&material) else {
                continue;
            };
            entity.insert(MeshMaterial3d(
                materials.add(to_standard_material(gltf_material)),
            ));
        }
        let child = entity.id();
        commands.entity(parent).add_child(child);
    }

    state.spawned = true;
    info!(
        "spawned closed music box rig: meshes={}, lid_meshes={}, cylinder_meshes={}",
        model.model.spec().closed_model.mesh_indices.len(),
        model.model.spec().lid.meshes.len(),
        model.model.spec().cylinder.meshes.len()
    );
}

fn apply_rig_controls(
    time: Res<Time>,
    mut controls: ResMut<ExhibitControls>,
    mut model: ResMut<ModelResource>,
    mut lid_query: Query<&mut Transform, (With<LidPivot>, Without<CylinderPivot>)>,
    mut cylinder_query: Query<&mut Transform, (With<CylinderPivot>, Without<LidPivot>)>,
) {
    model.model.set_lid_t(controls.lid_t);
    model.model.set_cylinder_degrees(controls.cylinder_degrees);
    model.model.set_cylinder_spin(controls.cylinder_spin);
    model.model.advance(time.delta_secs());
    controls.cylinder_degrees = model.model.state().cylinder_degrees;

    let lid_pose = model.model.lid_pose();
    for mut transform in &mut lid_query {
        transform.rotation = pose_rotation(lid_pose);
    }

    let cylinder_pose = model.model.cylinder_pose();
    for mut transform in &mut cylinder_query {
        transform.rotation = pose_rotation(cylinder_pose);
    }
}

fn report_model_load(mut state: ResMut<ModelSpawnState>, meshes: Query<&Mesh3d>) {
    if state.logged {
        return;
    }
    let mesh_count = meshes.iter().count();
    if mesh_count <= 2 {
        return;
    }
    info!("music box scene spawned; mesh component count: {mesh_count}");
    state.spawned = true;
    state.logged = true;
}

fn auto_screenshot(
    mut commands: Commands,
    mut state: ResMut<ScreenshotState>,
    model_state: Res<ModelSpawnState>,
) {
    if state.path.is_none() || state.requested {
        return;
    }
    if !model_state.spawned {
        return;
    }
    if state.frames_before_capture > 0 {
        state.frames_before_capture -= 1;
        return;
    }

    let path = state.path.clone().unwrap();
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
    state.requested = true;
}

fn exit_after_screenshot(state: Res<ScreenshotState>, mut exit: MessageWriter<AppExit>) {
    let Some(path) = &state.path else {
        return;
    };
    if state.requested && std::path::Path::new(path).exists() {
        exit.write(AppExit::Success);
    }
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

fn model_transform(model: &MovableMusicBoxModel) -> Transform {
    let placement = model.root_placement(EXHIBIT_TARGET.to_array(), PLATFORM_TOP_Y, MODEL_SCALE);
    Transform::from_translation(vec3(placement.translation))
        .with_rotation(Quat::from_array(placement.rotation))
        .with_scale(Vec3::splat(placement.scale))
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

fn load_movable_model() -> ModelResource {
    let spec = ModelSpec::from_toml_path(DEFAULT_MODEL_SPEC)
        .unwrap_or_else(|err| panic!("failed to load default music-box model spec: {err}"));
    ModelResource {
        model: MovableMusicBoxModel::new(spec),
    }
}

fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::from_array(value)
}

fn pose_rotation(pose: PivotPose) -> Quat {
    Quat::from_axis_angle(vec3(pose.axis), pose.angle_degrees.to_radians())
}

fn to_standard_material(material: &GltfMaterial) -> StandardMaterial {
    StandardMaterial {
        base_color: material.base_color,
        base_color_channel: material.base_color_channel.clone(),
        base_color_texture: material.base_color_texture.clone(),
        emissive: material.emissive,
        emissive_channel: material.emissive_channel.clone(),
        emissive_texture: material.emissive_texture.clone(),
        perceptual_roughness: material.perceptual_roughness,
        metallic: material.metallic,
        metallic_roughness_channel: material.metallic_roughness_channel.clone(),
        metallic_roughness_texture: material.metallic_roughness_texture.clone(),
        reflectance: material.reflectance,
        normal_map_channel: material.normal_map_channel.clone(),
        normal_map_texture: material.normal_map_texture.clone(),
        occlusion_channel: material.occlusion_channel.clone(),
        occlusion_texture: material.occlusion_texture.clone(),
        alpha_mode: material.alpha_mode,
        cull_mode: material.cull_mode,
        double_sided: material.double_sided,
        ..default()
    }
}
