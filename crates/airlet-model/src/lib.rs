use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

use glam::{Mat3, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModelSpec {
    pub asset: AssetSpec,
    #[serde(default)]
    pub basis: BasisSpec,
    pub closed_model: ClosedModelSpec,
    pub lid: RotatingPartSpec,
    pub cylinder: RotatingPartSpec,
    #[serde(default)]
    pub comb: MeshPartSpec,
}

impl ModelSpec {
    pub fn from_toml_str(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }

    pub fn from_toml_path(path: impl AsRef<Path>) -> Result<Self, ModelSpecError> {
        let path = path.as_ref();
        let input = fs::read_to_string(path).map_err(|source| ModelSpecError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        Self::from_toml_str(&input).map_err(|source| ModelSpecError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }
}

#[derive(Debug)]
pub enum ModelSpecError {
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    Parse {
        path: PathBuf,
        source: toml::de::Error,
    },
}

impl std::fmt::Display for ModelSpecError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read { path, source } => {
                write!(
                    formatter,
                    "failed to read model spec {}: {source}",
                    path.display()
                )
            }
            Self::Parse { path, source } => {
                write!(
                    formatter,
                    "failed to parse model spec {}: {source}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ModelSpecError {}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AssetSpec {
    pub gltf: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
pub struct BasisSpec {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub front: [f32; 3],
}

impl Default for BasisSpec {
    fn default() -> Self {
        Self {
            right: [1.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            front: [0.0, 0.0, 1.0],
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClosedModelSpec {
    pub mesh_indices: Vec<usize>,
    pub bounds_min: [f32; 3],
    pub bounds_max: [f32; 3],
    #[serde(default)]
    pub body_meshes: Vec<usize>,
    #[serde(default)]
    pub lid_meshes: Vec<usize>,
    #[serde(default)]
    pub hinge_meshes: Vec<usize>,
    #[serde(default)]
    pub handle_meshes: Vec<usize>,
    #[serde(default)]
    pub mechanism_meshes: Vec<usize>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MeshPartSpec {
    #[serde(default)]
    pub meshes: Vec<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RotatingPartSpec {
    #[serde(default)]
    pub meshes: Vec<usize>,
    pub pivot: [f32; 3],
    pub axis: [f32; 3],
    #[serde(default)]
    pub radius: f32,
    #[serde(default)]
    pub length: f32,
    #[serde(default)]
    pub closed_degrees: f32,
    #[serde(default)]
    pub open_degrees: f32,
    #[serde(default)]
    pub degrees_per_second: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshGroup {
    Static,
    Lid,
    Cylinder,
    Comb,
    Excluded,
}

#[derive(Debug, Clone, Copy)]
pub struct Placement {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct PivotPose {
    pub pivot: [f32; 3],
    pub axis: [f32; 3],
    pub angle_degrees: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct MovableModelState {
    pub lid_t: f32,
    pub cylinder_degrees: f32,
    pub cylinder_spin: bool,
}

impl Default for MovableModelState {
    fn default() -> Self {
        Self {
            lid_t: 0.0,
            cylinder_degrees: 0.0,
            cylinder_spin: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct MovableMusicBoxModel {
    spec: ModelSpec,
    state: MovableModelState,
}

impl MovableMusicBoxModel {
    pub fn new(spec: ModelSpec) -> Self {
        Self {
            spec,
            state: MovableModelState::default(),
        }
    }

    pub fn spec(&self) -> &ModelSpec {
        &self.spec
    }

    pub fn state(&self) -> MovableModelState {
        self.state
    }

    pub fn set_lid_t(&mut self, lid_t: f32) {
        self.state.lid_t = lid_t.clamp(0.0, 1.0);
    }

    pub fn set_cylinder_degrees(&mut self, cylinder_degrees: f32) {
        self.state.cylinder_degrees = cylinder_degrees;
    }

    pub fn set_cylinder_spin(&mut self, cylinder_spin: bool) {
        self.state.cylinder_spin = cylinder_spin;
    }

    pub fn advance(&mut self, dt_seconds: f32) {
        if self.state.cylinder_spin {
            self.state.cylinder_degrees += self.spec.cylinder.degrees_per_second * dt_seconds;
        }
    }

    pub fn root_placement(&self, target: [f32; 3], platform_top_y: f32, scale: f32) -> Placement {
        let (min, max) = self.aligned_closed_bounds();
        let center = (min + max) * 0.5;
        let target = vec3(target);
        let translation = Vec3::new(
            target.x - center.x * scale,
            platform_top_y - min.y * scale,
            target.z - center.z * scale,
        );
        Placement {
            translation: translation.to_array(),
            rotation: self.local_to_rig_rotation().to_array(),
            scale,
        }
    }

    pub fn local_to_rig_point(&self, point: [f32; 3]) -> [f32; 3] {
        (self.local_to_rig_matrix() * vec3(point)).to_array()
    }

    pub fn local_to_rig_axis(&self, axis: [f32; 3]) -> [f32; 3] {
        let axis = self.local_to_rig_matrix() * vec3(axis);
        normalized(axis.to_array())
    }

    pub fn group_for_mesh(&self, mesh_index: usize) -> MeshGroup {
        if !self.spec.closed_model.mesh_indices.contains(&mesh_index) {
            return MeshGroup::Excluded;
        }
        if self.spec.lid.meshes.contains(&mesh_index) {
            return MeshGroup::Lid;
        }
        if self.spec.cylinder.meshes.contains(&mesh_index) {
            return MeshGroup::Cylinder;
        }
        if self.spec.comb.meshes.contains(&mesh_index) {
            return MeshGroup::Comb;
        }
        MeshGroup::Static
    }

    pub fn closed_meshes(&self) -> HashSet<usize> {
        self.spec
            .closed_model
            .mesh_indices
            .iter()
            .copied()
            .collect()
    }

    pub fn lid_pose(&self) -> PivotPose {
        let lid = &self.spec.lid;
        let angle = lid.closed_degrees
            + (lid.open_degrees - lid.closed_degrees) * self.state.lid_t.clamp(0.0, 1.0);
        PivotPose {
            pivot: lid.pivot,
            axis: normalized(lid.axis),
            angle_degrees: angle,
        }
    }

    pub fn lid_pose_rig(&self) -> PivotPose {
        self.pose_to_rig(self.lid_pose())
    }

    pub fn cylinder_pose(&self) -> PivotPose {
        let cylinder = &self.spec.cylinder;
        PivotPose {
            pivot: cylinder.pivot,
            axis: normalized(cylinder.axis),
            angle_degrees: self.state.cylinder_degrees,
        }
    }

    pub fn cylinder_pose_rig(&self) -> PivotPose {
        self.pose_to_rig(self.cylinder_pose())
    }

    pub fn relative_translation(&self, translation: [f32; 3], group: MeshGroup) -> [f32; 3] {
        let pivot = match group {
            MeshGroup::Lid => self.spec.lid.pivot,
            MeshGroup::Cylinder => self.spec.cylinder.pivot,
            _ => [0.0, 0.0, 0.0],
        };
        (vec3(translation) - vec3(pivot)).to_array()
    }

    pub fn redefine_cylinder(&mut self, cylinder: RotatingPartSpec) {
        self.spec.cylinder = cylinder;
    }

    pub fn redefine_comb(&mut self, comb: MeshPartSpec) {
        self.spec.comb = comb;
    }

    fn local_to_rig_rotation(&self) -> Quat {
        Quat::from_mat3(&self.local_to_rig_matrix())
    }

    fn local_to_rig_matrix(&self) -> Mat3 {
        let source_to_local = Mat3::from_cols(
            vec3(normalized(self.spec.basis.right)),
            vec3(normalized(self.spec.basis.up)),
            -vec3(normalized(self.spec.basis.front)),
        );
        source_to_local.transpose()
    }

    fn aligned_closed_bounds(&self) -> (Vec3, Vec3) {
        let min = vec3(self.spec.closed_model.bounds_min);
        let max = vec3(self.spec.closed_model.bounds_max);
        let corners = [
            Vec3::new(min.x, min.y, min.z),
            Vec3::new(min.x, min.y, max.z),
            Vec3::new(min.x, max.y, min.z),
            Vec3::new(min.x, max.y, max.z),
            Vec3::new(max.x, min.y, min.z),
            Vec3::new(max.x, min.y, max.z),
            Vec3::new(max.x, max.y, min.z),
            Vec3::new(max.x, max.y, max.z),
        ];
        let matrix = self.local_to_rig_matrix();
        let mut aligned_min = Vec3::splat(f32::INFINITY);
        let mut aligned_max = Vec3::splat(f32::NEG_INFINITY);
        for corner in corners {
            let point = matrix * corner;
            aligned_min = aligned_min.min(point);
            aligned_max = aligned_max.max(point);
        }
        (aligned_min, aligned_max)
    }

    fn pose_to_rig(&self, pose: PivotPose) -> PivotPose {
        PivotPose {
            pivot: self.local_to_rig_point(pose.pivot),
            axis: self.local_to_rig_axis(pose.axis),
            angle_degrees: pose.angle_degrees,
        }
    }
}

fn vec3(value: [f32; 3]) -> Vec3 {
    Vec3::from_array(value)
}

fn normalized(value: [f32; 3]) -> [f32; 3] {
    let axis = vec3(value);
    if axis.length_squared() <= f32::EPSILON {
        Vec3::X.to_array()
    } else {
        axis.normalize().to_array()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = r#"
[asset]
gltf = "models/converted/music_box.glb"

[basis]
right = [1.0, 0.0, 0.0]
up = [0.0, 1.0, 0.0]
front = [0.0, 0.0, -1.0]

[closed_model]
mesh_indices = [0, 1, 2]
bounds_min = [-1.0, -1.0, -1.0]
bounds_max = [1.0, 0.0, 1.0]
body_meshes = [1]
lid_meshes = [0]
mechanism_meshes = [2]

[lid]
meshes = [0]
pivot = [0.0, 0.0, -1.0]
axis = [1.0, 0.0, 0.0]
radius = 0.0
length = 0.0
closed_degrees = 0.0
open_degrees = 80.0

[cylinder]
meshes = [2]
pivot = [0.0, -0.5, 0.0]
axis = [0.0, 0.0, 2.0]
radius = 0.25
length = 1.5
degrees_per_second = 120.0
"#;

    #[test]
    fn parses_spec_and_groups_meshes() {
        let spec = ModelSpec::from_toml_str(SPEC).unwrap();
        let model = MovableMusicBoxModel::new(spec);
        assert_eq!(model.group_for_mesh(0), MeshGroup::Lid);
        assert_eq!(model.group_for_mesh(1), MeshGroup::Static);
        assert_eq!(model.group_for_mesh(2), MeshGroup::Cylinder);
        assert_eq!(model.group_for_mesh(99), MeshGroup::Excluded);
    }

    #[test]
    fn exposes_parametric_lid_and_cylinder_poses() {
        let spec = ModelSpec::from_toml_str(SPEC).unwrap();
        let mut model = MovableMusicBoxModel::new(spec);
        model.set_lid_t(0.5);
        model.set_cylinder_spin(true);
        model.advance(0.25);

        assert_eq!(model.lid_pose().angle_degrees, 40.0);
        assert_eq!(model.cylinder_pose().angle_degrees, 30.0);
        assert_eq!(model.cylinder_pose().axis, [0.0, 0.0, 1.0]);
    }

    #[test]
    fn computes_root_placement_from_closed_bounds() {
        let spec = ModelSpec::from_toml_str(SPEC).unwrap();
        let model = MovableMusicBoxModel::new(spec);
        let placement = model.root_placement([0.0, 0.0, 0.0], 0.2, 2.0);
        assert_eq!(placement.translation, [0.0, 2.2, 0.0]);
        assert_eq!(placement.scale, 2.0);
    }

    #[test]
    fn maps_model_front_to_standard_negative_z() {
        let spec = ModelSpec::from_toml_str(SPEC).unwrap();
        let model = MovableMusicBoxModel::new(spec);
        assert_eq!(model.local_to_rig_axis([0.0, 0.0, -1.0]), [0.0, 0.0, -1.0]);
    }

    #[test]
    fn exposes_asset_and_rig_space_poses_separately() {
        let spec = ModelSpec::from_toml_str(&SPEC.replace(
            "right = [1.0, 0.0, 0.0]\nup = [0.0, 1.0, 0.0]\nfront = [0.0, 0.0, -1.0]",
            "right = [0.0, 0.0, -1.0]\nup = [0.0, 1.0, 0.0]\nfront = [-1.0, 0.0, 0.0]",
        ))
        .unwrap();
        let model = MovableMusicBoxModel::new(spec);

        assert_eq!(model.lid_pose().pivot, [0.0, 0.0, -1.0]);
        assert_eq!(model.lid_pose_rig().pivot, [1.0, 0.0, 0.0]);
        assert_eq!(model.lid_pose_rig().axis, [0.0, 0.0, 1.0]);
    }
}
