use std::num::NonZero;

use bevy::prelude::*;
use rodio::{Player, buffer::SamplesBuffer};

use crate::{
    lid::LidState,
    mechanism_view::MechanismResource,
    playback::{self, AudioOutputState},
    twin::MusicBoxTwinState,
};

const MOTION_EPSILON: f32 = 0.00001;
const TRANSIENT_LIMIT: usize = 16;
const RECENT_EVENT_LIMIT: usize = 12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LidDirection {
    Opening,
    Closing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindingDirection {
    Winding,
    Releasing,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MechanicalEvent {
    LidMotion {
        t: f32,
        angular_velocity: f32,
        direction: LidDirection,
    },
    LidClosedImpact {
        speed: f32,
    },
    LidOpenedStop {
        speed: f32,
    },
    WindingKeyMotion {
        angular_velocity: f32,
        direction: WindingDirection,
        spring_energy: f32,
    },
    CylinderMotion {
        angular_velocity: f32,
        phase: f32,
    },
    ToothCombContact {
        midi_note: i32,
        intensity: f32,
        phase: f32,
    },
}

impl MechanicalEvent {
    pub fn kind(self) -> MechanicalEventKind {
        match self {
            MechanicalEvent::LidMotion { .. } => MechanicalEventKind::LidMotion,
            MechanicalEvent::LidClosedImpact { .. } => MechanicalEventKind::LidClosedImpact,
            MechanicalEvent::LidOpenedStop { .. } => MechanicalEventKind::LidOpenedStop,
            MechanicalEvent::WindingKeyMotion { .. } => MechanicalEventKind::WindingKeyMotion,
            MechanicalEvent::CylinderMotion { .. } => MechanicalEventKind::CylinderMotion,
            MechanicalEvent::ToothCombContact { .. } => MechanicalEventKind::ToothCombContact,
        }
    }

    pub fn summary(self) -> String {
        match self {
            MechanicalEvent::LidMotion {
                t,
                angular_velocity,
                direction,
            } => format!("lid {:?} t={t:.2} v={angular_velocity:.2}", direction),
            MechanicalEvent::LidClosedImpact { speed } => {
                format!("lid close impact speed={speed:.2}")
            }
            MechanicalEvent::LidOpenedStop { speed } => {
                format!("lid open stop speed={speed:.2}")
            }
            MechanicalEvent::WindingKeyMotion {
                angular_velocity,
                direction,
                spring_energy,
            } => format!(
                "key {:?} v={angular_velocity:.1} energy={spring_energy:.2}",
                direction
            ),
            MechanicalEvent::CylinderMotion {
                angular_velocity,
                phase,
            } => format!("cylinder v={angular_velocity:.1} phase={phase:.2}"),
            MechanicalEvent::ToothCombContact {
                midi_note,
                intensity,
                phase,
            } => format!("tooth midi={midi_note} i={intensity:.2} phase={phase:.2}"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MechanicalEventKind {
    LidMotion,
    LidClosedImpact,
    LidOpenedStop,
    WindingKeyMotion,
    CylinderMotion,
    ToothCombContact,
}

#[derive(Resource, Debug, Clone)]
pub struct MechanicalAudioConfig {
    pub enabled: bool,
    pub master_gain: f32,
    pub lid_motion_gain: f32,
    pub lid_close_impact_gain: f32,
    pub lid_open_stop_gain: f32,
    pub winding_gain: f32,
    pub cylinder_gain: f32,
    pub tooth_scrape_gain: f32,
    pub frequency_scale: f32,
    pub roughness_scale: f32,
    pub decay_scale: f32,
    pub mute_lid: bool,
    pub mute_winding: bool,
    pub mute_cylinder: bool,
    pub mute_tooth: bool,
    pub solo_lid: bool,
    pub solo_winding: bool,
    pub solo_cylinder: bool,
    pub solo_tooth: bool,
}

impl Default for MechanicalAudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            master_gain: 1.0,
            lid_motion_gain: 1.0,
            lid_close_impact_gain: 1.0,
            lid_open_stop_gain: 1.0,
            winding_gain: 1.0,
            cylinder_gain: 1.0,
            tooth_scrape_gain: 1.0,
            frequency_scale: 1.0,
            roughness_scale: 1.0,
            decay_scale: 1.0,
            mute_lid: false,
            mute_winding: false,
            mute_cylinder: false,
            mute_tooth: false,
            solo_lid: false,
            solo_winding: false,
            solo_cylinder: false,
            solo_tooth: false,
        }
    }
}

impl MechanicalAudioConfig {
    pub fn event_gain(&self, kind: MechanicalEventKind) -> f32 {
        if !self.enabled || !self.kind_enabled(kind) {
            return 0.0;
        }
        let gain = match kind {
            MechanicalEventKind::LidMotion => self.lid_motion_gain,
            MechanicalEventKind::LidClosedImpact => self.lid_close_impact_gain,
            MechanicalEventKind::LidOpenedStop => self.lid_open_stop_gain,
            MechanicalEventKind::WindingKeyMotion => self.winding_gain,
            MechanicalEventKind::CylinderMotion => self.cylinder_gain,
            MechanicalEventKind::ToothCombContact => self.tooth_scrape_gain,
        };
        self.master_gain * gain
    }

    fn kind_enabled(&self, kind: MechanicalEventKind) -> bool {
        let any_solo = self.solo_lid || self.solo_winding || self.solo_cylinder || self.solo_tooth;
        let solo_match = match kind {
            MechanicalEventKind::LidMotion
            | MechanicalEventKind::LidClosedImpact
            | MechanicalEventKind::LidOpenedStop => self.solo_lid,
            MechanicalEventKind::WindingKeyMotion => self.solo_winding,
            MechanicalEventKind::CylinderMotion => self.solo_cylinder,
            MechanicalEventKind::ToothCombContact => self.solo_tooth,
        };
        if any_solo && !solo_match {
            return false;
        }
        match kind {
            MechanicalEventKind::LidMotion
            | MechanicalEventKind::LidClosedImpact
            | MechanicalEventKind::LidOpenedStop => !self.mute_lid,
            MechanicalEventKind::WindingKeyMotion => !self.mute_winding,
            MechanicalEventKind::CylinderMotion => !self.mute_cylinder,
            MechanicalEventKind::ToothCombContact => !self.mute_tooth,
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MechanicalEventStats {
    pub frame_events: usize,
    pub total_events: u64,
    pub lid_motion: u64,
    pub lid_closed_impact: u64,
    pub lid_opened_stop: u64,
    pub winding_key_motion: u64,
    pub cylinder_motion: u64,
    pub tooth_comb_contact: u64,
    pub recent: Vec<String>,
}

impl MechanicalEventStats {
    pub fn record_frame(&mut self, events: &[MechanicalEvent]) {
        self.frame_events = events.len();
        for event in events {
            self.total_events += 1;
            match event.kind() {
                MechanicalEventKind::LidMotion => self.lid_motion += 1,
                MechanicalEventKind::LidClosedImpact => self.lid_closed_impact += 1,
                MechanicalEventKind::LidOpenedStop => self.lid_opened_stop += 1,
                MechanicalEventKind::WindingKeyMotion => self.winding_key_motion += 1,
                MechanicalEventKind::CylinderMotion => self.cylinder_motion += 1,
                MechanicalEventKind::ToothCombContact => self.tooth_comb_contact += 1,
            }
            self.recent.push(event.summary());
        }
        if self.recent.len() > RECENT_EVENT_LIMIT {
            let remove_count = self.recent.len() - RECENT_EVENT_LIMIT;
            self.recent.drain(0..remove_count);
        }
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MechanicalAuditionQueue {
    pub requests: Vec<MechanicalEvent>,
}

impl MechanicalAuditionQueue {
    pub fn push(&mut self, event: MechanicalEvent) {
        self.requests.push(event);
    }
}

#[derive(Resource, Default)]
pub struct MechanicalEventQueue {
    pub events: Vec<MechanicalEvent>,
}

#[derive(Resource, Debug, Clone)]
pub struct MechanicalEventState {
    initialized: bool,
    previous_lid_t: f32,
    previous_key_degrees: f32,
    previous_cylinder_degrees: f32,
    previous_mechanical_seconds: f32,
}

impl Default for MechanicalEventState {
    fn default() -> Self {
        Self {
            initialized: false,
            previous_lid_t: 0.0,
            previous_key_degrees: 0.0,
            previous_cylinder_degrees: 0.0,
            previous_mechanical_seconds: 0.0,
        }
    }
}

#[derive(Resource, Default)]
pub struct MechanicalAudioState {
    lid_lane: ContinuousLane,
    winding_lane: ContinuousLane,
    cylinder_lane: ContinuousLane,
    transient_players: Vec<Player>,
    pub last_event_count: usize,
}

impl MechanicalAudioState {
    pub fn transient_count(&self) -> usize {
        self.transient_players.len()
    }

    pub fn lid_lane_active(&self) -> bool {
        self.lid_lane.player.is_some()
    }

    pub fn winding_lane_active(&self) -> bool {
        self.winding_lane.player.is_some()
    }

    pub fn cylinder_lane_active(&self) -> bool {
        self.cylinder_lane.player.is_some()
    }

    pub fn stop_all(&mut self) {
        if let Some(player) = self.lid_lane.player.take() {
            player.stop();
        }
        if let Some(player) = self.winding_lane.player.take() {
            player.stop();
        }
        if let Some(player) = self.cylinder_lane.player.take() {
            player.stop();
        }
        for player in self.transient_players.drain(..) {
            player.stop();
        }
        self.last_event_count = 0;
    }
}

#[derive(Default)]
struct ContinuousLane {
    player: Option<Player>,
}

pub fn emit_mechanical_events(
    time: Res<Time>,
    lid: Res<LidState>,
    twin: Res<MusicBoxTwinState>,
    mechanism: Res<MechanismResource>,
    mut state: ResMut<MechanicalEventState>,
    mut queue: ResMut<MechanicalEventQueue>,
) {
    queue.events.clear();
    let dt = time.delta_secs().max(f32::EPSILON);
    if !state.initialized {
        state.previous_lid_t = lid.t;
        state.previous_key_degrees = twin.key_degrees;
        state.previous_cylinder_degrees = twin.cylinder_degrees;
        state.previous_mechanical_seconds = twin.mechanical_seconds;
        state.initialized = true;
        return;
    }

    emit_lid_events(&mut queue.events, &state, lid.t, dt);
    emit_winding_events(&mut queue.events, &state, &twin, dt);
    emit_cylinder_events(&mut queue.events, &state, &twin, dt);
    emit_tooth_comb_events(&mut queue.events, &state, &twin, &mechanism);

    state.previous_lid_t = lid.t;
    state.previous_key_degrees = twin.key_degrees;
    state.previous_cylinder_degrees = twin.cylinder_degrees;
    state.previous_mechanical_seconds = twin.mechanical_seconds;
}

fn emit_lid_events(
    events: &mut Vec<MechanicalEvent>,
    state: &MechanicalEventState,
    lid_t: f32,
    dt: f32,
) {
    let delta = lid_t - state.previous_lid_t;
    if delta.abs() <= MOTION_EPSILON {
        return;
    }
    let angular_velocity = delta / dt;
    let speed = angular_velocity.abs();
    let direction = if delta > 0.0 {
        LidDirection::Opening
    } else {
        LidDirection::Closing
    };
    events.push(MechanicalEvent::LidMotion {
        t: lid_t,
        angular_velocity,
        direction,
    });
    if state.previous_lid_t > 0.0 && lid_t <= 0.0 {
        events.push(MechanicalEvent::LidClosedImpact { speed });
    }
    if state.previous_lid_t < 1.0 && lid_t >= 1.0 {
        events.push(MechanicalEvent::LidOpenedStop { speed });
    }
}

fn emit_winding_events(
    events: &mut Vec<MechanicalEvent>,
    state: &MechanicalEventState,
    twin: &MusicBoxTwinState,
    dt: f32,
) {
    let delta = twin.key_degrees - state.previous_key_degrees;
    if delta.abs() <= MOTION_EPSILON {
        return;
    }
    let direction = if delta < 0.0 {
        WindingDirection::Winding
    } else {
        WindingDirection::Releasing
    };
    events.push(MechanicalEvent::WindingKeyMotion {
        angular_velocity: delta / dt,
        direction,
        spring_energy: twin.spring_energy,
    });
}

fn emit_cylinder_events(
    events: &mut Vec<MechanicalEvent>,
    state: &MechanicalEventState,
    twin: &MusicBoxTwinState,
    dt: f32,
) {
    let delta = shortest_angle_delta(twin.cylinder_degrees, state.previous_cylinder_degrees);
    if delta.abs() <= MOTION_EPSILON {
        return;
    }
    events.push(MechanicalEvent::CylinderMotion {
        angular_velocity: delta / dt,
        phase: twin.cylinder_degrees.rem_euclid(360.0) / 360.0,
    });
}

fn emit_tooth_comb_events(
    events: &mut Vec<MechanicalEvent>,
    state: &MechanicalEventState,
    twin: &MusicBoxTwinState,
    mechanism: &MechanismResource,
) {
    if twin.mechanical_seconds <= state.previous_mechanical_seconds + f32::EPSILON {
        return;
    }
    let previous_tick =
        playback::seconds_to_cycle_tick(state.previous_mechanical_seconds, mechanism);
    let current_tick = playback::seconds_to_cycle_tick(twin.mechanical_seconds, mechanism);
    let ticks_per_turn = mechanism.ticks_per_turn.max(1);
    for event in &mechanism.comb_animation_events {
        if !event.contact_supported {
            continue;
        }
        let contact_tick = event.contact_start_tick.rem_euclid(ticks_per_turn);
        if crossed_cycle_tick(previous_tick, current_tick, contact_tick, ticks_per_turn) {
            events.push(MechanicalEvent::ToothCombContact {
                midi_note: event.midi_note,
                intensity: event.source_velocity.abs().clamp(0.05, 1.0),
                phase: contact_tick as f32 / ticks_per_turn as f32,
            });
        }
    }
}

pub fn play_mechanical_audio(
    time: Res<Time>,
    queue: Res<MechanicalEventQueue>,
    config: Res<MechanicalAudioConfig>,
    mut audition: ResMut<MechanicalAuditionQueue>,
    mut stats: ResMut<MechanicalEventStats>,
    mut audio: ResMut<AudioOutputState>,
    mut state: ResMut<MechanicalAudioState>,
) {
    stats.record_frame(&queue.events);
    let mut events = queue.events.clone();
    events.append(&mut audition.requests);
    state.last_event_count = events.len();
    state.transient_players.retain(|player| !player.empty());
    if events.is_empty() {
        return;
    }
    let Some(device) = audio.device.as_ref() else {
        audio.last_error = Some("audio device is unavailable for mechanical sounds".to_string());
        return;
    };
    let continuous_seconds = time.delta_secs().clamp(0.005, 0.04);
    let sample_rate = device.config().sample_rate();
    for event in &events {
        let event_gain = config.event_gain(event.kind());
        if event_gain <= f32::EPSILON {
            continue;
        }
        match *event {
            MechanicalEvent::LidMotion {
                angular_velocity,
                direction,
                ..
            } => {
                let hz = match direction {
                    LidDirection::Opening => 190.0,
                    LidDirection::Closing => 135.0,
                };
                append_lane_chunk(
                    &mut state.lid_lane,
                    device.mixer(),
                    sample_rate,
                    hz * config.frequency_scale,
                    angular_velocity.abs() * 0.015 * event_gain,
                    0.35 * config.roughness_scale,
                    continuous_seconds,
                );
            }
            MechanicalEvent::LidClosedImpact { speed } => {
                spawn_transient(
                    &mut state.transient_players,
                    device.mixer(),
                    sample_rate,
                    92.0 * config.frequency_scale,
                    speed * 0.18 * event_gain,
                    0.055 * config.decay_scale,
                    0.85 * config.roughness_scale,
                );
            }
            MechanicalEvent::LidOpenedStop { speed } => {
                spawn_transient(
                    &mut state.transient_players,
                    device.mixer(),
                    sample_rate,
                    310.0 * config.frequency_scale,
                    speed * 0.08 * event_gain,
                    0.045 * config.decay_scale,
                    0.4 * config.roughness_scale,
                );
            }
            MechanicalEvent::WindingKeyMotion {
                angular_velocity,
                direction,
                spring_energy,
            } => {
                let hz = match direction {
                    WindingDirection::Winding => 420.0,
                    WindingDirection::Releasing => 260.0,
                };
                append_lane_chunk(
                    &mut state.winding_lane,
                    device.mixer(),
                    sample_rate,
                    hz * config.frequency_scale,
                    angular_velocity.abs() * (0.00009 + spring_energy * 0.00004) * event_gain,
                    0.7 * config.roughness_scale,
                    continuous_seconds,
                );
            }
            MechanicalEvent::CylinderMotion {
                angular_velocity,
                phase,
            } => {
                append_lane_chunk(
                    &mut state.cylinder_lane,
                    device.mixer(),
                    sample_rate,
                    (70.0 + phase * 14.0) * config.frequency_scale,
                    angular_velocity.abs() * 0.00008 * event_gain,
                    0.2 * config.roughness_scale,
                    continuous_seconds,
                );
            }
            MechanicalEvent::ToothCombContact {
                midi_note,
                intensity,
                ..
            } => {
                spawn_transient(
                    &mut state.transient_players,
                    device.mixer(),
                    sample_rate,
                    (1200.0 + (midi_note as f32 - 60.0) * 9.0) * config.frequency_scale,
                    intensity * 0.025 * event_gain,
                    0.012 * config.decay_scale,
                    1.0 * config.roughness_scale,
                );
            }
        }
    }
}

fn append_lane_chunk(
    lane: &mut ContinuousLane,
    mixer: &rodio::mixer::Mixer,
    sample_rate: NonZero<u32>,
    frequency_hz: f32,
    gain: f32,
    roughness: f32,
    seconds: f32,
) {
    let player = lane
        .player
        .get_or_insert_with(|| Player::connect_new(mixer));
    let samples = render_chunk(
        sample_rate.get(),
        seconds,
        frequency_hz,
        gain.clamp(0.0, 0.08),
        roughness,
    );
    player.append(SamplesBuffer::new(
        NonZero::new(1).unwrap(),
        sample_rate,
        samples,
    ));
}

fn spawn_transient(
    players: &mut Vec<Player>,
    mixer: &rodio::mixer::Mixer,
    sample_rate: NonZero<u32>,
    frequency_hz: f32,
    gain: f32,
    seconds: f32,
    roughness: f32,
) {
    if players.len() >= TRANSIENT_LIMIT {
        players.remove(0).stop();
    }
    let player = Player::connect_new(mixer);
    player.append(SamplesBuffer::new(
        NonZero::new(1).unwrap(),
        sample_rate,
        render_chunk(
            sample_rate.get(),
            seconds,
            frequency_hz,
            gain.clamp(0.0, 0.2),
            roughness,
        ),
    ));
    players.push(player);
}

fn render_chunk(
    sample_rate: u32,
    seconds: f32,
    frequency_hz: f32,
    gain: f32,
    roughness: f32,
) -> Vec<f32> {
    let frames = (sample_rate as f32 * seconds.max(0.001)).round().max(1.0) as usize;
    let mut samples = Vec::with_capacity(frames);
    for frame in 0..frames {
        let t = frame as f32 / sample_rate as f32;
        let envelope = 1.0 - frame as f32 / frames as f32;
        let carrier = (std::f32::consts::TAU * frequency_hz.max(1.0) * t).sin();
        let scrape = pseudo_noise(frame) * roughness.clamp(0.0, 1.0);
        samples.push((carrier * (1.0 - roughness) + scrape) * envelope * gain);
    }
    samples
}

fn pseudo_noise(index: usize) -> f32 {
    let mut value = index as u32;
    value ^= value << 13;
    value ^= value >> 17;
    value ^= value << 5;
    value as f32 / u32::MAX as f32 * 2.0 - 1.0
}

fn shortest_angle_delta(current: f32, previous: f32) -> f32 {
    (current - previous + 180.0).rem_euclid(360.0) - 180.0
}

fn crossed_cycle_tick(previous: i64, current: i64, target: i64, ticks_per_turn: i64) -> bool {
    if previous == current {
        return false;
    }
    if current > previous {
        target > previous && target <= current
    } else {
        target > previous && target < ticks_per_turn || target <= current
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lid_delta_emits_motion_and_endpoint_events() {
        let mut events = Vec::new();
        let state = MechanicalEventState {
            initialized: true,
            previous_lid_t: 0.2,
            ..default()
        };

        emit_lid_events(&mut events, &state, 0.0, 0.1);

        assert!(matches!(
            events[0],
            MechanicalEvent::LidMotion {
                direction: LidDirection::Closing,
                ..
            }
        ));
        assert!(matches!(events[1], MechanicalEvent::LidClosedImpact { .. }));
    }

    #[test]
    fn winding_direction_follows_key_delta() {
        let mut events = Vec::new();
        let state = MechanicalEventState {
            initialized: true,
            previous_key_degrees: 10.0,
            ..default()
        };
        let twin = MusicBoxTwinState {
            key_degrees: -5.0,
            spring_energy: 0.3,
            ..default()
        };

        emit_winding_events(&mut events, &state, &twin, 0.1);

        assert!(matches!(
            events[0],
            MechanicalEvent::WindingKeyMotion {
                direction: WindingDirection::Winding,
                ..
            }
        ));
    }

    #[test]
    fn cycle_tick_crossing_handles_wrap() {
        assert!(crossed_cycle_tick(90, 10, 95, 100));
        assert!(crossed_cycle_tick(90, 10, 5, 100));
        assert!(!crossed_cycle_tick(20, 40, 10, 100));
    }
}
