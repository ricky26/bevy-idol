use bevy::asset::{AssetLoader, BoxedFuture, Error, LoadContext, LoadedAsset};
use bevy::core::Name;
use bevy::hierarchy::BuildWorldChildren;
use bevy::math::{Quat, Vec3};
use bevy::prelude::{Entity, SpatialBundle, Transform, World};
use bevy::scene::Scene;
use bevy::utils::default;
use gltf::Gltf;

#[derive(Default)]
pub struct VrmLoader;

impl AssetLoader for VrmLoader {
    fn load<'a>(&'a self, bytes: &'a [u8], load_context: &'a mut LoadContext) -> BoxedFuture<'a, anyhow::Result<(), Error>> {
        Box::pin(load(bytes, load_context))
    }

    fn extensions(&self) -> &[&str] {
        &["vrm"]
    }
}

struct LoadingVrm {
    world: World,
}

impl LoadingVrm {
    pub fn new(gltf: &Gltf) -> Self {
        let mut this = Self {
            world: World::default(),
        };
        this.load(gltf);
        this
    }

    fn load_node(&mut self, gltf: &Gltf, node: gltf::Node, parent: Option<Entity>) {
        let transform = match node.transform() {
            gltf::scene::Transform::Matrix { matrix } =>
                Transform::from_matrix(bevy::math::Mat4::from_cols_array_2d(&matrix)),
            gltf::scene::Transform::Decomposed { translation, rotation, scale } =>
                Transform {
                    translation: Vec3::from_slice(&translation),
                    rotation: Quat::from_slice(&rotation),
                    scale: Vec3::from_slice(&scale),
                },
        };
        let mut entity = self.world.spawn((
            SpatialBundle {
                transform,
                ..default()
            },
        ));

        if let Some(name) = node.name() {
            entity.insert(Name::new(name.to_owned()));
        }

        let id = entity.id();
        for child in node.children() {
            self.load_node(gltf, child, Some(id));
        }

        if let Some(parent) = parent {
            self.world.entity_mut(parent).add_child(id);
        }
    }

    fn load(&mut self, gltf: &Gltf) {
        for node in gltf.nodes() {
            self.load_node(gltf, node, None);
        }
    }
}

async fn load(bytes: &[u8], load_context: &mut LoadContext<'_>) -> anyhow::Result<(), Error> {
    let gltf = Gltf::from_slice(bytes)?;
    let loading = LoadingVrm::new(gltf);
    load_context.set_default_asset(LoadedAsset::new(Scene::new(loading.world)));
    Ok(())
}
