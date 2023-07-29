use ash::vk;
use ash::vk::{ExternalMemoryHandleTypeFlags, StructureType};
use bevy::prelude::Resource;
use bevy::render::render_resource::{Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages};
use bevy::render::renderer::{RenderDevice, RenderInstance};

use idol_api::TextureResponse;

// HACK: The memory bounds aren't accessible in the wgpu API
pub struct VkTextureHack {
    _raw: vk::Image,
    _drop_guard: Option<Box<dyn std::any::Any + Send + Sync>>,
    block: Option<gpu_alloc::MemoryBlock<vk::DeviceMemory>>,
}

#[derive(Resource)]
pub struct OutputTexture {
    pub width: u32,
    pub height: u32,
    pub output_texture: Texture,
}

impl OutputTexture {
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

    pub fn export(&self, instance: &RenderInstance, device: &RenderDevice) -> Option<TextureResponse> {
        let mut response = None;

        unsafe {
            if let Some(instance) = instance.as_hal::<wgpu_hal::vulkan::Api>() {
                let instance = instance.shared_instance().raw_instance();
                device.wgpu_device().as_hal::<wgpu_hal::api::Vulkan, _, _>(|h| {
                    let Some(device) = h else {
                        return;
                    };
                    let device = device.raw_device();
                    let factory = ash::extensions::khr::ExternalMemoryFd::new(instance, device);

                    self.output_texture.as_hal::<wgpu_hal::api::Vulkan, _>(|h| {
                        let Some(texture) = h else {
                            return;
                        };

                        let texture_hack: &VkTextureHack = std::mem::transmute(texture);
                        if let Some(block) = texture_hack.block.as_ref() {
                            if let Ok(fd) = factory.get_memory_fd(&vk::MemoryGetFdInfoKHR{
                                s_type: StructureType::MEMORY_GET_FD_INFO_KHR,
                                p_next: std::ptr::null(),
                                memory: *block.memory(),
                                handle_type: ExternalMemoryHandleTypeFlags::DMA_BUF_EXT,
                            }) {
                                response = Some(TextureResponse {
                                    fd,
                                    width: self.width,
                                    height: self.height,
                                });
                            }
                        }
                    });
                });
            }
        }

        response
    }
}
