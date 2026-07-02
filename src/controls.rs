use std::f32::consts::PI;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

use crate::lighting::ExhibitLightingConfig;
use crate::playback::{self, AudioOutputState, PlaybackCommand};
use crate::scene::EXHIBIT_TARGET;
use crate::scene::{CAMERA_MAX_RADIUS, CAMERA_MIN_RADIUS};
use crate::twin::MusicBoxTwinState;
use crate::winding::WindingState;

pub(crate) fn env_f32(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
}

#[derive(Resource)]
pub struct ExhibitControls {
    pub yaw: f32,
    pub pitch: f32,
    pub radius: f32,
    pub camera_target: Vec3,
    pub show_ui: bool,
    pub light_yaw: f32,
    pub light_pitch: f32,
    pub light_distance: f32,
    pub spot_inner_angle: f32,
    pub spot_outer_angle: f32,
    pub spot_intensity: f32,
    pub key_illuminance: f32,
    pub fill_intensity: f32,
    pub rim_intensity: f32,
    pub accent_intensity: f32,
    pub ambient_brightness: f32,
    pub environment_intensity: f32,
    pub volume: f32,
    pub playback_rate: f32,
    pub playback: PlaybackCommand,
    pub lid_t: f32,
    pub cylinder_degrees: f32,
}

impl Default for ExhibitControls {
    fn default() -> Self {
        let lighting = ExhibitLightingConfig::default();
        Self {
            yaw: 0.48,
            pitch: 0.36,
            radius: 0.38,
            camera_target: EXHIBIT_TARGET,
            show_ui: true,
            light_yaw: -0.45,
            light_pitch: 1.0,
            light_distance: 0.48,
            spot_inner_angle: lighting.spot.default_inner_angle,
            spot_outer_angle: lighting.spot.default_outer_angle,
            spot_intensity: lighting.spot.intensity,
            key_illuminance: lighting.key.illuminance,
            fill_intensity: lighting.fill.intensity,
            rim_intensity: lighting.rim.intensity,
            accent_intensity: lighting.accent.intensity,
            ambient_brightness: lighting.ambient_brightness,
            environment_intensity: lighting.environment_intensity,
            volume: 0.75,
            playback_rate: 1.0,
            playback: PlaybackCommand::Idle,
            lid_t: env_f32("AIRLET_LID_T", 0.0).clamp(0.0, 1.0),
            cylinder_degrees: 0.0,
        }
    }
}

pub fn control_panel(
    mut contexts: EguiContexts,
    mut controls: ResMut<ExhibitControls>,
    audio_output: Res<AudioOutputState>,
    winding: Res<WindingState>,
    twin: Res<MusicBoxTwinState>,
    lighting: Res<ExhibitLightingConfig>,
) -> Result {
    if !controls.show_ui {
        return Ok(());
    }
    egui::Window::new("Airlet Control")
        .default_width(280.0)
        .show(contexts.ctx_mut()?, |ui| {
            ui.heading("Performance");
            ui.horizontal(|ui| {
                if ui.button("Full Wind").clicked() {
                    controls.playback = PlaybackCommand::FullWind;
                }
                let pause_label = if matches!(twin.mode, crate::twin::TwinMode::Paused) {
                    "Continue"
                } else {
                    "Pause"
                };
                let can_toggle_pause = matches!(
                    twin.mode,
                    crate::twin::TwinMode::Playing | crate::twin::TwinMode::Paused
                );
                if ui
                    .add_enabled(can_toggle_pause, egui::Button::new(pause_label))
                    .clicked()
                {
                    controls.playback = PlaybackCommand::TogglePause;
                }
                if ui.button("Reset").clicked() {
                    controls.playback = PlaybackCommand::Reset;
                }
            });
            ui.add(egui::Slider::new(&mut controls.volume, 0.0..=1.5).text("Volume"));
            ui.add(egui::Slider::new(&mut controls.playback_rate, 0.25..=2.0).text("Rate"));
            ui.label(
                if matches!(twin.mode, crate::twin::TwinMode::Playing)
                    || !audio_output.active_cycles.is_empty()
                {
                    "Status: Playing"
                } else {
                    "Status: Paused"
                },
            );
            draw_winding_meter(ui, &winding, &twin);
            draw_audio_timeline(ui, &mut controls, &audio_output, &twin);
            if let Some(error) = &audio_output.last_error {
                ui.colored_label(egui::Color32::LIGHT_RED, error);
            }

            ui.separator();
            ui.heading("Spotlight");
            ui.add(
                egui::Slider::new(
                    &mut controls.spot_intensity,
                    lighting.spot.min_intensity..=lighting.spot.max_intensity,
                )
                .text("Spot intensity"),
            );
            ui.add(
                egui::Slider::new(
                    &mut controls.spot_outer_angle,
                    lighting.spot.min_outer_angle..=lighting.spot.max_outer_angle,
                )
                .text("Outer angle"),
            );
            controls.spot_outer_angle = lighting.spot.clamp_outer_angle(controls.spot_outer_angle);
            controls.spot_inner_angle = lighting
                .spot
                .clamp_inner_angle(controls.spot_inner_angle, controls.spot_outer_angle);
            let spot_inner_max = controls.spot_outer_angle;
            ui.add(
                egui::Slider::new(
                    &mut controls.spot_inner_angle,
                    lighting.spot.min_inner_angle..=spot_inner_max,
                )
                .text("Inner angle"),
            );
            ui.add(egui::Slider::new(&mut controls.light_yaw, -PI..=PI).text("Light yaw"));
            ui.add(egui::Slider::new(&mut controls.light_pitch, 0.25..=1.45).text("Light pitch"));

            ui.separator();
            ui.heading("Studio Lights");
            ui.add(
                egui::Slider::new(
                    &mut controls.key_illuminance,
                    ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.min
                        ..=ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.max,
                )
                .text("Key"),
            );
            ui.add(
                egui::Slider::new(
                    &mut controls.fill_intensity,
                    ExhibitLightingConfig::FILL_INTENSITY_RANGE.min
                        ..=ExhibitLightingConfig::FILL_INTENSITY_RANGE.max,
                )
                .text("Fill"),
            );
            ui.add(
                egui::Slider::new(
                    &mut controls.rim_intensity,
                    ExhibitLightingConfig::RIM_INTENSITY_RANGE.min
                        ..=ExhibitLightingConfig::RIM_INTENSITY_RANGE.max,
                )
                .text("Rim"),
            );
            ui.add(
                egui::Slider::new(
                    &mut controls.accent_intensity,
                    ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.min
                        ..=ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.max,
                )
                .text("Accent"),
            );

            ui.separator();
            ui.heading("Ambient / IBL");
            ui.add(
                egui::Slider::new(
                    &mut controls.ambient_brightness,
                    ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.min
                        ..=ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.max,
                )
                .text("Ambient"),
            );
            ui.add(
                egui::Slider::new(
                    &mut controls.environment_intensity,
                    ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE.min
                        ..=ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE.max,
                )
                .text("IBL"),
            );

            ui.separator();
            ui.heading("Camera");
            ui.horizontal(|ui| {
                ui.add(egui::DragValue::new(&mut controls.yaw).speed(0.02));
                ui.label("Yaw");
            });
            ui.add(
                egui::Slider::new(
                    &mut controls.pitch,
                    crate::scene::CAMERA_MIN_PITCH..=crate::scene::CAMERA_MAX_PITCH,
                )
                .text("Pitch"),
            );
            ui.add(
                egui::Slider::new(&mut controls.radius, CAMERA_MIN_RADIUS..=CAMERA_MAX_RADIUS)
                    .text("Distance"),
            );

            ui.add(egui::Slider::new(&mut controls.lid_t, 0.0..=1.0).text("Lid t"));
        });
    Ok(())
}

fn draw_winding_meter(ui: &mut egui::Ui, winding: &WindingState, twin: &MusicBoxTwinState) {
    let progress = if twin.max_spring_energy > f32::EPSILON {
        twin.spring_energy / twin.max_spring_energy
    } else {
        0.0
    }
    .clamp(0.0, 1.0);
    ui.add(
        egui::ProgressBar::new(progress)
            .desired_width(ui.available_width())
            .text(format!("Wind {:.0}%", progress * 100.0)),
    );
    ui.label(format!(
        "Twin: {:?}, key {:.0} deg",
        twin.mode, twin.key_degrees
    ));
    let pending_audio_starts = twin.pending_audio_start_count();
    if pending_audio_starts > 0 {
        ui.label(format!("Pending audio starts: {}", pending_audio_starts));
    } else if winding.last_started_cycles > 0 {
        ui.label("Last wind: release armed");
    }
}

fn draw_audio_timeline(
    ui: &mut egui::Ui,
    _controls: &mut ExhibitControls,
    audio_output: &AudioOutputState,
    twin: &MusicBoxTwinState,
) {
    let duration = playback::audio_duration_seconds(audio_output);
    if duration <= f32::EPSILON {
        ui.label("Timeline: unavailable");
        return;
    }

    ui.label(format!(
        "{} / {}",
        playback::format_seconds(twin.mechanical_seconds),
        playback::format_seconds(duration)
    ));
    if twin.mechanical_seconds > f32::EPSILON {
        ui.label(format!(
            "Mechanical time: {}",
            playback::format_seconds(twin.mechanical_seconds)
        ));
    }
    let pending_audio_starts = twin.pending_audio_start_count();
    if audio_output.active_cycles.len() > 1 || pending_audio_starts > 0 {
        ui.label(format!(
            "Layered cycles: {} active, {} queued",
            audio_output.active_cycles.len(),
            pending_audio_starts
        ));
    }
    let timeline_width = ui.available_width().clamp(220.0, 260.0);
    let desired_size = egui::vec2(timeline_width, 74.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, egui::Color32::from_gray(28));
    painter.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, egui::Color32::from_gray(76)),
        egui::StrokeKind::Inside,
    );

    if let Some(audio) = audio_output.audio.as_ref() {
        let samples = audio.samples();
        let channels = audio.channels().get() as usize;
        let columns = rect.width().round().max(1.0) as usize;
        let frames = samples.len() / channels.max(1);
        if frames > 0 {
            let center_y = rect.center().y;
            let half_height = rect.height() * 0.42;
            for column in 0..columns {
                let start_frame = column * frames / columns;
                let end_frame = ((column + 1) * frames / columns)
                    .min(frames)
                    .max(start_frame + 1);
                let mut peak = 0.0_f32;
                for frame in start_frame..end_frame {
                    for channel in 0..channels {
                        let index = frame * channels + channel;
                        if let Some(sample) = samples.get(index) {
                            peak = peak.max(sample.abs());
                        }
                    }
                }
                let x = rect.left() + column as f32 / columns.max(1) as f32 * rect.width();
                let y0 = center_y - peak * half_height;
                let y1 = center_y + peak * half_height;
                painter.line_segment(
                    [egui::pos2(x, y0), egui::pos2(x, y1)],
                    egui::Stroke::new(1.0, egui::Color32::from_rgb(135, 178, 178)),
                );
            }
        }
    }

    let cycle_seconds = duration.max(f32::EPSILON);
    let progress = twin.mechanical_seconds.rem_euclid(cycle_seconds) / cycle_seconds;
    let cursor_x = rect.left() + progress * rect.width();
    painter.line_segment(
        [
            egui::pos2(cursor_x, rect.top()),
            egui::pos2(cursor_x, rect.bottom()),
        ],
        egui::Stroke::new(2.0, egui::Color32::from_rgb(255, 206, 96)),
    );

    response.on_hover_text("Cycle phase preview");
}
