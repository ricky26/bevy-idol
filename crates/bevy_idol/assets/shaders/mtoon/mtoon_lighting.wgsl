#import bevy_pbr::mesh_view_types as mesh_view_types
#import bevy_pbr::mesh_view_bindings as view_bindings
#import bevy_pbr::mesh_types::MESH_FLAGS_SHADOW_RECEIVER_BIT
#import bevy_pbr::mesh_bindings::mesh
#import bevy_pbr::shadows as shadows
#import bevy_pbr::clustered_forward as clustering

#import "shaders/mtoon/mtoon_functions.wgsl" as mtoon_functions

const LUX_TO_FLAT = 0.0001;

struct ShadeInput {
    base_color: vec3<f32>,
    shade_color: vec3<f32>,
    shade_shift: f32,
    shade_toony: f32,
    frag_coord: vec4<f32>,
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    N: vec3<f32>,
    V: vec3<f32>,
    is_orthographic: bool,
    flags: u32,
};

fn shade_input_new() -> ShadeInput {
    var shade_input: ShadeInput;

    shade_input.base_color = vec3<f32>(1.0);
    shade_input.shade_color = vec3<f32>(0.0);

    shade_input.frag_coord = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    shade_input.world_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    shade_input.world_normal = vec3<f32>(0.0, 0.0, 1.0);

    shade_input.is_orthographic = false;

    shade_input.N = vec3<f32>(0.0, 0.0, 1.0);
    shade_input.V = vec3<f32>(1.0, 0.0, 0.0);

    shade_input.flags = 0u;

    return shade_input;
}

fn distance_attenuation(distance_square: f32, inverse_range_squared: f32) -> f32 {
    let factor = distance_square * inverse_range_squared;
    let smooth_factor = saturate(1.0 - factor * factor);
    let attenuation = smooth_factor * smooth_factor;
    return attenuation * 1.0 / max(distance_square, 0.0001);
}

fn shade_light(in: ShadeInput, NoL: f32) -> f32 {
    return smoothstep(in.shade_toony - 1.0, 1.0 - in.shade_toony, NoL + in.shade_shift);
}

fn light_contribution(in: ShadeInput, color: vec3<f32>, attenuation: f32, NoL: f32) -> vec3<f32> {
    let shade = saturate(shade_light(in, NoL));
    let shadow = 1.0; // TODO: implement additional light logic
    return mix(in.shade_color, in.base_color, shade) * shadow * color * LUX_TO_FLAT;
}

fn point_light(in: ShadeInput, light_id: u32, shadow: f32) -> vec3<f32> {
    let light = &view_bindings::clusterable_objects.data[light_id];
    let light_to_frag = (*light).position_radius.xyz - in.world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let range_attenuation = distance_attenuation(distance_square, (*light).color_inverse_square_range.w);

    let L = normalize(light_to_frag);
    let NoL = dot(in.N, L);
    return light_contribution(in, (*light).color_inverse_square_range.rgb, range_attenuation * shadow, NoL);
}

fn spot_light(in: ShadeInput, light_id: u32, shadow: f32) -> vec3<f32> {
    let light = &view_bindings::clusterable_objects.data[light_id];
    let light_to_frag = (*light).position_radius.xyz - in.world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let range_attenuation = distance_attenuation(distance_square, (*light).color_inverse_square_range.w);

    let L = normalize(light_to_frag);
    let NoL = dot(in.N, L);

    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if ((*light).flags & mesh_view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
        spot_dir.y = -spot_dir.y;
    }

    let cd = dot(-spot_dir, normalize(light_to_frag));
    let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
    let spot_attenuation = attenuation * attenuation;

    return light_contribution(in, (*light).color_inverse_square_range.rgb, range_attenuation * spot_attenuation * shadow, NoL);
}

fn directional_light(in: ShadeInput, light_id: u32, shadow: f32) -> vec3<f32> {
    let light = &view_bindings::lights.directional_lights[light_id];
    let L = (*light).direction_to_light.xyz;
    let NoL = dot(in.N, L);
    return light_contribution(in, (*light).color.rgb, shadow, NoL);
}

fn shade(
    in: ShadeInput,
) -> vec3<f32> {
    var output_color = vec3<f32>(0.0);

    let view_z = dot(vec4<f32>(
        view_bindings::view.view_from_world[0].z,
        view_bindings::view.view_from_world[1].z,
        view_bindings::view.view_from_world[2].z,
        view_bindings::view.view_from_world[3].z
    ), in.world_position);
    let cluster_index = clustering::fragment_cluster_index(in.frag_coord.xy, view_z, in.is_orthographic);
    let offset_and_counts = clustering::unpack_offset_and_counts(cluster_index);

    // Point lights (direct)
    for (var i: u32 = offset_and_counts[0]; i < offset_and_counts[0] + offset_and_counts[1]; i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_point_shadow(light_id, in.world_position, in.world_normal);
        }
        let light_contrib = point_light(in, light_id, shadow);
        output_color += light_contrib;
    }

    // Spot lights (direct)
    for (var i: u32 = offset_and_counts[0] + offset_and_counts[1]; i < offset_and_counts[0] + offset_and_counts[1] + offset_and_counts[2]; i = i + 1u) {
        let light_id = clustering::get_clusterable_object_id(i);
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::clusterable_objects.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_spot_shadow(light_id, in.world_position, in.world_normal);
        }
        let light_contrib = spot_light(in, light_id, shadow);
        output_color += light_contrib;
    }

    // directional lights (direct)
    let n_directional_lights = view_bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, in.world_normal, view_z);
        }
        var light_contrib = directional_light(in, i, shadow);
#ifdef DIRECTIONAL_LIGHT_SHADOW_MAP_DEBUG_CASCADES
        light_contrib = shadows::cascade_debug_visualization(light_contrib, i, view_z);
#endif
        output_color += light_contrib;
    }

    output_color += view_bindings::lights.ambient_color.rgb * in.base_color.rgb * LUX_TO_FLAT;
    return output_color;
}
