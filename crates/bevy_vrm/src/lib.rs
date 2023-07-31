use bevy::app::{App, Plugin};
use bevy::asset::AddAsset;
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::CompressedImageFormats;

pub use loader::{GltfError, VrmLoader};

mod loader;

pub struct VrmPlugin;

impl Plugin for VrmPlugin {
    fn build(&self, app: &mut App) {
        let supported_compressed_formats = match app.world.get_resource::<RenderDevice>() {
            Some(render_device) => CompressedImageFormats::from_features(render_device.features()),
            None => CompressedImageFormats::all(),
        };
        app.add_asset_loader(VrmLoader {
            supported_compressed_formats,
            custom_vertex_attributes: Default::default(),
        });
    }

    fn name(&self) -> &str {
        "VRM"
    }
}
