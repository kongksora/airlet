use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    sync::{
        Mutex,
        mpsc::{self, Receiver, Sender},
    },
    thread,
    time::Duration,
};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::controls::ExhibitControls;
use crate::lid::LidState;
use crate::lighting::ExhibitLightingConfig;
use crate::mechanism_view::{
    COMB_DEFLECTION_SCALE, COMB_FREE_LENGTH_RATIO, COMB_GHOST_SAMPLES, COMB_MAX_DEFLECTION_RAD,
    COMB_MAX_PLUCK_TICKS, COMB_MAX_VIBRATION_TICKS, COMB_MIN_DEFLECTION_RAD, COMB_MIN_PLUCK_TICKS,
    COMB_MIN_VIBRATION_TICKS, COMB_TINE_WIDTH_RATIO, COMB_TINE_WIDTH_SPACING_RATIO,
    CYLINDER_PLAYBACK_ROTATION_SIGN, DEFAULT_TOOTH_CLEARANCE_RATIO, MechanismResource,
    TOOTH_HEIGHT_RATIO, TOOTH_WIDTH_RATIO,
};
use crate::model_view::ModelResource;
use crate::playback::{self, AudioOutputState};
use crate::scene::{CAMERA_MAX_PITCH, CAMERA_MAX_RADIUS, CAMERA_MIN_PITCH, CAMERA_MIN_RADIUS};
use crate::screenshot::ScreenshotState;
use crate::twin::MusicBoxTwinState;
use crate::vec3;
use crate::visual_config::MechanismVisualConfig;
use crate::winding::WindingState;

pub const DEFAULT_DEBUG_BIND: &str = "127.0.0.1:4777";

#[derive(Resource)]
pub struct DebugEndpoint {
    pub bind: Option<String>,
    pub requests: Mutex<Receiver<DebugRequestEnvelope>>,
}

pub struct DebugRequestEnvelope {
    pub action: DebugAction,
    pub response: Sender<DebugResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum DebugAction {
    DescribeActions,
    DumpState,
    DumpMechanism,
    SetCamera {
        yaw: Option<f32>,
        pitch: Option<f32>,
        radius: Option<f32>,
        target: Option<[f32; 3]>,
    },
    SetUi {
        visible: bool,
    },
    SetLight {
        yaw: Option<f32>,
        pitch: Option<f32>,
        inner_angle: Option<f32>,
        outer_angle: Option<f32>,
        intensity: Option<f32>,
        key: Option<f32>,
        fill: Option<f32>,
        rim: Option<f32>,
        accent: Option<f32>,
        ambient: Option<f32>,
        environment: Option<f32>,
    },
    SetLid {
        t: f32,
    },
    SetWinding {
        hovered: Option<bool>,
        pressed: Option<bool>,
        wind_amount: Option<f32>,
        key_degrees: Option<f32>,
        pending_audio_cycles: Option<u32>,
    },
    FullWind,
    Pause,
    Reset,
    Screenshot {
        path: String,
    },
}

#[derive(Debug, Serialize)]
pub struct ActionCatalog {
    pub version: u32,
    pub actions: Vec<ActionSpec>,
}

#[derive(Debug, Serialize)]
pub struct ActionSpec {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub parameters: Vec<ActionParameterSpec>,
}

#[derive(Debug, Serialize)]
pub struct ActionParameterSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub required: bool,
    pub schema: Value,
}

impl ActionSpec {
    fn new(
        name: &'static str,
        title: &'static str,
        description: &'static str,
        parameters: Vec<ActionParameterSpec>,
    ) -> Self {
        Self {
            name,
            title,
            description,
            parameters,
        }
    }
}

fn parameter(
    name: &'static str,
    description: &'static str,
    required: bool,
    schema: Value,
) -> ActionParameterSpec {
    ActionParameterSpec {
        name,
        description,
        required,
        schema,
    }
}

pub fn action_catalog() -> ActionCatalog {
    ActionCatalog {
        version: 1,
        actions: vec![
            ActionSpec::new(
                "describe_actions",
                "Describe Actions",
                "Return the Rust-owned debug action catalog used by clients and MCP adapters.",
                Vec::new(),
            ),
            ActionSpec::new(
                "dump_state",
                "Dump State",
                "Return the current camera, light, rig, playback, winding, twin, and mechanism state.",
                Vec::new(),
            ),
            ActionSpec::new(
                "dump_mechanism",
                "Dump Mechanism",
                "Return detailed cylinder, comb, tooth, animation, and score-to-geometry mapping data.",
                Vec::new(),
            ),
            ActionSpec::new(
                "set_camera",
                "Set Camera",
                "Set orbit camera parameters and return the updated state.",
                vec![
                    parameter(
                        "yaw",
                        "Camera yaw in radians.",
                        false,
                        json!({"type": "number"}),
                    ),
                    parameter(
                        "pitch",
                        "Camera pitch in radians.",
                        false,
                        json!({"type": "number", "minimum": CAMERA_MIN_PITCH, "maximum": CAMERA_MAX_PITCH}),
                    ),
                    parameter(
                        "radius",
                        "Camera distance from target.",
                        false,
                        json!({"type": "number", "minimum": CAMERA_MIN_RADIUS, "maximum": CAMERA_MAX_RADIUS}),
                    ),
                    parameter(
                        "target",
                        "Camera target point as [x, y, z].",
                        false,
                        json!({"type": "array", "items": {"type": "number"}, "minItems": 3, "maxItems": 3}),
                    ),
                ],
            ),
            ActionSpec::new(
                "set_ui",
                "Set UI",
                "Show or hide the in-app egui control panel.",
                vec![parameter(
                    "visible",
                    "Whether the UI is visible.",
                    true,
                    json!({"type": "boolean"}),
                )],
            ),
            ActionSpec::new(
                "set_light",
                "Set Light",
                "Set spotlight direction plus studio, ambient, and IBL intensities, then return the updated state.",
                vec![
                    parameter(
                        "yaw",
                        "Light yaw in radians.",
                        false,
                        json!({"type": "number"}),
                    ),
                    parameter(
                        "pitch",
                        "Light pitch in radians.",
                        false,
                        json!({"type": "number", "minimum": 0.25, "maximum": 1.45}),
                    ),
                    parameter(
                        "inner_angle",
                        "Spotlight inner cone angle.",
                        false,
                        json!({"type": "number", "minimum": 0.08, "maximum": 1.0}),
                    ),
                    parameter(
                        "outer_angle",
                        "Spotlight outer cone angle.",
                        false,
                        json!({"type": "number", "minimum": 0.22, "maximum": 1.35}),
                    ),
                    parameter(
                        "intensity",
                        "Spotlight intensity.",
                        false,
                        json!({"type": "number", "minimum": 5000.0, "maximum": 1200000.0}),
                    ),
                    parameter(
                        "key",
                        "Key directional light illuminance.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.min, "maximum": ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.max}),
                    ),
                    parameter(
                        "fill",
                        "Fill point light intensity.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::FILL_INTENSITY_RANGE.min, "maximum": ExhibitLightingConfig::FILL_INTENSITY_RANGE.max}),
                    ),
                    parameter(
                        "rim",
                        "Rim point light intensity.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::RIM_INTENSITY_RANGE.min, "maximum": ExhibitLightingConfig::RIM_INTENSITY_RANGE.max}),
                    ),
                    parameter(
                        "accent",
                        "Warm accent point light intensity.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.min, "maximum": ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.max}),
                    ),
                    parameter(
                        "ambient",
                        "Global ambient light brightness.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.min, "maximum": ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.max}),
                    ),
                    parameter(
                        "environment",
                        "Solid-color environment map intensity for IBL-style fill.",
                        false,
                        json!({"type": "number", "minimum": ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE.min, "maximum": ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE.max}),
                    ),
                ],
            ),
            ActionSpec::new(
                "set_lid",
                "Set Lid",
                "Set lid open parameter t in [0, 1] and return the updated state.",
                vec![parameter(
                    "t",
                    "Lid open parameter.",
                    true,
                    json!({"type": "number", "minimum": 0.0, "maximum": 1.0}),
                )],
            ),
            ActionSpec::new(
                "set_winding",
                "Set Winding",
                "Set winding-key hover/press state and twin validation fields.",
                vec![
                    parameter(
                        "hovered",
                        "Debug override for winding-key hover state.",
                        false,
                        json!({"type": "boolean"}),
                    ),
                    parameter(
                        "pressed",
                        "Whether the winding key is pressed.",
                        false,
                        json!({"type": "boolean"}),
                    ),
                    parameter(
                        "wind_amount",
                        "Spring energy amount.",
                        false,
                        json!({"type": "number", "minimum": 0.0}),
                    ),
                    parameter(
                        "key_degrees",
                        "Crank key angle in degrees.",
                        false,
                        json!({"type": "number"}),
                    ),
                    parameter(
                        "pending_audio_cycles",
                        "Debug-only queued whole-cycle audio starts.",
                        false,
                        json!({"type": "integer", "minimum": 0}),
                    ),
                ],
            ),
            ActionSpec::new(
                "full_wind",
                "Full Wind",
                "Fully wind the spring once and release into playback.",
                Vec::new(),
            ),
            ActionSpec::new(
                "pause",
                "Pause Or Continue",
                "Pause playback, or continue from the paused mechanical phase.",
                Vec::new(),
            ),
            ActionSpec::new(
                "reset",
                "Reset",
                "Stop playback and reset the twin.",
                Vec::new(),
            ),
            ActionSpec::new(
                "screenshot",
                "Screenshot",
                "Capture the primary Bevy window to a PNG path.",
                vec![parameter(
                    "path",
                    "Output PNG path.",
                    true,
                    json!({"type": "string"}),
                )],
            ),
        ],
    }
}

#[derive(Debug, Serialize)]
pub struct DebugResponse {
    pub ok: bool,
    pub data: Value,
    pub error: Option<String>,
}

impl DebugResponse {
    pub fn ok(data: Value) -> Self {
        Self {
            ok: true,
            data,
            error: None,
        }
    }

    pub fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: Value::Null,
            error: Some(error.into()),
        }
    }
}

impl DebugEndpoint {
    /// Start a debug endpoint only when the user explicitly opts in.
    ///
    /// Set `AIRLET_DEBUG=1` to enable the local JSON debug endpoint.
    /// Set `AIRLET_DEBUG_BIND` to override the default `127.0.0.1:4777`.
    /// Any other value (unset, `0`, `false`) keeps the endpoint disabled.
    pub fn start_from_env() -> Self {
        let (sender, receiver) = mpsc::channel();
        let enabled = std::env::var("AIRLET_DEBUG")
            .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        if !enabled {
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

pub fn apply_debug_actions(
    endpoint: Res<DebugEndpoint>,
    mut controls: ResMut<ExhibitControls>,
    mut audio_output: ResMut<AudioOutputState>,
    mechanism: Res<MechanismResource>,
    model: Res<ModelResource>,
    mut screenshot: ResMut<ScreenshotState>,
    visual_config: Res<MechanismVisualConfig>,
    mut winding: ResMut<WindingState>,
    mut twin: ResMut<MusicBoxTwinState>,
    mut lid: ResMut<LidState>,
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
            &mut audio_output,
            &mechanism,
            &model,
            &mut screenshot,
            &visual_config,
            &mut winding,
            &mut twin,
            &mut lid,
            endpoint.bind.as_deref(),
        );
        let _ = envelope.response.send(response);
    }
}

fn handle_debug_action(
    action: DebugAction,
    controls: &mut ExhibitControls,
    audio_output: &mut AudioOutputState,
    mechanism: &MechanismResource,
    model: &ModelResource,
    screenshot: &mut ScreenshotState,
    visual_config: &MechanismVisualConfig,
    winding: &mut WindingState,
    twin: &mut MusicBoxTwinState,
    lid: &mut LidState,
    debug_bind: Option<&str>,
) -> DebugResponse {
    match action {
        DebugAction::DescribeActions => DebugResponse::ok(json!(action_catalog())),
        DebugAction::DumpState => DebugResponse::ok(debug_state_json(
            controls,
            audio_output,
            mechanism,
            model,
            winding,
            twin,
            lid,
            debug_bind,
        )),
        DebugAction::DumpMechanism => {
            DebugResponse::ok(debug_mechanism_json(mechanism, model, visual_config))
        }
        DebugAction::SetCamera {
            yaw,
            pitch,
            radius,
            target,
        } => {
            if let Some(yaw) = yaw {
                controls.yaw = yaw;
            }
            if let Some(pitch) = pitch {
                controls.pitch = pitch.clamp(CAMERA_MIN_PITCH, CAMERA_MAX_PITCH);
            }
            if let Some(radius) = radius {
                controls.radius = radius.clamp(CAMERA_MIN_RADIUS, CAMERA_MAX_RADIUS);
            }
            if let Some(target) = target {
                controls.camera_target = vec3(target);
            }
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::SetUi { visible } => {
            controls.show_ui = visible;
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::SetLight {
            yaw,
            pitch,
            inner_angle,
            outer_angle,
            intensity,
            key,
            fill,
            rim,
            accent,
            ambient,
            environment,
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
            if let Some(key) = key {
                controls.key_illuminance = ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.clamp(key);
            }
            if let Some(fill) = fill {
                controls.fill_intensity = ExhibitLightingConfig::FILL_INTENSITY_RANGE.clamp(fill);
            }
            if let Some(rim) = rim {
                controls.rim_intensity = ExhibitLightingConfig::RIM_INTENSITY_RANGE.clamp(rim);
            }
            if let Some(accent) = accent {
                controls.accent_intensity =
                    ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.clamp(accent);
            }
            if let Some(ambient) = ambient {
                controls.ambient_brightness =
                    ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.clamp(ambient);
            }
            if let Some(environment) = environment {
                controls.environment_intensity =
                    ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE.clamp(environment);
            }
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::SetLid { t } => {
            lid.set_manual(t);
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::SetWinding {
            hovered,
            pressed,
            wind_amount,
            key_degrees,
            pending_audio_cycles,
        } => {
            if let Some(hovered) = hovered {
                winding.hovered = hovered;
            }
            if let Some(pressed) = pressed {
                winding.pressed = pressed;
                if pressed {
                    twin.begin_winding();
                } else {
                    twin.release_winding(playback::mechanical_cycle_seconds(mechanism));
                }
            }
            if let Some(wind_amount) = wind_amount {
                twin.set_spring_energy_for_debug(wind_amount);
                winding.wind_amount = twin.spring_energy;
            }
            if let Some(key_degrees) = key_degrees {
                twin.set_key_degrees_for_debug(key_degrees);
                winding.key_degrees = key_degrees;
            }
            if let Some(pending_audio_cycles) = pending_audio_cycles {
                twin.set_pending_audio_cycles_for_debug(pending_audio_cycles);
            }
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::FullWind => {
            winding.clear_active_wind();
            playback::stop_audio(audio_output);
            twin.wind_full_and_release(playback::mechanical_cycle_seconds(mechanism));
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::Pause => {
            winding.clear_active_wind();
            playback::stop_audio(audio_output);
            twin.toggle_pause(playback::mechanical_cycle_seconds(mechanism));
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
            ))
        }
        DebugAction::Reset => {
            winding.clear_active_wind();
            twin.reset();
            playback::stop_audio(audio_output);
            controls.playback = crate::playback::PlaybackCommand::Reset;
            DebugResponse::ok(debug_state_json(
                controls,
                audio_output,
                mechanism,
                model,
                winding,
                twin,
                lid,
                debug_bind,
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

fn debug_state_json(
    controls: &ExhibitControls,
    audio_output: &AudioOutputState,
    mechanism: &MechanismResource,
    model: &ModelResource,
    winding: &WindingState,
    twin: &MusicBoxTwinState,
    lid: &LidState,
    debug_bind: Option<&str>,
) -> Value {
    let tick = playback::seconds_to_tick(twin.mechanical_seconds, mechanism);
    json!({
        "debug": {
            "bind": debug_bind,
        },
        "camera": {
            "yaw": controls.yaw,
            "pitch": controls.pitch,
            "radius": controls.radius,
            "target": controls.camera_target.to_array(),
        },
        "ui": {
            "visible": controls.show_ui,
        },
        "light": {
            "spot": {
                "yaw": controls.light_yaw,
                "pitch": controls.light_pitch,
                "distance": controls.light_distance,
                "inner_angle": controls.spot_inner_angle,
                "outer_angle": controls.spot_outer_angle,
                "intensity": controls.spot_intensity,
            },
            "studio": {
                "key_illuminance": controls.key_illuminance,
                "fill_intensity": controls.fill_intensity,
                "rim_intensity": controls.rim_intensity,
                "accent_intensity": controls.accent_intensity,
            },
            "ambient": {
                "brightness": controls.ambient_brightness,
            },
            "environment": {
                "intensity": controls.environment_intensity,
            },
        },
        "lid": {
            "mode": format!("{:?}", lid.mode),
            "t": lid.t,
            "target_t": lid.target_t,
            "velocity": lid.velocity,
            "max_speed": lid.max_speed,
        },
        "rig": {
            "cylinder_degrees": controls.cylinder_degrees,
            "cylinder_radius": model.model.spec().cylinder.radius,
            "cylinder_length": model.model.spec().cylinder.length,
        },
        "playback": {
            "is_playing": matches!(twin.mode, crate::twin::TwinMode::Playing) || !audio_output.active_cycles.is_empty(),
            "elapsed_seconds": twin.mechanical_seconds,
            "duration_seconds": playback::audio_duration_seconds(audio_output),
            "mechanical_cycle_seconds": playback::mechanical_cycle_seconds(mechanism),
            "active_cycle_count": audio_output.active_cycles.len(),
            "rate": controls.playback_rate,
            "tick": tick,
            "phase_degrees": playback::tick_to_cylinder_degrees(tick, mechanism),
            "pending_command": format!("{:?}", controls.playback),
            "last_error": audio_output.last_error,
        },
        "winding": {
            "hovered": winding.hovered,
            "pressed": winding.pressed,
            "wind_amount": winding.wind_amount,
            "max_wind_amount": winding.max_wind_amount,
            "key_degrees": winding.key_degrees,
            "last_released_wind_amount": winding.last_released_wind_amount,
            "last_started_cycles": winding.last_started_cycles,
        },
        "twin": {
            "mode": format!("{:?}", twin.mode),
            "spring_energy": twin.spring_energy,
            "max_spring_energy": twin.max_spring_energy,
            "key_degrees": twin.key_degrees,
            "cylinder_degrees": twin.cylinder_degrees,
            "mechanical_seconds": twin.mechanical_seconds,
            "next_cycle_index": twin.next_cycle_index,
            "pending_audio_cycles": twin.pending_audio_cycles,
            "pending_audio_resume_seconds": twin.pending_audio_resume_seconds,
            "pending_audio_start_count": twin.pending_audio_start_count(),
        },
        "mechanism": {
            "tooth_count": mechanism.hint.events.len(),
            "diagnostic_count": mechanism.hint.diagnostics.len(),
            "ticks_per_turn": mechanism.ticks_per_turn,
            "quarter_millis": mechanism.quarter_millis,
        }
    })
}

fn debug_mechanism_json(
    mechanism: &MechanismResource,
    model: &ModelResource,
    visual_config: &MechanismVisualConfig,
) -> Value {
    let cylinder_length = model.model.spec().cylinder.length.max(0.01);
    let cylinder_radius = model.model.spec().cylinder.radius.max(0.01);
    let axis = vec3(model.model.spec().cylinder.axis).normalize_or_zero();
    let radial_zero = crate::mechanism_view::measured_comb_radial_direction(&model.model, axis);
    let tangent_zero = axis.cross(radial_zero).normalize_or_zero();
    let positive_deflection_direction =
        crate::mechanism_view::basis_rotation(axis, radial_zero, tangent_zero) * -Vec3::Z;
    let tooth_travel_direction = tangent_zero * CYLINDER_PLAYBACK_ROTATION_SIGN;
    let comb_deflection_sign =
        crate::mechanism_view::comb_tine_deflection_sign(axis, radial_zero, tangent_zero);
    let calibration = crate::mechanism_view::mechanism_calibration(
        &mechanism.hint,
        Some(&model.model),
        cylinder_length,
    );
    let timing =
        crate::mechanism_view::timing_validation(&mechanism.hint, mechanism.ticks_per_turn);
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
    let comb_tine_length =
        crate::mechanism_view::measured_comb_tine_length(&model.model, cylinder_radius);
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
            let track_index =
                crate::mechanism_view::track_index(event.midi_note, &calibration);
            json!({
                "midi_note": event.midi_note,
                "track_index": track_index,
                "onset_tick": event.onset_tick,
                "angle_rad": event.angle_rad,
                "source_axial_position": event.axial_position,
                "model_axial_position": crate::mechanism_view::note_axial_position(event.midi_note, &calibration),
                "velocity_hint": event.velocity_hint,
            })
        })
        .collect::<Vec<_>>();
    let comb_tracks = (calibration.lowest_midi..=calibration.highest_midi)
        .map(|midi_note| {
            json!({
                "midi_note": midi_note,
                "track_index": crate::mechanism_view::track_index(midi_note, &calibration),
                "model_axial_position": crate::mechanism_view::note_axial_position(midi_note, &calibration),
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
            "deflection_scale": COMB_DEFLECTION_SCALE,
            "min_deflection_rad": COMB_MIN_DEFLECTION_RAD,
            "max_deflection_rad": COMB_MAX_DEFLECTION_RAD,
            "ghost_samples": COMB_GHOST_SAMPLES.len(),
            "ghost_phase_offsets": COMB_GHOST_SAMPLES,
            "comb_mesh_batches": 1 + COMB_GHOST_SAMPLES.len(),
            "free_tines_share_single_mesh": true,
            "cylinder_playback_rotation_sign": CYLINDER_PLAYBACK_ROTATION_SIGN,
            "positive_deflection_direction": positive_deflection_direction.to_array(),
            "tooth_travel_direction": tooth_travel_direction.to_array(),
            "oriented_deflection_sign": comb_deflection_sign,
            "release_alignment_preview": crate::comb_animation::release_alignment_preview(mechanism),
        },
        "comb": {
            "meshes": &model.model.spec().comb.meshes,
            "radial_direction": model.model.spec().comb.radial_direction,
            "axial_min": calibration.axial_min,
            "axial_max": calibration.axial_max,
            "tip_radius": crate::mechanism_view::measured_comb_tip_radius(&model.model, cylinder_radius, tooth_total_height),
            "root_radius": model.model.spec().comb.root_radius,
            "clearance": crate::mechanism_view::measured_comb_tip_radius(&model.model, cylinder_radius, tooth_total_height) - cylinder_radius,
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
        },
        "visual_config": {
            "tooth": {
                "width_ratio": visual_config.tooth.width_ratio,
                "height_ratio": visual_config.tooth.height_ratio,
                "default_clearance_ratio": visual_config.tooth.default_clearance_ratio,
            },
            "comb": {
                "tine_length_ratio": visual_config.comb.tine_length_ratio,
                "tine_width_ratio": visual_config.comb.tine_width_ratio,
                "tine_thickness_ratio": visual_config.comb.tine_thickness_ratio,
                "free_length_ratio": visual_config.comb.free_length_ratio,
                "tine_width_spacing_ratio": visual_config.comb.tine_width_spacing_ratio,
                "track_usable_length_ratio": visual_config.comb.track_usable_length_ratio,
            },
            "animation": {
                "min_pluck_ticks": visual_config.animation.min_pluck_ticks,
                "max_pluck_ticks": visual_config.animation.max_pluck_ticks,
                "min_vibration_ticks": visual_config.animation.min_vibration_ticks,
                "max_vibration_ticks": visual_config.animation.max_vibration_ticks,
                "lift_window_ratio": visual_config.animation.lift_window_ratio,
                "deflection_scale": visual_config.animation.deflection_scale,
                "min_deflection_rad": visual_config.animation.min_deflection_rad,
                "max_deflection_rad": visual_config.animation.max_deflection_rad,
                "ghost_samples": visual_config.animation.ghost_samples,
                "cylinder_playback_rotation_sign": visual_config.animation.cylinder_playback_rotation_sign,
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn action_catalog_contains_supported_actions_and_no_removed_controls() {
        let catalog = action_catalog();
        let names = catalog
            .actions
            .iter()
            .map(|action| action.name)
            .collect::<Vec<_>>();

        for expected in [
            "describe_actions",
            "dump_state",
            "dump_mechanism",
            "set_camera",
            "set_ui",
            "set_light",
            "set_lid",
            "set_winding",
            "full_wind",
            "pause",
            "reset",
            "screenshot",
        ] {
            assert!(names.contains(&expected), "missing action spec {expected}");
        }

        for removed in ["play", "stop", "set_cylinder", "seek_tick"] {
            assert!(
                !names.contains(&removed),
                "removed action still appears in catalog: {removed}"
            );
        }
    }

    #[test]
    fn action_catalog_parameter_schema_matches_serde_action_names() {
        let catalog = action_catalog();
        let set_lid = catalog
            .actions
            .iter()
            .find(|action| action.name == "set_lid")
            .expect("set_lid action spec");
        assert_eq!(set_lid.parameters.len(), 1);
        assert_eq!(set_lid.parameters[0].name, "t");
        assert!(set_lid.parameters[0].required);

        let set_camera = catalog
            .actions
            .iter()
            .find(|action| action.name == "set_camera")
            .expect("set_camera action spec");
        assert!(
            set_camera
                .parameters
                .iter()
                .all(|parameter| !parameter.required)
        );
        assert!(
            set_camera
                .parameters
                .iter()
                .any(|parameter| parameter.name == "target")
        );

        let set_light = catalog
            .actions
            .iter()
            .find(|action| action.name == "set_light")
            .expect("set_light action spec");
        for expected in [
            "yaw",
            "pitch",
            "inner_angle",
            "outer_angle",
            "intensity",
            "key",
            "fill",
            "rim",
            "accent",
            "ambient",
            "environment",
        ] {
            assert!(
                set_light
                    .parameters
                    .iter()
                    .any(|parameter| parameter.name == expected),
                "missing set_light parameter {expected}"
            );
        }
    }
}
