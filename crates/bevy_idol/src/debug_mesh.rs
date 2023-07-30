use anyhow::{anyhow, Error};
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::prelude::Mesh;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;

#[derive(Default)]
pub struct DebugMeshLoader;

fn read_float<'a>(iter: &mut impl Iterator<Item=&'a str>) -> anyhow::Result<f32> {
    Ok(iter.next()
        .ok_or_else(|| anyhow!("expected float"))?
        .parse::<f32>()?)
}

fn read_face<'a>(iter: &mut impl Iterator<Item=&'a str>) -> anyhow::Result<(u32, Option<u32>, Option<u32>)> {
    let face_str = iter.next().ok_or_else(|| anyhow!("expected face definition"))?;
    let mut parts = face_str.split("/");
    let position_index = parts.next().unwrap().parse::<u32>()?;
    let uv_index = match parts.next() {
        Some(x) => x.parse::<u32>().ok(),
        None => None,
    };
    let normal_index = match parts.next() {
        Some(x) => x.parse::<u32>().ok(),
        None => None,
    };
    Ok((position_index, uv_index, normal_index))
}

impl DebugMeshLoader {
    async fn load_mesh(bytes: &[u8], load_context: &mut LoadContext<'_>) -> anyhow::Result<(), Error> {
        let text = std::str::from_utf8(bytes)?;
        let mut raw_normals = Vec::new();
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut indices = Vec::new();

        for line in text.lines() {
            let mut parts = line.split_whitespace();
            let Some(command) = parts.next() else {
                continue;
            };

            match command {
                "v" => {
                    let x = read_float(&mut parts)?;
                    let y = read_float(&mut parts)?;
                    let z = read_float(&mut parts)?;
                    positions.push([x, y, z]);
                    normals.push([0., 0., 1.])
                }
                "vn" => {
                    let x = read_float(&mut parts)?;
                    let y = read_float(&mut parts)?;
                    let z = read_float(&mut parts)?;
                    raw_normals.push([x, y, z]);
                }
                "f" => {
                    let (a, _, an) = read_face(&mut parts)?;
                    let (b, _, bn) = read_face(&mut parts)?;
                    let (c, _, cn) = read_face(&mut parts)?;

                    if !raw_normals.is_empty() {
                        if let Some(an) = an {
                            normals[(a - 1) as usize] = raw_normals[(an - 1) as usize];
                        }

                        if let Some(bn) = bn {
                            normals[(b - 1) as usize] = raw_normals[(bn - 1) as usize];
                        }

                        if let Some(cn) = cn {
                            normals[(c - 1) as usize] = raw_normals[(cn - 1) as usize];
                        }
                    }

                    indices.push(a - 1);
                    indices.push(b - 1);
                    indices.push(c - 1);
                }
                _ => {}
            }
        }

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);

        if !raw_normals.is_empty() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        }

        mesh.set_indices(Some(Indices::U32(indices)));
        load_context.set_default_asset(LoadedAsset::new(mesh));
        Ok(())
    }
}

impl AssetLoader for DebugMeshLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), Error>> {
        Box::pin(Self::load_mesh(bytes, load_context))
    }

    fn extensions(&self) -> &[&str] {
        &["dobj"]
    }
}
