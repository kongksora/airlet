use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_PI_2, PI},
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use airlet::{
    audio::RenderedAudio,
    defaults,
    mechanism::{MechanismLayoutHint, MechanismPlanner, ToothHint},
    score::{PPQ, Timeline},
};
use airlet_model::{MeshGroup, ModelSpec, MovableMusicBoxModel, PivotPose};
use bevy::{
    asset::RenderAssetUsages,
    gltf::{Gltf, GltfMaterial, GltfMesh, GltfNode},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::{CascadeShadowConfigBuilder, DirectionalLightShadowMap, PointLightShadowMap},
    mesh::Indices,
    pbr::ContactShadows,
    prelude::*,
    render::render_resource::PrimitiveTopology,
    render::view::screenshot::{Screenshot, save_to_disk},
    window::WindowResolution,
};
use bevy_egui::{EguiContexts, EguiPlugin, EguiPrimaryContextPass, egui};
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const DEFAULT_MODEL_SPEC: &str = "assets/models/converted/spec.toml";
const EXHIBIT_TARGET: Vec3 = Vec3::new(0.0, 0.60, 0.0);
const PLATFORM_TOP_Y: f32 = 0.0;
const MODEL_SCALE: f32 = 1.8;
const TOOTH_WIDTH_RATIO: f32 = 0.028;
const TOOTH_HEIGHT_RATIO: f32 = 0.14;
const COMB_TINE_LENGTH_RATIO: f32 = 1.35;
const COMB_TINE_WIDTH_RATIO: f32 = 0.035;
const COMB_TINE_THICKNESS_RATIO: f32 = 0.025;
const COMB_FREE_LENGTH_RATIO: f32 = 0.72;
const COMB_TINE_WIDTH_SPACING_RATIO: f32 = 0.82;
const COMB_TRACK_USABLE_LENGTH_RATIO: f32 = 0.86;
const DEFAULT_TOOTH_CLEARANCE_RATIO: f32 = 0.92;
const COMB_MIN_PLUCK_TICKS: i64 = PPQ / 16;
const COMB_MAX_PLUCK_TICKS: i64 = PPQ / 3;
const COMB_MIN_VIBRATION_TICKS: i64 = PPQ / 2;
const COMB_MAX_VIBRATION_TICKS: i64 = PPQ * 3;
const COMB_GHOST_SAMPLES: [f32; 4] = [-0.38, -0.18, 0.18, 0.38];
const DEFAULT_DEBUG_BIND: &str = "127.0.0.1:4777";

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
        .insert_resource(ClearColor(Color::srgb(0.035, 0.034, 0.032)))
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .insert_resource(PointLightShadowMap { size: 4096 })
        .insert_resource(GlobalAmbientLight {
            color: Color::srgb(0.78, 0.75, 0.70),
            brightness: 38.0,
            ..default()
        })
        .init_resource::<ExhibitControls>()
        .init_resource::<ScreenshotState>()
        .insert_resource(load_movable_model())
        .insert_resource(load_mechanism_layout())
        .init_resource::<PlaybackState>()
        .insert_resource(DebugEndpoint::start_from_env())
        .add_systems(Startup, (setup_scene, setup_audio))
        .add_systems(
            Update,
            (
                apply_debug_actions,
                orbit_camera,
                apply_camera_transform,
                apply_spotlight_controls,
                apply_rig_controls,
                animate_comb_tines,
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
            spot_inner_angle: 0.20,
            spot_outer_angle: 0.42,
            spot_intensity: 1_700_000.0,
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
    pub elapsed_seconds: f32,
    pub last_error: Option<String>,
}

#[derive(Resource)]
struct MechanismResource {
    hint: MechanismLayoutHint,
    comb_animation_events: Vec<CombAnimationEvent>,
    ticks_per_turn: i64,
    quarter_millis: u64,
}

#[derive(Debug, Clone, Serialize)]
struct CombAnimationEvent {
    midi_note: i32,
    onset_tick: i64,
    pluck_start_tick: i64,
    release_tick: i64,
    max_deflection_rad: f32,
    vibration_ticks: i64,
    vibration_hz: f32,
    damping: f32,
    smear_samples: usize,
    source_protrusion: f32,
    source_tooth_length: f32,
    source_velocity: f32,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct MechanismCalibration {
    lowest_midi: i32,
    highest_midi: i32,
    track_count: usize,
    cylinder_length: f32,
    usable_length: f32,
    side_margin: f32,
    track_spacing: f32,
    axial_min: f32,
    axial_max: f32,
}

#[derive(Debug, Clone, Serialize)]
struct TimingGroup {
    key_tick: i64,
    events: Vec<TimingEvent>,
}

#[derive(Debug, Clone, Serialize)]
struct TimingEvent {
    onset_tick: i64,
    midi_note: i32,
}

#[derive(Debug, Clone, Serialize)]
struct TimingValidation {
    ticks_per_turn: i64,
    same_onset_groups: Vec<TimingGroup>,
    same_phase_groups: Vec<TimingGroup>,
    same_onset_group_count: usize,
    same_phase_group_count: usize,
}

#[derive(Resource)]
struct DebugEndpoint {
    bind: Option<String>,
    requests: Mutex<Receiver<DebugRequestEnvelope>>,
}

struct DebugRequestEnvelope {
    action: DebugAction,
    response: Sender<DebugResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum DebugAction {
    DumpState,
    DumpMechanism,
    SetCamera {
        yaw: Option<f32>,
        pitch: Option<f32>,
        radius: Option<f32>,
    },
    SetLight {
        yaw: Option<f32>,
        pitch: Option<f32>,
        inner_angle: Option<f32>,
        outer_angle: Option<f32>,
        intensity: Option<f32>,
    },
    SetLid {
        t: f32,
    },
    SetCylinder {
        degrees: f32,
    },
    SeekTick {
        tick: i64,
    },
    Play,
    Stop,
    Screenshot {
        path: String,
    },
}

#[derive(Debug, Serialize)]
struct DebugResponse {
    ok: bool,
    data: Value,
    error: Option<String>,
}

impl DebugResponse {
    fn ok(data: Value) -> Self {
        Self {
            ok: true,
            data,
            error: None,
        }
    }

    fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: Value::Null,
            error: Some(error.into()),
        }
    }
}

impl DebugEndpoint {
    fn start_from_env() -> Self {
        let (sender, receiver) = mpsc::channel();
        if std::env::var("AIRLET_DEBUG")
            .map(|value| value == "0" || value.eq_ignore_ascii_case("false"))
            .unwrap_or(false)
        {
            return Self {
                bind: None,
                requests: Mutex::new(receiver),
            };
        }

        let bind = std::env::var("AIRLET_DEBUG_BIND").unwrap_or_else(|_| DEFAULT_DEBUG_BIND.into());
        let thread_bind = bind.clone();
        thread::Builder::new()
            .name("airlet-debug-endpoint".to_string())
            .spawn(move || run_debug_endpoint(&thread_bind, sender))
            .expect("failed to spawn airlet debug endpoint");
        Self {
            bind: Some(bind),
            requests: Mutex::new(receiver),
        }
    }
}

fn run_debug_endpoint(bind: &str, sender: Sender<DebugRequestEnvelope>) {
    let listener = match TcpListener::bind(bind) {
        Ok(listener) => listener,
        Err(err) => {
            error!("failed to bind Airlet debug endpoint {bind}: {err}");
            return;
        }
    };
    info!("Airlet debug endpoint listening on {bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => handle_debug_stream(stream, &sender),
            Err(err) => error!("debug endpoint connection error: {err}"),
        }
    }
}

fn handle_debug_stream(mut stream: TcpStream, sender: &Sender<DebugRequestEnvelope>) {
    let cloned = match stream.try_clone() {
        Ok(stream) => stream,
        Err(err) => {
            let _ = writeln!(
                stream,
                "{}",
                serde_json::to_string(&DebugResponse::error(format!(
                    "failed to clone stream: {err}"
                )))
                .unwrap()
            );
            return;
        }
    };
    let reader = BufReader::new(cloned);
    for line in reader.lines() {
        let response = match line {
            Ok(line) if line.trim().is_empty() => continue,
            Ok(line) => dispatch_debug_line(&line, sender),
            Err(err) => DebugResponse::error(format!("failed to read request: {err}")),
        };
        let encoded = serde_json::to_string(&response).unwrap_or_else(|err| {
            serde_json::to_string(&DebugResponse::error(format!(
                "failed to serialize response: {err}"
            )))
            .unwrap()
        });
        if writeln!(stream, "{encoded}").is_err() {
            break;
        }
    }
}

fn dispatch_debug_line(line: &str, sender: &Sender<DebugRequestEnvelope>) -> DebugResponse {
    let action = match serde_json::from_str::<DebugAction>(line) {
        Ok(action) => action,
        Err(err) => return DebugResponse::error(format!("invalid debug action: {err}")),
    };
    let (response_sender, response_receiver) = mpsc::channel();
    if sender
        .send(DebugRequestEnvelope {
            action,
            response: response_sender,
        })
        .is_err()
    {
        return DebugResponse::error("debug action receiver is unavailable");
    }
    response_receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap_or_else(|_| DebugResponse::error("debug action timed out"))
}

#[derive(Resource)]
pub struct ScreenshotState {
    pub path: Option<String>,
    pub requested: bool,
    pub frames_before_capture: u32,
    pub exit_after_capture: bool,
}

impl Default for ScreenshotState {
    fn default() -> Self {
        let path = std::env::var("AIRLET_SCREENSHOT").ok();
        Self {
            exit_after_capture: path.is_some(),
            path,
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

#[derive(Component)]
struct ProceduralMechanism;

#[derive(Component)]
struct CombTineVisual {
    midi_note: i32,
    rest_rotation: Quat,
    smear_sample: Option<f32>,
}

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
        Mesh3d(meshes.add(Cylinder::new(2.25, 0.24).mesh().resolution(128))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            perceptual_roughness: 0.72,
            metallic: 0.0,
            ..default()
        })),
        Transform::from_xyz(0.0, -0.12, 0.0),
    ));

    commands.spawn((
        Name::new("Key Directional Light"),
        DirectionalLight {
            illuminance: 700.0,
            shadow_maps_enabled: true,
            ..default()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: 2.0,
            maximum_distance: 8.0,
            ..default()
        }
        .build(),
        Transform::from_xyz(0.0, 4.0, 0.0).looking_at(EXHIBIT_TARGET, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Fill Light"),
        PointLight {
            intensity: 1_800.0,
            range: 16.0,
            shadow_maps_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.2, 4.4, 4.6),
    ));

    commands.spawn((
        Name::new("Rim Light"),
        PointLight {
            intensity: 2_200.0,
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
        ContactShadows {
            linear_steps: 32,
            thickness: 0.03,
            length: 0.65,
        },
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

fn apply_debug_actions(
    endpoint: Res<DebugEndpoint>,
    mut controls: ResMut<ExhibitControls>,
    mut playback: ResMut<PlaybackState>,
    mechanism: Res<MechanismResource>,
    model: Res<ModelResource>,
    mut screenshot: ResMut<ScreenshotState>,
) {
    let envelopes = {
        let receiver = endpoint.requests.lock().expect("debug receiver poisoned");
        let mut envelopes = Vec::new();
        while let Ok(envelope) = receiver.try_recv() {
            envelopes.push(envelope);
        }
        envelopes
    };
    for envelope in envelopes {
        let response = handle_debug_action(
            envelope.action,
            &mut controls,
            &mut playback,
            &mechanism,
            &model,
            &mut screenshot,
            endpoint.bind.as_deref(),
        );
        let _ = envelope.response.send(response);
    }
}

fn handle_debug_action(
    action: DebugAction,
    controls: &mut ExhibitControls,
    playback: &mut PlaybackState,
    mechanism: &MechanismResource,
    model: &ModelResource,
    screenshot: &mut ScreenshotState,
    debug_bind: Option<&str>,
) -> DebugResponse {
    match action {
        DebugAction::DumpState => DebugResponse::ok(debug_state_json(
            controls, playback, mechanism, model, debug_bind,
        )),
        DebugAction::DumpMechanism => DebugResponse::ok(debug_mechanism_json(mechanism, model)),
        DebugAction::SetCamera { yaw, pitch, radius } => {
            if let Some(yaw) = yaw {
                controls.yaw = yaw;
            }
            if let Some(pitch) = pitch {
                controls.pitch = pitch.clamp(0.08, 1.25);
            }
            if let Some(radius) = radius {
                controls.radius = radius.clamp(3.4, 12.0);
            }
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::SetLight {
            yaw,
            pitch,
            inner_angle,
            outer_angle,
            intensity,
        } => {
            if let Some(yaw) = yaw {
                controls.light_yaw = yaw;
            }
            if let Some(pitch) = pitch {
                controls.light_pitch = pitch.clamp(0.25, 1.45);
            }
            if let Some(outer_angle) = outer_angle {
                controls.spot_outer_angle = outer_angle.clamp(0.22, 1.35);
            }
            if let Some(inner_angle) = inner_angle {
                controls.spot_inner_angle =
                    inner_angle.clamp(0.08, controls.spot_outer_angle - 0.02);
            }
            if let Some(intensity) = intensity {
                controls.spot_intensity = intensity.clamp(5_000.0, 1_200_000.0);
            }
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::SetLid { t } => {
            controls.lid_t = t.clamp(0.0, 1.0);
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::SetCylinder { degrees } => {
            playback.is_playing = false;
            controls.cylinder_spin = false;
            controls.cylinder_degrees = degrees;
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::SeekTick { tick } => {
            playback.is_playing = false;
            controls.cylinder_spin = false;
            playback.elapsed_seconds = tick_to_seconds(tick, mechanism);
            controls.cylinder_degrees = tick_to_cylinder_degrees(tick, mechanism);
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::Play => {
            controls.playback = PlaybackCommand::Start;
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::Stop => {
            controls.playback = PlaybackCommand::Stop;
            DebugResponse::ok(debug_state_json(
                controls, playback, mechanism, model, debug_bind,
            ))
        }
        DebugAction::Screenshot { path } => {
            screenshot.path = Some(path.clone());
            screenshot.requested = false;
            screenshot.frames_before_capture = 2;
            screenshot.exit_after_capture = false;
            DebugResponse::ok(json!({ "screenshot": { "path": path, "requested": true } }))
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
    mechanism: Res<MechanismResource>,
    gltfs: Res<Assets<Gltf>>,
    nodes: Res<Assets<GltfNode>>,
    meshes: Res<Assets<GltfMesh>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut render_meshes: ResMut<Assets<Mesh>>,
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
            MeshGroup::Static => root,
            MeshGroup::Lid => lid_pivot,
            MeshGroup::Cylinder | MeshGroup::Comb => continue,
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

    spawn_hint_mechanism(
        &mut commands,
        &mut render_meshes,
        &mut materials,
        root,
        cylinder_pivot,
        cylinder_pose,
        &model.model,
        &mechanism.hint,
    );

    state.spawned = true;
    info!(
        "spawned closed music box rig: meshes={}, lid_meshes={}, cylinder_meshes={}, teeth={}",
        model.model.spec().closed_model.mesh_indices.len(),
        model.model.spec().lid.meshes.len(),
        model.model.spec().cylinder.meshes.len(),
        mechanism.hint.events.len()
    );
}

fn apply_rig_controls(
    time: Res<Time>,
    mut controls: ResMut<ExhibitControls>,
    mut model: ResMut<ModelResource>,
    mechanism: Res<MechanismResource>,
    mut playback: ResMut<PlaybackState>,
    mut lid_query: Query<&mut Transform, (With<LidPivot>, Without<CylinderPivot>)>,
    mut cylinder_query: Query<&mut Transform, (With<CylinderPivot>, Without<LidPivot>)>,
) {
    model.model.set_lid_t(controls.lid_t);
    if playback.is_playing {
        playback.elapsed_seconds += time.delta_secs();
        controls.cylinder_degrees = synced_cylinder_degrees(playback.elapsed_seconds, &mechanism);
        model.model.set_cylinder_degrees(controls.cylinder_degrees);
        model.model.set_cylinder_spin(false);
    } else {
        model.model.set_cylinder_degrees(controls.cylinder_degrees);
        model.model.set_cylinder_spin(controls.cylinder_spin);
    }
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

fn animate_comb_tines(
    playback: Res<PlaybackState>,
    mechanism: Res<MechanismResource>,
    mut query: Query<(&CombTineVisual, &mut Transform, &mut Visibility)>,
) {
    let current_tick = (playback.is_playing || playback.elapsed_seconds > f32::EPSILON)
        .then(|| seconds_to_tick(playback.elapsed_seconds, &mechanism));
    for (tine, mut transform, mut visibility) in &mut query {
        let sample = current_tick
            .and_then(|tick| comb_tine_sample(tine.midi_note, tick, tine.smear_sample, &mechanism));
        let deflection = sample.map(|sample| sample.deflection_rad).unwrap_or(0.0);
        *visibility = if sample
            .map(|sample| sample.visible || tine.smear_sample.is_none())
            .unwrap_or(tine.smear_sample.is_none())
        {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        transform.rotation = tine.rest_rotation * Quat::from_rotation_x(deflection);
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

fn spawn_hint_mechanism(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    root: Entity,
    cylinder_pivot: Entity,
    cylinder_pose: PivotPose,
    model: &MovableMusicBoxModel,
    hint: &MechanismLayoutHint,
) {
    let axis = vec3(cylinder_pose.axis).normalize_or_zero();
    let radial_zero = measured_comb_radial_direction(model, axis);
    let tangent_zero = axis.cross(radial_zero).normalize_or_zero();
    let cylinder_radius = model.spec().cylinder.radius.max(0.01);
    let cylinder_length = model.spec().cylinder.length.max(0.01);
    let calibration = mechanism_calibration(hint, Some(model), cylinder_length);
    let measured_clearance = model.spec().comb.clearance.max(0.0);
    let tooth_total_height = if measured_clearance > f32::EPSILON {
        measured_clearance * DEFAULT_TOOTH_CLEARANCE_RATIO
    } else {
        cylinder_radius * TOOTH_HEIGHT_RATIO
    };
    let tooth_width = cylinder_length * TOOTH_WIDTH_RATIO;
    let tooth_radius = tooth_width
        .min(calibration.track_spacing * 0.32)
        .min(cylinder_radius * 0.055)
        .max(cylinder_radius * 0.012);
    let tooth_shank_height = (tooth_total_height - tooth_radius).max(tooth_radius);
    let comb_tine_length = measured_comb_tine_length(model, cylinder_radius);
    let comb_free_length = comb_tine_length * COMB_FREE_LENGTH_RATIO;
    let comb_fixed_length = (comb_tine_length - comb_free_length).max(cylinder_radius * 0.035);
    let comb_tine_width = if calibration.track_spacing > f32::EPSILON {
        calibration.track_spacing * COMB_TINE_WIDTH_SPACING_RATIO
    } else {
        cylinder_length * COMB_TINE_WIDTH_RATIO
    };
    let comb_tine_thickness = cylinder_radius * COMB_TINE_THICKNESS_RATIO;
    let comb_fixed_width = calibration.usable_length
        + if calibration.track_count > 1 {
            calibration.track_spacing
        } else {
            comb_tine_width
        };
    let comb_fixed_thickness = comb_tine_thickness * 2.4;
    let cylinder_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.69, 0.23),
        metallic: 0.88,
        perceptual_roughness: 0.24,
        reflectance: 0.78,
        ..default()
    });
    let tooth_material = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.78, 0.25),
        metallic: 0.92,
        perceptual_roughness: 0.20,
        reflectance: 0.82,
        ..default()
    });
    let comb_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.74, 0.77, 0.76),
        metallic: 0.94,
        perceptual_roughness: 0.18,
        reflectance: 0.88,
        ..default()
    });
    let comb_ghost_material = materials.add(StandardMaterial {
        base_color: Color::srgba(0.82, 0.88, 0.88, 0.22),
        metallic: 0.94,
        perceptual_roughness: 0.12,
        reflectance: 0.88,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });

    let cylinder = commands
        .spawn((
            Name::new("Hint Cylinder Body"),
            ProceduralMechanism,
            Mesh3d(
                meshes.add(
                    Cylinder::new(cylinder_radius, cylinder_length)
                        .mesh()
                        .resolution(48),
                ),
            ),
            MeshMaterial3d(cylinder_material),
            Transform::from_rotation(Quat::from_rotation_arc(Vec3::Y, axis)),
            Visibility::Visible,
        ))
        .id();
    commands.entity(cylinder_pivot).add_child(cylinder);

    for tooth in &hint.events {
        let shank_transform = tooth_transform(
            tooth,
            axis,
            radial_zero,
            tangent_zero,
            cylinder_radius,
            &calibration,
            tooth_shank_height * 0.5,
        );
        let shank = commands
            .spawn((
                Name::new(format!(
                    "Hint Tooth Shank midi {} tick {}",
                    tooth.midi_note, tooth.onset_tick
                )),
                ProceduralMechanism,
                Mesh3d(
                    meshes.add(
                        Cylinder::new(tooth_radius, tooth_shank_height)
                            .mesh()
                            .resolution(16),
                    ),
                ),
                MeshMaterial3d(tooth_material.clone()),
                shank_transform,
                Visibility::Visible,
            ))
            .id();
        commands.entity(cylinder_pivot).add_child(shank);

        let cap_transform = tooth_transform(
            tooth,
            axis,
            radial_zero,
            tangent_zero,
            cylinder_radius,
            &calibration,
            tooth_shank_height,
        );
        let cap = commands
            .spawn((
                Name::new(format!(
                    "Hint Tooth Hemispherical Cap midi {} tick {}",
                    tooth.midi_note, tooth.onset_tick
                )),
                ProceduralMechanism,
                Mesh3d(meshes.add(hemisphere_mesh(tooth_radius, 16, 6))),
                MeshMaterial3d(tooth_material.clone()),
                cap_transform,
                Visibility::Visible,
            ))
            .id();
        commands.entity(cylinder_pivot).add_child(cap);
    }

    let tip_radius = measured_comb_tip_radius(model, cylinder_radius, tooth_total_height);
    let fixed_position = axis * ((calibration.axial_min + calibration.axial_max) * 0.5)
        + radial_zero * (tip_radius + comb_free_length + comb_fixed_length * 0.5);
    let fixed_rotation = basis_rotation(axis, radial_zero, tangent_zero);
    let fixed = commands
        .spawn((
            Name::new("Comb Fixed Base"),
            ProceduralMechanism,
            Mesh3d(meshes.add(Cuboid::new(
                comb_fixed_width,
                comb_fixed_length,
                comb_fixed_thickness,
            ))),
            MeshMaterial3d(comb_material.clone()),
            Transform::from_translation(vec3(cylinder_pose.pivot) + fixed_position)
                .with_rotation(fixed_rotation),
            Visibility::Visible,
        ))
        .id();
    commands.entity(root).add_child(fixed);

    for midi_note in calibration.lowest_midi..=calibration.highest_midi {
        spawn_comb_tine_visual(
            commands,
            meshes,
            root,
            cylinder_pose.pivot,
            midi_note,
            None,
            &comb_material,
            axis,
            radial_zero,
            tangent_zero,
            tip_radius,
            comb_free_length,
            comb_tine_width,
            comb_tine_thickness,
            &calibration,
        );
        for smear_sample in COMB_GHOST_SAMPLES {
            spawn_comb_tine_visual(
                commands,
                meshes,
                root,
                cylinder_pose.pivot,
                midi_note,
                Some(smear_sample),
                &comb_ghost_material,
                axis,
                radial_zero,
                tangent_zero,
                tip_radius,
                comb_free_length,
                comb_tine_width,
                comb_tine_thickness,
                &calibration,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_comb_tine_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    cylinder_pivot: [f32; 3],
    midi_note: i32,
    smear_sample: Option<f32>,
    material: &Handle<StandardMaterial>,
    axis: Vec3,
    radial_zero: Vec3,
    tangent_zero: Vec3,
    tip_radius: f32,
    comb_free_length: f32,
    comb_tine_width: f32,
    comb_tine_thickness: f32,
    calibration: &MechanismCalibration,
) {
    let axial = note_axial_position(midi_note, calibration);
    let pivot_position = axis * axial + radial_zero * (tip_radius + comb_free_length);
    let rotation = basis_rotation(axis, radial_zero, tangent_zero);
    let suffix = smear_sample
        .map(|sample| format!(" ghost {sample:.2}"))
        .unwrap_or_default();
    let visibility = if smear_sample.is_some() {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let pivot = commands
        .spawn((
            Name::new(format!("Comb Tine Pivot midi {midi_note}{suffix}")),
            CombTineVisual {
                midi_note,
                rest_rotation: rotation,
                smear_sample,
            },
            ProceduralMechanism,
            Transform::from_translation(vec3(cylinder_pivot) + pivot_position)
                .with_rotation(rotation),
            visibility,
        ))
        .id();
    let tine = commands
        .spawn((
            Name::new(format!("Comb Free Tine midi {midi_note}{suffix}")),
            ProceduralMechanism,
            Mesh3d(meshes.add(Cuboid::new(
                comb_tine_width,
                comb_free_length,
                comb_tine_thickness,
            ))),
            MeshMaterial3d(material.clone()),
            Transform::from_xyz(0.0, -comb_free_length * 0.5, 0.0),
            visibility,
        ))
        .id();
    commands.entity(pivot).add_child(tine);
    commands.entity(root).add_child(pivot);
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
    if state.exit_after_capture && state.requested && std::path::Path::new(path).exists() {
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
    playback.elapsed_seconds = 0.0;
    playback.last_error = None;
}

fn stop_playback(playback: &mut PlaybackState) {
    if let Some(player) = playback.player.take() {
        player.stop();
    }
    playback.is_playing = false;
    playback.elapsed_seconds = 0.0;
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

fn synced_cylinder_degrees(elapsed_seconds: f32, mechanism: &MechanismResource) -> f32 {
    let elapsed_millis = (elapsed_seconds.max(0.0) * 1000.0) as i64;
    let tick = elapsed_millis * PPQ / mechanism.quarter_millis as i64;
    tick_to_cylinder_degrees(tick, mechanism)
}

fn tick_to_cylinder_degrees(tick: i64, mechanism: &MechanismResource) -> f32 {
    let wrapped =
        tick.rem_euclid(mechanism.ticks_per_turn) as f32 / mechanism.ticks_per_turn as f32;
    -wrapped * 360.0
}

fn tick_to_seconds(tick: i64, mechanism: &MechanismResource) -> f32 {
    tick as f32 * mechanism.quarter_millis as f32 / PPQ as f32 / 1000.0
}

fn seconds_to_tick(seconds: f32, mechanism: &MechanismResource) -> i64 {
    (seconds.max(0.0) * 1000.0 * PPQ as f32 / mechanism.quarter_millis as f32).round() as i64
}

#[derive(Debug, Clone, Copy)]
struct CombTineSample {
    deflection_rad: f32,
    visible: bool,
}

fn comb_tine_sample(
    midi_note: i32,
    current_tick: i64,
    smear_sample: Option<f32>,
    mechanism: &MechanismResource,
) -> Option<CombTineSample> {
    let event = nearest_comb_animation_event(midi_note, current_tick, mechanism)?;
    let deflection_rad = comb_tine_deflection_for_event(event, current_tick, smear_sample);
    let visible = smear_sample.is_none()
        || (current_tick >= event.release_tick
            && current_tick <= event.release_tick + event.vibration_ticks);
    Some(CombTineSample {
        deflection_rad,
        visible,
    })
}

fn comb_tine_deflection_for_event(
    event: &CombAnimationEvent,
    current_tick: i64,
    smear_sample: Option<f32>,
) -> f32 {
    let release_tick = event.onset_tick;
    if current_tick < release_tick {
        let pluck_start = event.pluck_start_tick;
        if current_tick < pluck_start {
            return 0.0;
        }
        let progress = (current_tick - pluck_start) as f32 / (release_tick - pluck_start) as f32;
        let eased = progress.clamp(0.0, 1.0).powf(1.7);
        -event.max_deflection_rad * eased
    } else {
        let sample_ticks = smear_sample
            .map(|sample| (sample * visual_vibration_period_ticks(event)).round() as i64)
            .unwrap_or(0);
        let elapsed_ticks = current_tick + sample_ticks - release_tick;
        if elapsed_ticks < 0 || elapsed_ticks > event.vibration_ticks {
            return 0.0;
        }
        let progress = elapsed_ticks as f32 / event.vibration_ticks.max(1) as f32;
        let envelope = (-event.damping * progress).exp();
        -event.max_deflection_rad * envelope * (2.0 * PI * event.vibration_hz * progress).cos()
    }
}

fn nearest_comb_animation_event<'a>(
    midi_note: i32,
    current_tick: i64,
    mechanism: &'a MechanismResource,
) -> Option<&'a CombAnimationEvent> {
    mechanism
        .comb_animation_events
        .iter()
        .filter(|event| event.midi_note == midi_note)
        .filter(|event| {
            current_tick >= event.pluck_start_tick
                && current_tick <= event.release_tick + event.vibration_ticks
        })
        .min_by_key(|event| (current_tick - event.release_tick).abs())
}

fn release_alignment_preview(mechanism: &MechanismResource) -> Vec<Value> {
    mechanism
        .comb_animation_events
        .iter()
        .take(64)
        .map(|event| {
            json!({
                "midi_note": event.midi_note,
                "onset_tick": event.onset_tick,
                "pluck_start_tick": event.pluck_start_tick,
                "pluck_window_ticks": event.release_tick - event.pluck_start_tick,
                "release_tick": event.release_tick,
                "release_equals_audio_onset": true,
                "max_deflection_rad": event.max_deflection_rad,
                "vibration_ticks": event.vibration_ticks,
                "vibration_hz": event.vibration_hz,
                "smear_samples": event.smear_samples,
                "source_protrusion": event.source_protrusion,
                "source_tooth_length": event.source_tooth_length,
                "source_velocity": event.source_velocity,
            })
        })
        .collect()
}

fn visual_vibration_period_ticks(event: &CombAnimationEvent) -> f32 {
    event.vibration_ticks.max(1) as f32 / event.vibration_hz.max(1.0)
}

fn derive_comb_animation_events(
    hint: &MechanismLayoutHint,
    ticks_per_turn: i64,
) -> Vec<CombAnimationEvent> {
    hint.events
        .iter()
        .map(|tooth| derive_comb_animation_event(tooth, ticks_per_turn))
        .collect()
}

fn derive_comb_animation_event(tooth: &ToothHint, ticks_per_turn: i64) -> CombAnimationEvent {
    let circumference = (2.0 * PI * tooth.radius.max(0.01)).max(0.01);
    let footprint_turn_ratio = (tooth.length_along_rotation.max(0.01) / circumference).max(0.0);
    let pluck_ticks = (footprint_turn_ratio * ticks_per_turn as f32 * 1.75).round() as i64;
    let pluck_ticks = pluck_ticks.clamp(COMB_MIN_PLUCK_TICKS, COMB_MAX_PLUCK_TICKS);
    let velocity = tooth.velocity_hint.clamp(0.0, 1.0);
    let protrusion_ratio = (tooth.protrusion / tooth.radius.max(0.01)).clamp(0.0, 0.25);
    let max_deflection_rad = (0.055 + velocity * 0.14 + protrusion_ratio * 0.36).clamp(0.055, 0.26);
    let vibration_ticks =
        ((0.55 + velocity * 1.35 + protrusion_ratio * 2.0) * PPQ as f32).round() as i64;
    let vibration_ticks = vibration_ticks.clamp(COMB_MIN_VIBRATION_TICKS, COMB_MAX_VIBRATION_TICKS);
    let pitch_factor = ((tooth.midi_note - 60) as f32 * 0.35).clamp(-5.0, 8.0);
    let vibration_hz = 18.0 + pitch_factor;
    let damping = (5.5 - velocity * 1.7 + protrusion_ratio * 2.5).clamp(3.8, 6.8);
    CombAnimationEvent {
        midi_note: tooth.midi_note,
        onset_tick: tooth.onset_tick,
        pluck_start_tick: tooth.onset_tick - pluck_ticks,
        release_tick: tooth.onset_tick,
        max_deflection_rad,
        vibration_ticks,
        vibration_hz,
        damping,
        smear_samples: COMB_GHOST_SAMPLES.len(),
        source_protrusion: tooth.protrusion,
        source_tooth_length: tooth.length_along_rotation,
        source_velocity: velocity,
    }
}

fn debug_state_json(
    controls: &ExhibitControls,
    playback: &PlaybackState,
    mechanism: &MechanismResource,
    model: &ModelResource,
    debug_bind: Option<&str>,
) -> Value {
    let tick = seconds_to_tick(playback.elapsed_seconds, mechanism);
    json!({
        "debug": {
            "bind": debug_bind,
        },
        "camera": {
            "yaw": controls.yaw,
            "pitch": controls.pitch,
            "radius": controls.radius,
        },
        "light": {
            "yaw": controls.light_yaw,
            "pitch": controls.light_pitch,
            "distance": controls.light_distance,
            "inner_angle": controls.spot_inner_angle,
            "outer_angle": controls.spot_outer_angle,
            "intensity": controls.spot_intensity,
        },
        "rig": {
            "lid_t": controls.lid_t,
            "cylinder_degrees": controls.cylinder_degrees,
            "cylinder_spin": controls.cylinder_spin,
            "cylinder_radius": model.model.spec().cylinder.radius,
            "cylinder_length": model.model.spec().cylinder.length,
        },
        "playback": {
            "is_playing": playback.is_playing,
            "elapsed_seconds": playback.elapsed_seconds,
            "tick": tick,
            "phase_degrees": tick_to_cylinder_degrees(tick, mechanism),
            "pending_command": format!("{:?}", controls.playback),
            "last_error": playback.last_error,
        },
        "mechanism": {
            "tooth_count": mechanism.hint.events.len(),
            "diagnostic_count": mechanism.hint.diagnostics.len(),
            "ticks_per_turn": mechanism.ticks_per_turn,
            "quarter_millis": mechanism.quarter_millis,
        }
    })
}

fn debug_mechanism_json(mechanism: &MechanismResource, model: &ModelResource) -> Value {
    let cylinder_length = model.model.spec().cylinder.length.max(0.01);
    let cylinder_radius = model.model.spec().cylinder.radius.max(0.01);
    let calibration = mechanism_calibration(&mechanism.hint, Some(&model.model), cylinder_length);
    let timing = timing_validation(&mechanism.hint, mechanism.ticks_per_turn);
    let measured_clearance = model.model.spec().comb.clearance.max(0.0);
    let tooth_total_height = if measured_clearance > f32::EPSILON {
        measured_clearance * DEFAULT_TOOTH_CLEARANCE_RATIO
    } else {
        cylinder_radius * TOOTH_HEIGHT_RATIO
    };
    let tooth_width = cylinder_length * TOOTH_WIDTH_RATIO;
    let tooth_radius = tooth_width
        .min(calibration.track_spacing * 0.32)
        .min(cylinder_radius * 0.055)
        .max(cylinder_radius * 0.012);
    let comb_tine_length = measured_comb_tine_length(&model.model, cylinder_radius);
    let comb_free_length = comb_tine_length * COMB_FREE_LENGTH_RATIO;
    let comb_fixed_length = (comb_tine_length - comb_free_length).max(cylinder_radius * 0.035);
    let comb_tine_width = if calibration.track_spacing > f32::EPSILON {
        calibration.track_spacing * COMB_TINE_WIDTH_SPACING_RATIO
    } else {
        cylinder_length * COMB_TINE_WIDTH_RATIO
    };
    let teeth = mechanism
        .hint
        .events
        .iter()
        .take(64)
        .map(|event| {
            let track_index = track_index(event.midi_note, &calibration);
            json!({
                "midi_note": event.midi_note,
                "track_index": track_index,
                "onset_tick": event.onset_tick,
                "angle_rad": event.angle_rad,
                "source_axial_position": event.axial_position,
                "model_axial_position": note_axial_position(event.midi_note, &calibration),
                "velocity_hint": event.velocity_hint,
            })
        })
        .collect::<Vec<_>>();
    let comb_tracks = (calibration.lowest_midi..=calibration.highest_midi)
        .map(|midi_note| {
            json!({
                "midi_note": midi_note,
                "track_index": track_index(midi_note, &calibration),
                "model_axial_position": note_axial_position(midi_note, &calibration),
            })
        })
        .collect::<Vec<_>>();
    json!({
        "cylinder": {
            "radius": model.model.spec().cylinder.radius,
            "length": model.model.spec().cylinder.length,
            "pivot": model.model.spec().cylinder.pivot,
            "axis": model.model.spec().cylinder.axis,
            "ticks_per_turn": mechanism.ticks_per_turn,
        },
        "calibration": calibration,
        "timing": timing,
        "comb_animation": {
            "event_count": mechanism.comb_animation_events.len(),
            "min_pluck_ticks": COMB_MIN_PLUCK_TICKS,
            "max_pluck_ticks": COMB_MAX_PLUCK_TICKS,
            "min_vibration_ticks": COMB_MIN_VIBRATION_TICKS,
            "max_vibration_ticks": COMB_MAX_VIBRATION_TICKS,
            "ghost_samples": COMB_GHOST_SAMPLES.len(),
            "ghost_phase_offsets": COMB_GHOST_SAMPLES,
            "release_alignment_preview": release_alignment_preview(mechanism),
        },
        "comb": {
            "meshes": &model.model.spec().comb.meshes,
            "radial_direction": model.model.spec().comb.radial_direction,
            "axial_min": calibration.axial_min,
            "axial_max": calibration.axial_max,
            "tip_radius": measured_comb_tip_radius(&model.model, cylinder_radius, tooth_total_height),
            "root_radius": model.model.spec().comb.root_radius,
            "clearance": measured_comb_tip_radius(&model.model, cylinder_radius, tooth_total_height) - cylinder_radius,
            "measured_clearance": model.model.spec().comb.clearance,
            "tine_length": comb_tine_length,
            "free_tine_length": comb_free_length,
            "fixed_base_length": comb_fixed_length,
            "rendered_track_count": calibration.track_count,
            "tooth_tip_radius": cylinder_radius + tooth_total_height,
            "tooth_cap_radius": tooth_radius,
            "tooth_radius_to_track_spacing": if calibration.track_spacing > f32::EPSILON {
                tooth_radius / calibration.track_spacing
            } else {
                0.0
            },
            "tine_width_to_track_spacing": if calibration.track_spacing > f32::EPSILON {
                comb_tine_width / calibration.track_spacing
            } else {
                0.0
            },
            "lowest_midi": calibration.lowest_midi,
            "highest_midi": calibration.highest_midi,
            "track_count": calibration.track_count,
            "usable_length": calibration.usable_length,
            "side_margin": calibration.side_margin,
            "track_spacing": calibration.track_spacing,
            "tracks": comb_tracks,
        },
        "hint": {
            "source_radius": mechanism.hint.cylinder_radius,
            "source_length": mechanism.hint.cylinder_length,
            "track_spacing": mechanism.hint.track_spacing,
            "tooth_count": mechanism.hint.events.len(),
            "teeth_preview": teeth,
            "diagnostics": mechanism.hint.diagnostics,
        }
    })
}

fn load_movable_model() -> ModelResource {
    let spec = ModelSpec::from_toml_path(DEFAULT_MODEL_SPEC)
        .unwrap_or_else(|err| panic!("failed to load default music-box model spec: {err}"));
    ModelResource {
        model: MovableMusicBoxModel::new(spec),
    }
}

fn load_mechanism_layout() -> MechanismResource {
    let plan = defaults::air_intro_plan();
    let timeline = plan.composed_score().expand();
    let ticks_per_turn = timeline_end_tick(&timeline).max(1);
    let note_range = timeline
        .events
        .iter()
        .filter(|event| event.midi_note > 0)
        .map(|event| event.midi_note)
        .fold(None, |range: Option<(i32, i32)>, midi| match range {
            Some((lowest, highest)) => Some((lowest.min(midi), highest.max(midi))),
            None => Some((midi, midi)),
        });
    let (lowest_midi, highest_midi) = note_range.unwrap_or((60, 60));
    let track_count = highest_midi - lowest_midi + 1;
    let mut planner = MechanismPlanner::default();
    planner.lowest_midi = lowest_midi.min(highest_midi);
    planner.highest_midi = highest_midi.max(lowest_midi);
    planner.track_spacing = 1.0;
    planner.cylinder_length = track_count.max(1) as f32;
    planner.ticks_per_turn = ticks_per_turn;
    let hint = planner.plan(&timeline);
    let comb_animation_events = derive_comb_animation_events(&hint, ticks_per_turn);
    MechanismResource {
        hint,
        comb_animation_events,
        ticks_per_turn,
        quarter_millis: timeline.tempo.ticks_to_millis(PPQ),
    }
}

fn timeline_end_tick(timeline: &Timeline) -> i64 {
    timeline
        .events
        .iter()
        .filter(|event| event.midi_note > 0)
        .map(|event| event.onset.0 + event.duration.ticks())
        .max()
        .unwrap_or(PPQ * 4)
}

fn tooth_transform(
    tooth: &ToothHint,
    axis: Vec3,
    radial_zero: Vec3,
    tangent_zero: Vec3,
    cylinder_radius: f32,
    calibration: &MechanismCalibration,
    radial_offset: f32,
) -> Transform {
    let angle = tooth.angle_rad;
    let radial = radial_zero * angle.cos() + tangent_zero * angle.sin();
    let tangent = (-radial_zero * angle.sin() + tangent_zero * angle.cos()).normalize_or_zero();
    let axial = note_axial_position(tooth.midi_note, calibration);
    let position = axis * axial + radial * (cylinder_radius + radial_offset);
    Transform::from_translation(position).with_rotation(basis_rotation(axis, radial, tangent))
}

fn mechanism_calibration(
    hint: &MechanismLayoutHint,
    model: Option<&MovableMusicBoxModel>,
    cylinder_length: f32,
) -> MechanismCalibration {
    let (lowest_midi, highest_midi) = hint
        .events
        .iter()
        .map(|event| event.midi_note)
        .fold((i32::MAX, i32::MIN), |(lowest, highest), midi| {
            (lowest.min(midi), highest.max(midi))
        });
    let (lowest_midi, highest_midi) = if lowest_midi <= highest_midi {
        (lowest_midi, highest_midi)
    } else {
        (60, 60)
    };
    let track_count = (highest_midi - lowest_midi + 1).max(1) as usize;
    let measured_axial = model.and_then(|model| {
        let comb = &model.spec().comb;
        if comb.axial_max > comb.axial_min {
            Some((comb.axial_min, comb.axial_max))
        } else {
            None
        }
    });
    let (axial_min, axial_max) = measured_axial.unwrap_or_else(|| {
        let usable_length = cylinder_length * COMB_TRACK_USABLE_LENGTH_RATIO;
        (-usable_length * 0.5, usable_length * 0.5)
    });
    let usable_length = axial_max - axial_min;
    let side_margin = (cylinder_length - usable_length).max(0.0) * 0.5;
    let track_spacing = if track_count > 1 {
        usable_length / (track_count - 1) as f32
    } else {
        0.0
    };
    MechanismCalibration {
        lowest_midi,
        highest_midi,
        track_count,
        cylinder_length,
        usable_length,
        side_margin,
        track_spacing,
        axial_min,
        axial_max,
    }
}

fn track_index(midi_note: i32, calibration: &MechanismCalibration) -> usize {
    (midi_note - calibration.lowest_midi).max(0) as usize
}

fn note_axial_position(midi_note: i32, calibration: &MechanismCalibration) -> f32 {
    if calibration.track_count <= 1 {
        0.0
    } else {
        let track = track_index(midi_note, calibration).min(calibration.track_count - 1);
        calibration.axial_min + track as f32 * calibration.track_spacing
    }
}

fn measured_comb_radial_direction(model: &MovableMusicBoxModel, axis: Vec3) -> Vec3 {
    let measured = vec3(model.spec().comb.radial_direction);
    let measured = measured - axis * measured.dot(axis);
    if measured.length_squared() > 1e-6 {
        measured.normalize()
    } else {
        cylinder_radial_frame(axis).0
    }
}

fn measured_comb_tip_radius(
    model: &MovableMusicBoxModel,
    cylinder_radius: f32,
    tooth_total_height: f32,
) -> f32 {
    let tip_radius = model.spec().comb.tip_radius;
    if tip_radius > cylinder_radius {
        tip_radius
    } else {
        cylinder_radius + tooth_total_height * 1.2
    }
}

fn measured_comb_tine_length(model: &MovableMusicBoxModel, cylinder_radius: f32) -> f32 {
    let comb = &model.spec().comb;
    if comb.tine_length > f32::EPSILON {
        comb.tine_length
    } else if comb.root_radius > comb.tip_radius {
        comb.root_radius - comb.tip_radius
    } else {
        cylinder_radius * COMB_TINE_LENGTH_RATIO
    }
}

fn hemisphere_mesh(radius: f32, sectors: u32, stacks: u32) -> Mesh {
    let sectors = sectors.max(3);
    let stacks = stacks.max(2);
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut uvs = Vec::<[f32; 2]>::new();
    let mut indices = Vec::<u32>::new();

    positions.push([0.0, radius, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 1.0]);

    for stack in 1..=stacks {
        let phi = FRAC_PI_2 * stack as f32 / stacks as f32;
        let ring_radius = radius * phi.sin();
        let y = radius * phi.cos();
        for sector in 0..sectors {
            let theta = 2.0 * PI * sector as f32 / sectors as f32;
            let x = ring_radius * theta.cos();
            let z = ring_radius * theta.sin();
            let normal = Vec3::new(x, y, z).normalize_or_zero();
            positions.push([x, y, z]);
            normals.push(normal.to_array());
            uvs.push([
                sector as f32 / sectors as f32,
                1.0 - stack as f32 / stacks as f32,
            ]);
        }
    }

    for sector in 0..sectors {
        let current = 1 + sector;
        let next = 1 + (sector + 1) % sectors;
        indices.extend_from_slice(&[0, next, current]);
    }

    for stack in 1..stacks {
        let prev = 1 + (stack - 1) * sectors;
        let curr = 1 + stack * sectors;
        for sector in 0..sectors {
            let prev_a = prev + sector;
            let prev_b = prev + (sector + 1) % sectors;
            let curr_a = curr + sector;
            let curr_b = curr + (sector + 1) % sectors;
            indices.extend_from_slice(&[prev_a, prev_b, curr_b]);
            indices.extend_from_slice(&[prev_a, curr_b, curr_a]);
        }
    }

    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
    .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
    .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    .with_inserted_indices(Indices::U32(indices))
}

fn timing_validation(hint: &MechanismLayoutHint, ticks_per_turn: i64) -> TimingValidation {
    let same_onset_groups = grouped_timing_events(hint.events.iter().map(|event| {
        (
            event.onset_tick,
            TimingEvent {
                onset_tick: event.onset_tick,
                midi_note: event.midi_note,
            },
        )
    }));
    let same_phase_groups = grouped_timing_events(hint.events.iter().map(|event| {
        (
            event.onset_tick.rem_euclid(ticks_per_turn),
            TimingEvent {
                onset_tick: event.onset_tick,
                midi_note: event.midi_note,
            },
        )
    }));
    TimingValidation {
        ticks_per_turn,
        same_onset_group_count: same_onset_groups.len(),
        same_phase_group_count: same_phase_groups.len(),
        same_onset_groups,
        same_phase_groups,
    }
}

fn grouped_timing_events(
    keyed_events: impl IntoIterator<Item = (i64, TimingEvent)>,
) -> Vec<TimingGroup> {
    let mut groups = BTreeMap::<i64, Vec<TimingEvent>>::new();
    for (key_tick, event) in keyed_events {
        groups.entry(key_tick).or_default().push(event);
    }
    groups
        .into_iter()
        .filter_map(|(key_tick, events)| {
            if events.len() > 1 {
                Some(TimingGroup { key_tick, events })
            } else {
                None
            }
        })
        .collect()
}

fn cylinder_radial_frame(axis: Vec3) -> (Vec3, Vec3) {
    let mut radial = Vec3::Y - axis * Vec3::Y.dot(axis);
    if radial.length_squared() < 1e-6 {
        radial = Vec3::X - axis * Vec3::X.dot(axis);
    }
    let radial = radial.normalize_or_zero();
    let tangent = axis.cross(radial).normalize_or_zero();
    (radial, tangent)
}

fn basis_rotation(x_axis: Vec3, y_axis: Vec3, z_axis: Vec3) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(
        x_axis.normalize_or_zero(),
        y_axis.normalize_or_zero(),
        z_axis.normalize_or_zero(),
    ))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_clock_drives_cylinder_phase_from_ticks() {
        let mechanism = MechanismResource {
            hint: MechanismLayoutHint {
                cylinder_radius: 18.0,
                cylinder_length: 80.0,
                track_spacing: 2.0,
                events: Vec::new(),
                diagnostics: Vec::new(),
            },
            comb_animation_events: Vec::new(),
            ticks_per_turn: PPQ * 4,
            quarter_millis: 500,
        };

        assert_eq!(synced_cylinder_degrees(0.0, &mechanism), -0.0);
        assert_eq!(synced_cylinder_degrees(1.0, &mechanism), -180.0);
        assert_eq!(synced_cylinder_degrees(2.0, &mechanism), -0.0);
    }

    #[test]
    fn calibration_maps_notes_to_even_comb_tracks() {
        let hint = MechanismLayoutHint {
            cylinder_radius: 18.0,
            cylinder_length: 12.0,
            track_spacing: 1.0,
            events: vec![
                ToothHint {
                    midi_note: 60,
                    onset_tick: 0,
                    angle_rad: 0.0,
                    axial_position: 0.0,
                    radius: 18.0,
                    protrusion: 1.0,
                    width: 0.8,
                    length_along_rotation: 1.2,
                    velocity_hint: 0.8,
                },
                ToothHint {
                    midi_note: 64,
                    onset_tick: PPQ,
                    angle_rad: 1.0,
                    axial_position: 4.0,
                    radius: 18.0,
                    protrusion: 1.0,
                    width: 0.8,
                    length_along_rotation: 1.2,
                    velocity_hint: 0.8,
                },
            ],
            diagnostics: Vec::new(),
        };

        let calibration = mechanism_calibration(&hint, None, 10.0);

        assert_eq!(calibration.lowest_midi, 60);
        assert_eq!(calibration.highest_midi, 64);
        assert_eq!(calibration.track_count, 5);
        assert!((calibration.usable_length - 8.6).abs() < 1e-5);
        assert!((note_axial_position(60, &calibration) + 4.3).abs() < 1e-5);
        assert!(note_axial_position(62, &calibration).abs() < 1e-5);
        assert!((note_axial_position(64, &calibration) - 4.3).abs() < 1e-5);
    }

    #[test]
    fn default_mechanism_uses_full_song_turn_without_phase_collisions() {
        let mechanism = load_mechanism_layout();
        let timing = timing_validation(&mechanism.hint, mechanism.ticks_per_turn);

        assert_eq!(mechanism.ticks_per_turn, 29_760);
        assert_eq!(timing.same_onset_group_count, 0);
        assert_eq!(timing.same_phase_group_count, 0);
        assert!(
            mechanism
                .hint
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("outside comb range"))
        );
        assert!(
            mechanism
                .hint
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("dense teeth near angle"))
        );
    }

    #[test]
    fn default_model_spec_exposes_measured_comb_clearance() {
        let model = load_movable_model();
        let spec = model.model.spec();
        let calibration = mechanism_calibration(
            &load_mechanism_layout().hint,
            Some(&model.model),
            spec.cylinder.length,
        );

        assert!(spec.cylinder.radius > 0.0);
        assert!(spec.comb.tip_radius > spec.cylinder.radius);
        assert!(spec.comb.clearance > 0.0);
        assert_eq!(spec.comb.meshes, vec![23]);
        assert!((calibration.axial_min - spec.comb.axial_min).abs() < 1e-6);
        assert!((calibration.axial_max - spec.comb.axial_max).abs() < 1e-6);
    }

    #[test]
    fn comb_tine_release_is_aligned_to_audio_onset() {
        let mechanism = load_mechanism_layout();
        let event = mechanism.comb_animation_events.first().unwrap();
        let before_pluck = comb_tine_sample(
            event.midi_note,
            event.pluck_start_tick - 1,
            None,
            &mechanism,
        )
        .map(|sample| sample.deflection_rad)
        .unwrap_or(0.0);
        let pre_release =
            comb_tine_sample(event.midi_note, event.release_tick - 1, None, &mechanism)
                .unwrap()
                .deflection_rad;
        let at_release = comb_tine_sample(event.midi_note, event.release_tick, None, &mechanism)
            .unwrap()
            .deflection_rad;
        let ghost_at_release = comb_tine_sample(
            event.midi_note,
            event.release_tick,
            Some(COMB_GHOST_SAMPLES[0]),
            &mechanism,
        )
        .unwrap();

        assert_eq!(before_pluck, 0.0);
        assert!(pre_release < 0.0);
        assert!(at_release < 0.0);
        assert!(ghost_at_release.visible);

        let preview = release_alignment_preview(&mechanism);
        assert_eq!(preview[0]["onset_tick"], preview[0]["release_tick"]);
        assert_eq!(preview[0]["release_equals_audio_onset"], true);
    }

    #[test]
    fn comb_animation_is_derived_from_tooth_hint_strength() {
        let weak = ToothHint {
            midi_note: 60,
            onset_tick: PPQ,
            angle_rad: 0.0,
            axial_position: 0.0,
            radius: 18.0,
            protrusion: 0.7,
            width: 0.5,
            length_along_rotation: 0.6,
            velocity_hint: 0.2,
        };
        let strong = ToothHint {
            protrusion: 1.5,
            length_along_rotation: 1.8,
            velocity_hint: 0.9,
            ..weak.clone()
        };

        let weak_event = derive_comb_animation_event(&weak, PPQ * 8);
        let strong_event = derive_comb_animation_event(&strong, PPQ * 8);

        assert!(strong_event.pluck_start_tick < weak_event.pluck_start_tick);
        assert!(strong_event.max_deflection_rad > weak_event.max_deflection_rad);
        assert!(strong_event.vibration_ticks > weak_event.vibration_ticks);
        assert_eq!(strong_event.release_tick, strong.onset_tick);
    }
}
