use bevy::app::{App, Plugin, Update};
use bevy::asset::{Asset, AssetApp, Assets, Handle, ReflectAsset};
use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::CompressedImageFormats;
use bevy::scene::Scene;
use bevy::utils::HashMap;

pub use loader::{VrmError, VrmLoader};

use crate::extensions::mtoon::MToonMaterial;
use crate::extensions::vrm::{apply_transform_look_at, Eye, Humanoid, LookAtRangeMap, LookAtTarget, MorphTargetLookAt, TransformLookAt};

pub mod extensions;

mod loader;

#[derive(Default, Bundle)]
pub struct VrmBundle {
    pub vrm: Handle<Vrm>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    pub view_visibility: ViewVisibility,
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

#[derive(Clone, Debug, Reflect, Asset)]
#[reflect(Debug, Asset)]
pub struct Vrm {
    pub meshes: Vec<Handle<Mesh>>,
    pub default_scene: Option<String>,
    pub scenes: HashMap<String, Handle<Scene>>,
}

pub struct VrmPlugin;

impl Plugin for VrmPlugin {
    fn build(&self, app: &mut App) {
        let supported_compressed_formats = match app.world().get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),
            None => CompressedImageFormats::all(),
        };
        app
            .add_plugins(MaterialPlugin::<MToonMaterial>::default())
            .register_asset_loader(VrmLoader {
                supported_compressed_formats,
                custom_vertex_attributes: Default::default(),
            })
            .add_systems(Update, (spawn_vrms, apply_transform_look_at))
            .init_asset::<MToonMaterial>()
            .register_asset_reflect::<MToonMaterial>()
            .init_asset::<Vrm>()
            .register_asset_reflect::<Vrm>()
            .register_type::<Humanoid>()
            .register_type::<Eye>()
            .register_type::<LookAtTarget>()
            .register_type::<LookAtRangeMap>()
            .register_type::<TransformLookAt>()
            .register_type::<MorphTargetLookAt>()
            .init_asset::<Vrm>();
    }

    fn name(&self) -> &str {
        "VRM"
    }
}
