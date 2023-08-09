use bevy::asset::{Assets, Handle};
use bevy::math::Vec3;
use bevy::prelude::{Commands, Component, Entity, Image, Mesh, Query, Res, ResMut, With};
use bevy::render::mesh::morph::{MorphAttributes, MorphTargetImage};
use serde::{Deserialize, Serialize};
use bevy_vrm::Vrm;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Vec3Dto {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl From<Vec3Dto> for Vec3 {
    fn from(value: Vec3Dto) -> Self {
        Vec3::new(value.x, value.y, value.z)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlendShapeDto {
    #[serde(rename = "shapeKeyName")]
    pub name: String,
    #[serde(rename = "blendShapeWeight")]
    pub weight: f32,
    #[serde(rename = "elements")]
    pub indices: Vec<u32>,
    #[serde(rename = "v3_vertices")]
    pub positions: Vec<Vec3Dto>,
    #[serde(rename = "vertexCount")]
    pub vertex_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlendShapesDto {
    #[serde(rename = "blendShapeDatas")]
    pub blend_shapes: Vec<BlendShapeDto>,
}

#[derive(Debug, Clone)]
pub struct BlendShape {
    pub name: String,
    pub vertex_count: u32,
    pub weight: f32,
    pub indices: Vec<u32>,
    pub positions: Vec<Vec3>,
}

#[derive(Debug, Clone)]
pub struct BlendShapeLibrary {
    pub blend_shapes: Vec<BlendShape>,
}

impl BlendShapeLibrary {
    pub fn from_slice(src: &[u8]) -> anyhow::Result<BlendShapeLibrary> {
        let dto = serde_json::from_slice::<BlendShapesDto>(src)?;
        Ok(Self {
            blend_shapes: dto.blend_shapes.into_iter()
                .map(|s| BlendShape {
                    name: s.name,
                    vertex_count: s.vertex_count,
                    weight: s.weight,
                    indices: s.indices,
                    positions: s.positions.into_iter().map(From::from).collect(),
                })
                .collect(),
        })
    }
}

#[derive(Component)]
pub struct AddBlendShapes {
    pub blend_shapes: Vec<BlendShape>,
}

pub fn apply_blend_shapes(
    mut commands: Commands,
    vrms: Res<Assets<Vrm>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut images: ResMut<Assets<Image>>,
    entities: Query<(Entity, &Handle<Vrm>, &AddBlendShapes), With<AddBlendShapes>>,
) {
    for (entity, vrm, to_add) in &entities {
        let Some(vrm) = vrms.get(vrm) else {
            continue;
        };
        commands.entity(entity).remove::<AddBlendShapes>();

        for mesh in vrm.meshes.iter() {
            let Some(mesh) = meshes.get_mut(mesh) else {
                continue;
            };

            // At the moment I think only the face will have morph targets.
            if !mesh.has_morph_targets() {
                continue;
            }

            // TODO: At the moment this will just replace all morph targets.
            let mut morph_target_names = Vec::new();
            let mut morph_targets = Vec::new();

            let vertex_count = mesh.count_vertices();
            for blend_shape in to_add.blend_shapes.iter()
                .filter(|s| s.vertex_count as usize == vertex_count) {
                let mut elements = vec![MorphAttributes::default(); vertex_count];
                for (i, position) in blend_shape.indices.iter()
                    .zip(blend_shape.positions.iter()) {
                    elements[*i as usize].position = *position;
                }

                morph_target_names.push(blend_shape.name.to_string());
                morph_targets.push(elements.into_iter());
            }

            log::info!("Adding morph targets: {:?}", &morph_target_names);
            let morph_image = MorphTargetImage::new(morph_targets.into_iter(), vertex_count)
                .expect("failed to create morph target image");
            mesh.set_morph_targets(images.add(morph_image.0));
            mesh.set_morph_target_names(morph_target_names);
        }
    }
}
