use bevy::{
    asset::RenderAssetUsages,
    math::IVec3,
    mesh::{Indices, VertexAttributeValues},
    picking::hover::Hovered,
    prelude::*,
    render::render_resource::{Face, PrimitiveTopology},
};

use crate::controls::ExhibitControls;
use crate::winding::WindingState;

const OUTLINE_OFFSET: f32 = 0.006;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineKind {
    WindingKey,
    Lid,
}

#[derive(Component)]
pub struct OutlineTarget {
    pub kind: OutlineKind,
}

#[derive(Component)]
pub struct OutlineShell {
    pub kind: OutlineKind,
}

pub fn outline_shell_material(cull_face: Face) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgba(1.0, 0.86, 0.20, 0.72),
        emissive: LinearRgba::rgb(1.25, 0.72, 0.08),
        alpha_mode: AlphaMode::Blend,
        cull_mode: Some(cull_face),
        unlit: true,
        ..default()
    }
}

pub fn outline_shell_cull_face(mesh: &Mesh, parent_transform: &Transform) -> Face {
    let signed_volume = mesh_signed_volume(mesh).unwrap_or(0.0);
    outline_cull_face_from_signed_volume(signed_volume, parent_transform)
}

pub fn outline_shell_cull_face_for_meshes<'a>(
    meshes: impl IntoIterator<Item = &'a Mesh>,
    parent_transform: &Transform,
) -> Face {
    let signed_volume = meshes
        .into_iter()
        .filter_map(mesh_signed_volume)
        .sum::<f32>();
    outline_cull_face_from_signed_volume(signed_volume, parent_transform)
}

fn outline_cull_face_from_signed_volume(signed_volume: f32, parent_transform: &Transform) -> Face {
    let transform_handedness = parent_transform
        .to_matrix()
        .determinant()
        .signum()
        .max(-1.0);
    if signed_volume * transform_handedness < -1e-7 {
        Face::Back
    } else {
        Face::Front
    }
}

pub fn outline_shell_mesh(mesh: &Mesh) -> Option<Mesh> {
    outline_shell_mesh_for_meshes([mesh])
}

pub fn outline_shell_mesh_for_meshes<'a>(
    meshes: impl IntoIterator<Item = &'a Mesh>,
) -> Option<Mesh> {
    let mut positions = Vec::<[f32; 3]>::new();
    let mut normals = Vec::<[f32; 3]>::new();
    let mut indices = Vec::<u32>::new();

    for mesh in meshes {
        let source_positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
        let source_normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL)?.as_float3()?;
        if source_positions.len() != source_normals.len() {
            return None;
        }
        let base = positions.len() as u32;
        positions.extend_from_slice(source_positions);
        normals.extend_from_slice(source_normals);
        if let Some(source_indices) = mesh.indices() {
            match source_indices {
                Indices::U16(source_indices) => {
                    indices.extend(source_indices.iter().map(|index| base + *index as u32));
                }
                Indices::U32(source_indices) => {
                    indices.extend(source_indices.iter().map(|index| base + *index));
                }
            }
        } else {
            indices.extend((0..source_positions.len()).map(|index| base + index as u32));
        }
    }

    if positions.is_empty() || normals.is_empty() || indices.is_empty() {
        return None;
    }

    let signed_volume = signed_volume_for_triangles(&positions, &indices);
    let normal_sign = if signed_volume < -1e-7 { -1.0 } else { 1.0 };
    let directions = welded_offset_directions(&positions, &normals, normal_sign);
    let outline_positions = positions
        .iter()
        .zip(directions)
        .map(|(position, direction)| {
            let position = Vec3::from_array(*position);
            (position + direction * OUTLINE_OFFSET).to_array()
        })
        .collect::<Vec<_>>();

    let mut outline = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    outline.insert_attribute(Mesh::ATTRIBUTE_POSITION, outline_positions);
    outline.insert_attribute(
        Mesh::ATTRIBUTE_NORMAL,
        VertexAttributeValues::Float32x3(normals),
    );
    outline.insert_indices(Indices::U32(indices));
    Some(outline)
}

fn welded_offset_directions(
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    normal_sign: f32,
) -> Vec<Vec3> {
    let mut groups = std::collections::HashMap::<IVec3, Vec3>::new();
    for (position, normal) in positions.iter().zip(normals) {
        let key = quantized_position(*position);
        let normal = Vec3::from_array(*normal).normalize_or_zero() * normal_sign;
        groups
            .entry(key)
            .and_modify(|sum| *sum += normal)
            .or_insert(normal);
    }
    positions
        .iter()
        .zip(normals)
        .map(|(position, normal)| {
            let fallback = Vec3::from_array(*normal).normalize_or_zero() * normal_sign;
            groups
                .get(&quantized_position(*position))
                .copied()
                .unwrap_or(fallback)
                .normalize_or(fallback)
        })
        .collect()
}

fn quantized_position(position: [f32; 3]) -> IVec3 {
    const POSITION_WELD_SCALE: f32 = 1_000_000.0;
    IVec3::new(
        (position[0] * POSITION_WELD_SCALE).round() as i32,
        (position[1] * POSITION_WELD_SCALE).round() as i32,
        (position[2] * POSITION_WELD_SCALE).round() as i32,
    )
}

pub fn spawn_outline_shell(
    commands: &mut Commands,
    parent: Entity,
    mesh: Handle<Mesh>,
    material: Handle<StandardMaterial>,
    kind: OutlineKind,
    transform: Transform,
) {
    let shell = commands
        .spawn((
            Name::new(match kind {
                OutlineKind::WindingKey => "Winding Key Outline",
                OutlineKind::Lid => "Lid Outline",
            }),
            OutlineShell { kind },
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            Visibility::Hidden,
        ))
        .id();
    commands.entity(parent).add_child(shell);
}

fn mesh_signed_volume(mesh: &Mesh) -> Option<f32> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    let indices = mesh_triangle_indices(mesh, positions.len())?;
    let indices = indices
        .into_iter()
        .flat_map(|[a, b, c]| [a as u32, b as u32, c as u32])
        .collect::<Vec<_>>();
    (!indices.is_empty()).then_some(signed_volume_for_triangles(positions, &indices))
}

fn signed_volume_for_triangles(positions: &[[f32; 3]], indices: &[u32]) -> f32 {
    let mut signed_volume = 0.0;
    for chunk in indices.chunks_exact(3) {
        let Some(a) = positions.get(chunk[0] as usize).copied() else {
            continue;
        };
        let Some(b) = positions.get(chunk[1] as usize).copied() else {
            continue;
        };
        let Some(c) = positions.get(chunk[2] as usize).copied() else {
            continue;
        };
        let a = Vec3::from_array(a);
        let b = Vec3::from_array(b);
        let c = Vec3::from_array(c);
        signed_volume += a.dot(b.cross(c)) / 6.0;
    }
    signed_volume
}

fn mesh_triangle_indices(mesh: &Mesh, vertex_count: usize) -> Option<Vec<[usize; 3]>> {
    let mut triangles = Vec::new();
    if let Some(indices) = mesh.indices() {
        match indices {
            Indices::U16(indices) => {
                for chunk in indices.chunks_exact(3) {
                    triangles.push([chunk[0] as usize, chunk[1] as usize, chunk[2] as usize]);
                }
            }
            Indices::U32(indices) => {
                for chunk in indices.chunks_exact(3) {
                    triangles.push([chunk[0] as usize, chunk[1] as usize, chunk[2] as usize]);
                }
            }
        }
    } else {
        for index in (0..vertex_count).step_by(3) {
            if index + 2 < vertex_count {
                triangles.push([index, index + 1, index + 2]);
            }
        }
    }
    Some(triangles)
}

pub fn update_interactive_outlines(
    winding: Res<WindingState>,
    targets: Query<(&OutlineTarget, &Hovered)>,
    mut shells: Query<(&OutlineShell, &mut Visibility)>,
) {
    let winding_active = winding.pressed
        || targets
            .iter()
            .any(|(target, hovered)| target.kind == OutlineKind::WindingKey && hovered.get());
    let lid_active = targets
        .iter()
        .any(|(target, hovered)| target.kind == OutlineKind::Lid && hovered.get());

    for (shell, mut visibility) in &mut shells {
        let active = match shell.kind {
            OutlineKind::WindingKey => winding_active,
            OutlineKind::Lid => lid_active,
        };
        *visibility = if active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn toggle_lid_on_click(
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    targets: Query<(&OutlineTarget, &Hovered)>,
    mut controls: ResMut<ExhibitControls>,
) {
    if !mouse_buttons.just_pressed(MouseButton::Left) {
        return;
    }
    let lid_hovered = targets
        .iter()
        .any(|(target, hovered)| target.kind == OutlineKind::Lid && hovered.get());
    if lid_hovered {
        controls.lid_t = toggled_lid_t(controls.lid_t);
    }
}

pub fn toggled_lid_t(lid_t: f32) -> f32 {
    if lid_t < 0.5 { 1.0 } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lid_toggle_targets_opposite_endpoint() {
        assert_eq!(toggled_lid_t(0.0), 1.0);
        assert_eq!(toggled_lid_t(0.49), 1.0);
        assert_eq!(toggled_lid_t(0.5), 0.0);
        assert_eq!(toggled_lid_t(1.0), 0.0);
    }

    #[test]
    fn outline_shell_offsets_by_fixed_distance() {
        let mesh = Cuboid::new(2.0, 2.0, 2.0).mesh().build();
        let outline = outline_shell_mesh(&mesh).unwrap();
        let source_positions = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();
        let outline_positions = outline
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();

        for (source, outline) in source_positions.iter().zip(outline_positions) {
            let distance = Vec3::from_array(*outline).distance(Vec3::from_array(*source));
            assert!(
                (distance - OUTLINE_OFFSET).abs() < 1e-6,
                "distance={distance}"
            );
        }
    }

    #[test]
    fn outline_shell_mesh_is_available_to_renderer() {
        let mesh = Cuboid::new(2.0, 2.0, 2.0).mesh().build();
        let outline = outline_shell_mesh(&mesh).unwrap();

        assert!(outline.asset_usage.contains(RenderAssetUsages::MAIN_WORLD));
        assert!(
            outline
                .asset_usage
                .contains(RenderAssetUsages::RENDER_WORLD)
        );
    }

    #[test]
    fn outline_shell_offsets_along_normals_by_fixed_distance() {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::MAIN_WORLD,
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[2.0, 0.0, 0.0], [4.0, 0.0, 0.0], [4.0, 2.0, 0.0]],
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
        );
        let outline = outline_shell_mesh(&mesh).unwrap();
        let positions = outline
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();

        assert_relative_eq(
            Vec3::from_array(positions[0]),
            Vec3::new(2.0, OUTLINE_OFFSET, 0.0),
        );
        assert_relative_eq(
            Vec3::from_array(positions[1]),
            Vec3::new(4.0, OUTLINE_OFFSET, 0.0),
        );
        assert_relative_eq(
            Vec3::from_array(positions[2]),
            Vec3::new(4.0, 2.0 + OUTLINE_OFFSET, 0.0),
        );
    }

    #[test]
    fn outline_shell_welds_split_normals_at_same_position() {
        let mut mesh = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![[1.0, 0.0, 0.0], [1.0, 0.0, 0.0]],
        );
        mesh.insert_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        );
        let outline = outline_shell_mesh(&mesh).unwrap();
        let positions = outline
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();

        assert_relative_eq(
            Vec3::from_array(positions[0]),
            Vec3::from_array(positions[1]),
        );
    }

    #[test]
    fn outline_shell_welds_split_normals_across_mesh_primitives() {
        let mut first = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        );
        first.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1.0, 0.0, 0.0]]);
        first.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[1.0, 0.0, 0.0]]);
        let mut second = Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            RenderAssetUsages::MAIN_WORLD,
        );
        second.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[1.0, 0.0, 0.0]]);
        second.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]]);

        let outline = outline_shell_mesh_for_meshes([&first, &second]).unwrap();
        let positions = outline
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|attribute| attribute.as_float3())
            .unwrap();

        assert_relative_eq(
            Vec3::from_array(positions[0]),
            Vec3::from_array(positions[1]),
        );
    }

    #[test]
    fn outward_wound_shell_uses_front_face_culling() {
        let mesh = Cuboid::new(2.0, 2.0, 2.0).mesh().build();

        assert_eq!(
            outline_shell_cull_face(&mesh, &Transform::default()),
            Face::Front
        );
    }

    #[test]
    fn inward_wound_shell_uses_back_face_culling() {
        let mesh = inward_wound_tetrahedron();

        assert_eq!(
            outline_shell_cull_face(&mesh, &Transform::default()),
            Face::Back
        );
    }

    fn inward_wound_tetrahedron() -> Mesh {
        Mesh::new(
            bevy::render::render_resource::PrimitiveTopology::TriangleList,
            bevy::asset::RenderAssetUsages::MAIN_WORLD,
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_POSITION,
            vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
            ],
        )
        .with_inserted_attribute(
            Mesh::ATTRIBUTE_NORMAL,
            vec![
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
                [0.0, 0.0, -1.0],
            ],
        )
        .with_inserted_indices(Indices::U32(vec![0, 2, 1, 0, 1, 3, 0, 3, 2, 1, 3, 2]))
    }

    fn assert_relative_eq(left: Vec3, right: Vec3) {
        assert!(
            left.abs_diff_eq(right, 1e-6),
            "left={left:?} right={right:?}"
        );
    }
}
