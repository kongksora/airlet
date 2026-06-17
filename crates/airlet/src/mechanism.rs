use serde::{Deserialize, Serialize};

use crate::score::{PPQ, Timeline};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MechanismLayoutHint {
    pub cylinder_radius: f32,
    pub cylinder_length: f32,
    pub track_spacing: f32,
    pub events: Vec<ToothHint>,
    pub diagnostics: Vec<LayoutDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToothHint {
    pub midi_note: i32,
    pub onset_tick: i64,
    pub angle_rad: f32,
    pub axial_position: f32,
    pub radius: f32,
    pub protrusion: f32,
    pub width: f32,
    pub length_along_rotation: f32,
    pub velocity_hint: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutDiagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Info,
    Warning,
}

#[derive(Debug, Clone)]
pub struct MechanismPlanner {
    pub cylinder_radius: f32,
    pub cylinder_length: f32,
    pub track_spacing: f32,
    pub lowest_midi: i32,
    pub highest_midi: i32,
    pub ticks_per_turn: i64,
    pub base_protrusion: f32,
    pub protrusion_by_velocity: f32,
    pub tooth_width: f32,
    pub tooth_length_along_rotation: f32,
    pub min_same_track_ticks: i64,
    pub dense_angle_epsilon_rad: f32,
}

impl Default for MechanismPlanner {
    fn default() -> Self {
        Self {
            cylinder_radius: 18.0,
            cylinder_length: 80.0,
            track_spacing: 2.0,
            lowest_midi: 48,
            highest_midi: 84,
            ticks_per_turn: PPQ * 4,
            base_protrusion: 0.8,
            protrusion_by_velocity: 0.6,
            tooth_width: 0.8,
            tooth_length_along_rotation: 1.2,
            min_same_track_ticks: PPQ / 4,
            dense_angle_epsilon_rad: 0.015,
        }
    }
}

impl MechanismPlanner {
    pub fn plan(&self, timeline: &Timeline) -> MechanismLayoutHint {
        let mut events = Vec::new();
        let mut diagnostics = Vec::new();

        for event in timeline.events.iter().filter(|event| event.midi_note > 0) {
            if event.midi_note < self.lowest_midi || event.midi_note > self.highest_midi {
                diagnostics.push(LayoutDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: format!("midi note {} is outside comb range", event.midi_note),
                });
            }

            let track = event.midi_note - self.lowest_midi;
            let axial_position = track as f32 * self.track_spacing;
            let angle_rad = tick_to_angle(event.onset.0, self.ticks_per_turn);
            events.push(ToothHint {
                midi_note: event.midi_note,
                onset_tick: event.onset.0,
                angle_rad,
                axial_position,
                radius: self.cylinder_radius,
                protrusion: self.base_protrusion + self.protrusion_by_velocity * event.velocity,
                width: self.tooth_width,
                length_along_rotation: self.tooth_length_along_rotation,
                velocity_hint: event.velocity,
            });
        }

        events.sort_by(|a, b| {
            a.midi_note
                .cmp(&b.midi_note)
                .then_with(|| a.onset_tick.cmp(&b.onset_tick))
        });
        for pair in events.windows(2) {
            let a = &pair[0];
            let b = &pair[1];
            if a.midi_note == b.midi_note && b.onset_tick - a.onset_tick < self.min_same_track_ticks
            {
                diagnostics.push(LayoutDiagnostic {
                    severity: DiagnosticSeverity::Warning,
                    message: format!(
                        "midi note {} repeats within {} ticks",
                        a.midi_note,
                        b.onset_tick - a.onset_tick
                    ),
                });
            }
        }

        let mut by_angle = events.clone();
        by_angle.sort_by(|a, b| a.angle_rad.total_cmp(&b.angle_rad));
        for pair in by_angle.windows(2) {
            if (pair[1].angle_rad - pair[0].angle_rad).abs() < self.dense_angle_epsilon_rad {
                diagnostics.push(LayoutDiagnostic {
                    severity: DiagnosticSeverity::Info,
                    message: format!(
                        "dense teeth near angle {:.3} rad: midi {} and {}",
                        pair[0].angle_rad, pair[0].midi_note, pair[1].midi_note
                    ),
                });
            }
        }

        MechanismLayoutHint {
            cylinder_radius: self.cylinder_radius,
            cylinder_length: self.cylinder_length,
            track_spacing: self.track_spacing,
            events,
            diagnostics,
        }
    }
}

fn tick_to_angle(tick: i64, ticks_per_turn: i64) -> f32 {
    let wrapped = tick.rem_euclid(ticks_per_turn) as f32 / ticks_per_turn as f32;
    wrapped * std::f32::consts::TAU
}
