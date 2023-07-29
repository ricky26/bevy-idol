use bevy::prelude::Resource;
use bevy::render::render_resource::{Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;

#[derive(Resource)]
pub struct OutputTexture {
    width: u32,
    height: u32,
    output_texture: Texture,
}

impl OutputTexture {
    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn texture(&self) -> &Texture {
        &self.output_texture
    }

    pub fn new(device: &RenderDevice, width: u32, height: u32) -> Self {
        let output_size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let output_texture = device.create_texture(&TextureDescriptor {
            label: None,
            size: output_size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[TextureFormat::Bgra8UnormSrgb],
        });

        Self {
            width,
            height,
            output_texture,
        }
    }
}
