use std::collections::HashMap;

use anyhow::anyhow;
use bevy::math::Vec3;
use bevy::prelude::{default, Mesh, Resource, Transform};
use bevy::reflect::List;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

use idol_api::FaceLandmark;

#[derive(Debug)]
pub struct Face {
    pub landmarks: Vec<FaceLandmark>,
    pub blend_shapes: HashMap<String, f32>,
    pub transform: Transform,
}

#[derive(Debug, Default, Resource)]
pub struct Faces {
    pub faces: Vec<Face>,
}

/// The debug mesh has some very important properties that most loaders don't
/// really care about, such as vertex ordering.
pub fn load_debug_mesh(path: &str) -> anyhow::Result<Mesh> {
    let (mut models, _) = tobj::load_obj(path, &tobj::LoadOptions {
        ..default()
    })?;
    if models.len() != 1 {
        return Err(anyhow!("debug meshes must contain exactly one mesh"));
    }
    let source_mesh = models.pop().unwrap().mesh;
    let vertices = source_mesh.positions.chunks(3)
        .map(|c| [c[0], c[1], c[2]])
        .collect::<Vec<_>>();

    // Fake some normals for now.
    let normals = vertices.iter().map(|_| Vec3::Z).collect::<Vec<_>>();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_indices(Some(Indices::U32(source_mesh.indices)));
    Ok(mesh)
}
