use bevy::prelude::*;

use crate::controls::ExhibitControls;
use crate::mechanism_view::MechanismResource;
use crate::playback::{self, AudioOutputState};
use crate::winding::WindingState;

const WIND_ENERGY_PER_SECOND: f32 = 0.42;
const RELEASE_ENERGY_PER_SECOND: f32 = 0.04;
const KEY_WIND_DEGREES_PER_SECOND: f32 = 360.0;
const KEY_UNWIND_DEGREES_PER_ENERGY: f32 = 900.0;
const RELEASE_THRESHOLD: f32 = 0.02;

pub struct MusicBoxTwinPlugin;

impl Plugin for MusicBoxTwinPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MusicBoxTwinState>();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TwinMode {
    Idle,
    Winding,
    Playing,
    Paused,
    Exhausted,
}

#[derive(Resource, Debug, Clone)]
pub struct MusicBoxTwinState {
    pub mode: TwinMode,
    pub spring_energy: f32,
    pub max_spring_energy: f32,
    pub key_degrees: f32,
    pub cylinder_degrees: f32,
    pub mechanical_seconds: f32,
    pub next_cycle_index: u32,
    pub pending_audio_cycles: u32,
    pub pending_audio_resume_seconds: Option<f32>,
}

impl Default for MusicBoxTwinState {
    fn default() -> Self {
        Self {
            mode: TwinMode::Idle,
            spring_energy: 0.0,
            max_spring_energy: 1.0,
            key_degrees: 0.0,
            cylinder_degrees: 0.0,
            mechanical_seconds: 0.0,
            next_cycle_index: 0,
            pending_audio_cycles: 0,
            pending_audio_resume_seconds: None,
        }
    }
}

impl MusicBoxTwinState {
    pub fn reset(&mut self) {
        self.mode = TwinMode::Idle;
        self.spring_energy = 0.0;
        self.key_degrees = 0.0;
        self.cylinder_degrees = 0.0;
        self.mechanical_seconds = 0.0;
        self.next_cycle_index = 0;
        self.pending_audio_cycles = 0;
        self.pending_audio_resume_seconds = None;
    }

    pub fn is_mechanically_active(&self) -> bool {
        matches!(
            self.mode,
            TwinMode::Winding | TwinMode::Playing | TwinMode::Paused
        ) || self.spring_energy > f32::EPSILON
            || self.mechanical_seconds > f32::EPSILON
    }

    pub fn comb_animation_seconds(&self) -> Option<f32> {
        matches!(self.mode, TwinMode::Playing).then_some(self.mechanical_seconds)
    }

    pub fn should_stop_audio_output(&self) -> bool {
        matches!(
            self.mode,
            TwinMode::Winding | TwinMode::Paused | TwinMode::Exhausted
        )
    }

    pub fn pending_audio_start_count(&self) -> u32 {
        self.pending_audio_cycles + u32::from(self.pending_audio_resume_seconds.is_some())
    }

    pub fn begin_winding(&mut self) {
        self.mode = TwinMode::Winding;
        self.pending_audio_cycles = 0;
        self.pending_audio_resume_seconds = None;
    }

    pub fn wind_full_and_release(&mut self, cycle_seconds: f32) {
        let step = wind_step(
            self.spring_energy,
            self.key_degrees,
            f32::INFINITY,
            self.max_spring_energy,
        );
        self.spring_energy = if step.at_capacity {
            self.max_spring_energy
        } else {
            step.spring_energy.min(self.max_spring_energy)
        };
        self.key_degrees = step.key_degrees;
        self.pending_audio_cycles = 0;
        self.pending_audio_resume_seconds = None;
        if self.spring_energy > RELEASE_THRESHOLD {
            self.mode = TwinMode::Playing;
            self.pending_audio_resume_seconds =
                Some(cycle_phase_seconds(self.mechanical_seconds, cycle_seconds));
            self.next_cycle_index = next_cycle_index_after(self.mechanical_seconds, cycle_seconds);
        }
    }

    pub fn pause(&mut self) {
        if self.mode == TwinMode::Playing {
            self.mode = TwinMode::Paused;
            self.pending_audio_cycles = 0;
            self.pending_audio_resume_seconds = None;
        }
    }

    pub fn resume(&mut self, cycle_seconds: f32) {
        if self.mode != TwinMode::Paused {
            return;
        }
        if self.spring_energy > RELEASE_THRESHOLD {
            self.mode = TwinMode::Playing;
            self.pending_audio_resume_seconds =
                Some(cycle_phase_seconds(self.mechanical_seconds, cycle_seconds));
            self.next_cycle_index = next_cycle_index_after(self.mechanical_seconds, cycle_seconds);
        } else {
            self.mode = TwinMode::Exhausted;
        }
    }

    pub fn toggle_pause(&mut self, cycle_seconds: f32) {
        match self.mode {
            TwinMode::Playing => self.pause(),
            TwinMode::Paused => self.resume(cycle_seconds),
            _ => {}
        }
    }

    pub fn release_winding(&mut self, cycle_seconds: f32) {
        if self.mode != TwinMode::Winding {
            return;
        }
        if self.spring_energy > RELEASE_THRESHOLD {
            self.mode = TwinMode::Playing;
            self.pending_audio_resume_seconds =
                Some(cycle_phase_seconds(self.mechanical_seconds, cycle_seconds));
            self.next_cycle_index = next_cycle_index_after(self.mechanical_seconds, cycle_seconds);
        } else {
            self.mode = TwinMode::Idle;
        }
    }

    pub fn tick(&mut self, dt: f32, cycle_seconds: f32, winding_pressed: bool) {
        let was_winding = self.mode == TwinMode::Winding;
        if winding_pressed {
            self.begin_winding();
            self.tick_winding(dt);
            return;
        }

        if was_winding {
            self.release_winding(cycle_seconds);
        }

        if self.mode == TwinMode::Playing {
            self.tick_playing(dt, cycle_seconds);
        }
    }

    pub fn set_spring_energy_for_debug(&mut self, spring_energy: f32) {
        self.spring_energy = spring_energy.clamp(0.0, self.max_spring_energy.max(0.0));
        if self.spring_energy <= f32::EPSILON && self.mode == TwinMode::Playing {
            self.mode = TwinMode::Exhausted;
        }
    }

    pub fn set_key_degrees_for_debug(&mut self, key_degrees: f32) {
        self.key_degrees = key_degrees;
    }

    pub fn set_pending_audio_cycles_for_debug(&mut self, pending_audio_cycles: u32) {
        if self.mode != TwinMode::Winding {
            self.pending_audio_cycles = pending_audio_cycles;
            self.pending_audio_resume_seconds = None;
        }
    }

    pub fn seek_mechanical_seconds(&mut self, seconds: f32, cycle_seconds: f32) {
        self.mode = TwinMode::Idle;
        self.spring_energy = 0.0;
        self.key_degrees = 0.0;
        self.mechanical_seconds = seconds.max(0.0);
        self.pending_audio_cycles = 0;
        self.pending_audio_resume_seconds = None;
        self.next_cycle_index = if cycle_seconds > f32::EPSILON {
            (self.mechanical_seconds / cycle_seconds).floor() as u32 + 1
        } else {
            0
        };
    }

    fn tick_winding(&mut self, dt: f32) {
        let step = wind_step(
            self.spring_energy,
            self.key_degrees,
            dt,
            self.max_spring_energy,
        );
        self.spring_energy = step.spring_energy;
        self.key_degrees = step.key_degrees;
    }

    fn tick_playing(&mut self, dt: f32, cycle_seconds: f32) {
        let step = release_step(
            self.spring_energy,
            self.key_degrees,
            self.mechanical_seconds,
            dt,
            cycle_seconds,
        );
        self.spring_energy = step.spring_energy;
        self.key_degrees = step.key_degrees;
        self.mechanical_seconds = step.mechanical_seconds;
        let crossings = pending_cycle_crossings(
            self.mechanical_seconds,
            self.next_cycle_index,
            cycle_seconds,
            self.spring_energy,
        );
        self.pending_audio_cycles = self.pending_audio_cycles.saturating_add(crossings);
        self.next_cycle_index = self.next_cycle_index.saturating_add(crossings);
        if step.exhausted {
            self.mode = TwinMode::Exhausted;
            self.spring_energy = 0.0;
        }
    }
}

pub fn update_music_box_twin(
    time: Res<Time>,
    mechanism: Res<MechanismResource>,
    mut winding: ResMut<WindingState>,
    mut twin: ResMut<MusicBoxTwinState>,
) {
    let cycle_seconds = playback::mechanical_cycle_seconds(&mechanism).max(f32::EPSILON);
    let dt = time.delta_secs().max(0.0);
    let pressed = winding.pressed;
    twin.tick(dt, cycle_seconds, pressed);
    twin.cylinder_degrees = playback::synced_cylinder_degrees(twin.mechanical_seconds, &mechanism);
    winding.wind_amount = twin.spring_energy;
    winding.max_wind_amount = twin.max_spring_energy;
    winding.key_degrees = twin.key_degrees;
}

pub fn schedule_twin_audio_cycles(
    mut twin: ResMut<MusicBoxTwinState>,
    mut audio_output: ResMut<AudioOutputState>,
    controls: Res<ExhibitControls>,
) {
    if twin.should_stop_audio_output() {
        twin.pending_audio_cycles = 0;
        twin.pending_audio_resume_seconds = None;
        playback::stop_audio(&mut audio_output);
        return;
    }
    if let Some(offset_seconds) = twin.pending_audio_resume_seconds.take() {
        playback::start_audio_cycle_at(
            &mut audio_output,
            controls.volume,
            controls.playback_rate,
            offset_seconds,
        );
    }
    let cycles = std::mem::take(&mut twin.pending_audio_cycles);
    for _ in 0..cycles {
        playback::start_audio_cycle(&mut audio_output, controls.volume, controls.playback_rate);
    }
}

pub struct WindStep {
    pub spring_energy: f32,
    pub key_degrees: f32,
    pub at_capacity: bool,
}

pub fn wind_step(
    spring_energy: f32,
    key_degrees: f32,
    dt: f32,
    max_spring_energy: f32,
) -> WindStep {
    let max_spring_energy = max_spring_energy.max(0.0);
    let spring_energy = spring_energy.clamp(0.0, max_spring_energy);
    let capacity = (max_spring_energy - spring_energy).max(0.0);
    let requested_dt = dt.max(0.0);
    let effective_dt = if WIND_ENERGY_PER_SECOND > f32::EPSILON {
        requested_dt.min(capacity / WIND_ENERGY_PER_SECOND)
    } else {
        0.0
    };
    let energy =
        (spring_energy + effective_dt * WIND_ENERGY_PER_SECOND).clamp(0.0, max_spring_energy);
    let key = key_degrees - effective_dt * KEY_WIND_DEGREES_PER_SECOND;
    WindStep {
        spring_energy: energy,
        key_degrees: key,
        at_capacity: energy >= max_spring_energy - f32::EPSILON,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ReleaseStep {
    pub spring_energy: f32,
    pub key_degrees: f32,
    pub mechanical_seconds: f32,
    pub exhausted: bool,
}

pub fn release_step(
    spring_energy: f32,
    key_degrees: f32,
    mechanical_seconds: f32,
    dt: f32,
    cycle_seconds: f32,
) -> ReleaseStep {
    let consumed = (dt.max(0.0) * RELEASE_ENERGY_PER_SECOND).min(spring_energy.max(0.0));
    let spring_energy = spring_energy - consumed;
    let cycle_fraction = if cycle_seconds > f32::EPSILON {
        consumed / RELEASE_ENERGY_PER_SECOND / cycle_seconds
    } else {
        0.0
    };
    ReleaseStep {
        spring_energy,
        key_degrees: key_degrees + consumed * KEY_UNWIND_DEGREES_PER_ENERGY,
        mechanical_seconds: mechanical_seconds + cycle_fraction * cycle_seconds,
        exhausted: spring_energy <= f32::EPSILON,
    }
}

pub fn pending_cycle_crossings(
    mechanical_seconds: f32,
    next_cycle_index: u32,
    cycle_seconds: f32,
    spring_energy: f32,
) -> u32 {
    if spring_energy <= 0.0 || cycle_seconds <= f32::EPSILON {
        return 0;
    }
    let mut count = 0;
    let mut index = next_cycle_index;
    while mechanical_seconds >= index as f32 * cycle_seconds {
        count += 1;
        index += 1;
    }
    count
}

pub fn cycle_phase_seconds(mechanical_seconds: f32, cycle_seconds: f32) -> f32 {
    if cycle_seconds <= f32::EPSILON {
        return 0.0;
    }
    mechanical_seconds.max(0.0).rem_euclid(cycle_seconds)
}

pub fn next_cycle_index_after(mechanical_seconds: f32, cycle_seconds: f32) -> u32 {
    if cycle_seconds <= f32::EPSILON {
        return 0;
    }
    (mechanical_seconds.max(0.0) / cycle_seconds).floor() as u32 + 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn winding_turns_key_backward_and_stores_energy() {
        let step = wind_step(0.0, 0.0, 1.0, 1.0);

        assert_eq!(step.spring_energy, 0.42);
        assert_eq!(step.key_degrees, -360.0);
        assert!(!step.at_capacity);
    }

    #[test]
    fn full_spring_blocks_more_reverse_key_motion() {
        let step = wind_step(1.0, -720.0, 1.0, 1.0);

        assert_eq!(step.spring_energy, 1.0);
        assert_eq!(step.key_degrees, -720.0);
        assert!(step.at_capacity);
    }

    #[test]
    fn releasing_turns_key_forward_and_consumes_energy() {
        let step = release_step(0.5, -360.0, 0.0, 1.0, 2.0);

        assert_eq!(step.spring_energy, 0.46);
        assert!(step.key_degrees > -360.0);
        assert_eq!(step.mechanical_seconds, 1.0);
        assert!(!step.exhausted);
    }

    #[test]
    fn releasing_exhausts_when_energy_runs_out() {
        let step = release_step(0.03, -60.0, 1.0, 1.0, 2.0);

        assert_eq!(step.spring_energy, 0.0);
        assert!(step.exhausted);
    }

    #[test]
    fn cycle_crossings_are_driven_by_mechanical_phase() {
        assert_eq!(pending_cycle_crossings(0.1, 0, 15.5, 1.0), 1);
        assert_eq!(pending_cycle_crossings(15.4, 1, 15.5, 1.0), 0);
        assert_eq!(pending_cycle_crossings(15.5, 1, 15.5, 1.0), 1);
        assert_eq!(pending_cycle_crossings(31.0, 1, 15.5, 1.0), 2);
        assert_eq!(pending_cycle_crossings(31.0, 1, 15.5, 0.0), 0);
    }

    #[test]
    fn winding_state_does_not_emit_audio_or_advance_cylinder() {
        let mut twin = MusicBoxTwinState {
            pending_audio_cycles: 3,
            mechanical_seconds: 5.0,
            cylinder_degrees: -90.0,
            ..default()
        };

        twin.tick(1.0, 2.0, true);

        assert_eq!(twin.mode, TwinMode::Winding);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.mechanical_seconds, 5.0);
        assert_eq!(twin.cylinder_degrees, -90.0);
    }

    #[test]
    fn release_from_winding_enters_playing_from_current_key_angle() {
        let mut twin = MusicBoxTwinState::default();
        twin.tick(1.0, 2.0, true);
        let wound_key = twin.key_degrees;

        twin.tick(0.5, 2.0, false);

        assert_eq!(twin.mode, TwinMode::Playing);
        assert!(twin.key_degrees > wound_key);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.pending_audio_resume_seconds, Some(0.0));
        assert_eq!(twin.pending_audio_start_count(), 1);
    }

    #[test]
    fn release_after_exhaustion_resumes_audio_from_current_cycle_phase() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Exhausted,
            mechanical_seconds: 5.25,
            next_cycle_index: 3,
            ..default()
        };

        twin.tick(1.0, 2.0, true);
        twin.tick(0.0, 2.0, false);

        assert_eq!(twin.mode, TwinMode::Playing);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.pending_audio_resume_seconds, Some(1.25));
        assert_eq!(twin.next_cycle_index, 3);
    }

    #[test]
    fn full_wind_button_winds_to_capacity_and_starts_from_current_phase() {
        let mut twin = MusicBoxTwinState {
            mechanical_seconds: 5.25,
            key_degrees: -120.0,
            ..default()
        };

        twin.wind_full_and_release(2.0);

        assert_eq!(twin.mode, TwinMode::Playing);
        assert!((twin.spring_energy - twin.max_spring_energy).abs() < 0.0001);
        assert!(twin.key_degrees < -120.0);
        assert_eq!(twin.pending_audio_resume_seconds, Some(1.25));
        assert_eq!(twin.next_cycle_index, 3);
    }

    #[test]
    fn pause_and_continue_keep_mechanical_phase_and_resume_audio_from_phase() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Playing,
            spring_energy: 0.5,
            mechanical_seconds: 5.25,
            pending_audio_cycles: 2,
            ..default()
        };

        twin.toggle_pause(2.0);
        twin.tick(1.0, 2.0, false);

        assert_eq!(twin.mode, TwinMode::Paused);
        assert_eq!(twin.mechanical_seconds, 5.25);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.pending_audio_resume_seconds, None);

        twin.toggle_pause(2.0);

        assert_eq!(twin.mode, TwinMode::Playing);
        assert_eq!(twin.mechanical_seconds, 5.25);
        assert_eq!(twin.pending_audio_resume_seconds, Some(1.25));
        assert_eq!(twin.next_cycle_index, 3);
    }

    #[test]
    fn exhausted_twin_stops_motion_and_scheduling() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Playing,
            spring_energy: 0.01,
            key_degrees: -45.0,
            next_cycle_index: 1,
            ..default()
        };

        twin.tick(1.0, 2.0, false);
        let key = twin.key_degrees;
        let mechanical_seconds = twin.mechanical_seconds;
        twin.tick(1.0, 2.0, false);

        assert_eq!(twin.mode, TwinMode::Exhausted);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.key_degrees, key);
        assert_eq!(twin.mechanical_seconds, mechanical_seconds);
    }

    #[test]
    fn seek_sets_mechanical_time_without_audio_events() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Playing,
            spring_energy: 0.5,
            pending_audio_cycles: 2,
            ..default()
        };

        twin.seek_mechanical_seconds(5.2, 2.0);

        assert_eq!(twin.mode, TwinMode::Idle);
        assert_eq!(twin.spring_energy, 0.0);
        assert_eq!(twin.key_degrees, 0.0);
        assert_eq!(twin.mechanical_seconds, 5.2);
        assert_eq!(twin.next_cycle_index, 3);
        assert_eq!(twin.pending_audio_cycles, 0);
        assert_eq!(twin.pending_audio_resume_seconds, None);
    }

    #[test]
    fn cycle_phase_helpers_keep_absolute_time_and_loop_phase_separate() {
        assert_eq!(cycle_phase_seconds(5.25, 2.0), 1.25);
        assert_eq!(next_cycle_index_after(5.25, 2.0), 3);
    }

    #[test]
    fn comb_animation_clock_only_runs_while_playing() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Winding,
            spring_energy: 0.5,
            mechanical_seconds: 1.5,
            ..default()
        };
        assert_eq!(twin.comb_animation_seconds(), None);

        twin.mode = TwinMode::Idle;
        assert_eq!(twin.comb_animation_seconds(), None);

        twin.mode = TwinMode::Exhausted;
        assert_eq!(twin.comb_animation_seconds(), None);

        twin.mode = TwinMode::Paused;
        assert_eq!(twin.comb_animation_seconds(), None);

        twin.mode = TwinMode::Playing;
        assert_eq!(twin.comb_animation_seconds(), Some(1.5));
    }

    #[test]
    fn audio_output_is_forced_off_while_winding_or_exhausted() {
        let mut twin = MusicBoxTwinState {
            mode: TwinMode::Winding,
            ..default()
        };
        assert!(twin.should_stop_audio_output());

        twin.mode = TwinMode::Exhausted;
        assert!(twin.should_stop_audio_output());

        twin.mode = TwinMode::Paused;
        assert!(twin.should_stop_audio_output());

        twin.mode = TwinMode::Playing;
        assert!(!twin.should_stop_audio_output());

        twin.mode = TwinMode::Idle;
        assert!(!twin.should_stop_audio_output());
    }
}
