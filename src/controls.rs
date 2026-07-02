use std::f32::consts::PI;

use bevy::{ecs::system::SystemParam, picking::hover::Hovered, prelude::*};
use bevy_egui::{EguiContexts, egui};

use crate::lid::LidState;
use crate::lighting::ExhibitLightingConfig;
use crate::mechanical_audio::{
    MechanicalAudioConfig, MechanicalAudioState, MechanicalAuditionQueue, MechanicalEvent,
    MechanicalEventQueue, MechanicalEventStats, WindingDirection,
};
use crate::mechanism_view::MechanismResource;
use crate::outline::{OutlineKind, OutlineShell, OutlineTarget};
use crate::playback::{self, AudioOutputState, PlaybackCommand};
use crate::scene::EXHIBIT_TARGET;
use crate::scene::{CAMERA_MAX_RADIUS, CAMERA_MIN_RADIUS};
use crate::screenshot::ScreenshotState;
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
            cylinder_degrees: 0.0,
        }
    }
}

#[derive(SystemParam)]
pub struct ControlPanelParams<'w, 's> {
    controls: ResMut<'w, ExhibitControls>,
    lid: ResMut<'w, LidState>,
    audio_output: ResMut<'w, AudioOutputState>,
    winding: ResMut<'w, WindingState>,
    twin: ResMut<'w, MusicBoxTwinState>,
    lighting: Res<'w, ExhibitLightingConfig>,
    mechanism: Res<'w, MechanismResource>,
    event_queue: Res<'w, MechanicalEventQueue>,
    event_stats: ResMut<'w, MechanicalEventStats>,
    audio_config: ResMut<'w, MechanicalAudioConfig>,
    audition: ResMut<'w, MechanicalAuditionQueue>,
    mechanical_audio: ResMut<'w, MechanicalAudioState>,
    screenshot: ResMut<'w, ScreenshotState>,
    outline_targets: Query<'w, 's, (&'static OutlineTarget, &'static Hovered)>,
    outline_shells: Query<'w, 's, (&'static OutlineShell, &'static Visibility)>,
    meshes: Query<'w, 's, &'static Mesh3d>,
}

pub fn control_panel(mut contexts: EguiContexts, mut panel: ControlPanelParams) -> Result {
    if !panel.controls.show_ui {
        return Ok(());
    }
    egui::Window::new("Airlet Control")
        .default_width(360.0)
        .default_height(760.0)
        .show(contexts.ctx_mut()?, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    draw_transport_panel(
                        ui,
                        &mut panel.controls,
                        &mut panel.twin,
                        &panel.mechanism,
                        &mut panel.audio_output,
                        &mut panel.mechanical_audio,
                    );
                    draw_lid_panel(ui, &mut panel.lid);
                    draw_winding_panel(ui, &mut panel.winding, &mut panel.twin);
                    draw_mechanical_events_panel(ui, &panel.event_queue, &mut panel.event_stats);
                    draw_mechanical_audio_panel(
                        ui,
                        &mut panel.audio_config,
                        &mut panel.audition,
                        &panel.mechanical_audio,
                    );
                    draw_audio_core_panel(
                        ui,
                        &mut panel.controls,
                        &mut panel.audio_output,
                        &panel.mechanical_audio,
                    );
                    draw_spotlight_panel(ui, &mut panel.controls, &panel.lighting);
                    draw_studio_lights_panel(ui, &mut panel.controls);
                    draw_ambient_panel(ui, &mut panel.controls);
                    draw_camera_panel(ui, &mut panel.controls);
                    draw_model_picking_panel(
                        ui,
                        &panel.outline_targets,
                        &panel.outline_shells,
                        panel.meshes.iter().count(),
                    );
                    draw_developer_panel(ui, &mut panel.controls, &mut panel.screenshot);
                });
        });
    Ok(())
}

fn draw_transport_panel(
    ui: &mut egui::Ui,
    controls: &mut ExhibitControls,
    twin: &mut MusicBoxTwinState,
    mechanism: &MechanismResource,
    audio_output: &mut AudioOutputState,
    mechanical_audio: &mut MechanicalAudioState,
) {
    section_heading(ui, "Transport / Twin");
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
    ui.horizontal(|ui| {
        if ui.button("Stop music").clicked() {
            playback::stop_audio(audio_output);
        }
        if ui.button("Stop mech").clicked() {
            mechanical_audio.stop_all();
        }
        if ui.button("Stop all").clicked() {
            playback::stop_audio(audio_output);
            mechanical_audio.stop_all();
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
    ui.label(format!(
        "Twin: {:?}, energy {:.3}/{:.3}",
        twin.mode, twin.spring_energy, twin.max_spring_energy
    ));
    ui.label(format!(
        "Key {:.1} deg, cylinder {:.1} deg, cycle {}",
        twin.key_degrees, twin.cylinder_degrees, twin.next_cycle_index
    ));
    let mut energy = twin.spring_energy;
    if ui
        .add(
            egui::Slider::new(&mut energy, 0.0..=twin.max_spring_energy.max(0.001))
                .text("Spring energy"),
        )
        .changed()
    {
        twin.set_spring_energy_for_debug(energy);
    }
    let mut key_degrees = twin.key_degrees;
    if ui
        .add(
            egui::DragValue::new(&mut key_degrees)
                .speed(5.0)
                .prefix("Key deg "),
        )
        .changed()
    {
        twin.set_key_degrees_for_debug(key_degrees);
    }
    let cycle_seconds = playback::mechanical_cycle_seconds(mechanism).max(f32::EPSILON);
    let mut mechanical_seconds = twin.mechanical_seconds;
    if ui
        .add(
            egui::Slider::new(&mut mechanical_seconds, 0.0..=cycle_seconds * 4.0)
                .text("Mechanical time"),
        )
        .changed()
    {
        twin.seek_mechanical_seconds(mechanical_seconds, cycle_seconds);
    }
    draw_audio_timeline(ui, audio_output, twin);
    if let Some(error) = &audio_output.last_error {
        ui.colored_label(egui::Color32::LIGHT_RED, error);
    }
}

fn draw_lid_panel(ui: &mut egui::Ui, lid: &mut LidState) {
    section_heading(ui, "Lid");
    ui.label(format!(
        "Mode: {:?}, target {:.0}%, velocity {:.3}",
        lid.mode,
        lid.target_t * 100.0,
        lid.velocity
    ));
    ui.horizontal(|ui| {
        if ui.button("Open").clicked() {
            lid.open();
        }
        if ui.button("Close").clicked() {
            lid.close();
        }
        if ui.button("Toggle").clicked() {
            lid.toggle();
        }
    });
    let mut lid_t = lid.t;
    if ui
        .add(egui::Slider::new(&mut lid_t, 0.0..=1.0).text("Lid"))
        .changed()
    {
        lid.set_manual(lid_t);
    }
    ui.add(egui::Slider::new(&mut lid.max_speed, 0.05..=4.0).text("Max speed"));
}

fn draw_winding_panel(ui: &mut egui::Ui, winding: &mut WindingState, twin: &mut MusicBoxTwinState) {
    section_heading(ui, "Winding");
    draw_winding_meter(ui, winding, twin);
    ui.horizontal(|ui| {
        ui.checkbox(&mut winding.hovered, "Hovered");
        ui.checkbox(&mut winding.pressed, "Pressed");
    });
    ui.label(format!(
        "Released {:.3}, armed cycles {}",
        winding.last_released_wind_amount, winding.last_started_cycles
    ));
    let mut max_energy = twin.max_spring_energy;
    if ui
        .add(egui::Slider::new(&mut max_energy, 0.1..=3.0).text("Max spring"))
        .changed()
    {
        twin.max_spring_energy = max_energy.max(0.1);
        twin.spring_energy = twin.spring_energy.min(twin.max_spring_energy);
    }
}

fn draw_mechanical_events_panel(
    ui: &mut egui::Ui,
    queue: &MechanicalEventQueue,
    stats: &mut MechanicalEventStats,
) {
    section_heading(ui, "Mechanical Events");
    ui.horizontal(|ui| {
        ui.label(format!("Frame {}", stats.frame_events));
        ui.label(format!("Total {}", stats.total_events));
        if ui.button("Clear").clicked() {
            stats.clear();
        }
    });
    ui.label(format!(
        "Lid motion {}, close {}, open {}",
        stats.lid_motion, stats.lid_closed_impact, stats.lid_opened_stop
    ));
    ui.label(format!(
        "Winding {}, cylinder {}, tooth {}",
        stats.winding_key_motion, stats.cylinder_motion, stats.tooth_comb_contact
    ));
    if !queue.events.is_empty() {
        ui.label("Current frame:");
        for event in queue.events.iter().take(5) {
            ui.monospace(event.summary());
        }
    }
    if !stats.recent.is_empty() {
        ui.label("Recent:");
        for event in stats.recent.iter().rev().take(6) {
            ui.monospace(event);
        }
    }
}

fn draw_mechanical_audio_panel(
    ui: &mut egui::Ui,
    config: &mut MechanicalAudioConfig,
    audition: &mut MechanicalAuditionQueue,
    state: &MechanicalAudioState,
) {
    section_heading(ui, "Mechanical Audio");
    ui.horizontal(|ui| {
        ui.checkbox(&mut config.enabled, "Enabled");
        ui.label(format!(
            "Lanes L{} W{} C{} / transients {}",
            active_label(state.lid_lane_active()),
            active_label(state.winding_lane_active()),
            active_label(state.cylinder_lane_active()),
            state.transient_count()
        ));
    });
    ui.add(egui::Slider::new(&mut config.master_gain, 0.0..=3.0).text("Master"));
    ui.add(egui::Slider::new(&mut config.lid_motion_gain, 0.0..=3.0).text("Lid motion"));
    ui.add(
        egui::Slider::new(&mut config.lid_close_impact_gain, 0.0..=3.0).text("Lid close impact"),
    );
    ui.add(egui::Slider::new(&mut config.lid_open_stop_gain, 0.0..=3.0).text("Lid open stop"));
    ui.add(egui::Slider::new(&mut config.winding_gain, 0.0..=3.0).text("Winding"));
    ui.add(egui::Slider::new(&mut config.cylinder_gain, 0.0..=3.0).text("Cylinder"));
    ui.add(egui::Slider::new(&mut config.tooth_scrape_gain, 0.0..=3.0).text("Tooth scrape"));
    ui.add(egui::Slider::new(&mut config.frequency_scale, 0.25..=3.0).text("Frequency"));
    ui.add(egui::Slider::new(&mut config.roughness_scale, 0.0..=2.0).text("Roughness"));
    ui.add(egui::Slider::new(&mut config.decay_scale, 0.25..=3.0).text("Decay"));
    ui.horizontal(|ui| {
        ui.checkbox(&mut config.mute_lid, "Mute lid");
        ui.checkbox(&mut config.mute_winding, "Mute wind");
        ui.checkbox(&mut config.mute_cylinder, "Mute cyl");
        ui.checkbox(&mut config.mute_tooth, "Mute tooth");
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut config.solo_lid, "Solo lid");
        ui.checkbox(&mut config.solo_winding, "Solo wind");
        ui.checkbox(&mut config.solo_cylinder, "Solo cyl");
        ui.checkbox(&mut config.solo_tooth, "Solo tooth");
    });
    ui.horizontal(|ui| {
        if ui.button("Close hit").clicked() {
            audition.push(MechanicalEvent::LidClosedImpact { speed: 1.0 });
        }
        if ui.button("Open stop").clicked() {
            audition.push(MechanicalEvent::LidOpenedStop { speed: 1.0 });
        }
        if ui.button("Wind").clicked() {
            audition.push(MechanicalEvent::WindingKeyMotion {
                angular_velocity: -360.0,
                direction: WindingDirection::Winding,
                spring_energy: 0.5,
            });
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Release").clicked() {
            audition.push(MechanicalEvent::WindingKeyMotion {
                angular_velocity: 240.0,
                direction: WindingDirection::Releasing,
                spring_energy: 0.5,
            });
        }
        if ui.button("Cylinder").clicked() {
            audition.push(MechanicalEvent::CylinderMotion {
                angular_velocity: 180.0,
                phase: 0.25,
            });
        }
        if ui.button("Tooth").clicked() {
            audition.push(MechanicalEvent::ToothCombContact {
                midi_note: 72,
                intensity: 1.0,
                phase: 0.25,
            });
        }
    });
}

fn draw_audio_core_panel(
    ui: &mut egui::Ui,
    controls: &mut ExhibitControls,
    audio_output: &mut AudioOutputState,
    mechanical_audio: &MechanicalAudioState,
) {
    section_heading(ui, "Audio Core");
    ui.label(format!(
        "Music players {}, mechanical events {}",
        audio_output.active_cycles.len(),
        mechanical_audio.last_event_count
    ));
    ui.label(format!(
        "Duration {}",
        playback::format_seconds(playback::audio_duration_seconds(audio_output))
    ));
    ui.add(egui::Slider::new(&mut controls.volume, 0.0..=1.5).text("Music volume"));
    ui.add(egui::Slider::new(&mut controls.playback_rate, 0.25..=2.0).text("Music rate"));
}

fn draw_spotlight_panel(
    ui: &mut egui::Ui,
    controls: &mut ExhibitControls,
    lighting: &ExhibitLightingConfig,
) {
    section_heading(ui, "Spotlight");
    ui.add(
        egui::Slider::new(
            &mut controls.spot_intensity,
            lighting.spot.min_intensity..=lighting.spot.max_intensity,
        )
        .text("Intensity"),
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
    ui.add(egui::Slider::new(&mut controls.light_yaw, -PI..=PI).text("Yaw"));
    ui.add(egui::Slider::new(&mut controls.light_pitch, 0.25..=1.45).text("Pitch"));
}

fn draw_studio_lights_panel(ui: &mut egui::Ui, controls: &mut ExhibitControls) {
    section_heading(ui, "Studio Lights");
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
}

fn draw_ambient_panel(ui: &mut egui::Ui, controls: &mut ExhibitControls) {
    section_heading(ui, "Ambient / IBL");
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
}

fn draw_camera_panel(ui: &mut egui::Ui, controls: &mut ExhibitControls) {
    section_heading(ui, "Camera");
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
}

fn draw_model_picking_panel(
    ui: &mut egui::Ui,
    outline_targets: &Query<(&OutlineTarget, &Hovered)>,
    outline_shells: &Query<(&OutlineShell, &Visibility)>,
    mesh_count: usize,
) {
    section_heading(ui, "Model / Picking / Outline");
    let mut lid_targets = 0usize;
    let mut key_targets = 0usize;
    let mut hovered_lid = 0usize;
    let mut hovered_key = 0usize;
    for (target, hovered) in outline_targets.iter() {
        match target.kind {
            OutlineKind::Lid => {
                lid_targets += 1;
                hovered_lid += usize::from(hovered.get());
            }
            OutlineKind::WindingKey => {
                key_targets += 1;
                hovered_key += usize::from(hovered.get());
            }
        }
    }
    let mut visible_lid_shells = 0usize;
    let mut visible_key_shells = 0usize;
    for (shell, visibility) in outline_shells.iter() {
        if *visibility == Visibility::Hidden {
            continue;
        }
        match shell.kind {
            OutlineKind::Lid => visible_lid_shells += 1,
            OutlineKind::WindingKey => visible_key_shells += 1,
        }
    }
    ui.label(format!("Mesh components: {mesh_count}"));
    ui.label(format!(
        "Pick targets: lid {lid_targets} ({hovered_lid} hovered), key {key_targets} ({hovered_key} hovered)"
    ));
    ui.label(format!(
        "Visible outlines: lid {visible_lid_shells}, key {visible_key_shells}"
    ));
}

fn draw_developer_panel(
    ui: &mut egui::Ui,
    controls: &mut ExhibitControls,
    screenshot: &mut ScreenshotState,
) {
    section_heading(ui, "Developer Utilities");
    ui.checkbox(&mut controls.show_ui, "Show UI");
    if ui.button("Screenshot").clicked() {
        screenshot.path = Some("target/debug-panel-capture.png".to_string());
        screenshot.requested = false;
        screenshot.frames_before_capture = 2;
        screenshot.exit_after_capture = false;
    }
    if let Some(path) = &screenshot.path {
        ui.monospace(format!("Screenshot: {path}"));
    }
}

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.separator();
    ui.heading(text);
}

fn active_label(active: bool) -> &'static str {
    if active { "+" } else { "-" }
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
