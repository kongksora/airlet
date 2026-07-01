use std::f32::consts::PI;

use airlet::mechanism::ToothHint;
use serde::Serialize;
use serde_json::Value;

use crate::mechanism_view::{
    COMB_DEFLECTION_SCALE, COMB_GHOST_SAMPLES, COMB_LIFT_WINDOW_RATIO, COMB_MAX_DEFLECTION_RAD,
    COMB_MAX_PLUCK_TICKS, COMB_MAX_VIBRATION_TICKS, COMB_MIN_DEFLECTION_RAD, COMB_MIN_PLUCK_TICKS,
    COMB_MIN_VIBRATION_TICKS, MechanismResource,
};

#[derive(Debug, Clone, Serialize)]
pub struct CombAnimationEvent {
    pub midi_note: i32,
    pub onset_tick: i64,
    pub pluck_start_tick: i64,
    pub contact_start_tick: i64,
    pub max_deflection_start_tick: i64,
    pub release_tick: i64,
    pub contact_supported: bool,
    pub contact_window_ticks: i64,
    pub required_pluck_ticks: i64,
    pub max_deflection_rad: f32,
    pub vibration_ticks: i64,
    pub vibration_hz: f32,
    pub damping: f32,
    pub smear_samples: usize,
    pub source_protrusion: f32,
    pub source_tooth_length: f32,
    pub source_velocity: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct CombTineSample {
    pub deflection_rad: f32,
    pub visible: bool,
}

pub fn comb_tine_sample(
    midi_note: i32,
    current_tick: i64,
    smear_sample: Option<f32>,
    mechanism: &MechanismResource,
) -> Option<CombTineSample> {
    let event = nearest_comb_animation_event(midi_note, current_tick, mechanism)?;
    if !event.contact_supported {
        return None;
    }
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
        if current_tick < event.contact_start_tick {
            return 0.0;
        }
        if current_tick >= event.max_deflection_start_tick {
            return event.max_deflection_rad;
        }
        let lift_ticks = (event.max_deflection_start_tick - event.contact_start_tick).max(1);
        let progress = (current_tick - event.contact_start_tick) as f32 / lift_ticks as f32;
        let eased = progress.clamp(0.0, 1.0).powf(1.7);
        event.max_deflection_rad * eased
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
        event.max_deflection_rad * envelope * (2.0 * PI * event.vibration_hz * progress).cos()
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
        .filter(|event| event.contact_supported)
        .filter(|event| {
            current_tick >= event.pluck_start_tick
                && current_tick <= event.release_tick + event.vibration_ticks
        })
        .min_by_key(|event| (current_tick - event.release_tick).abs())
}

pub fn release_alignment_preview(mechanism: &MechanismResource) -> Vec<Value> {
    mechanism
        .comb_animation_events
        .iter()
        .take(64)
        .map(|event| {
            serde_json::json!({
                "midi_note": event.midi_note,
                "onset_tick": event.onset_tick,
                "contact_start_tick": event.contact_start_tick,
                "pluck_start_tick": event.pluck_start_tick,
                "pluck_window_ticks": event.release_tick - event.pluck_start_tick,
                "lift_window_ticks": event.max_deflection_start_tick - event.contact_start_tick,
                "max_deflection_start_tick": event.max_deflection_start_tick,
                "max_deflection_hold_ticks": event.release_tick - event.max_deflection_start_tick,
                "release_tick": event.release_tick,
                "release_equals_audio_onset": true,
                "contact_supported": event.contact_supported,
                "contact_window_ticks": event.contact_window_ticks,
                "required_pluck_ticks": event.required_pluck_ticks,
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

pub fn derive_comb_animation_events(
    hint: &airlet::mechanism::MechanismLayoutHint,
    ticks_per_turn: i64,
) -> Vec<CombAnimationEvent> {
    hint.events
        .iter()
        .map(|tooth| derive_comb_animation_event(tooth, ticks_per_turn))
        .collect()
}

fn derive_comb_animation_event(tooth: &ToothHint, ticks_per_turn: i64) -> CombAnimationEvent {
    use airlet::score::PPQ;

    let circumference = (2.0 * PI * tooth.radius.max(0.01)).max(0.01);
    let footprint_turn_ratio = (tooth.length_along_rotation.max(0.01) / circumference).max(0.0);
    let contact_window_ticks = (footprint_turn_ratio * ticks_per_turn as f32 * 1.75).round() as i64;
    let required_pluck_ticks = COMB_MIN_PLUCK_TICKS;
    let contact_supported = contact_window_ticks >= required_pluck_ticks;
    let contact_start_tick = if contact_supported {
        tooth.onset_tick - contact_window_ticks
    } else {
        tooth.onset_tick
    };
    let lift_ticks = if contact_supported {
        ((contact_window_ticks as f32 * COMB_LIFT_WINDOW_RATIO).round() as i64)
            .clamp(COMB_MIN_PLUCK_TICKS, COMB_MAX_PLUCK_TICKS)
            .min(contact_window_ticks)
    } else {
        0
    };
    let max_deflection_start_tick = contact_start_tick + lift_ticks;
    let velocity = tooth.velocity_hint.clamp(0.0, 1.0);
    let protrusion_ratio = (tooth.protrusion / tooth.radius.max(0.01)).clamp(0.0, 0.25);
    let raw_deflection = 0.055 + velocity * 0.14 + protrusion_ratio * 0.36;
    let max_deflection_rad = if contact_supported {
        (raw_deflection * COMB_DEFLECTION_SCALE)
            .clamp(COMB_MIN_DEFLECTION_RAD, COMB_MAX_DEFLECTION_RAD)
    } else {
        0.0
    };
    let vibration_ticks =
        ((0.55 + velocity * 1.35 + protrusion_ratio * 2.0) * PPQ as f32).round() as i64;
    let vibration_ticks = if contact_supported {
        vibration_ticks.clamp(COMB_MIN_VIBRATION_TICKS, COMB_MAX_VIBRATION_TICKS)
    } else {
        0
    };
    let pitch_factor = ((tooth.midi_note - 60) as f32 * 0.35).clamp(-5.0, 8.0);
    let vibration_hz = 18.0 + pitch_factor;
    let damping = (5.5 - velocity * 1.7 + protrusion_ratio * 2.5).clamp(3.8, 6.8);
    CombAnimationEvent {
        midi_note: tooth.midi_note,
        onset_tick: tooth.onset_tick,
        pluck_start_tick: contact_start_tick,
        contact_start_tick,
        max_deflection_start_tick,
        release_tick: tooth.onset_tick,
        contact_supported,
        contact_window_ticks,
        required_pluck_ticks,
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

#[cfg(test)]
mod tests {
    use super::*;
    use airlet::mechanism::MechanismLayoutHint;
    use airlet::score::PPQ;

    #[test]
    fn comb_tine_release_is_aligned_to_audio_onset() {
        let mechanism = crate::model_view::tests::load_test_mechanism();
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
        assert!(pre_release > 0.0);
        assert!(at_release > 0.0);
        assert!(ghost_at_release.visible);
        assert_eq!(event.pluck_start_tick, event.contact_start_tick);
        assert!(event.contact_start_tick < event.max_deflection_start_tick);
        assert!(event.max_deflection_start_tick < event.release_tick);

        let preview = release_alignment_preview(&mechanism);
        assert_eq!(preview[0]["onset_tick"], preview[0]["release_tick"]);
        assert_eq!(preview[0]["release_equals_audio_onset"], true);
    }

    #[test]
    fn comb_pluck_window_has_contact_lift_hold_and_release_phases() {
        let mechanism = crate::model_view::tests::load_test_mechanism();
        let event = mechanism.comb_animation_events.first().unwrap();

        let pre_contact = comb_tine_sample(
            event.midi_note,
            event.contact_start_tick - 1,
            None,
            &mechanism,
        )
        .map(|sample| sample.deflection_rad)
        .unwrap_or(0.0);
        let contact_start =
            comb_tine_sample(event.midi_note, event.contact_start_tick, None, &mechanism)
                .unwrap()
                .deflection_rad;
        let mid_lift_tick = (event.contact_start_tick + event.max_deflection_start_tick) / 2;
        let mid_lift = comb_tine_sample(event.midi_note, mid_lift_tick, None, &mechanism)
            .unwrap()
            .deflection_rad;
        let max_hold = comb_tine_sample(
            event.midi_note,
            event.max_deflection_start_tick,
            None,
            &mechanism,
        )
        .unwrap()
        .deflection_rad;
        let pre_release =
            comb_tine_sample(event.midi_note, event.release_tick - 1, None, &mechanism)
                .unwrap()
                .deflection_rad;

        assert_eq!(pre_contact, 0.0);
        assert_eq!(contact_start, 0.0);
        assert!(mid_lift > contact_start);
        assert_eq!(max_hold, event.max_deflection_rad);
        assert_eq!(pre_release, event.max_deflection_rad);
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

    #[test]
    fn too_short_tooth_does_not_fake_pluck_or_vibration() {
        let tooth = ToothHint {
            midi_note: 60,
            onset_tick: PPQ,
            angle_rad: 0.0,
            axial_position: 0.0,
            radius: 18.0,
            protrusion: 1.5,
            width: 0.5,
            length_along_rotation: 0.01,
            velocity_hint: 1.0,
        };
        let event = derive_comb_animation_event(&tooth, PPQ * 8);
        let mechanism = MechanismResource {
            hint: MechanismLayoutHint {
                cylinder_radius: 18.0,
                cylinder_length: 80.0,
                track_spacing: 2.0,
                events: vec![tooth],
                diagnostics: Vec::new(),
            },
            comb_animation_events: vec![event.clone()],
            ticks_per_turn: PPQ * 8,
            quarter_millis: 500,
        };

        assert!(!event.contact_supported);
        assert!(event.contact_window_ticks < event.required_pluck_ticks);
        assert_eq!(event.contact_start_tick, event.onset_tick);
        assert_eq!(event.max_deflection_start_tick, event.onset_tick);
        assert_eq!(event.max_deflection_rad, 0.0);
        assert_eq!(event.vibration_ticks, 0);
        assert!(comb_tine_sample(event.midi_note, event.release_tick, None, &mechanism).is_none());
    }
}
