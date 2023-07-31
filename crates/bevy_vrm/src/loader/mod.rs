use std::path::Path;

use anyhow::Result;
use base64::Engine;
use bevy::asset::{AssetLoader, BoxedFuture, LoadContext, LoadedAsset};
use bevy::asset::{
    AssetIoError, AssetPath, Handle, HandleId,
};
use bevy::core::Name;
use bevy::hierarchy::{BuildWorldChildren, WorldChildBuilder};
use bevy::log;
use bevy::math::{Mat4, Vec3};
use bevy::pbr::{AlphaMode, PbrBundle, StandardMaterial};
use bevy::prelude::{Camera3dBundle, Entity, World};
use bevy::render::{
    camera::{Camera, OrthographicProjection, PerspectiveProjection, Projection, ScalingMode},
    color::Color,
    mesh::{
        Indices,
        Mesh,
        MeshVertexAttribute, morph::{MeshMorphWeights, MorphAttributes, MorphTargetImage, MorphWeights}, skinning::{SkinnedMesh, SkinnedMeshInverseBindposes}, VertexAttributeValues,
    },
    prelude::SpatialBundle,
    primitives::Aabb,
    render_resource::{AddressMode, Face, FilterMode, PrimitiveTopology, SamplerDescriptor},
    texture::{CompressedImageFormats, Image, ImageSampler, ImageType, TextureError},
};
use bevy::scene::Scene;
use bevy::tasks::IoTaskPool;
use bevy::transform::components::Transform;
use bevy::utils::{HashMap, HashSet};
use gltf::{
    accessor::Iter,
    Material,
    mesh::{Mode, util::ReadIndices},
    Node, Primitive, texture::{MagFilter, MinFilter, WrappingMode},
};
use serde::Deserialize;
use thiserror::Error;

use vertex_attributes::*;

mod vertex_attributes;

/// An error that occurs when loading a glTF file.
#[derive(Error, Debug)]
pub enum GltfError {
    #[error("unsupported primitive mode")]
    UnsupportedPrimitive { mode: Mode },
    #[error("invalid glTF file: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("binary blob is missing")]
    MissingBlob,
    #[error("failed to decode base64 mesh data")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("unsupported buffer format")]
    BufferFormatUnsupported,
    #[error("invalid image mime type: {0}")]
    InvalidImageMimeType(String),
    #[error("You may need to add the feature for the file format: {0}")]
    ImageError(#[from] TextureError),
    #[error("failed to load an asset path: {0}")]
    AssetIoError(#[from] AssetIoError),
    #[error("Missing sampler for animation {0}")]
    MissingAnimationSampler(usize),
    #[error("failed to generate tangents: {0}")]
    GenerateTangentsError(#[from] bevy::render::mesh::GenerateTangentsError),
    #[error("failed to generate morph targets: {0}")]
    MorphTarget(#[from] bevy::render::mesh::morph::MorphBuildError),
}

/// Loads glTF files with all of their data as their corresponding bevy representations.
pub struct VrmLoader {
    pub(crate) supported_compressed_formats: CompressedImageFormats,
    pub(crate) custom_vertex_attributes: HashMap<String, MeshVertexAttribute>,
}

impl AssetLoader for VrmLoader {
    fn load<'a>(
        &'a self,
        bytes: &'a [u8],
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<()>> {
        Box::pin(async move { Ok(load_gltf(bytes, load_context, self).await?) })
    }

    fn extensions(&self) -> &[&str] {
        &["vrm"]
    }
}

/// Loads an entire glTF file.
async fn load_gltf<'a, 'b>(
    bytes: &'a [u8],
    load_context: &'a mut LoadContext<'b>,
    loader: &VrmLoader,
) -> Result<(), GltfError> {
    let gltf = gltf::Gltf::from_slice(bytes)?;
    let buffer_data = load_buffers(&gltf, load_context, load_context.path()).await?;

    let mut materials = vec![];
    let mut named_materials = HashMap::default();
    let mut linear_textures = HashSet::default();
    for material in gltf.materials() {
        let handle = load_material(&material, load_context);
        if let Some(name) = material.name() {
            named_materials.insert(name.to_string(), handle.clone());
        }
        materials.push(handle);
        if let Some(texture) = material.normal_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material.occlusion_texture() {
            linear_textures.insert(texture.texture().index());
        }
        if let Some(texture) = material
            .pbr_metallic_roughness()
            .metallic_roughness_texture()
        {
            linear_textures.insert(texture.texture().index());
        }
    }

    #[cfg(feature = "bevy_animation")]
        let paths = {
        let mut paths = HashMap::<usize, (usize, Vec<Name>)>::new();
        for scene in gltf.scenes() {
            for node in scene.nodes() {
                let root_index = node.index();
                paths_recur(node, &[], &mut paths, root_index);
            }
        }
        paths
    };

    #[cfg(feature = "bevy_animation")]
        let (animations, named_animations, animation_roots) = {
        use bevy_animation::Keyframes;
        use gltf::animation::util::ReadOutputs;
        let mut animations = vec![];
        let mut named_animations = HashMap::default();
        let mut animation_roots = HashSet::default();
        for animation in gltf.animations() {
            let mut animation_clip = bevy_animation::AnimationClip::default();
            for channel in animation.channels() {
                match channel.sampler().interpolation() {
                    gltf::animation::Interpolation::Linear => (),
                    other => warn!(
                        "Animation interpolation {:?} is not supported, will use linear",
                        other
                    ),
                };
                let node = channel.target().node();
                let reader = channel.reader(|buffer| Some(&buffer_data[buffer.index()]));
                let keyframe_timestamps: Vec<f32> = if let Some(inputs) = reader.read_inputs() {
                    match inputs {
                        gltf::accessor::Iter::Standard(times) => times.collect(),
                        gltf::accessor::Iter::Sparse(_) => {
                            warn!("Sparse accessor not supported for animation sampler input");
                            continue;
                        }
                    }
                } else {
                    warn!("Animations without a sampler input are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                let keyframes = if let Some(outputs) = reader.read_outputs() {
                    match outputs {
                        ReadOutputs::Translations(tr) => {
                            Keyframes::Translation(tr.map(Vec3::from).collect())
                        }
                        ReadOutputs::Rotations(rots) => Keyframes::Rotation(
                            rots.into_f32().map(bevy_math::Quat::from_array).collect(),
                        ),
                        ReadOutputs::Scales(scale) => {
                            Keyframes::Scale(scale.map(Vec3::from).collect())
                        }
                        ReadOutputs::MorphTargetWeights(weights) => {
                            Keyframes::Weights(weights.into_f32().collect())
                        }
                    }
                } else {
                    warn!("Animations without a sampler output are not supported");
                    return Err(GltfError::MissingAnimationSampler(animation.index()));
                };

                if let Some((root_index, path)) = paths.get(&node.index()) {
                    animation_roots.insert(root_index);
                    animation_clip.add_curve_to_path(
                        bevy_animation::EntityPath {
                            parts: path.clone(),
                        },
                        bevy_animation::VariableCurve {
                            keyframe_timestamps,
                            keyframes,
                        },
                    );
                } else {
                    warn!(
                        "Animation ignored for node {}: part of its hierarchy is missing a name",
                        node.index()
                    );
                }
            }
            let handle = load_context.set_labeled_asset(
                &format!("Animation{}", animation.index()),
                LoadedAsset::new(animation_clip),
            );
            if let Some(name) = animation.name() {
                named_animations.insert(name.to_string(), handle.clone());
            }
            animations.push(handle);
        }
        (animations, named_animations, animation_roots)
    };

    for gltf_mesh in gltf.meshes() {
        for primitive in gltf_mesh.primitives() {
            let primitive_label = primitive_label(&gltf_mesh, &primitive);
            let primitive_topology = get_primitive_topology(primitive.mode())?;

            let mut mesh = Mesh::new(primitive_topology);

            // Read vertex attributes
            for (semantic, accessor) in primitive.attributes() {
                match convert_attribute(
                    semantic,
                    accessor,
                    &buffer_data,
                    &loader.custom_vertex_attributes,
                ) {
                    Ok((attribute, values)) => mesh.insert_attribute(attribute, values),
                    Err(err) => log::warn!("{}", err),
                }
            }

            // Read vertex indices
            let reader = primitive.reader(|buffer| Some(buffer_data[buffer.index()].as_slice()));
            if let Some(indices) = reader.read_indices() {
                mesh.set_indices(Some(match indices {
                    ReadIndices::U8(is) => Indices::U16(is.map(|x| x as u16).collect()),
                    ReadIndices::U16(is) => Indices::U16(is.collect()),
                    ReadIndices::U32(is) => Indices::U32(is.collect()),
                }));
            };

            {
                let morph_target_reader = reader.read_morph_targets();
                if morph_target_reader.len() != 0 {
                    let morph_targets_label = morph_targets_label(&gltf_mesh, &primitive);
                    let morph_target_image = MorphTargetImage::new(
                        morph_target_reader.map(PrimitiveMorphAttributesIter),
                        mesh.count_vertices(),
                    )?;
                    let handle = load_context.set_labeled_asset(
                        &morph_targets_label,
                        LoadedAsset::new(morph_target_image.0),
                    );

                    mesh.set_morph_targets(handle);
                    let extras = gltf_mesh.extras().as_ref();
                    if let Option::<MorphTargetNames>::Some(names) =
                        extras.and_then(|extras| serde_json::from_str(extras.get()).ok())
                    {
                        mesh.set_morph_target_names(names.target_names);
                    }
                }
            }

            if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_none()
                && matches!(mesh.primitive_topology(), PrimitiveTopology::TriangleList)
            {
                let vertex_count_before = mesh.count_vertices();
                mesh.duplicate_vertices();
                mesh.compute_flat_normals();
                let vertex_count_after = mesh.count_vertices();

                if vertex_count_before != vertex_count_after {
                    log::debug!("Missing vertex normals in indexed geometry, computing them as flat. Vertex count increased from {} to {}", vertex_count_before, vertex_count_after);
                } else {
                    log::debug!(
                        "Missing vertex normals in indexed geometry, computing them as flat."
                    );
                }
            }

            if let Some(vertex_attribute) = reader
                .read_tangents()
                .map(|v| VertexAttributeValues::Float32x4(v.collect()))
            {
                mesh.insert_attribute(Mesh::ATTRIBUTE_TANGENT, vertex_attribute);
            } else if mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some()
                && primitive.material().normal_texture().is_some()
            {
                log::debug!(
                    "Missing vertex tangents, computing them using the mikktspace algorithm"
                );
                if let Err(err) = mesh.generate_tangents() {
                    log::warn!(
                        "Failed to generate vertex tangents using the mikktspace algorithm: {:?}",
                        err
                    );
                }
            }

            load_context.set_labeled_asset(&primitive_label, LoadedAsset::new(mesh));
        }
    }

    // TODO: use the threaded impl on wasm once wasm thread pool doesn't deadlock on it
    // See https://github.com/bevyengine/bevy/issues/1924 for more details
    // The taskpool use is also avoided when there is only one texture for performance reasons and
    // to avoid https://github.com/bevyengine/bevy/pull/2725
    if gltf.textures().len() == 1 || cfg!(target_arch = "wasm32") {
        for gltf_texture in gltf.textures() {
            let (texture, label) = load_texture(
                gltf_texture,
                &buffer_data,
                &linear_textures,
                load_context,
                loader.supported_compressed_formats,
            )
                .await?;
            load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
        }
    } else {
        #[cfg(not(target_arch = "wasm32"))]
        IoTaskPool::get()
            .scope(|scope| {
                gltf.textures().for_each(|gltf_texture| {
                    let linear_textures = &linear_textures;
                    let load_context: &LoadContext = load_context;
                    let buffer_data = &buffer_data;
                    scope.spawn(async move {
                        load_texture(
                            gltf_texture,
                            buffer_data,
                            linear_textures,
                            load_context,
                            loader.supported_compressed_formats,
                        )
                            .await
                    });
                });
            })
            .into_iter()
            .filter_map(|res| {
                if let Err(err) = res.as_ref() {
                    log::warn!("Error loading glTF texture: {}", err);
                }
                res.ok()
            })
            .for_each(|(texture, label)| {
                load_context.set_labeled_asset(&label, LoadedAsset::new(texture));
            });
    }

    let skinned_mesh_inverse_bindposes: Vec<_> = gltf
        .skins()
        .map(|gltf_skin| {
            let reader = gltf_skin.reader(|buffer| Some(&buffer_data[buffer.index()]));
            let inverse_bindposes: Vec<Mat4> = reader
                .read_inverse_bind_matrices()
                .unwrap()
                .map(|mat| Mat4::from_cols_array_2d(&mat))
                .collect();

            load_context.set_labeled_asset(
                &skin_label(&gltf_skin),
                LoadedAsset::new(SkinnedMeshInverseBindposes::from(inverse_bindposes)),
            )
        })
        .collect();

    let mut has_default = false;
    let mut active_camera_found = false;
    for scene in gltf.scenes() {
        let mut err = None;
        let mut world = World::default();
        let mut node_index_to_entity_map = HashMap::new();
        let mut entity_to_skin_index_map = HashMap::new();

        world
            .spawn(SpatialBundle::INHERITED_IDENTITY)
            .with_children(|parent| {
                for node in scene.nodes() {
                    let result = load_node(
                        &node,
                        parent,
                        load_context,
                        &mut node_index_to_entity_map,
                        &mut entity_to_skin_index_map,
                        &mut active_camera_found,
                    );
                    if result.is_err() {
                        err = Some(result);
                        return;
                    }
                }
            });
        if let Some(Err(err)) = err {
            return Err(err);
        }

        for (&entity, &skin_index) in &entity_to_skin_index_map {
            let mut entity = world.entity_mut(entity);
            let skin = gltf.skins().nth(skin_index).unwrap();
            let joint_entities: Vec<_> = skin
                .joints()
                .map(|node| node_index_to_entity_map[&node.index()])
                .collect();

            entity.insert(SkinnedMesh {
                inverse_bindposes: skinned_mesh_inverse_bindposes[skin_index].clone(),
                joints: joint_entities,
            });
        }

        let new_scene = Scene::new(world);

        if !has_default {
            has_default = true;
            load_context.set_default_asset(LoadedAsset::new(new_scene));
        } else {
            load_context.set_labeled_asset(
                &scene_label(&scene), LoadedAsset::new(new_scene));
        }
    }

    if !has_default {
        return Err(GltfError::MissingBlob);
    }

    Ok(())
}

fn node_name(node: &Node) -> Name {
    let name = node
        .name()
        .map(|s| s.to_string())
        .unwrap_or_else(|| format!("GltfNode{}", node.index()));
    Name::new(name)
}

/// Loads a glTF texture as a bevy [`Image`] and returns it together with its label.
async fn load_texture<'a>(
    gltf_texture: gltf::Texture<'a>,
    buffer_data: &[Vec<u8>],
    linear_textures: &HashSet<usize>,
    load_context: &LoadContext<'a>,
    supported_compressed_formats: CompressedImageFormats,
) -> Result<(Image, String), GltfError> {
    let is_srgb = !linear_textures.contains(&gltf_texture.index());
    let mut texture = match gltf_texture.source().source() {
        gltf::image::Source::View { view, mime_type } => {
            let start = view.offset();
            let end = view.offset() + view.length();
            let buffer = &buffer_data[view.buffer().index()][start..end];
            Image::from_buffer(
                buffer,
                ImageType::MimeType(mime_type),
                supported_compressed_formats,
                is_srgb,
            )?
        }
        gltf::image::Source::Uri { uri, mime_type } => {
            let uri = percent_encoding::percent_decode_str(uri)
                .decode_utf8()
                .unwrap();
            let uri = uri.as_ref();
            let (bytes, image_type) = if let Ok(data_uri) = DataUri::parse(uri) {
                (data_uri.decode()?, ImageType::MimeType(data_uri.mime_type))
            } else {
                let parent = load_context.path().parent().unwrap();
                let image_path = parent.join(uri);
                let bytes = load_context.read_asset_bytes(image_path.clone()).await?;

                let extension = Path::new(uri).extension().unwrap().to_str().unwrap();
                let image_type = ImageType::Extension(extension);

                (bytes, image_type)
            };

            Image::from_buffer(
                &bytes,
                mime_type.map(ImageType::MimeType).unwrap_or(image_type),
                supported_compressed_formats,
                is_srgb,
            )?
        }
    };
    texture.sampler_descriptor = ImageSampler::Descriptor(texture_sampler(&gltf_texture));

    Ok((texture, texture_label(&gltf_texture)))
}

/// Loads a glTF material as a bevy [`StandardMaterial`] and returns it.
fn load_material(material: &Material, load_context: &mut LoadContext) -> Handle<StandardMaterial> {
    let material_label = material_label(material);

    let pbr = material.pbr_metallic_roughness();

    let color = pbr.base_color_factor();
    let base_color_texture = pbr.base_color_texture().map(|info| {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        load_context.get_handle(path)
    });

    let normal_map_texture: Option<Handle<Image>> =
        material.normal_texture().map(|normal_texture| {
            // TODO: handle normal_texture.scale
            // TODO: handle normal_texture.tex_coord() (the *set* index for the right texcoords)
            let label = texture_label(&normal_texture.texture());
            let path = AssetPath::new_ref(load_context.path(), Some(&label));
            load_context.get_handle(path)
        });

    let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
        // TODO: handle info.tex_coord() (the *set* index for the right texcoords)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        load_context.get_handle(path)
    });

    let occlusion_texture = material.occlusion_texture().map(|occlusion_texture| {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&occlusion_texture.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        load_context.get_handle(path)
    });

    let emissive = material.emissive_factor();
    let emissive_texture = material.emissive_texture().map(|info| {
        // TODO: handle occlusion_texture.tex_coord() (the *set* index for the right texcoords)
        // TODO: handle occlusion_texture.strength() (a scalar multiplier for occlusion strength)
        let label = texture_label(&info.texture());
        let path = AssetPath::new_ref(load_context.path(), Some(&label));
        load_context.get_handle(path)
    });

    load_context.set_labeled_asset(
        &material_label,
        LoadedAsset::new(StandardMaterial {
            base_color: Color::rgba_linear(color[0], color[1], color[2], color[3]),
            base_color_texture,
            perceptual_roughness: pbr.roughness_factor(),
            metallic: pbr.metallic_factor(),
            metallic_roughness_texture,
            normal_map_texture,
            double_sided: material.double_sided(),
            cull_mode: if material.double_sided() {
                None
            } else {
                Some(Face::Back)
            },
            occlusion_texture,
            emissive: Color::rgb_linear(emissive[0], emissive[1], emissive[2]),
            emissive_texture,
            unlit: material.unlit(),
            alpha_mode: alpha_mode(material),
            ..Default::default()
        }),
    )
}

/// Loads a glTF node.
fn load_node(
    gltf_node: &gltf::Node,
    world_builder: &mut WorldChildBuilder,
    load_context: &mut LoadContext,
    node_index_to_entity_map: &mut HashMap<usize, Entity>,
    entity_to_skin_index_map: &mut HashMap<Entity, usize>,
    active_camera_found: &mut bool,
) -> Result<(), GltfError> {
    let transform = gltf_node.transform();
    let mut gltf_error = None;
    let transform = Transform::from_matrix(Mat4::from_cols_array_2d(&transform.matrix()));
    let mut node = world_builder.spawn(SpatialBundle::from(transform));

    node.insert(node_name(gltf_node));

    // create camera node
    if let Some(camera) = gltf_node.camera() {
        let projection = match camera.projection() {
            gltf::camera::Projection::Orthographic(orthographic) => {
                let xmag = orthographic.xmag();
                let orthographic_projection = OrthographicProjection {
                    near: orthographic.znear(),
                    far: orthographic.zfar(),
                    scaling_mode: ScalingMode::FixedHorizontal(1.0),
                    scale: xmag,
                    ..Default::default()
                };

                Projection::Orthographic(orthographic_projection)
            }
            gltf::camera::Projection::Perspective(perspective) => {
                let mut perspective_projection: PerspectiveProjection = PerspectiveProjection {
                    fov: perspective.yfov(),
                    near: perspective.znear(),
                    ..Default::default()
                };
                if let Some(zfar) = perspective.zfar() {
                    perspective_projection.far = zfar;
                }
                if let Some(aspect_ratio) = perspective.aspect_ratio() {
                    perspective_projection.aspect_ratio = aspect_ratio;
                }
                Projection::Perspective(perspective_projection)
            }
        };
        node.insert(Camera3dBundle {
            projection,
            transform,
            camera: Camera {
                is_active: !*active_camera_found,
                ..Default::default()
            },
            ..Default::default()
        });

        *active_camera_found = true;
    }

    // Map node index to entity
    node_index_to_entity_map.insert(gltf_node.index(), node.id());

    if let Some(mesh) = gltf_node.mesh() {
        if let Some(weights) = mesh.weights() {
            let first_mesh = if let Some(primitive) = mesh.primitives().next() {
                let primitive_label = primitive_label(&mesh, &primitive);
                let path = AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                Some(Handle::weak(HandleId::from(path)))
            } else {
                None
            };
            node.insert(MorphWeights::new(weights.to_vec(), first_mesh)?);
        }
    };

    node.with_children(|parent| {
        if let Some(mesh) = gltf_node.mesh() {
            // append primitives
            for primitive in mesh.primitives() {
                let material = primitive.material();
                let material_label = material_label(&material);

                // This will make sure we load the default material now since it would not have been
                // added when iterating over all the gltf materials (since the default material is
                // not explicitly listed in the gltf).
                if !load_context.has_labeled_asset(&material_label) {
                    load_material(&material, load_context);
                }

                let primitive_label = primitive_label(&mesh, &primitive);
                let bounds = primitive.bounding_box();
                let mesh_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&primitive_label));
                let material_asset_path =
                    AssetPath::new_ref(load_context.path(), Some(&material_label));

                let mut primitive_entity = parent.spawn(PbrBundle {
                    mesh: load_context.get_handle(mesh_asset_path),
                    material: load_context.get_handle(material_asset_path),
                    ..Default::default()
                });
                let target_count = primitive.morph_targets().len();
                if target_count != 0 {
                    let weights = match mesh.weights() {
                        Some(weights) => weights.to_vec(),
                        None => vec![0.0; target_count],
                    };
                    // unwrap: the parent's call to `MeshMorphWeights::new`
                    // means this code doesn't run if it returns an `Err`.
                    // According to https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#morph-targets
                    // they should all have the same length.
                    // > All morph target accessors MUST have the same count as
                    // > the accessors of the original primitive.
                    primitive_entity.insert(MeshMorphWeights::new(weights).unwrap());
                }
                primitive_entity.insert(Aabb::from_min_max(
                    Vec3::from_slice(&bounds.min),
                    Vec3::from_slice(&bounds.max),
                ));

                primitive_entity.insert(Name::new(primitive_name(&mesh, &primitive)));
                // Mark for adding skinned mesh
                if let Some(skin) = gltf_node.skin() {
                    entity_to_skin_index_map.insert(primitive_entity.id(), skin.index());
                }
            }
        }

        // append other nodes
        for child in gltf_node.children() {
            if let Err(err) = load_node(
                &child,
                parent,
                load_context,
                node_index_to_entity_map,
                entity_to_skin_index_map,
                active_camera_found,
            ) {
                gltf_error = Some(err);
                return;
            }
        }
    });
    if let Some(err) = gltf_error {
        Err(err)
    } else {
        Ok(())
    }
}

/// Returns the label for the `mesh` and `primitive`.
fn primitive_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!("Mesh{}/Primitive{}", mesh.index(), primitive.index())
}

fn primitive_name(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    let mesh_name = mesh.name().unwrap_or("Mesh");
    if mesh.primitives().len() > 1 {
        format!("{}.{}", mesh_name, primitive.index())
    } else {
        mesh_name.to_string()
    }
}

/// Returns the label for the morph target of `primitive`.
fn morph_targets_label(mesh: &gltf::Mesh, primitive: &Primitive) -> String {
    format!(
        "Mesh{}/Primitive{}/MorphTargets",
        mesh.index(),
        primitive.index()
    )
}

/// Returns the label for the `material`.
fn material_label(material: &gltf::Material) -> String {
    if let Some(index) = material.index() {
        format!("Material{index}")
    } else {
        "MaterialDefault".to_string()
    }
}

/// Returns the label for the `texture`.
fn texture_label(texture: &gltf::Texture) -> String {
    format!("Texture{}", texture.index())
}

/// Returns the label for the `scene`.
fn scene_label(scene: &gltf::Scene) -> String {
    format!("Scene{}", scene.index())
}

fn skin_label(skin: &gltf::Skin) -> String {
    format!("Skin{}", skin.index())
}

/// Extracts the texture sampler data from the glTF texture.
fn texture_sampler<'a>(texture: &gltf::Texture) -> SamplerDescriptor<'a> {
    let gltf_sampler = texture.sampler();

    SamplerDescriptor {
        address_mode_u: texture_address_mode(&gltf_sampler.wrap_s()),
        address_mode_v: texture_address_mode(&gltf_sampler.wrap_t()),

        mag_filter: gltf_sampler
            .mag_filter()
            .map(|mf| match mf {
                MagFilter::Nearest => FilterMode::Nearest,
                MagFilter::Linear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().mag_filter),

        min_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::NearestMipmapNearest
                | MinFilter::NearestMipmapLinear => FilterMode::Nearest,
                MinFilter::Linear
                | MinFilter::LinearMipmapNearest
                | MinFilter::LinearMipmapLinear => FilterMode::Linear,
            })
            .unwrap_or(SamplerDescriptor::default().min_filter),

        mipmap_filter: gltf_sampler
            .min_filter()
            .map(|mf| match mf {
                MinFilter::Nearest
                | MinFilter::Linear
                | MinFilter::NearestMipmapNearest
                | MinFilter::LinearMipmapNearest => FilterMode::Nearest,
                MinFilter::NearestMipmapLinear | MinFilter::LinearMipmapLinear => {
                    FilterMode::Linear
                }
            })
            .unwrap_or(SamplerDescriptor::default().mipmap_filter),

        ..Default::default()
    }
}

/// Maps the texture address mode form glTF to wgpu.
fn texture_address_mode(gltf_address_mode: &gltf::texture::WrappingMode) -> AddressMode {
    match gltf_address_mode {
        WrappingMode::ClampToEdge => AddressMode::ClampToEdge,
        WrappingMode::Repeat => AddressMode::Repeat,
        WrappingMode::MirroredRepeat => AddressMode::MirrorRepeat,
    }
}

/// Maps the `primitive_topology` form glTF to `wgpu`.
fn get_primitive_topology(mode: Mode) -> Result<PrimitiveTopology, GltfError> {
    match mode {
        Mode::Points => Ok(PrimitiveTopology::PointList),
        Mode::Lines => Ok(PrimitiveTopology::LineList),
        Mode::LineStrip => Ok(PrimitiveTopology::LineStrip),
        Mode::Triangles => Ok(PrimitiveTopology::TriangleList),
        Mode::TriangleStrip => Ok(PrimitiveTopology::TriangleStrip),
        mode => Err(GltfError::UnsupportedPrimitive { mode }),
    }
}

fn alpha_mode(material: &Material) -> AlphaMode {
    match material.alpha_mode() {
        gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
        gltf::material::AlphaMode::Mask => AlphaMode::Mask(material.alpha_cutoff().unwrap_or(0.5)),
        gltf::material::AlphaMode::Blend => AlphaMode::Blend,
    }
}

/// Loads the raw glTF buffer data for a specific glTF file.
async fn load_buffers(
    gltf: &gltf::Gltf,
    load_context: &LoadContext<'_>,
    asset_path: &Path,
) -> Result<Vec<Vec<u8>>, GltfError> {
    const VALID_MIME_TYPES: &[&str] = &["application/octet-stream", "application/gltf-buffer"];

    let mut buffer_data = Vec::new();
    for buffer in gltf.buffers() {
        match buffer.source() {
            gltf::buffer::Source::Uri(uri) => {
                let uri = percent_encoding::percent_decode_str(uri)
                    .decode_utf8()
                    .unwrap();
                let uri = uri.as_ref();
                let buffer_bytes = match DataUri::parse(uri) {
                    Ok(data_uri) if VALID_MIME_TYPES.contains(&data_uri.mime_type) => {
                        data_uri.decode()?
                    }
                    Ok(_) => return Err(GltfError::BufferFormatUnsupported),
                    Err(()) => {
                        // TODO: Remove this and add dep
                        let buffer_path = asset_path.parent().unwrap().join(uri);
                        load_context.read_asset_bytes(buffer_path).await?
                    }
                };
                buffer_data.push(buffer_bytes);
            }
            gltf::buffer::Source::Bin => {
                if let Some(blob) = gltf.blob.as_deref() {
                    buffer_data.push(blob.into());
                } else {
                    return Err(GltfError::MissingBlob);
                }
            }
        }
    }

    Ok(buffer_data)
}

struct DataUri<'a> {
    mime_type: &'a str,
    base64: bool,
    data: &'a str,
}

fn split_once(input: &str, delimiter: char) -> Option<(&str, &str)> {
    let mut iter = input.splitn(2, delimiter);
    Some((iter.next()?, iter.next()?))
}

impl<'a> DataUri<'a> {
    fn parse(uri: &'a str) -> Result<DataUri<'a>, ()> {
        let uri = uri.strip_prefix("data:").ok_or(())?;
        let (mime_type, data) = split_once(uri, ',').ok_or(())?;

        let (mime_type, base64) = match mime_type.strip_suffix(";base64") {
            Some(mime_type) => (mime_type, true),
            None => (mime_type, false),
        };

        Ok(DataUri {
            mime_type,
            base64,
            data,
        })
    }

    fn decode(&self) -> Result<Vec<u8>, base64::DecodeError> {
        if self.base64 {
            base64::engine::general_purpose::STANDARD.decode(self.data)
        } else {
            Ok(self.data.as_bytes().to_owned())
        }
    }
}

pub(super) struct PrimitiveMorphAttributesIter<'s>(
    pub (
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
        Option<Iter<'s, [f32; 3]>>,
    ),
);

impl<'s> Iterator for PrimitiveMorphAttributesIter<'s> {
    type Item = MorphAttributes;

    fn next(&mut self) -> Option<Self::Item> {
        let position = self.0.0.as_mut().and_then(|p| p.next());
        let normal = self.0.1.as_mut().and_then(|n| n.next());
        let tangent = self.0.2.as_mut().and_then(|t| t.next());
        if position.is_none() && normal.is_none() && tangent.is_none() {
            return None;
        }

        Some(MorphAttributes {
            position: position.map(|p| p.into()).unwrap_or(Vec3::ZERO),
            normal: normal.map(|n| n.into()).unwrap_or(Vec3::ZERO),
            tangent: tangent.map(|t| t.into()).unwrap_or(Vec3::ZERO),
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MorphTargetNames {
    pub target_names: Vec<String>,
}
