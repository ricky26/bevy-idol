#import "shaders/mtoon/mtoon_types.wgsl"::MToonMaterial

@group(2) @binding(0)
var<uniform> material: MToonMaterial;
@group(2) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(2) @binding(2)
var base_color_sampler: sampler;
@group(2) @binding(3)
var emissive_texture: texture_2d<f32>;
@group(2) @binding(4)
var emissive_sampler: sampler;
@group(2) @binding(5)
var shade_color_texture: texture_2d<f32>;
@group(2) @binding(6)
var shade_color_sampler: sampler;
@group(2) @binding(7)
var shading_shift_texture: texture_2d<f32>;
@group(2) @binding(8)
var shading_shift_sampler: sampler;
@group(2) @binding(9)
var normal_map_texture: texture_2d<f32>;
@group(2) @binding(10)
var normal_map_sampler: sampler;
@group(2) @binding(11)
var matcap_texture: texture_2d<f32>;
@group(2) @binding(12)
var matcap_sampler: sampler;
@group(2) @binding(13)
var rim_multiply_texture: texture_2d<f32>;
@group(2) @binding(14)
var rim_multiply_sampler: sampler;
@group(2) @binding(15)
var uv_animation_mask_texture: texture_2d<f32>;
@group(2) @binding(16)
var uv_animation_mask_sampler: sampler;

