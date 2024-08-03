#import "shaders/mtoon/mtoon_types.wgsl" as mtoon_types
#import "shaders/mtoon/mtoon_types.wgsl"::MToonMaterial
#import "shaders/mtoon/mtoon_bindings.wgsl" as mtoon_bindings

fn alpha_discard(material: MToonMaterial, output_color: vec4<f32>) -> f32 {
    var color = output_color;
    let alpha_mode = material.flags & mtoon_types::MTOON_FLAGS_ALPHA_MODE_RESERVED_BITS;
    if alpha_mode == mtoon_types::MTOON_FLAGS_ALPHA_MODE_OPAQUE {
        // NOTE: If rendering as opaque, alpha should be ignored so set to 1.0
        return 1.0;
    }

#ifdef MAY_DISCARD
    else if alpha_mode == mtoon_types::MTOON_FLAGS_ALPHA_MODE_MASK {
        if color.a >= material.alpha_cutoff {
            // NOTE: If rendering as masked alpha and >= the cutoff, render as fully opaque
            return 1.0;
        } else {
            // NOTE: output_color.a < in.material.alpha_cutoff should not be rendered
            discard;
        }
    }
#endif

    return color.a;
}

fn apply_normal_mapping(
    material_flags: u32,
    world_normal: vec3<f32>,
#ifdef VERTEX_TANGENTS
    world_tangent: vec4<f32>,
#endif
#ifdef VERTEX_UVS
    uv: vec2<f32>,
    texture_normal: vec3<f32>,
#endif
    mip_bias: f32,
) -> vec3<f32> {
    var N: vec3<f32> = world_normal;

#ifdef VERTEX_TANGENTS
    var T: vec3<f32> = world_tangent.xyz;
    var B: vec3<f32> = world_tangent.w * cross(N, T);
#endif

#ifdef VERTEX_TANGENTS
#ifdef VERTEX_UVS
    if (material_flags & mtoon_types::MTOON_FLAGS_NORMAL_TEXTURE_BIT) != 0u {
        var Nt = texture_normal;
        if (material_flags & mtoon_types::MTOON_FLAGS_TWO_COMPONENT_NORMAL_MAP_BIT) != 0u {
            Nt = vec3<f32>(Nt.rg * 2.0 - 1.0, 0.0);
            Nt.z = sqrt(1.0 - Nt.x * Nt.x - Nt.y * Nt.y);
        } else {
            Nt = Nt * 2.0 - 1.0;
        }
        N = Nt.x * T + Nt.y * B + Nt.z * N;
    }
#endif
#endif

    return normalize(N);
}

