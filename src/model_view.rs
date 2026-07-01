use airlet::{defaults, score::Timeline};
use airlet_model::{MeshGroup, ModelSpec, MovableMusicBoxModel, PivotPose};
use bevy::{
    gltf::{Gltf, GltfMaterial, GltfMesh, GltfNode},
    picking::{hover::Hovered, prelude::Pickable},
    prelude::*,
};

use crate::comb_animation::derive_comb_animation_events;
use crate::lighting::{ExhibitLightingConfig, TextureMaterialClass, apply_texture_class};
use crate::mechanism_view::{MechanismResource, spawn_hint_mechanism};
use crate::scene::{CylinderPivot, LidPivot, WindingKeyPivot};
use crate::winding::WindingKeyPart;

pub const DEFAULT_MODEL_SPEC: &str = "assets/models/converted/spec.toml";
pub const MECHANICAL_TAIL_REST_TICKS: i64 = airlet::score::PPQ * 4;

// ── resources ───────────────────────────────────────────────────

#[derive(Resource)]
pub struct ModelGltfHandle(pub Handle<Gltf>);

#[derive(Resource)]
pub struct ModelResource {
    pub model: MovableMusicBoxModel,
}

#[derive(Resource, Default)]
pub struct ModelSpawnState {
    pub spawned: bool,
    pub logged: bool,
}

pub struct PendingPrimitive {
    pub group: MeshGroup,
    pub name: String,
    pub transform: Transform,
    pub mesh: Handle<Mesh>,
    pub material: Option<Handle<GltfMaterial>>,
}

// ── model loading ───────────────────────────────────────────────

pub fn load_movable_model() -> ModelResource {
    let spec = ModelSpec::from_toml_path(DEFAULT_MODEL_SPEC)
        .unwrap_or_else(|err| panic!("failed to load default music-box model spec: {err}"));
    ModelResource {
        model: MovableMusicBoxModel::new(spec),
    }
}

pub fn load_mechanism_layout() -> MechanismResource {
    use airlet::mechanism::MechanismPlanner;
    use airlet::score::PPQ;

    let plan = defaults::air_intro_plan();
    let timeline = plan.composed_score().expand();
    let ticks_per_turn = (timeline_end_tick(&timeline) + MECHANICAL_TAIL_REST_TICKS).max(1);
    let note_range = timeline
        .events
        .iter()
        .filter(|event| event.midi_note > 0)
        .map(|event| event.midi_note)
        .fold(None, |range: Option<(i32, i32)>, midi| match range {
            Some((lowest, highest)) => Some((lowest.min(midi), highest.max(midi))),
            None => Some((midi, midi)),
        });
    let (lowest_midi, highest_midi) = note_range.unwrap_or((60, 60));
    let track_count = highest_midi - lowest_midi + 1;
    let mut planner = MechanismPlanner::default();
    planner.lowest_midi = lowest_midi.min(highest_midi);
    planner.highest_midi = highest_midi.max(lowest_midi);
    planner.track_spacing = 1.0;
    planner.cylinder_length = track_count.max(1) as f32;
    planner.ticks_per_turn = ticks_per_turn;
    let hint = planner.plan(&timeline);
    let comb_animation_events = derive_comb_animation_events(&hint, ticks_per_turn);
    MechanismResource {
        hint,
        comb_animation_events,
        ticks_per_turn,
        quarter_millis: timeline.tempo.ticks_to_millis(PPQ),
    }
}

pub fn timeline_end_tick(timeline: &Timeline) -> i64 {
    timeline
        .events
        .iter()
        .filter(|event| event.midi_note > 0)
        .map(|event| event.onset.0 + event.duration.ticks())
        .max()
        .unwrap_or(airlet::score::PPQ * 4)
}

// ── model spawning ──────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn spawn_spec_model(
    mut commands: Commands,
    handle: Option<Res<ModelGltfHandle>>,
    model: Res<ModelResource>,
    mechanism: Res<MechanismResource>,
    gltfs: Res<Assets<Gltf>>,
    nodes: Res<Assets<GltfNode>>,
    meshes: Res<Assets<GltfMesh>>,
    gltf_materials: Res<Assets<GltfMaterial>>,
    mut render_meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    lighting: Res<ExhibitLightingConfig>,
    mut state: ResMut<ModelSpawnState>,
) {
    if state.spawned {
        return;
    }
    let Some(handle) = handle else {
        return;
    };
    let Some(gltf) = gltfs.get(&handle.0) else {
        return;
    };
    if gltf.nodes.iter().any(|node| nodes.get(node).is_none())
        || gltf.meshes.iter().any(|mesh| meshes.get(mesh).is_none())
        || gltf
            .materials
            .iter()
            .any(|material| gltf_materials.get(material).is_none())
    {
        return;
    }

    let closed = model.model.closed_meshes();
    let mut pending = Vec::new();
    for node_handle in &gltf.nodes {
        let Some(node) = nodes.get(node_handle) else {
            continue;
        };
        let Some(mesh_handle) = &node.mesh else {
            continue;
        };
        let Some(mesh) = meshes.get(mesh_handle) else {
            continue;
        };
        if !closed.contains(&mesh.index) {
            continue;
        }
        let group = model.model.group_for_mesh(mesh.index);
        for primitive in &mesh.primitives {
            pending.push(PendingPrimitive {
                group,
                name: format!("{} primitive {}", node.name, primitive.index),
                transform: node.transform,
                mesh: primitive.mesh.clone(),
                material: primitive.material.clone(),
            });
        }
    }

    let root = commands
        .spawn((
            Name::new("Music Box Closed Model"),
            crate::scene::model_transform(&model.model),
            Visibility::Visible,
        ))
        .id();
    let lid_pose = model.model.lid_pose();
    let lid_pivot = commands
        .spawn((
            Name::new("Music Box Lid Pivot"),
            LidPivot,
            Transform::from_translation(crate::vec3(lid_pose.pivot)),
            Visibility::Visible,
        ))
        .id();
    let cylinder_pose = model.model.cylinder_pose();
    let cylinder_pivot = commands
        .spawn((
            Name::new("Music Box Cylinder Pivot"),
            CylinderPivot,
            Transform::from_translation(crate::vec3(cylinder_pose.pivot)),
            Visibility::Visible,
        ))
        .id();
    commands.entity(root).add_child(lid_pivot);
    commands.entity(root).add_child(cylinder_pivot);
    let winding_key_pivot = model.model.winding_key_pose().map(|pose| {
        let pivot = commands
            .spawn((
                Name::new("Music Box Winding Key Pivot"),
                WindingKeyPivot,
                Transform::from_translation(crate::vec3(pose.pivot)),
                Visibility::Visible,
            ))
            .id();
        commands.entity(root).add_child(pivot);
        pivot
    });

    for primitive in pending {
        let parent = match primitive.group {
            MeshGroup::Static => root,
            MeshGroup::Lid => lid_pivot,
            MeshGroup::WindingKey => winding_key_pivot.unwrap_or(root),
            MeshGroup::Cylinder | MeshGroup::Comb => continue,
            MeshGroup::Excluded => continue,
        };
        let mut transform = primitive.transform;
        transform.translation = crate::vec3(
            model
                .model
                .relative_translation(transform.translation.to_array(), primitive.group),
        );
        let mut entity =
            commands.spawn((Name::new(primitive.name), Mesh3d(primitive.mesh), transform));
        if let Some(material) = &primitive.material {
            let Some(gltf_material) = gltf_materials.get(material) else {
                continue;
            };
            let normal = tuned_model_material(gltf_material, primitive.group, &lighting);
            let normal_material = materials.add(normal);
            entity.insert(MeshMaterial3d(normal_material.clone()));
            if primitive.group == MeshGroup::WindingKey {
                let mut hover = lighting.winding_key_hover.material();
                apply_texture_class(&mut hover, &asset_server, TextureMaterialClass::AgedBrass);
                let hover_material = materials.add(hover);
                entity.insert((
                    WindingKeyPart {
                        normal_material,
                        hover_material,
                    },
                    Pickable::default(),
                    Hovered(false),
                ));
            }
        }
        let child = entity.id();
        commands.entity(parent).add_child(child);
    }

    spawn_hint_mechanism(
        &mut commands,
        &mut render_meshes,
        &mut materials,
        &asset_server,
        &lighting,
        root,
        cylinder_pivot,
        cylinder_pose,
        &model.model,
        &mechanism.hint,
    );

    state.spawned = true;
    info!(
        "spawned closed music box rig: meshes={}, lid_meshes={}, cylinder_meshes={}, teeth={}",
        model.model.spec().closed_model.mesh_indices.len(),
        model.model.spec().lid.meshes.len(),
        model.model.spec().cylinder.meshes.len(),
        mechanism.hint.events.len()
    );
}

// ── rig controls ────────────────────────────────────────────────

pub fn apply_rig_controls(
    mut controls: ResMut<crate::controls::ExhibitControls>,
    mut model: ResMut<ModelResource>,
    twin: Res<crate::twin::MusicBoxTwinState>,
    mut lid_query: Query<&mut Transform, (With<LidPivot>, Without<CylinderPivot>)>,
    mut cylinder_query: Query<&mut Transform, (With<CylinderPivot>, Without<LidPivot>)>,
) {
    model.model.set_lid_t(controls.lid_t);
    if twin.is_mechanically_active() {
        controls.cylinder_degrees = twin.cylinder_degrees;
        model.model.set_cylinder_degrees(controls.cylinder_degrees);
    } else {
        model.model.set_cylinder_degrees(controls.cylinder_degrees);
    }
    controls.cylinder_degrees = model.model.state().cylinder_degrees;

    let lid_pose = model.model.lid_pose();
    for mut transform in &mut lid_query {
        transform.rotation = pose_rotation(lid_pose);
    }

    let cylinder_pose = model.model.cylinder_pose();
    for mut transform in &mut cylinder_query {
        transform.rotation = pose_rotation(cylinder_pose);
    }
}

// ── mesh count reporting ────────────────────────────────────────

pub fn report_model_load(mut state: ResMut<ModelSpawnState>, meshes: Query<&Mesh3d>) {
    if state.logged {
        return;
    }
    let mesh_count = meshes.iter().count();
    if mesh_count <= 2 {
        return;
    }
    info!("music box scene spawned; mesh component count: {mesh_count}");
    state.spawned = true;
    state.logged = true;
}

// ── conversion helpers ──────────────────────────────────────────

fn tuned_model_material(
    gltf_material: &GltfMaterial,
    group: MeshGroup,
    lighting: &ExhibitLightingConfig,
) -> StandardMaterial {
    let mut material = to_standard_material(gltf_material);
    if let Some(tuning) = lighting.model_tuning(group) {
        tuning.apply(&mut material);
    }
    material
}

fn to_standard_material(material: &GltfMaterial) -> StandardMaterial {
    StandardMaterial {
        base_color: material.base_color,
        base_color_channel: material.base_color_channel.clone(),
        base_color_texture: material.base_color_texture.clone(),
        emissive: material.emissive,
        emissive_channel: material.emissive_channel.clone(),
        emissive_texture: material.emissive_texture.clone(),
        perceptual_roughness: material.perceptual_roughness,
        metallic: material.metallic,
        metallic_roughness_channel: material.metallic_roughness_channel.clone(),
        metallic_roughness_texture: material.metallic_roughness_texture.clone(),
        reflectance: material.reflectance,
        normal_map_channel: material.normal_map_channel.clone(),
        normal_map_texture: material.normal_map_texture.clone(),
        occlusion_channel: material.occlusion_channel.clone(),
        occlusion_texture: material.occlusion_texture.clone(),
        alpha_mode: material.alpha_mode,
        cull_mode: material.cull_mode,
        double_sided: material.double_sided,
        ..default()
    }
}

pub fn pose_rotation(pose: PivotPose) -> Quat {
    Quat::from_axis_angle(crate::vec3(pose.axis), pose.angle_degrees.to_radians())
}

// ── tests ───────────────────────────────────────────────────────

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::mechanism_view::{MechanismResource, mechanism_calibration, timing_validation};

    pub(crate) fn load_test_mechanism() -> MechanismResource {
        load_mechanism_layout()
    }

    #[test]
    fn default_mechanism_uses_full_song_turn_without_phase_collisions() {
        let mechanism = load_mechanism_layout();
        let timing = timing_validation(&mechanism.hint, mechanism.ticks_per_turn);

        assert_eq!(
            timeline_end_tick(&defaults::air_intro_plan().composed_score().expand()),
            29_760
        );
        assert_eq!(mechanism.ticks_per_turn, 33_600);
        assert_eq!(timing.same_onset_group_count, 0);
        assert_eq!(timing.same_phase_group_count, 0);
        assert!(
            mechanism
                .hint
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("outside comb range"))
        );
        assert!(
            mechanism
                .hint
                .diagnostics
                .iter()
                .all(|diagnostic| !diagnostic.message.contains("dense teeth near angle"))
        );
    }

    #[test]
    fn default_model_spec_exposes_measured_comb_clearance() {
        let model = load_movable_model();
        let spec = model.model.spec();
        let calibration = mechanism_calibration(
            &load_mechanism_layout().hint,
            Some(&model.model),
            spec.cylinder.length,
        );

        assert!(spec.cylinder.radius > 0.0);
        assert!(spec.comb.tip_radius > spec.cylinder.radius);
        assert!(spec.comb.clearance > 0.0);
        assert_eq!(spec.comb.meshes, vec![23]);
        assert_eq!(
            spec.lid.meshes,
            vec![0, 1, 2, 4, 5, 6, 7, 10, 12, 13, 14, 16, 17]
        );
        assert_eq!(spec.asset.gltf, "generated/music_box_aligned_base.glb");
        assert_eq!(spec.basis.right, [1.0, 0.0, 0.0]);
        assert_eq!(spec.basis.up, [0.0, 1.0, 0.0]);
        assert_eq!(spec.basis.front, [0.0, 0.0, -1.0]);
        assert!(spec.closed_model.hinge_meshes.is_empty());
        assert_eq!(spec.lid.pivot, [-0.0666563, -0.938679, -1.58481]);
        assert_eq!(spec.lid.axis, [1.0, 0.0, 0.0]);
        assert_eq!(spec.lid.open_degrees, -110.0);
        assert_eq!(spec.cylinder.axis, [0.0, 0.0, 1.0]);
        assert_eq!(
            spec.winding_key.as_ref().unwrap().pivot,
            [-0.0499405, -0.97183, -1.48757]
        );
        assert_eq!(
            spec.winding_key.as_ref().unwrap().axis,
            [1.0, 0.0, -4.44078e-6]
        );
        assert!((calibration.axial_min - spec.comb.axial_min).abs() < 1e-6);
        assert!((calibration.axial_max - spec.comb.axial_max).abs() < 1e-6);
    }
}
