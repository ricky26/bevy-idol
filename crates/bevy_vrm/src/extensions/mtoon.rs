use bevy::math::Vec3;
use serde::{Deserialize, Serialize};
use crate::extensions::TextureInfo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MToonExtensionJson {
    pub spec_version: String,
    pub transparent_with_z_write: bool,
    pub render_queue_offset_number: i32,
    pub shade_color_factor: Vec3,
    pub shade_multiply_texture: TextureInfo,
    pub shading_shift_factor: f32,
    pub shading_toony_factor: f32,
    pub gi_equalization_factor: f32,
    // pub
}

impl Default for MToonExtensionJson {
    fn default() -> Self {
        Self {
            spec_version: String::new(),
            transparent_with_z_write: false,
            render_queue_offset_number: 0,
            shade_color_factor: Vec3::ONE,
            shade_multiply_texture: Default::default(),
            shading_shift_factor: 0.0,
            shading_toony_factor: 0.9,
            gi_equalization_factor: 0.9,
        }
    }
}
