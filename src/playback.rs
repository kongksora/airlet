use airlet::{audio::RenderedAudio, defaults, score::PPQ};
use bevy::prelude::*;
use rodio::{DeviceSinkBuilder, MixerDeviceSink, Player, buffer::SamplesBuffer};

use crate::controls::ExhibitControls;
use crate::mechanism_view::MechanismResource;
use crate::twin::MusicBoxTwinState;
use crate::winding::WindingState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackCommand {
    Idle,
    FullWind,
    TogglePause,
    Reset,
}

#[derive(Resource, Default)]
pub struct AudioOutputState {
    pub device: Option<MixerDeviceSink>,
    pub active_cycles: Vec<ActiveCyclePlayer>,
    pub audio: Option<RenderedAudio>,
    pub last_error: Option<String>,
}

pub struct ActiveCyclePlayer {
    pub player: Player,
}

pub fn setup_audio(mut audio_output: ResMut<AudioOutputState>) {
    match DeviceSinkBuilder::open_default_sink() {
        Ok(mut device) => {
            device.log_on_drop(false);
            let sample_rate = device.config().sample_rate();
            audio_output.audio = Some(defaults::air_intro_audio(sample_rate));
            audio_output.device = Some(device);
        }
        Err(err) => {
            audio_output.last_error = Some(format!("audio device error: {err}"));
        }
    }
}

pub fn apply_playback_controls(
    mut controls: ResMut<ExhibitControls>,
    mechanism: Res<MechanismResource>,
    mut audio_output: ResMut<AudioOutputState>,
    mut winding: ResMut<WindingState>,
    mut twin: ResMut<MusicBoxTwinState>,
) {
    let volume = controls.volume;
    let rate = controls.playback_rate.clamp(0.25, 2.0);
    controls.playback_rate = rate;
    for active in &audio_output.active_cycles {
        active.player.set_volume(volume);
        active.player.set_speed(rate);
    }
    audio_output
        .active_cycles
        .retain(|active| !active.player.empty());

    match controls.playback {
        PlaybackCommand::Idle => {}
        PlaybackCommand::FullWind => {
            winding.clear_active_wind();
            stop_audio(&mut audio_output);
            twin.wind_full_and_release(mechanical_cycle_seconds(&mechanism));
            controls.playback = PlaybackCommand::Idle;
        }
        PlaybackCommand::TogglePause => {
            winding.clear_active_wind();
            stop_audio(&mut audio_output);
            twin.toggle_pause(mechanical_cycle_seconds(&mechanism));
            controls.playback = PlaybackCommand::Idle;
        }
        PlaybackCommand::Reset => {
            winding.clear_active_wind();
            twin.reset();
            stop_audio(&mut audio_output);
            controls.cylinder_degrees =
                synced_cylinder_degrees(twin.mechanical_seconds, &mechanism);
            controls.playback = PlaybackCommand::Idle;
        }
    }
}

pub fn start_audio_cycle(audio_output: &mut AudioOutputState, volume: f32, rate: f32) {
    append_cycle(audio_output, volume, rate, 0);
    audio_output.last_error = None;
}

pub fn start_audio_cycle_at(
    audio_output: &mut AudioOutputState,
    volume: f32,
    rate: f32,
    offset_seconds: f32,
) {
    let start_sample = audio_output
        .audio
        .as_ref()
        .map(|audio| audio_start_sample(audio, offset_seconds))
        .unwrap_or(0);
    append_cycle(audio_output, volume, rate, start_sample);
    audio_output.last_error = None;
}

fn append_cycle(audio_output: &mut AudioOutputState, volume: f32, rate: f32, start_sample: usize) {
    let Some(device) = audio_output.device.as_ref() else {
        audio_output.last_error = Some("audio device is unavailable".to_string());
        return;
    };
    let Some(audio) = audio_output.audio.as_ref() else {
        audio_output.last_error = Some("default performance audio is unavailable".to_string());
        return;
    };
    let player = Player::connect_new(device.mixer());
    player.set_volume(volume);
    player.set_speed(rate);
    player.append(SamplesBuffer::new(
        audio.channels(),
        audio.sample_rate(),
        audio.samples()[start_sample.min(audio.samples().len())..].to_vec(),
    ));
    audio_output
        .active_cycles
        .push(ActiveCyclePlayer { player });
}

pub fn audio_duration_seconds(audio_output: &AudioOutputState) -> f32 {
    audio_output
        .audio
        .as_ref()
        .map(|audio| audio.duration().as_secs_f32())
        .unwrap_or(0.0)
}

pub fn mechanical_cycle_seconds(mechanism: &MechanismResource) -> f32 {
    tick_to_seconds(mechanism.ticks_per_turn, mechanism)
}

pub fn stop_audio(audio_output: &mut AudioOutputState) {
    for active in audio_output.active_cycles.drain(..) {
        active.player.stop();
    }
}

pub fn audio_start_sample(audio: &RenderedAudio, seconds: f32) -> usize {
    let channels = audio.channels().get() as usize;
    let sample_rate = audio.sample_rate().get() as f32;
    let frame = (seconds.max(0.0) * sample_rate).round() as usize;
    (frame * channels).min(audio.samples().len())
}

pub fn format_seconds(seconds: f32) -> String {
    let seconds = seconds.max(0.0);
    let whole = seconds.floor() as u32;
    let minutes = whole / 60;
    let seconds_part = whole % 60;
    let tenths = ((seconds - whole as f32) * 10.0).floor() as u32;
    format!("{minutes}:{seconds_part:02}.{tenths}")
}

pub fn synced_cylinder_degrees(elapsed_seconds: f32, mechanism: &MechanismResource) -> f32 {
    let tick = seconds_to_tick(elapsed_seconds, mechanism);
    tick_to_cylinder_degrees(tick, mechanism)
}

pub fn tick_to_cylinder_degrees(tick: i64, mechanism: &MechanismResource) -> f32 {
    let wrapped =
        tick.rem_euclid(mechanism.ticks_per_turn) as f32 / mechanism.ticks_per_turn as f32;
    crate::mechanism_view::CYLINDER_PLAYBACK_ROTATION_SIGN * wrapped * 360.0
}

pub fn tick_to_seconds(tick: i64, mechanism: &MechanismResource) -> f32 {
    tick as f32 * mechanism.quarter_millis as f32 / PPQ as f32 / 1000.0
}

pub fn seconds_to_tick(seconds: f32, mechanism: &MechanismResource) -> i64 {
    (seconds.max(0.0) * 1000.0 * PPQ as f32 / mechanism.quarter_millis as f32).round() as i64
}

pub fn seconds_to_cycle_tick(seconds: f32, mechanism: &MechanismResource) -> i64 {
    seconds_to_tick(seconds, mechanism).rem_euclid(mechanism.ticks_per_turn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mechanism_view::tests::dummy_mechanism_resource;

    #[test]
    fn playback_clock_drives_cylinder_phase_from_ticks() {
        let mechanism = dummy_mechanism_resource();

        assert_eq!(synced_cylinder_degrees(0.0, &mechanism), -0.0);
        assert_eq!(synced_cylinder_degrees(1.0, &mechanism), -180.0);
        assert_eq!(synced_cylinder_degrees(2.0, &mechanism), -0.0);
    }

    #[test]
    fn audio_start_sample_maps_to_channel_aligned_sample_index() {
        let audio = RenderedAudio::mono(std::num::NonZero::new(10).unwrap(), vec![0.0; 100]);

        assert_eq!(audio_start_sample(&audio, 2.4), 24);
        assert_eq!(format_seconds(65.4), "1:05.4");
    }

    #[test]
    fn seconds_to_cycle_tick_wraps_repeated_turns() {
        let mechanism = dummy_mechanism_resource();

        assert_eq!(
            seconds_to_cycle_tick(2.25, &mechanism),
            seconds_to_cycle_tick(0.25, &mechanism)
        );
    }
}
