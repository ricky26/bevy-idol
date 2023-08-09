use bevy::asset::Handle;
use bevy::math::{Vec3, Vec4};
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::{AlphaMode, Color, Image, ReflectDefault};
use bevy::reflect::{Reflect, TypeUuid};
use bevy::render::mesh::MeshVertexBufferLayout;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{AsBindGroup, AsBindGroupShaderType, Face, RenderPipelineDescriptor, ShaderRef, ShaderType, SpecializedMeshPipelineError, TextureFormat};
use serde::{Deserialize, Serialize};

use crate::extensions::TextureInfo;

#[derive(Clone, Debug, Reflect, Serialize, Deserialize)]
#[reflect(Debug, Default)]
pub struct ShadingShiftTextureInfo {
    #[serde(flatten)]
    #[reflect(ignore)]
    pub texture_info: TextureInfo,
    pub scale: f32,
}

impl Default for ShadingShiftTextureInfo {
    fn default() -> Self {
        Self {
            texture_info: Default::default(),
            scale: 1.,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, Reflect, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[reflect(Debug, Default)]
pub enum OutlineWidthMode {
    #[default]
    None,
    WorldCoordinates,
    ScreenCoordinates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct MToonExtensionJson {
    pub spec_version: String,
    pub transparent_with_z_write: bool,
    pub render_queue_offset_number: i32,
    pub shade_color_factor: Vec3,
    pub shade_multiply_texture: Option<TextureInfo>,
    pub shading_shift_factor: f32,
    pub shading_shift_texture: Option<ShadingShiftTextureInfo>,
    pub shading_toony_factor: f32,
    pub gi_equalization_factor: f32,
    pub matcap_factor: Vec3,
    pub matcap_texture: Option<TextureInfo>,
    pub parametric_rim_color_factor: Vec3,
    pub rim_multiply_texture: Option<TextureInfo>,
    pub rim_lighting_mix_factor: f32,
    pub parametric_rim_fresnel_power_factor: f32,
    pub parametric_rim_lift_factor: f32,
    pub outline_width_mode: OutlineWidthMode,
    pub outline_width_factor: f32,
    pub outline_width_multiply_texture: Option<TextureInfo>,
    pub outline_color_factor: Vec3,
    pub outline_lighting_mix_factor: f32,
    pub uv_animation_mask_texture: Option<TextureInfo>,
    pub uv_animation_scroll_x_speed_factor: f32,
    pub uv_animation_scroll_y_speed_factor: f32,
    pub uv_animation_rotation_speed_factor: f32,
}

impl Default for MToonExtensionJson {
    fn default() -> Self {
        Self {
            spec_version: String::new(),
            transparent_with_z_write: false,
            render_queue_offset_number: 0,
            shade_color_factor: Vec3::ONE,
            shade_multiply_texture: None,
            shading_shift_factor: 0.0,
            shading_shift_texture: None,
            shading_toony_factor: 0.9,
            gi_equalization_factor: 0.9,
            matcap_factor: Vec3::ZERO,
            matcap_texture: None,
            parametric_rim_color_factor: Vec3::ZERO,
            rim_multiply_texture: None,
            rim_lighting_mix_factor: 0.0,
            parametric_rim_fresnel_power_factor: 1.0,
            parametric_rim_lift_factor: 0.0,
            outline_width_mode: OutlineWidthMode::None,
            outline_width_factor: 0.0,
            outline_width_multiply_texture: None,
            outline_color_factor: Vec3::ZERO,
            outline_lighting_mix_factor: 1.0,
            uv_animation_mask_texture: None,
            uv_animation_scroll_x_speed_factor: 0.0,
            uv_animation_scroll_y_speed_factor: 0.0,
            uv_animation_rotation_speed_factor: 0.0,
        }
    }
}

#[derive(AsBindGroup, TypeUuid, Debug, Clone, Reflect)]
#[uuid = "39201294-87dc-4b8d-b300-967bdc96cd92"]
#[bind_group_data(MToonMaterialKey)]
#[uniform(0, MToonMaterialUniform)]
#[reflect(Debug, Default)]
pub struct MToonMaterial {
    pub alpha_mode: AlphaMode,
    pub double_sided: bool,
    pub fog_enabled: bool,
    #[reflect(ignore)]
    pub cull_mode: Option<Face>,
    pub transparent_with_z_write: bool,
    pub depth_bias: i32,
    pub base_color: Color,
    #[texture(1)]
    #[sampler(2)]
    pub base_color_texture: Option<Handle<Image>>,
    pub emissive: Color,
    #[texture(3)]
    #[sampler(4)]
    pub emissive_texture: Option<Handle<Image>>,
    pub shade_color: Color,
    #[texture(5)]
    #[sampler(6)]
    pub shade_color_texture: Option<Handle<Image>>,
    pub shading_shift_factor: f32,
    pub shading_shift_scale: f32,
    #[texture(7)]
    #[sampler(8)]
    pub shading_shift_texture: Option<Handle<Image>>,
    pub shading_toony_factor: f32,
    #[texture(9)]
    #[sampler(10)]
    pub normal_map_texture: Option<Handle<Image>>,
    pub gi_equalization_factor: f32,
    pub matcap_factor: Vec3,
    #[texture(11)]
    #[sampler(12)]
    pub matcap_texture: Option<Handle<Image>>,
    pub parametric_rim_color_factor: Vec3,
    #[texture(13)]
    #[sampler(14)]
    pub rim_color_texture: Option<Handle<Image>>,
    pub rim_lighting_mix_factor: f32,
    pub parametric_rim_fresnel_power_factor: f32,
    pub parametric_rim_lift_factor: f32,
    // pub outline_width_mode: OutlineWidthMode,
    // pub outline_width_factor: f32,
    // #[texture(15)]
    // #[sampler(16)]
    // pub outline_width_multiply_texture: Option<Handle<Image>>,
    // pub outline_color_factor: Vec3,
    // pub outline_lighting_mix_factor: f32,
    #[texture(15)]
    #[sampler(16)]
    pub uv_animation_mask_texture: Option<Handle<Image>>,
    pub uv_animation_scroll_x_speed_factor: f32,
    pub uv_animation_scroll_y_speed_factor: f32,
    pub uv_animation_rotation_speed_factor: f32,
}

impl Default for MToonMaterial {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::Opaque,
            double_sided: false,
            cull_mode: Some(Face::Back),
            fog_enabled: true,
            transparent_with_z_write: false,
            depth_bias: 0,
            base_color: Color::WHITE,
            base_color_texture: None,
            emissive: Color::NONE,
            emissive_texture: None,
            shade_color: Color::BLACK,
            shade_color_texture: None,
            shading_shift_factor: 0.0,
            shading_shift_scale: 1.0,
            shading_shift_texture: None,
            shading_toony_factor: 0.9,
            normal_map_texture: None,
            gi_equalization_factor: 0.9,
            matcap_factor: Vec3::ZERO,
            matcap_texture: None,
            parametric_rim_color_factor: Vec3::ZERO,
            rim_color_texture: None,
            rim_lighting_mix_factor: 0.0,
            parametric_rim_fresnel_power_factor: 1.0,
            parametric_rim_lift_factor: 0.0,
            // outline_width_mode: OutlineWidthMode::None,
            // outline_width_factor: 0.0,
            // outline_width_multiply_texture: None,
            // outline_color_factor: Vec3::ZERO,
            // outline_lighting_mix_factor: 1.0,
            uv_animation_mask_texture: None,
            uv_animation_scroll_x_speed_factor: 0.0,
            uv_animation_scroll_y_speed_factor: 0.0,
            uv_animation_rotation_speed_factor: 0.0,
        }
    }
}

impl Material for MToonMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/mtoon/mtoon.wgsl".into()
    }

    #[inline]
    fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    #[inline]
    fn depth_bias(&self) -> f32 {
        self.depth_bias as f32
    }

    fn specialize(
        _pipeline: &MaterialPipeline<Self>,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayout,
        key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = key.bind_group_data.cull_mode;

        // if let Some(fragment) = descriptor.fragment.as_mut() {
        //     let shader_defs = &mut fragment.shader_defs;
        //
        //     match key.bind_group_data.outline_width_mode {
        //         OutlineWidthMode::None => {},
        //         OutlineWidthMode::ScreenCoordinates =>
        //             shader_defs.push("OUTLINE_WIDTH_SCREEN".into()),
        //         OutlineWidthMode::WorldCoordinates =>
        //             shader_defs.push("OUTLINE_WIDTH_WORLD".into()),
        //     }
        // }

        if let Some(depth_stencil) = descriptor.depth_stencil.as_mut() {
            depth_stencil.bias.constant = key.bind_group_data.depth_bias;
        }

        Ok(())
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct MToonMaterialFlags: u32 {
        const BASE_COLOR_TEXTURE         = (1 << 0);
        const EMISSIVE_TEXTURE           = (1 << 1);
        const NORMAL_TEXTURE             = (1 << 2);
        const TWO_COMPONENT_NORMAL_MAP   = (1 << 3);
        const SHADE_COLOR_TEXTURE        = (1 << 4);
        const SHADE_SHIFT_TEXTURE        = (1 << 5);
        const MATCAP_TEXTURE             = (1 << 6);
        const RIM_TEXTURE                = (1 << 7);
        const UV_ANIM_MASK_TEXTURE       = (1 << 8);
        const DOUBLE_SIDED               = (1 << 9);
        const FOG_ENABLED                = (1 << 10);
        const ALPHA_MODE_RESERVED_BITS   = (Self::ALPHA_MODE_MASK_BITS << Self::ALPHA_MODE_SHIFT_BITS); // ← Bitmask reserving bits for the `AlphaMode`
        const ALPHA_MODE_OPAQUE          = (0 << Self::ALPHA_MODE_SHIFT_BITS);                          // ← Values are just sequential values bitshifted into
        const ALPHA_MODE_MASK            = (1 << Self::ALPHA_MODE_SHIFT_BITS);                          //   the bitmask, and can range from 0 to 7.
        const ALPHA_MODE_BLEND           = (2 << Self::ALPHA_MODE_SHIFT_BITS);                          //
        const ALPHA_MODE_PREMULTIPLIED   = (3 << Self::ALPHA_MODE_SHIFT_BITS);                          //
        const ALPHA_MODE_ADD             = (4 << Self::ALPHA_MODE_SHIFT_BITS);                          //   Right now only values 0–5 are used, which still gives
        const ALPHA_MODE_MULTIPLY        = (5 << Self::ALPHA_MODE_SHIFT_BITS);                          // ← us "room" for two more modes without adding more bits
        const NONE                       = 0;
        const UNINITIALIZED              = 0xFFFF;
    }
}

impl MToonMaterialFlags {
    const ALPHA_MODE_MASK_BITS: u32 = 0b111;
    const ALPHA_MODE_SHIFT_BITS: u32 = 32 - Self::ALPHA_MODE_MASK_BITS.count_ones();
}

#[derive(Clone, Default, ShaderType)]
pub struct MToonMaterialUniform {
    pub flags: u32,
    pub base_color: Vec4,
    pub shade_color: Color,
    pub emissive: Vec4,
    pub alpha_cutoff: f32,
    pub shading_shift_factor: f32,
    pub shading_shift_scale: f32,
    pub shading_toony_factor: f32,
    pub gi_equalization_factor: f32,
    pub matcap_factor: Vec3,
    pub parametric_rim_color_factor: Vec3,
    pub rim_lighting_mix_factor: f32,
    pub parametric_rim_fresnel_power_factor: f32,
    pub parametric_rim_lift_factor: f32,
    // pub outline_width_factor: f32,
    // pub outline_color_factor: Vec3,
    // pub outline_lighting_mix_factor: f32,
    pub uv_animation_scroll_x_speed_factor: f32,
    pub uv_animation_scroll_y_speed_factor: f32,
    pub uv_animation_rotation_speed_factor: f32,
}

impl AsBindGroupShaderType<MToonMaterialUniform> for MToonMaterial {
    fn as_bind_group_shader_type(&self, images: &RenderAssets<Image>) -> MToonMaterialUniform {
        let mut flags = MToonMaterialFlags::NONE;

        if self.base_color_texture.is_some() {
            flags |= MToonMaterialFlags::BASE_COLOR_TEXTURE;
        }

        if self.emissive_texture.is_some() {
            flags |= MToonMaterialFlags::EMISSIVE_TEXTURE;
        }

        if self.shade_color_texture.is_some() {
            flags |= MToonMaterialFlags::SHADE_COLOR_TEXTURE;
        }

        if self.shading_shift_texture.is_some() {
            flags |= MToonMaterialFlags::SHADE_SHIFT_TEXTURE;
        }

        if self.matcap_texture.is_some() {
            flags |= MToonMaterialFlags::MATCAP_TEXTURE;
        }

        if self.rim_color_texture.is_some() {
            flags |= MToonMaterialFlags::RIM_TEXTURE;
        }

        if self.uv_animation_mask_texture.is_some() {
            flags |= MToonMaterialFlags::UV_ANIM_MASK_TEXTURE;
        }

        if self.double_sided {
            flags |= MToonMaterialFlags::DOUBLE_SIDED;
        }

        if self.fog_enabled {
            flags |= MToonMaterialFlags::FOG_ENABLED;
        }

        let has_normal_map = self.normal_map_texture.is_some();
        if has_normal_map {
            if let Some(texture) = images.get(self.normal_map_texture.as_ref().unwrap()) {
                match texture.texture_format {
                    TextureFormat::Rg8Unorm
                    | TextureFormat::Rg16Unorm
                    | TextureFormat::Bc5RgUnorm
                    | TextureFormat::EacRg11Unorm => {
                        flags |= MToonMaterialFlags::TWO_COMPONENT_NORMAL_MAP;
                    }
                    _ => {}
                }
            }
        }

        let mut alpha_cutoff = 0.5;
        match self.alpha_mode {
            AlphaMode::Opaque => flags |= MToonMaterialFlags::ALPHA_MODE_OPAQUE,
            AlphaMode::Mask(c) => {
                alpha_cutoff = c;
                flags |= MToonMaterialFlags::ALPHA_MODE_MASK;
            }
            AlphaMode::Blend => flags |= MToonMaterialFlags::ALPHA_MODE_BLEND,
            AlphaMode::Premultiplied => flags |= MToonMaterialFlags::ALPHA_MODE_PREMULTIPLIED,
            AlphaMode::Add => flags |= MToonMaterialFlags::ALPHA_MODE_ADD,
            AlphaMode::Multiply => flags |= MToonMaterialFlags::ALPHA_MODE_MULTIPLY,
        };

        MToonMaterialUniform {
            flags: flags.bits(),
            base_color: self.base_color.into(),
            emissive: self.emissive.into(),
            alpha_cutoff,
            shade_color: self.shade_color,
            shading_shift_factor: self.shading_shift_factor,
            shading_shift_scale: self.shading_shift_scale,
            shading_toony_factor: self.shading_toony_factor,
            gi_equalization_factor: self.gi_equalization_factor,
            matcap_factor: self.matcap_factor,
            parametric_rim_color_factor: self.parametric_rim_color_factor,
            rim_lighting_mix_factor: self.rim_lighting_mix_factor,
            parametric_rim_fresnel_power_factor: self.parametric_rim_fresnel_power_factor,
            parametric_rim_lift_factor: self.parametric_rim_lift_factor,
            // outline_width_factor: self.outline_width_factor,
            // outline_color_factor: self.outline_color_factor,
            // outline_lighting_mix_factor: self.rim_lighting_mix_factor,
            uv_animation_scroll_x_speed_factor: self.uv_animation_scroll_x_speed_factor,
            uv_animation_scroll_y_speed_factor: self.uv_animation_scroll_y_speed_factor,
            uv_animation_rotation_speed_factor: self.uv_animation_rotation_speed_factor,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MToonMaterialKey {
    cull_mode: Option<Face>,
    depth_bias: i32,
    // outline_width_mode: OutlineWidthMode,
}

impl From<&MToonMaterial> for MToonMaterialKey {
    fn from(material: &MToonMaterial) -> Self {
        MToonMaterialKey {
            cull_mode: material.cull_mode,
            depth_bias: material.depth_bias,
            // outline_width_mode: material.outline_width_mode,
        }
    }
}

