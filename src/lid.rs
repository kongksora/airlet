use bevy::prelude::*;

const DEFAULT_LID_SPEED: f32 = 1.35;
const ENDPOINT_EPSILON: f32 = 0.0001;

pub struct LidPlugin;

impl Plugin for LidPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LidState>();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidMode {
    Closed,
    Opening,
    Open,
    Closing,
    Manual,
}

#[derive(Resource, Debug, Clone)]
pub struct LidState {
    pub mode: LidMode,
    pub t: f32,
    pub previous_t: f32,
    pub target_t: f32,
    pub velocity: f32,
    pub previous_velocity: f32,
    pub max_speed: f32,
}

impl Default for LidState {
    fn default() -> Self {
        let t = crate::controls::env_f32("AIRLET_LID_T", 0.0).clamp(0.0, 1.0);
        Self {
            mode: endpoint_mode(t).unwrap_or(LidMode::Manual),
            t,
            previous_t: t,
            target_t: t,
            velocity: 0.0,
            previous_velocity: 0.0,
            max_speed: DEFAULT_LID_SPEED,
        }
    }
}

impl LidState {
    pub fn toggle(&mut self) {
        match self.mode {
            LidMode::Closed | LidMode::Closing => self.open(),
            LidMode::Open | LidMode::Opening | LidMode::Manual => {
                if self.t < 0.5 {
                    self.open();
                } else {
                    self.close();
                }
            }
        }
    }

    pub fn open(&mut self) {
        self.target_t = 1.0;
        self.mode = if self.t >= 1.0 - ENDPOINT_EPSILON {
            LidMode::Open
        } else {
            LidMode::Opening
        };
    }

    pub fn close(&mut self) {
        self.target_t = 0.0;
        self.mode = if self.t <= ENDPOINT_EPSILON {
            LidMode::Closed
        } else {
            LidMode::Closing
        };
    }

    pub fn set_manual(&mut self, t: f32) {
        let t = t.clamp(0.0, 1.0);
        self.target_t = t;
        self.t = t;
        self.mode = endpoint_mode(t).unwrap_or(LidMode::Manual);
    }

    pub fn tick(&mut self, dt: f32) {
        self.previous_t = self.t;
        self.previous_velocity = self.velocity;
        self.velocity = 0.0;

        let direction = match self.mode {
            LidMode::Opening => 1.0,
            LidMode::Closing => -1.0,
            LidMode::Closed | LidMode::Open | LidMode::Manual => return,
        };
        let dt = dt.max(0.0);
        let step = self.max_speed.max(0.0) * dt;
        if step <= f32::EPSILON {
            return;
        }

        let remaining = (self.target_t - self.t).abs();
        let applied = step.min(remaining);
        self.t = (self.t + direction * applied).clamp(0.0, 1.0);
        self.velocity = if dt > f32::EPSILON {
            direction * applied / dt
        } else {
            0.0
        };

        if (self.target_t - self.t).abs() <= ENDPOINT_EPSILON {
            self.t = self.target_t;
            self.mode = endpoint_mode(self.t).unwrap_or(LidMode::Manual);
            self.velocity = 0.0;
        }
    }

    pub fn delta_t(&self) -> f32 {
        self.t - self.previous_t
    }

    pub fn last_motion_velocity(&self, dt: f32) -> f32 {
        if dt > f32::EPSILON {
            self.delta_t() / dt
        } else {
            self.previous_velocity
        }
    }
}

fn endpoint_mode(t: f32) -> Option<LidMode> {
    if t <= ENDPOINT_EPSILON {
        Some(LidMode::Closed)
    } else if t >= 1.0 - ENDPOINT_EPSILON {
        Some(LidMode::Open)
    } else {
        None
    }
}

pub fn update_lid_state(time: Res<Time>, mut lid: ResMut<LidState>) {
    lid.tick(time.delta_secs());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closed_lid_toggle_opens() {
        let mut lid = LidState::default();

        lid.toggle();
        lid.tick(0.1);

        assert_eq!(lid.mode, LidMode::Opening);
        assert!(lid.t > 0.0);
    }

    #[test]
    fn closing_lid_toggle_interrupts_to_opening() {
        let mut lid = LidState {
            mode: LidMode::Closing,
            t: 0.4,
            previous_t: 0.5,
            target_t: 0.0,
            velocity: -1.0,
            previous_velocity: -1.0,
            max_speed: 1.0,
        };

        lid.toggle();
        lid.tick(0.1);

        assert_eq!(lid.mode, LidMode::Opening);
        assert!(lid.t > 0.4);
    }

    #[test]
    fn manual_set_uses_same_state() {
        let mut lid = LidState::default();

        lid.set_manual(0.6);

        assert_eq!(lid.mode, LidMode::Manual);
        assert_eq!(lid.t, 0.6);
        assert_eq!(lid.target_t, 0.6);
    }
}
