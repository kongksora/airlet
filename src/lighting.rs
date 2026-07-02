use airlet_model::MeshGroup;
use bevy::{
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    light::{DirectionalLightShadowMap, PointLightShadowMap},
    pbr::{ContactShadows, ScreenSpaceAmbientOcclusion, ScreenSpaceAmbientOcclusionQualityLevel},
    post_process::bloom::Bloom,
    prelude::*,
};

#[derive(Resource, Clone)]
pub struct ExhibitLightingConfig {
    pub ambient_color: Color,
    pub ambient_brightness: f32,
    pub directional_shadow_map_size: usize,
    pub point_shadow_map_size: usize,
    pub contact_shadows: ContactShadows,
    pub exposure: Exposure,
    pub tonemapping: Tonemapping,
    pub bloom: Bloom,
    pub screen_space_ao: ScreenSpaceAmbientOcclusion,
    pub environment_color: Color,
    pub environment_intensity: f32,
    pub platform_radius: f32,
    pub platform_height: f32,
    pub platform_material: MaterialRecipe,
    pub key: DirectionalLightRecipe,
    pub fill: PointLightRecipe,
    pub rim: PointLightRecipe,
    pub accent: PointLightRecipe,
    pub spot: SpotLightRecipe,
    pub model_static: MaterialTuning,
    pub model_lid: MaterialTuning,
    pub winding_key: MaterialTuning,
    pub winding_key_hover: MaterialRecipe,
    pub cylinder: MaterialRecipe,
    pub tooth: MaterialRecipe,
    pub comb: MaterialRecipe,
    pub comb_ghost: MaterialRecipe,
}

#[derive(Debug, Clone, Copy)]
pub struct IntensityRange {
    pub min: f32,
    pub max: f32,
}

impl IntensityRange {
    pub const fn new(min: f32, max: f32) -> Self {
        Self { min, max }
    }

    pub fn clamp(&self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

impl Default for ExhibitLightingConfig {
    fn default() -> Self {
        Self {
            ambient_color: Color::srgb(0.78, 0.75, 0.70),
            ambient_brightness: 0.08,
            directional_shadow_map_size: 4096,
            point_shadow_map_size: 4096,
            contact_shadows: ContactShadows {
                linear_steps: 48,
                thickness: 0.006,
                length: 0.12,
            },
            exposure: Exposure::INDOOR,
            tonemapping: Tonemapping::TonyMcMapface,
            bloom: Bloom {
                intensity: 0.08,
                low_frequency_boost: 0.18,
                high_pass_frequency: 0.92,
                ..Bloom::NATURAL
            },
            screen_space_ao: ScreenSpaceAmbientOcclusion {
                quality_level: ScreenSpaceAmbientOcclusionQualityLevel::High,
                constant_object_thickness: 0.012,
            },
            environment_color: Color::srgb(0.78, 0.80, 0.86),
            environment_intensity: 0.35,
            platform_radius: 0.28,
            platform_height: 0.025,
            platform_material: MaterialRecipe {
                base_color: Color::srgb(0.24, 0.23, 0.21),
                metallic: 0.0,
                perceptual_roughness: 0.82,
                reflectance: 0.18,
                alpha_mode: AlphaMode::Opaque,
                emissive: LinearRgba::BLACK,
            },
            key: DirectionalLightRecipe {
                color: Color::srgb(1.0, 0.94, 0.84),
                illuminance: 45.0,
                position: Vec3::new(-0.26, 0.42, 0.24),
                first_cascade_far_bound: 0.22,
                maximum_distance: 1.2,
                shadow_depth_bias: 0.004,
                shadow_normal_bias: 0.08,
                contact_shadows_enabled: true,
            },
            fill: PointLightRecipe {
                color: Color::srgb(0.72, 0.82, 1.0),
                intensity: 0.0,
                range: 0.95,
                radius: 0.12,
                position: Vec3::new(-0.34, 0.30, 0.36),
                shadow_maps_enabled: false,
                contact_shadows_enabled: false,
                shadow_depth_bias: 0.02,
                shadow_normal_bias: 0.08,
                shadow_map_near_z: 0.01,
            },
            rim: PointLightRecipe {
                color: Color::srgb(0.80, 0.90, 1.0),
                intensity: 5.0,
                range: 0.8,
                radius: 0.05,
                position: Vec3::new(0.28, 0.25, -0.36),
                shadow_maps_enabled: false,
                contact_shadows_enabled: false,
                shadow_depth_bias: 0.02,
                shadow_normal_bias: 0.08,
                shadow_map_near_z: 0.01,
            },
            accent: PointLightRecipe {
                color: Color::srgb(1.0, 0.76, 0.38),
                intensity: 6.0,
                range: 0.55,
                radius: 0.035,
                position: Vec3::new(0.14, 0.12, 0.16),
                shadow_maps_enabled: false,
                contact_shadows_enabled: false,
                shadow_depth_bias: 0.02,
                shadow_normal_bias: 0.08,
                shadow_map_near_z: 0.01,
            },
            spot: SpotLightRecipe {
                color: Color::srgb(1.0, 0.90, 0.72),
                intensity: 18_000.0,
                min_intensity: 100.0,
                max_intensity: 60_000.0,
                range: 0.9,
                radius: 0.01,
                default_inner_angle: 0.20,
                default_outer_angle: 0.30,
                min_inner_angle: 0.03,
                min_outer_angle: 0.08,
                max_outer_angle: 1.35,
                shadow_maps_enabled: true,
                contact_shadows_enabled: true,
                shadow_depth_bias: 0.004,
                shadow_normal_bias: 0.08,
                shadow_map_near_z: 0.01,
            },
            model_static: MaterialTuning {
                metallic: None,
                max_metallic: Some(0.9),
                perceptual_roughness: Some(0.48),
                min_roughness: Some(0.32),
                reflectance: Some(0.58),
                base_color_tint: Color::srgba(1.0, 0.97, 0.91, 1.0),
            },
            model_lid: MaterialTuning {
                metallic: None,
                max_metallic: Some(0.86),
                perceptual_roughness: Some(0.42),
                min_roughness: Some(0.28),
                reflectance: Some(0.62),
                base_color_tint: Color::srgba(1.0, 0.98, 0.94, 1.0),
            },
            winding_key: MaterialTuning {
                metallic: Some(0.88),
                max_metallic: None,
                perceptual_roughness: Some(0.22),
                min_roughness: None,
                reflectance: Some(0.82),
                base_color_tint: Color::srgba(1.0, 0.84, 0.38, 1.0),
            },
            winding_key_hover: MaterialRecipe {
                base_color: Color::srgb(1.0, 0.78, 0.20),
                metallic: 0.88,
                perceptual_roughness: 0.18,
                reflectance: 0.88,
                alpha_mode: AlphaMode::Opaque,
                emissive: LinearRgba::rgb(0.42, 0.24, 0.035),
            },
            cylinder: MaterialRecipe {
                base_color: Color::srgb(0.96, 0.66, 0.22),
                metallic: 0.92,
                perceptual_roughness: 0.20,
                reflectance: 0.82,
                alpha_mode: AlphaMode::Opaque,
                emissive: LinearRgba::BLACK,
            },
            tooth: MaterialRecipe {
                base_color: Color::srgb(1.0, 0.76, 0.24),
                metallic: 0.94,
                perceptual_roughness: 0.16,
                reflectance: 0.86,
                alpha_mode: AlphaMode::Opaque,
                emissive: LinearRgba::BLACK,
            },
            comb: MaterialRecipe {
                base_color: Color::srgb(0.88, 0.90, 0.88),
                metallic: 0.95,
                perceptual_roughness: 0.12,
                reflectance: 0.92,
                alpha_mode: AlphaMode::Opaque,
                emissive: LinearRgba::BLACK,
            },
            comb_ghost: MaterialRecipe {
                base_color: Color::srgba(0.84, 0.90, 0.88, 0.24),
                metallic: 0.94,
                perceptual_roughness: 0.12,
                reflectance: 0.88,
                alpha_mode: AlphaMode::Blend,
                emissive: LinearRgba::rgb(0.025, 0.035, 0.03),
            },
        }
    }
}

impl ExhibitLightingConfig {
    pub const KEY_ILLUMINANCE_RANGE: IntensityRange = IntensityRange::new(0.0, 2_400.0);
    pub const FILL_INTENSITY_RANGE: IntensityRange = IntensityRange::new(0.0, 4_000.0);
    pub const RIM_INTENSITY_RANGE: IntensityRange = IntensityRange::new(0.0, 5_000.0);
    pub const ACCENT_INTENSITY_RANGE: IntensityRange = IntensityRange::new(0.0, 2_500.0);
    pub const AMBIENT_BRIGHTNESS_RANGE: IntensityRange = IntensityRange::new(0.0, 80.0);
    pub const ENVIRONMENT_INTENSITY_RANGE: IntensityRange = IntensityRange::new(0.0, 360.0);

    pub fn directional_shadow_map(&self) -> DirectionalLightShadowMap {
        DirectionalLightShadowMap {
            size: self.directional_shadow_map_size,
        }
    }

    pub fn point_shadow_map(&self) -> PointLightShadowMap {
        PointLightShadowMap {
            size: self.point_shadow_map_size,
        }
    }

    pub fn ambient_light(&self) -> GlobalAmbientLight {
        GlobalAmbientLight {
            color: self.ambient_color,
            brightness: self.ambient_brightness,
            ..default()
        }
    }

    pub fn model_tuning(&self, group: MeshGroup) -> Option<&MaterialTuning> {
        match group {
            MeshGroup::Static => Some(&self.model_static),
            MeshGroup::Lid => Some(&self.model_lid),
            MeshGroup::WindingKey => Some(&self.winding_key),
            MeshGroup::Cylinder | MeshGroup::Comb | MeshGroup::Excluded => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureMaterialClass {
    AgedBrass,
    PolishedSteel,
    DarkMetal,
    DarkStage,
}

impl TextureMaterialClass {
    fn stem(self) -> &'static str {
        match self {
            Self::AgedBrass => "aged_brass",
            Self::PolishedSteel => "polished_steel",
            Self::DarkMetal => "dark_metal",
            Self::DarkStage => "dark_stage",
        }
    }

    fn pbr(self) -> (f32, f32, f32) {
        match self {
            Self::AgedBrass => (0.96, 0.24, 0.86),
            Self::PolishedSteel => (0.98, 0.12, 0.86),
            Self::DarkMetal => (0.82, 0.54, 0.45),
            Self::DarkStage => (0.0, 0.86, 0.18),
        }
    }
}

pub fn apply_texture_class(
    material: &mut StandardMaterial,
    asset_server: &AssetServer,
    class: TextureMaterialClass,
) {
    let stem = class.stem();
    material.base_color_texture =
        Some(asset_server.load(format!("textures/procedural/{stem}_base.png")));
    material.metallic_roughness_texture =
        Some(asset_server.load(format!("textures/procedural/{stem}_orm.png")));
    material.normal_map_texture =
        Some(asset_server.load(format!("textures/procedural/{stem}_normal.png")));
    let alpha = material.base_color.to_srgba().alpha;
    material.base_color = Color::srgba(1.0, 1.0, 1.0, alpha);
    let (metallic, roughness, reflectance) = class.pbr();
    material.metallic = metallic;
    material.perceptual_roughness = roughness;
    material.reflectance = reflectance;
}

#[derive(Debug, Clone, Copy)]
pub struct DirectionalLightRecipe {
    pub color: Color,
    pub illuminance: f32,
    pub position: Vec3,
    pub first_cascade_far_bound: f32,
    pub maximum_distance: f32,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub contact_shadows_enabled: bool,
}

impl DirectionalLightRecipe {
    pub fn light(&self) -> DirectionalLight {
        DirectionalLight {
            color: self.color,
            illuminance: self.illuminance,
            shadow_maps_enabled: true,
            contact_shadows_enabled: self.contact_shadows_enabled,
            shadow_depth_bias: self.shadow_depth_bias,
            shadow_normal_bias: self.shadow_normal_bias,
            ..default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PointLightRecipe {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub position: Vec3,
    pub shadow_maps_enabled: bool,
    pub contact_shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub shadow_map_near_z: f32,
}

impl PointLightRecipe {
    pub fn light(&self) -> PointLight {
        PointLight {
            color: self.color,
            intensity: self.intensity,
            range: self.range,
            radius: self.radius,
            shadow_maps_enabled: self.shadow_maps_enabled,
            contact_shadows_enabled: self.contact_shadows_enabled,
            shadow_depth_bias: self.shadow_depth_bias,
            shadow_normal_bias: self.shadow_normal_bias,
            shadow_map_near_z: self.shadow_map_near_z,
            ..default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SpotLightRecipe {
    pub color: Color,
    pub intensity: f32,
    pub min_intensity: f32,
    pub max_intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub default_inner_angle: f32,
    pub default_outer_angle: f32,
    pub min_inner_angle: f32,
    pub min_outer_angle: f32,
    pub max_outer_angle: f32,
    pub shadow_maps_enabled: bool,
    pub contact_shadows_enabled: bool,
    pub shadow_depth_bias: f32,
    pub shadow_normal_bias: f32,
    pub shadow_map_near_z: f32,
}

impl SpotLightRecipe {
    pub fn light(&self, inner_angle: f32, outer_angle: f32, intensity: f32) -> SpotLight {
        SpotLight {
            color: self.color,
            intensity: intensity.clamp(self.min_intensity, self.max_intensity),
            inner_angle: self.clamp_inner_angle(inner_angle, outer_angle),
            outer_angle: self.clamp_outer_angle(outer_angle),
            range: self.range,
            radius: self.radius,
            shadow_maps_enabled: self.shadow_maps_enabled,
            contact_shadows_enabled: self.contact_shadows_enabled,
            shadow_depth_bias: self.shadow_depth_bias,
            shadow_normal_bias: self.shadow_normal_bias,
            shadow_map_near_z: self.shadow_map_near_z,
            ..default()
        }
    }

    pub fn clamp_outer_angle(&self, outer_angle: f32) -> f32 {
        outer_angle.clamp(self.min_outer_angle, self.max_outer_angle)
    }

    pub fn clamp_inner_angle(&self, inner_angle: f32, outer_angle: f32) -> f32 {
        inner_angle.clamp(self.min_inner_angle, self.clamp_outer_angle(outer_angle))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialRecipe {
    pub base_color: Color,
    pub metallic: f32,
    pub perceptual_roughness: f32,
    pub reflectance: f32,
    pub alpha_mode: AlphaMode,
    pub emissive: LinearRgba,
}

impl MaterialRecipe {
    pub fn material(&self) -> StandardMaterial {
        StandardMaterial {
            base_color: self.base_color,
            metallic: self.metallic,
            perceptual_roughness: self.perceptual_roughness,
            reflectance: self.reflectance,
            alpha_mode: self.alpha_mode,
            emissive: self.emissive,
            ..default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MaterialTuning {
    pub metallic: Option<f32>,
    pub max_metallic: Option<f32>,
    pub perceptual_roughness: Option<f32>,
    pub min_roughness: Option<f32>,
    pub reflectance: Option<f32>,
    pub base_color_tint: Color,
}

impl MaterialTuning {
    pub fn apply(&self, material: &mut StandardMaterial) {
        if let Some(metallic) = self.metallic {
            material.metallic = metallic;
        }
        if let Some(max_metallic) = self.max_metallic {
            material.metallic = material.metallic.min(max_metallic);
        }
        if let Some(roughness) = self.perceptual_roughness {
            material.perceptual_roughness = roughness;
        }
        if let Some(min_roughness) = self.min_roughness {
            material.perceptual_roughness = material.perceptual_roughness.max(min_roughness);
        }
        if let Some(reflectance) = self.reflectance {
            material.reflectance = reflectance;
        }
        material.base_color = multiply_color(material.base_color, self.base_color_tint);
    }
}

fn multiply_color(color: Color, tint: Color) -> Color {
    let color = color.to_srgba();
    let tint = tint.to_srgba();
    Color::srgba(
        color.red * tint.red,
        color.green * tint.green,
        color.blue * tint.blue,
        color.alpha * tint.alpha,
    )
}
