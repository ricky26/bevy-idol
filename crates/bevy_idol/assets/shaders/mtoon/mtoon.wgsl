#import bevy_pbr::pbr_functions as pbr_functions
#import bevy_pbr::pbr_types as pbr_types
#import bevy_pbr::prepass_utils

#import bevy_pbr::mesh_vertex_output MeshVertexOutput
#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::mesh_view_bindings view, fog, screen_space_ambient_occlusion_texture
#import bevy_pbr::mesh_view_types FOG_MODE_OFF
#import bevy_core_pipeline::tonemapping screen_space_dither, powsafe, tone_mapping
#import bevy_pbr::parallax_mapping parallaxed_uv

#ifdef SCREEN_SPACE_AMBIENT_OCCLUSION
#import bevy_pbr::gtao_utils gtao_multibounce
#endif

#import "shaders/mtoon/mtoon_types.wgsl" as mtoon_types
#import "shaders/mtoon/mtoon_bindings.wgsl" as mtoon_bindings
#import "shaders/mtoon/mtoon_functions.wgsl" as mtoon_functions
#import "shaders/mtoon/mtoon_lighting.wgsl" ShadeInput, shade_input_new, shade

@fragment
fn fragment(
    in: MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    var base_color: vec4<f32> = mtoon_bindings::material.base_color;
    var shade_color: vec4<f32> = mtoon_bindings::material.shade_color;
    var shading_shift: f32 = mtoon_bindings::material.shading_shift_factor;
    var shading_toony_factor: f32 = mtoon_bindings::material.shading_toony_factor;

    let is_orthographic = view.projection[3].w == 1.0;
    let V = pbr_functions::calculate_view(in.world_position, is_orthographic);
#ifdef VERTEX_UVS
    var uv = in.uv;
#endif

#ifdef VERTEX_COLORS
    base_color = base_color * in.color;
#endif
#ifdef VERTEX_UVS
    if ((mtoon_bindings::material.flags & mtoon_types::MTOON_FLAGS_BASE_COLOR_TEXTURE_BIT) != 0u) {
        base_color *= textureSampleBias(mtoon_bindings::base_color_texture, mtoon_bindings::base_color_sampler, uv, view.mip_bias);
    }
    if ((mtoon_bindings::material.flags & mtoon_types::MTOON_FLAGS_SHADE_SHIFT_TEXTURE_BIT) != 0u) {
        shading_shift += mtoon_bindings::material.shading_shift_scale * textureSampleBias(
            mtoon_bindings::shading_shift_texture, mtoon_bindings::shading_shift_sampler, uv, view.mip_bias).r;
    }
    if ((mtoon_bindings::material.flags & mtoon_types::MTOON_FLAGS_SHADE_COLOR_TEXTURE_BIT) != 0u) {
        shade_color *= textureSampleBias(mtoon_bindings::shade_color_texture, mtoon_bindings::shade_color_sampler, uv, view.mip_bias);
    }
#endif

    var shade_input: ShadeInput;
    shade_input.flags = mesh[in.instance_index].flags;
    shade_input.V = V;
    shade_input.frag_coord = in.position;
    shade_input.world_position = in.world_position;
    shade_input.is_orthographic = is_orthographic;
    shade_input.world_normal = pbr_functions::prepare_world_normal(
        in.world_normal,
        (mtoon_bindings::material.flags & mtoon_types::MTOON_FLAGS_DOUBLE_SIDED_BIT) != 0u,
        is_front,
    );

#ifdef LOAD_PREPASS_NORMALS
    shade_input.N = bevy_pbr::prepass_utils::prepass_normal(in.position, 0u);
#else
    shade_input.N = mtoon_functions::apply_normal_mapping(
        mtoon_bindings::material.flags,
        shade_input.world_normal,
#ifdef VERTEX_TANGENTS
#ifdef STANDARDMATERIAL_NORMAL_MAP
        in.world_tangent,
#endif
#endif
#ifdef VERTEX_UVS
        uv,
#endif
        view.mip_bias,
    );
#endif

    var shading: vec4<f32> = shade(shade_input);
    shading += shading_shift;
    shading = smoothstep(vec4(shading_toony_factor - 1.0), vec4(1.0 - shading_toony_factor), saturate(shading));

    var output_color: vec4<f32> = mix(shade_color, base_color, shading);

    if (fog.mode != FOG_MODE_OFF && (mtoon_bindings::material.flags & mtoon_types::MTOON_FLAGS_FOG_ENABLED_BIT) != 0u) {
        output_color = pbr_functions::apply_fog(fog, output_color, in.world_position.xyz, view.world_position.xyz);
    }

#ifdef TONEMAP_IN_SHADER
    output_color = tone_mapping(output_color, view.color_grading);
#ifdef DEBAND_DITHER
    var output_rgb = output_color.rgb;
    output_rgb = powsafe(output_rgb, 1.0 / 2.2);
    output_rgb = output_rgb + screen_space_dither(in.position.xy);
    // This conversion back to linear space is required because our output texture format is
    // SRGB; the GPU will assume our output is linear and will apply an SRGB conversion.
    output_rgb = powsafe(output_rgb, 2.2);
    output_color = vec4(output_rgb, output_color.a);
#endif
#endif
#ifdef PREMULTIPLY_ALPHA
    // This works because the alpha flags are in the same bits as in the standard shader.
    output_color = pbr_functions::premultiply_alpha(mtoon_bindings::material.flags, output_color);
#endif
    return output_color;
}
