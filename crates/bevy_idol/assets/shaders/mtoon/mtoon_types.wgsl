const MTOON_FLAGS_BASE_COLOR_TEXTURE_BIT: u32 = 1u;
const MTOON_FLAGS_EMISSIVE_TEXTURE_BIT: u32 = 2u;
const MTOON_FLAGS_NORMAL_TEXTURE_BIT: u32 = 4u;
const MTOON_FLAGS_TWO_COMPONENT_NORMAL_MAP_BIT: u32 = 8u;
const MTOON_FLAGS_SHADE_COLOR_TEXTURE_BIT: u32 = 16u;
const MTOON_FLAGS_SHADE_SHIFT_TEXTURE_BIT: u32 = 32u;
const MTOON_FLAGS_MATCAP_TEXTURE_BIT: u32 = 64u;
const MTOON_FLAGS_RIM_TEXTURE_BIT: u32 = 128u;
const MTOON_FLAGS_UV_ANIM_MASK_TEXTURE_BIT: u32 = 256u;
const MTOON_FLAGS_DOUBLE_SIDED_BIT: u32 = 512u;
const MTOON_FLAGS_FOG_ENABLED_BIT: u32 = 1024u;
const MTOON_FLAGS_ALPHA_MODE_RESERVED_BITS: u32 = 3758096384u;
const MTOON_FLAGS_ALPHA_MODE_OPAQUE: u32 = 0u;
const MTOON_FLAGS_ALPHA_MODE_MASK: u32 = 536870912u;
const MTOON_FLAGS_ALPHA_MODE_BLEND: u32 = 1073741824u;
const MTOON_FLAGS_ALPHA_MODE_PREMULTIPLIED: u32 = 1610612736u;
const MTOON_FLAGS_ALPHA_MODE_ADD: u32 = 2147483648u;
const MTOON_FLAGS_ALPHA_MODE_MULTIPLY: u32 = 2684354560u;

struct MToonMaterial {
    flags: u32,
    base_color: vec4<f32>,
    shade_color: vec4<f32>,
    emissive: vec4<f32>,
    alpha_cutoff: f32,
    shading_shift_factor: f32,
    shading_shift_scale: f32,
    shading_toony_factor: f32,
    gi_equalization_factor: f32,
    matcap_factor: vec3<f32>,
    parametric_rim_color_factor: vec3<f32>,
    rim_lighting_mix_factor: f32,
    parametric_rim_fresnel_power_factor: f32,
    parametric_rim_lift_factor: f32,
    uv_animation_scroll_x_speed_factor: f32,
    uv_animation_scroll_y_speed_factor: f32,
    uv_animation_rotation_speed_factor: f32,
};

