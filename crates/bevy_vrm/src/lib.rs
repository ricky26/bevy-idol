use bevy::app::{App, Plugin, Update};
use bevy::asset::{AddAsset, Assets, Handle};
use bevy::prelude::{Bundle, Commands, ComputedVisibility, Entity, GlobalTransform, Mesh, Query, Res, Transform, Visibility, Without};
use bevy::reflect::{TypePath, TypeUuid};
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::CompressedImageFormats;
use bevy::scene::Scene;
use bevy::utils::HashMap;

pub use loader::{VrmError, VrmLoader};

pub mod extensions;

mod loader;

#[derive(Default, Bundle)]
pub struct VrmBundle {
    pub vrm: Handle<Vrm>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub computed_visibility: ComputedVisibility,
}

pub fn spawn_vrms(
    mut commands: Commands,
    vrms: Res<Assets<Vrm>>,
    to_spawn: Query<(Entity, &Handle<Vrm>), Without<Handle<Scene>>>,
) {
    for (entity, vrm) in &to_spawn {
        if let Some(vrm) = vrms.get(vrm) {
            let scene = if let Some(scene_name) = vrm.default_scene.as_ref() {
                vrm.scenes.get(scene_name).unwrap().clone()
            } else {
                Handle::default()
            };
            commands.entity(entity).insert(scene);
        }
    }
}

#[derive(Debug, Clone, TypeUuid, TypePath)]
#[uuid = "3b8759d9-4314-4bab-9c2c-d6e197b3fcbe"]
pub struct Vrm {
    pub meshes: Vec<Handle<Mesh>>,
    pub default_scene: Option<String>,
    pub scenes: HashMap<String, Handle<Scene>>,
}

pub struct VrmPlugin;

impl Plugin for VrmPlugin {
    fn build(&self, app: &mut App) {
        let supported_compressed_formats = match app.world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),
            None => CompressedImageFormats::all(),
        };
        app
            .add_asset_loader(VrmLoader {
                supported_compressed_formats,
                custom_vertex_attributes: Default::default(),
            })
            .add_systems(Update, spawn_vrms)
            .add_asset::<Vrm>();
    }

    fn name(&self) -> &str {
        "VRM"
    }
}
