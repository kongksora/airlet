use std::{collections::BTreeMap, ops::Range};

use airlet::{
    mechanism::{MechanismLayoutHint, ToothHint},
    score::PPQ,
};
use airlet_model::{MovableMusicBoxModel, PivotPose};
use bevy::{
    asset::RenderAssetUsages, mesh::Indices, prelude::*, render::render_resource::PrimitiveTopology,
};
use serde::Serialize;

use crate::comb_animation::{CombAnimationEvent, comb_tine_sample};
use crate::lighting::{ExhibitLightingConfig, TextureMaterialClass, apply_texture_class};

// ── visual proportions (mirrors MechanismVisualConfig::default()) ─
pub const TOOTH_WIDTH_RATIO: f32 = 0.028;
pub const TOOTH_HEIGHT_RATIO: f32 = 0.14;
pub const COMB_TINE_LENGTH_RATIO: f32 = 1.35;
pub const COMB_TINE_WIDTH_RATIO: f32 = 0.035;
pub const COMB_TINE_THICKNESS_RATIO: f32 = 0.025;
pub const COMB_FREE_LENGTH_RATIO: f32 = 0.72;
pub const COMB_TINE_WIDTH_SPACING_RATIO: f32 = 0.82;
pub const COMB_TRACK_USABLE_LENGTH_RATIO: f32 = 0.86;
pub const DEFAULT_TOOTH_CLEARANCE_RATIO: f32 = 0.92;

pub const COMB_MIN_PLUCK_TICKS: i64 = PPQ / 16;
pub const COMB_MAX_PLUCK_TICKS: i64 = PPQ / 3;
pub const COMB_MIN_VIBRATION_TICKS: i64 = PPQ / 2;
pub const COMB_MAX_VIBRATION_TICKS: i64 = PPQ * 3;
pub const COMB_LIFT_WINDOW_RATIO: f32 = 0.65;
pub const COMB_DEFLECTION_SCALE: f32 = 0.65;
pub const COMB_MIN_DEFLECTION_RAD: f32 = 0.035;
pub const COMB_MAX_DEFLECTION_RAD: f32 = 0.18;
pub const COMB_GHOST_SAMPLES: [f32; 4] = [-0.38, -0.18, 0.18, 0.38];
pub const CYLINDER_PLAYBACK_ROTATION_SIGN: f32 = -1.0;

// ── resources ───────────────────────────────────────────────────

#[derive(Resource)]
pub struct MechanismResource {
    pub hint: MechanismLayoutHint,
    pub comb_animation_events: Vec<CombAnimationEvent>,
    pub ticks_per_turn: i64,
    pub quarter_millis: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MechanismCalibration {
    pub lowest_midi: i32,
    pub highest_midi: i32,
    pub track_count: usize,
    pub cylinder_length: f32,
    pub usable_length: f32,
    pub side_margin: f32,
    pub track_spacing: f32,
    pub axial_min: f32,
    pub axial_max: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingGroup {
    pub key_tick: i64,
    pub events: Vec<TimingEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingEvent {
    pub onset_tick: i64,
    pub midi_note: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TimingValidation {
    pub ticks_per_turn: i64,
    pub same_onset_groups: Vec<TimingGroup>,
    pub same_phase_groups: Vec<TimingGroup>,
    pub same_onset_group_count: usize,
    pub same_phase_group_count: usize,
}

// ── comb mesh model ─────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CombTineRange {
    pub midi_note: i32,
    pub pivot: Vec3,
    pub vertex_range: Range<usize>,
    pub index_range: Range<usize>,
}

#[derive(Debug, Clone)]
pub struct CombFixedBaseRange {
    pub center: Vec3,
    pub width: f32,
    pub length: f32,
    pub thickness: f32,
    pub vertex_range: Range<usize>,
    pub index_range: Range<usize>,
}

#[derive(Component)]
pub struct CombMeshModel {
    pub mesh: Handle<Mesh>,
    pub tines: Vec<CombTineRange>,
    pub fixed_base: Option<CombFixedBaseRange>,
    pub length: f32,
    pub width: f32,
    pub thickness: f32,
    pub segment_count: usize,
    pub deflection_sign: f32,
    pub smear_sample: Option<f32>,
}

impl CombMeshModel {
    pub fn mesh_for_tick(
        &self,
        current_tick: Option<i64>,
        mechanism: &MechanismResource,
    ) -> (Mesh, bool) {
        let mut any_visible = self.smear_sample.is_none();
        let mut active_tines = Vec::with_capacity(self.tines.len());
        let deflections = self
            .tines
            .iter()
            .map(|tine| {
                let sample = current_tick.and_then(|tick| {
                    comb_tine_sample(tine.midi_note, tick, self.smear_sample, mechanism)
                });
                let visible = sample
                    .map(|sample| sample.visible || self.smear_sample.is_none())
                    .unwrap_or(self.smear_sample.is_none());
                any_visible |= visible;
                active_tines.push(visible);
                if visible {
                    sample
                        .map(|sample| sample.deflection_rad * self.deflection_sign)
                        .unwrap_or(0.0)
                } else {
                    0.0
                }
            })
            .collect::<Vec<_>>();
        (
            comb_mesh(
                self.fixed_base.clone(),
                &self.tines,
                self.length,
                self.width,
                self.thickness,
                self.segment_count,
                &deflections,
                &active_tines,
                self.smear_sample.is_some(),
            ),
            any_visible,
        )
    }
}

// ── systems ─────────────────────────────────────────────────────

pub fn animate_comb_tines(
    twin: Res<crate::twin::MusicBoxTwinState>,
    mechanism: Res<MechanismResource>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut query: Query<(&CombMeshModel, &mut Visibility)>,
) {
    let current_tick = twin
        .comb_animation_seconds()
        .map(|seconds| crate::playback::seconds_to_cycle_tick(seconds, &mechanism));
    for (comb, mut visibility) in &mut query {
        let (mesh, visible) = comb.mesh_for_tick(current_tick, &mechanism);
        *visibility = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        if let Some(mut stored_mesh) = meshes.get_mut(&comb.mesh) {
            *stored_mesh = mesh;
        }
    }
}

// ── mechanism spawning ──────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn spawn_hint_mechanism(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    lighting: &ExhibitLightingConfig,
    root: Entity,
    cylinder_pivot: Entity,
    cylinder_pose: PivotPose,
    model: &MovableMusicBoxModel,
    hint: &MechanismLayoutHint,
) {
    use crate::vec3;

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
    let comb_segment_count = 5usize;
    let comb_fixed_width = calibration.usable_length
        + if calibration.track_count > 1 {
            calibration.track_spacing
        } else {
            comb_tine_width
        };
    let comb_fixed_thickness = comb_tine_thickness * 2.4;
    let cylinder_material = materials.add({
        let mut material = lighting.cylinder.material();
        apply_texture_class(&mut material, asset_server, TextureMaterialClass::AgedBrass);
        material
    });
    let tooth_material = materials.add({
        let mut material = lighting.tooth.material();
        apply_texture_class(&mut material, asset_server, TextureMaterialClass::AgedBrass);
        material
    });
    let comb_material = materials.add({
        let mut material = lighting.comb.material();
        apply_texture_class(
            &mut material,
            asset_server,
            TextureMaterialClass::PolishedSteel,
        );
        material
    });
    let comb_ghost_material = materials.add({
        let mut material = lighting.comb_ghost.material();
        apply_texture_class(
            &mut material,
            asset_server,
            TextureMaterialClass::PolishedSteel,
        );
        material.alpha_mode = AlphaMode::Blend;
        material.base_color = lighting.comb_ghost.base_color;
        material
    });

    let cylinder = commands
        .spawn((
            Name::new("Hint Cylinder Body"),
            crate::scene::ProceduralMechanism,
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
                crate::scene::ProceduralMechanism,
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
                crate::scene::ProceduralMechanism,
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
    let fixed_local_center = Vec3::new(
        (calibration.axial_min + calibration.axial_max) * 0.5,
        tip_radius + comb_free_length + comb_fixed_length * 0.5,
        0.0,
    );
    debug_assert!(
        (fixed_position - (axis * fixed_local_center.x + radial_zero * fixed_local_center.y))
            .length()
            < 1e-4
    );
    let tine_pivots = (calibration.lowest_midi..=calibration.highest_midi)
        .map(|midi_note| {
            (
                midi_note,
                Vec3::new(
                    note_axial_position(midi_note, &calibration),
                    tip_radius + comb_free_length,
                    0.0,
                ),
            )
        })
        .collect::<Vec<_>>();
    let fixed_base = Some(CombFixedBaseRange {
        center: fixed_local_center,
        width: comb_fixed_width,
        length: comb_fixed_length,
        thickness: comb_fixed_thickness,
        vertex_range: 0..0,
        index_range: 0..0,
    });
    spawn_comb_mesh_visual(
        commands,
        meshes,
        root,
        cylinder_pose.pivot,
        None,
        &comb_material,
        axis,
        radial_zero,
        tangent_zero,
        fixed_base,
        &tine_pivots,
        comb_free_length,
        comb_tine_width,
        comb_tine_thickness,
        comb_segment_count,
    );
    for smear_sample in COMB_GHOST_SAMPLES {
        spawn_comb_mesh_visual(
            commands,
            meshes,
            root,
            cylinder_pose.pivot,
            Some(smear_sample),
            &comb_ghost_material,
            axis,
            radial_zero,
            tangent_zero,
            None,
            &tine_pivots,
            comb_free_length,
            comb_tine_width,
            comb_tine_thickness,
            comb_segment_count,
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn spawn_comb_mesh_visual(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    root: Entity,
    cylinder_pivot: [f32; 3],
    smear_sample: Option<f32>,
    material: &Handle<StandardMaterial>,
    axis: Vec3,
    radial_zero: Vec3,
    tangent_zero: Vec3,
    fixed_base: Option<CombFixedBaseRange>,
    tine_pivots: &[(i32, Vec3)],
    comb_free_length: f32,
    comb_tine_width: f32,
    comb_tine_thickness: f32,
    segment_count: usize,
) {
    use crate::vec3;

    let rotation = basis_rotation(axis, radial_zero, tangent_zero);
    let deflection_sign = comb_tine_deflection_sign(axis, radial_zero, tangent_zero);
    let suffix = smear_sample
        .map(|sample| format!(" ghost batch {sample:.2}"))
        .unwrap_or_default();
    let visibility = if smear_sample.is_some() {
        Visibility::Hidden
    } else {
        Visibility::Visible
    };
    let (fixed_base, tines) = comb_mesh_ranges(fixed_base, tine_pivots, segment_count);
    let deflections = vec![0.0; tines.len()];
    let active_tines = vec![smear_sample.is_none(); tines.len()];
    let mesh = meshes.add(comb_mesh(
        fixed_base.clone(),
        &tines,
        comb_free_length,
        comb_tine_width,
        comb_tine_thickness,
        segment_count,
        &deflections,
        &active_tines,
        smear_sample.is_some(),
    ));
    let comb = commands
        .spawn((
            Name::new(format!("Comb Mesh{suffix}")),
            CombMeshModel {
                mesh: mesh.clone(),
                tines,
                fixed_base,
                length: comb_free_length,
                width: comb_tine_width,
                thickness: comb_tine_thickness,
                segment_count,
                deflection_sign,
                smear_sample,
            },
            crate::scene::ProceduralMechanism,
            Mesh3d(mesh),
            MeshMaterial3d(material.clone()),
            Transform::from_translation(vec3(cylinder_pivot)).with_rotation(rotation),
            visibility,
        ))
        .id();
    commands.entity(root).add_child(comb);
}

// ── tooth positioning ───────────────────────────────────────────

pub fn tooth_transform(
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

// ── calibration helpers ─────────────────────────────────────────

pub fn mechanism_calibration(
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

pub fn track_index(midi_note: i32, calibration: &MechanismCalibration) -> usize {
    (midi_note - calibration.lowest_midi).max(0) as usize
}

pub fn note_axial_position(midi_note: i32, calibration: &MechanismCalibration) -> f32 {
    if calibration.track_count <= 1 {
        0.0
    } else {
        let track = track_index(midi_note, calibration).min(calibration.track_count - 1);
        calibration.axial_min + track as f32 * calibration.track_spacing
    }
}

// ── measured comb geometry ──────────────────────────────────────

pub fn measured_comb_radial_direction(model: &MovableMusicBoxModel, axis: Vec3) -> Vec3 {
    use crate::vec3;
    let measured = vec3(model.spec().comb.radial_direction);
    let measured = measured - axis * measured.dot(axis);
    if measured.length_squared() > 1e-6 {
        measured.normalize()
    } else {
        cylinder_radial_frame(axis).0
    }
}

pub fn measured_comb_tip_radius(
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

pub fn measured_comb_tine_length(model: &MovableMusicBoxModel, cylinder_radius: f32) -> f32 {
    let comb = &model.spec().comb;
    if comb.tine_length > f32::EPSILON {
        comb.tine_length
    } else if comb.root_radius > comb.tip_radius {
        comb.root_radius - comb.tip_radius
    } else {
        cylinder_radius * COMB_TINE_LENGTH_RATIO
    }
}

// ── mesh construction ───────────────────────────────────────────

pub fn comb_mesh_ranges(
    fixed_base: Option<CombFixedBaseRange>,
    tine_pivots: &[(i32, Vec3)],
    segment_count: usize,
) -> (Option<CombFixedBaseRange>, Vec<CombTineRange>) {
    let segment_count = segment_count.max(1);
    let mut vertex_offset = 0usize;
    let mut index_offset = 0usize;
    let fixed_base = fixed_base.map(|base| {
        let vertex_count = 8;
        let index_count = 36;
        let ranged_base = CombFixedBaseRange {
            vertex_range: vertex_offset..vertex_offset + vertex_count,
            index_range: index_offset..index_offset + index_count,
            ..base
        };
        vertex_offset += vertex_count;
        index_offset += index_count;
        ranged_base
    });
    let tine_vertex_count = segment_count * 16 + 8;
    let tine_index_count = segment_count * 24 + 12;
    let tines = tine_pivots
        .iter()
        .map(|(midi_note, pivot)| {
            let range = CombTineRange {
                midi_note: *midi_note,
                pivot: *pivot,
                vertex_range: vertex_offset..vertex_offset + tine_vertex_count,
                index_range: index_offset..index_offset + tine_index_count,
            };
            vertex_offset += tine_vertex_count;
            index_offset += tine_index_count;
            range
        })
        .collect();
    (fixed_base, tines)
}

pub fn comb_mesh(
    fixed_base: Option<CombFixedBaseRange>,
    tines: &[CombTineRange],
    length: f32,
    width: f32,
    thickness: f32,
    segment_count: usize,
    deflections: &[f32],
    active_tines: &[bool],
    collapse_inactive_tines: bool,
) -> Mesh {
    let segment_count = segment_count.max(1);
    let vertex_capacity = fixed_base
        .as_ref()
        .map(|base| base.vertex_range.end)
        .into_iter()
        .chain(tines.last().map(|tine| tine.vertex_range.end))
        .max()
        .unwrap_or(0);
    let index_capacity = fixed_base
        .as_ref()
        .map(|base| base.index_range.end)
        .into_iter()
        .chain(tines.last().map(|tine| tine.index_range.end))
        .max()
        .unwrap_or(0);
    let mut positions = Vec::<[f32; 3]>::with_capacity(vertex_capacity);
    let mut normals = Vec::<[f32; 3]>::with_capacity(vertex_capacity);
    let mut uvs = Vec::<[f32; 2]>::with_capacity(vertex_capacity);
    let mut indices = Vec::<u32>::with_capacity(index_capacity);

    if let Some(base) = fixed_base {
        append_comb_fixed_base(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut indices,
            base.center,
            base.width,
            base.length,
            base.thickness,
        );
    }

    for (index, tine) in tines.iter().enumerate() {
        let active = active_tines.get(index).copied().unwrap_or(true);
        let deflection = deflections.get(index).copied().unwrap_or(0.0);
        append_comb_tine(
            &mut positions,
            &mut normals,
            &mut uvs,
            &mut indices,
            tine.pivot,
            length,
            width,
            thickness,
            segment_count,
            deflection,
            collapse_inactive_tines && !active,
        );
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

#[allow(clippy::too_many_arguments)]
fn append_comb_tine(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    pivot: Vec3,
    length: f32,
    width: f32,
    thickness: f32,
    segment_count: usize,
    deflection: f32,
    collapsed: bool,
) {
    let half_width = width * 0.5;
    let half_thickness = thickness * 0.5;
    let mut rings = Vec::<[Vec3; 4]>::with_capacity(segment_count + 1);

    for ring in 0..=segment_count {
        let progress = ring as f32 / segment_count as f32;
        let distance = length * progress;
        let angle = deflection * progress.powf(1.15) * 0.55;
        let center = if collapsed {
            pivot
        } else {
            pivot + Quat::from_rotation_x(angle) * Vec3::new(0.0, -distance, 0.0)
        };
        let tangent_angle = deflection * progress.powf(1.35);
        let rotation = Quat::from_rotation_x(tangent_angle);

        let corners = if collapsed {
            [center, center, center, center]
        } else {
            [
                center + rotation * Vec3::new(-half_width, 0.0, -half_thickness),
                center + rotation * Vec3::new(half_width, 0.0, -half_thickness),
                center + rotation * Vec3::new(half_width, 0.0, half_thickness),
                center + rotation * Vec3::new(-half_width, 0.0, half_thickness),
            ]
        };
        rings.push(corners);
    }

    for ring in 0..segment_count {
        let progress = ring as f32 / segment_count as f32;
        let next_progress = (ring + 1) as f32 / segment_count as f32;
        for side in 0..4 {
            append_comb_quad(
                positions,
                normals,
                uvs,
                indices,
                [
                    rings[ring][side],
                    rings[ring][(side + 1) % 4],
                    rings[ring + 1][(side + 1) % 4],
                    rings[ring + 1][side],
                ],
                [
                    [progress, 0.0],
                    [progress, 1.0],
                    [next_progress, 1.0],
                    [next_progress, 0.0],
                ],
            );
        }
    }
    append_comb_quad(
        positions,
        normals,
        uvs,
        indices,
        [rings[0][0], rings[0][3], rings[0][2], rings[0][1]],
        [[0.0, 0.0], [0.0, 1.0], [1.0, 1.0], [1.0, 0.0]],
    );
    append_comb_quad(
        positions,
        normals,
        uvs,
        indices,
        [
            rings[segment_count][0],
            rings[segment_count][1],
            rings[segment_count][2],
            rings[segment_count][3],
        ],
        [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    );
}

fn append_comb_quad(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    corners: [Vec3; 4],
    quad_uvs: [[f32; 2]; 4],
) {
    let base_index = positions.len() as u32;
    let normal = (corners[1] - corners[0])
        .cross(corners[2] - corners[0])
        .normalize_or_zero();
    for (corner, uv) in corners.into_iter().zip(quad_uvs) {
        positions.push(corner.to_array());
        normals.push(normal.to_array());
        uvs.push(uv);
    }
    indices.extend_from_slice(&[
        base_index,
        base_index + 1,
        base_index + 2,
        base_index,
        base_index + 2,
        base_index + 3,
    ]);
}

fn append_comb_fixed_base(
    positions: &mut Vec<[f32; 3]>,
    normals: &mut Vec<[f32; 3]>,
    uvs: &mut Vec<[f32; 2]>,
    indices: &mut Vec<u32>,
    center: Vec3,
    width: f32,
    length: f32,
    thickness: f32,
) {
    let base_index = positions.len() as u32;
    let hx = width * 0.5;
    let hy = length * 0.5;
    let hz = thickness * 0.5;
    let corners = [
        Vec3::new(-hx, -hy, -hz),
        Vec3::new(hx, -hy, -hz),
        Vec3::new(hx, hy, -hz),
        Vec3::new(-hx, hy, -hz),
        Vec3::new(-hx, -hy, hz),
        Vec3::new(hx, -hy, hz),
        Vec3::new(hx, hy, hz),
        Vec3::new(-hx, hy, hz),
    ];
    for corner in corners {
        let position = center + corner;
        positions.push(position.to_array());
        normals.push(corner.normalize_or_zero().to_array());
        uvs.push([0.0, 0.0]);
    }
    for index in [
        0, 2, 1, 0, 3, 2, 4, 5, 6, 4, 6, 7, 0, 1, 5, 0, 5, 4, 1, 2, 6, 1, 6, 5, 2, 3, 7, 2, 7, 6,
        3, 0, 4, 3, 4, 7,
    ] {
        indices.push(base_index + index);
    }
}

pub fn hemisphere_mesh(radius: f32, sectors: u32, stacks: u32) -> Mesh {
    use std::f32::consts::{FRAC_PI_2, PI};

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

// ── timing validation ───────────────────────────────────────────

pub fn timing_validation(hint: &MechanismLayoutHint, ticks_per_turn: i64) -> TimingValidation {
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

// ── math helpers ────────────────────────────────────────────────

pub fn cylinder_radial_frame(axis: Vec3) -> (Vec3, Vec3) {
    let mut radial = Vec3::Y - axis * Vec3::Y.dot(axis);
    if radial.length_squared() < 1e-6 {
        radial = Vec3::X - axis * Vec3::X.dot(axis);
    }
    let radial = radial.normalize_or_zero();
    let tangent = axis.cross(radial).normalize_or_zero();
    (radial, tangent)
}

pub fn comb_tine_deflection_sign(axis: Vec3, radial: Vec3, tangent: Vec3) -> f32 {
    let rotation = basis_rotation(axis, radial, tangent);
    let positive_deflection_direction = rotation * -Vec3::Z;
    let tooth_travel_direction = tangent.normalize_or_zero() * CYLINDER_PLAYBACK_ROTATION_SIGN;
    if positive_deflection_direction.dot(tooth_travel_direction) >= 0.0 {
        1.0
    } else {
        -1.0
    }
}

pub fn basis_rotation(x_axis: Vec3, y_axis: Vec3, z_axis: Vec3) -> Quat {
    Quat::from_mat3(&Mat3::from_cols(
        x_axis.normalize_or_zero(),
        y_axis.normalize_or_zero(),
        z_axis.normalize_or_zero(),
    ))
}

// ── tests ───────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use airlet::mechanism::MechanismLayoutHint;

    pub(crate) fn dummy_mechanism_resource() -> MechanismResource {
        MechanismResource {
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
        }
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
    fn comb_tine_direction_follows_cylinder_travel_direction() {
        let axis = Vec3::X;
        let radial = Vec3::Y;
        let tangent = axis.cross(radial).normalize_or_zero();
        let sign = comb_tine_deflection_sign(axis, radial, tangent);
        let oriented_positive_deflection =
            basis_rotation(axis, radial, tangent) * (-Vec3::Z * sign);
        let tooth_travel_direction = tangent * CYLINDER_PLAYBACK_ROTATION_SIGN;

        assert_eq!(sign, 1.0);
        assert!(oriented_positive_deflection.dot(tooth_travel_direction) > 0.99);
    }

    #[test]
    fn comb_mesh_ranges_partition_each_tine_in_one_mesh() {
        let fixed_base = Some(CombFixedBaseRange {
            center: Vec3::Y,
            width: 4.0,
            length: 1.0,
            thickness: 0.2,
            vertex_range: 0..0,
            index_range: 0..0,
        });
        let pivots = vec![
            (60, Vec3::new(-1.0, 2.0, 0.0)),
            (61, Vec3::new(0.0, 2.0, 0.0)),
            (62, Vec3::new(1.0, 2.0, 0.0)),
        ];
        let (fixed_base, tines) = comb_mesh_ranges(fixed_base, &pivots, 5);
        let fixed_base = fixed_base.unwrap();

        assert_eq!(fixed_base.vertex_range, 0..8);
        assert_eq!(fixed_base.index_range, 0..36);
        assert_eq!(tines.len(), 3);
        assert_eq!(tines[0].midi_note, 60);
        assert_eq!(tines[0].vertex_range, 8..96);
        assert_eq!(tines[0].index_range, 36..168);
        assert_eq!(tines[1].vertex_range, 96..184);
        assert_eq!(tines[1].index_range, 168..300);
        assert_eq!(tines[2].vertex_range, 184..272);
        assert_eq!(tines[2].index_range, 300..432);
    }

    #[test]
    fn comb_ghost_mesh_keeps_ranges_but_collapses_inactive_tines() {
        let pivots = vec![(60, Vec3::new(0.0, 2.0, 0.0))];
        let (_, tines) = comb_mesh_ranges(None, &pivots, 2);
        let mesh = comb_mesh(None, &tines, 1.0, 0.2, 0.1, 2, &[0.15], &[false], true);
        let positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();

        assert_eq!(tines[0].vertex_range, 0..40);
        assert!(
            positions
                .iter()
                .all(|position| *position == [0.0, 2.0, 0.0])
        );
    }

    #[test]
    fn comb_tine_mesh_uses_per_face_normals() {
        let pivots = vec![(60, Vec3::new(0.0, 2.0, 0.0))];
        let (_, tines) = comb_mesh_ranges(None, &pivots, 1);
        let mesh = comb_mesh(None, &tines, 1.0, 0.2, 0.1, 1, &[0.0], &[true], false);
        let normals = mesh
            .attribute(Mesh::ATTRIBUTE_NORMAL)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();
        let mut unique_normals = Vec::<[f32; 3]>::new();
        for normal in normals {
            if !unique_normals.iter().any(|existing| {
                existing
                    .iter()
                    .zip(normal)
                    .all(|(left, right)| (*left - *right).abs() < 1.0e-5)
            }) {
                unique_normals.push(*normal);
            }
        }

        assert_eq!(tines[0].vertex_range, 0..24);
        assert!(unique_normals.len() >= 6);
    }
}
