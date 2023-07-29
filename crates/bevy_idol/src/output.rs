use bevy::prelude::Resource;
use bevy::render::render_resource::{Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::RenderDevice;
use wgpu_hal::vulkan::{AsExternalMemoryRequest};

use idol_api::TextureResponse;

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

    pub fn export(&self, device: &RenderDevice) -> Option<TextureResponse> {
        let mut result = None;
        unsafe {
            let mut request = None;

            self.output_texture.as_hal::<wgpu_hal::api::Vulkan, _>(|h| {
                if let Some(texture) = h {
                    request = texture.as_external_memory_request()
                }
            });

            if let Some(request) = request {
                device.wgpu_device().as_hal::<wgpu_hal::api::Vulkan, _, _>(|h| {
                    let device = match h {
                        Some(x) => x,
                        None => return,
                    };

                    if let Some(mem) = device.create_external_memory_fd(request).unwrap() {
                        result = Some(TextureResponse {
                            fd: mem.fd,
                            width: self.width,
                            height: self.height,
                        });
                    }
                })
            }
        }
        result
    }
}
