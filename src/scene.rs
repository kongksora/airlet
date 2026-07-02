use std::f32::consts::FRAC_PI_2;

use airlet_model::MovableMusicBoxModel;
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{Hdr, PerspectiveProjection, Projection, visibility::NoFrustumCulling},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::{
        CascadeShadowConfigBuilder, EnvironmentMapLight,
        cluster::{ClusterConfig, ClusterFarZMode, ClusterZConfig},
    },
    picking::prelude::MeshPickingCamera,
    prelude::*,
};

use crate::controls::ExhibitControls;
use crate::lighting::{ExhibitLightingConfig, TextureMaterialClass, apply_texture_class};
use crate::model_view::ModelResource;

pub const EXHIBIT_TARGET: Vec3 = Vec3::new(0.0, 0.032, 0.0);
pub const PLATFORM_TOP_Y: f32 = 0.0;
pub const CAMERA_MIN_RADIUS: f32 = 0.20;
pub const CAMERA_MAX_RADIUS: f32 = 1.2;
pub const CAMERA_MIN_PITCH: f32 = 0.0;
pub const CAMERA_MAX_PITCH: f32 = 1.25;
pub const CAMERA_NEAR: f32 = 0.005;
pub const CAMERA_FAR: f32 = 2.0;
pub const CAMERA_CLUSTER_FIRST_SLICE_DEPTH: f32 = 0.08;
pub const MODEL_SCALE: f32 = 0.2095999;

#[derive(Component)]
pub struct ExhibitCamera;

#[derive(Component)]
pub struct ExhibitSpotlight;

#[derive(Component)]
pub struct ExhibitSpotlightFallback;

#[derive(Component)]
pub struct LightingKey;

#[derive(Component)]
pub struct LightingFill;

#[derive(Component)]
pub struct LightingRim;

#[derive(Component)]
pub struct LightingAccent;

#[derive(Component)]
pub struct LidPivot;

#[derive(Component)]
pub struct CylinderPivot;

#[derive(Component)]
pub struct WindingKeyPivot;

#[derive(Component)]
pub struct ProceduralMechanism;

pub fn setup_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    controls: Res<ExhibitControls>,
    lighting: Res<ExhibitLightingConfig>,
    model: Res<ModelResource>,
) {
    commands.init_resource::<crate::model_view::ModelSpawnState>();
    let gltf_path = model
        .model
        .spec()
        .asset
        .baked_gltf
        .clone()
        .unwrap_or_else(|| model.model.spec().asset.gltf.clone());
    commands.insert_resource(crate::model_view::ModelGltfHandle(
        asset_server.load(gltf_path),
    ));

    commands.spawn((
        Name::new("Exhibit Platform"),
        Mesh3d(
            meshes.add(
                Cylinder::new(lighting.platform_radius, lighting.platform_height)
                    .mesh()
                    .resolution(128),
            ),
        ),
        MeshMaterial3d(materials.add({
            let mut material = lighting.platform_material.material();
            apply_texture_class(
                &mut material,
                &asset_server,
                TextureMaterialClass::DarkStage,
            );
            material
        })),
        Transform::from_xyz(0.0, -lighting.platform_height * 0.5, 0.0),
    ));

    commands.spawn((
        Name::new("Lighting Key"),
        LightingKey,
        DirectionalLight {
            illuminance: ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE
                .clamp(controls.key_illuminance),
            ..lighting.key.light()
        },
        CascadeShadowConfigBuilder {
            first_cascade_far_bound: lighting.key.first_cascade_far_bound,
            maximum_distance: lighting.key.maximum_distance,
            ..default()
        }
        .build(),
        Transform::from_translation(lighting.key.position).looking_at(EXHIBIT_TARGET, Vec3::Y),
    ));

    commands.spawn((
        Name::new("Lighting Fill"),
        LightingFill,
        PointLight {
            intensity: ExhibitLightingConfig::FILL_INTENSITY_RANGE.clamp(controls.fill_intensity),
            ..lighting.fill.light()
        },
        Transform::from_translation(lighting.fill.position),
    ));

    commands.spawn((
        Name::new("Lighting Rim"),
        LightingRim,
        PointLight {
            intensity: ExhibitLightingConfig::RIM_INTENSITY_RANGE.clamp(controls.rim_intensity),
            ..lighting.rim.light()
        },
        Transform::from_translation(lighting.rim.position),
    ));

    commands.spawn((
        Name::new("Lighting Accent"),
        LightingAccent,
        PointLight {
            intensity: ExhibitLightingConfig::ACCENT_INTENSITY_RANGE
                .clamp(controls.accent_intensity),
            ..lighting.accent.light()
        },
        Transform::from_translation(lighting.accent.position),
    ));

    commands.spawn((
        Name::new("Lighting Spot"),
        ExhibitSpotlight,
        lighting.spot.light(
            controls.spot_inner_angle,
            controls.spot_outer_angle,
            controls.spot_intensity,
        ),
        NoFrustumCulling,
        spotlight_transform(&controls),
    ));

    commands.spawn((
        Name::new("Lighting Spot Fallback"),
        ExhibitSpotlightFallback,
        spotlight_fallback_light(&controls, &lighting),
        NoFrustumCulling,
        spotlight_fallback_transform(&controls),
    ));

    commands
        .spawn((
            Name::new("Exhibit Camera"),
            ExhibitCamera,
            MeshPickingCamera,
            Camera3d::default(),
            Projection::Perspective(PerspectiveProjection {
                near: CAMERA_NEAR,
                far: CAMERA_FAR,
                near_clip_plane: Vec4::new(0.0, 0.0, -1.0, -CAMERA_NEAR),
                ..default()
            }),
            Hdr,
            Msaa::Off,
            TemporalAntiAliasing::default(),
            lighting.exposure,
            lighting.tonemapping,
            lighting.bloom.clone(),
            lighting.screen_space_ao.clone(),
            lighting.contact_shadows.clone(),
            EnvironmentMapLight {
                intensity: ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE
                    .clamp(controls.environment_intensity),
                ..EnvironmentMapLight::solid_color(&mut images, lighting.environment_color)
            },
            camera_transform(&controls),
        ))
        .insert(ClusterConfig::FixedZ {
            total: 4096,
            z_slices: 24,
            z_config: ClusterZConfig {
                first_slice_depth: CAMERA_CLUSTER_FIRST_SLICE_DEPTH,
                far_z_mode: ClusterFarZMode::Constant(CAMERA_FAR),
            },
            dynamic_resizing: true,
        });
}

pub fn orbit_camera(
    mut controls: ResMut<ExhibitControls>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if mouse_buttons.pressed(MouseButton::Right) {
        controls.yaw -= mouse_motion.delta.x * 0.006;
        controls.pitch = (controls.pitch + mouse_motion.delta.y * 0.004)
            .clamp(CAMERA_MIN_PITCH, CAMERA_MAX_PITCH);
    }

    if mouse_scroll.delta.y != 0.0 {
        controls.radius = (controls.radius - mouse_scroll.delta.y * 0.35)
            .clamp(CAMERA_MIN_RADIUS, CAMERA_MAX_RADIUS);
    }
}

pub fn apply_camera_transform(
    controls: Res<ExhibitControls>,
    mut camera: Query<&mut Transform, With<ExhibitCamera>>,
) {
    if !controls.is_changed() {
        return;
    }

    for mut transform in &mut camera {
        *transform = camera_transform(&controls);
    }
}

pub fn apply_lighting_controls(
    controls: Res<ExhibitControls>,
    lighting: Res<ExhibitLightingConfig>,
    mut ambient: ResMut<GlobalAmbientLight>,
    mut camera_environment: Query<&mut EnvironmentMapLight, With<ExhibitCamera>>,
    mut key_lights: Query<&mut DirectionalLight, With<LightingKey>>,
    mut fill_lights: Query<
        &mut PointLight,
        (
            With<LightingFill>,
            Without<LightingRim>,
            Without<LightingAccent>,
        ),
    >,
    mut rim_lights: Query<
        &mut PointLight,
        (
            With<LightingRim>,
            Without<LightingFill>,
            Without<LightingAccent>,
        ),
    >,
    mut accent_lights: Query<
        &mut PointLight,
        (
            With<LightingAccent>,
            Without<LightingFill>,
            Without<LightingRim>,
        ),
    >,
    mut lights: Query<
        (&mut SpotLight, &mut Transform),
        (With<ExhibitSpotlight>, Without<ExhibitSpotlightFallback>),
    >,
    mut spot_fallbacks: Query<
        (&mut PointLight, &mut Transform),
        (
            With<ExhibitSpotlightFallback>,
            Without<ExhibitSpotlight>,
            Without<LightingFill>,
            Without<LightingRim>,
            Without<LightingAccent>,
        ),
    >,
) {
    if !controls.is_changed() {
        return;
    }

    ambient.brightness =
        ExhibitLightingConfig::AMBIENT_BRIGHTNESS_RANGE.clamp(controls.ambient_brightness);

    for mut environment in &mut camera_environment {
        environment.intensity = ExhibitLightingConfig::ENVIRONMENT_INTENSITY_RANGE
            .clamp(controls.environment_intensity);
    }

    for mut light in &mut key_lights {
        light.illuminance =
            ExhibitLightingConfig::KEY_ILLUMINANCE_RANGE.clamp(controls.key_illuminance);
    }

    for mut light in &mut fill_lights {
        light.intensity =
            ExhibitLightingConfig::FILL_INTENSITY_RANGE.clamp(controls.fill_intensity);
    }

    for mut light in &mut rim_lights {
        light.intensity = ExhibitLightingConfig::RIM_INTENSITY_RANGE.clamp(controls.rim_intensity);
    }

    for mut light in &mut accent_lights {
        light.intensity =
            ExhibitLightingConfig::ACCENT_INTENSITY_RANGE.clamp(controls.accent_intensity);
    }

    for (mut light, mut transform) in &mut lights {
        light.intensity = controls
            .spot_intensity
            .clamp(lighting.spot.min_intensity, lighting.spot.max_intensity);
        light.inner_angle = lighting
            .spot
            .clamp_inner_angle(controls.spot_inner_angle, controls.spot_outer_angle);
        light.outer_angle = lighting.spot.clamp_outer_angle(controls.spot_outer_angle);
        *transform = spotlight_transform(&controls);
    }

    for (mut light, mut transform) in &mut spot_fallbacks {
        *light = spotlight_fallback_light(&controls, &lighting);
        *transform = spotlight_fallback_transform(&controls);
    }
}

pub fn camera_transform(controls: &ExhibitControls) -> Transform {
    let horizontal = controls.radius * controls.pitch.cos();
    let position = Vec3::new(
        horizontal * controls.yaw.sin(),
        controls.radius * controls.pitch.sin(),
        horizontal * controls.yaw.cos(),
    ) + controls.camera_target;
    Transform::from_translation(position).looking_at(controls.camera_target, Vec3::Y)
}

pub fn model_transform(model: &MovableMusicBoxModel) -> Transform {
    let placement = model.root_placement(EXHIBIT_TARGET.to_array(), PLATFORM_TOP_Y, MODEL_SCALE);
    Transform::from_translation(crate::vec3(placement.translation))
        .with_rotation(Quat::from_array(placement.rotation))
        .with_scale(Vec3::splat(placement.scale))
}

pub fn spotlight_transform(controls: &ExhibitControls) -> Transform {
    let pitch = controls.light_pitch.clamp(0.1, FRAC_PI_2 - 0.02);
    let horizontal = controls.light_distance * pitch.cos();
    let position = Vec3::new(
        horizontal * controls.light_yaw.sin(),
        controls.light_distance * pitch.sin(),
        horizontal * controls.light_yaw.cos(),
    ) + EXHIBIT_TARGET;
    Transform::from_translation(position).looking_at(EXHIBIT_TARGET, Vec3::Y)
}

fn spotlight_fallback_light(
    controls: &ExhibitControls,
    lighting: &ExhibitLightingConfig,
) -> PointLight {
    let outer_angle = lighting.spot.clamp_outer_angle(controls.spot_outer_angle);
    let footprint = controls.light_distance * outer_angle.tan().max(0.0);
    let range = (0.42 + footprint * 0.65).clamp(0.36, lighting.spot.range);
    let intensity = controls
        .spot_intensity
        .clamp(lighting.spot.min_intensity, lighting.spot.max_intensity)
        * 0.0055;
    PointLight {
        color: lighting.spot.color,
        intensity,
        range,
        radius: lighting.spot.radius,
        shadow_maps_enabled: false,
        contact_shadows_enabled: false,
        shadow_depth_bias: lighting.spot.shadow_depth_bias,
        shadow_normal_bias: lighting.spot.shadow_normal_bias,
        shadow_map_near_z: lighting.spot.shadow_map_near_z,
        ..default()
    }
}

fn spotlight_fallback_transform(controls: &ExhibitControls) -> Transform {
    let spot = spotlight_transform(controls).translation;
    let target = EXHIBIT_TARGET + Vec3::new(0.0, 0.035, 0.0);
    let position = target.lerp(spot, 0.22);
    Transform::from_translation(position)
}
