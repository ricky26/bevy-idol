//#import bevy_pbr::utils PI
#import bevy_pbr::mesh_view_types as mesh_view_types
#import bevy_pbr::mesh_view_bindings as view_bindings
#import bevy_pbr::mesh_types MESH_FLAGS_SHADOW_RECEIVER_BIT
#import bevy_pbr::mesh_bindings mesh
#import bevy_pbr::shadows as shadows
#import bevy_pbr::clustered_forward as clustering

#import "shaders/mtoon/mtoon_functions.wgsl" as mtoon_functions
#import "shaders/mtoon/mtoon_types.wgsl" as mtoon_types

struct ShadeInput {
    material: mtoon_types::MToonMaterial,
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

    shade_input.frag_coord = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    shade_input.world_position = vec4<f32>(0.0, 0.0, 0.0, 1.0);
    shade_input.world_normal = vec3<f32>(0.0, 0.0, 1.0);

    shade_input.is_orthographic = false;

    shade_input.N = vec3<f32>(0.0, 0.0, 1.0);
    shade_input.V = vec3<f32>(1.0, 0.0, 0.0);

    shade_input.flags = 0u;

    return shade_input;
}

fn getDistanceAttenuation(distanceSquare: f32, inverseRangeSquared: f32) -> f32 {
    let factor = distanceSquare * inverseRangeSquared;
    let smoothFactor = saturate(1.0 - factor * factor);
    let attenuation = smoothFactor * smoothFactor;
    return attenuation * 1.0 / max(distanceSquare, 0.0001);
}

fn point_light(world_position: vec3<f32>, light_id: u32, N: vec3<f32>) -> vec3<f32> {
    let light = &view_bindings::point_lights.data[light_id];
    let light_to_frag = (*light).position_radius.xyz - world_position.xyz;
    let distance_square = dot(light_to_frag, light_to_frag);
    let rangeAttenuation = getDistanceAttenuation(distance_square, (*light).color_inverse_square_range.w);

    let L = normalize(light_to_frag);
    let NoL = saturate(dot(N, L));
    return NoL * rangeAttenuation * (*light).color_inverse_square_range.rgb;
}

fn spot_light(world_position: vec3<f32>, light_id: u32, N: vec3<f32>) -> vec3<f32> {
    let point_light = point_light(world_position, light_id, N);
    let light = &view_bindings::point_lights.data[light_id];

    var spot_dir = vec3<f32>((*light).light_custom_data.x, 0.0, (*light).light_custom_data.y);
    spot_dir.y = sqrt(max(0.0, 1.0 - spot_dir.x * spot_dir.x - spot_dir.z * spot_dir.z));
    if ((*light).flags & mesh_view_types::POINT_LIGHT_FLAGS_SPOT_LIGHT_Y_NEGATIVE) != 0u {
        spot_dir.y = -spot_dir.y;
    }
    let light_to_frag = (*light).position_radius.xyz - world_position.xyz;

    let cd = dot(-spot_dir, normalize(light_to_frag));
    let attenuation = saturate(cd * (*light).light_custom_data.z + (*light).light_custom_data.w);
    let spot_attenuation = attenuation * attenuation;

    return point_light * spot_attenuation;
}

fn directional_light(light_id: u32, N: vec3<f32>) -> vec3<f32> {
    let light = &view_bindings::lights.directional_lights[light_id];
    let L = (*light).direction_to_light.xyz;

    let NoL = saturate(dot(N, L));
    return NoL * (*light).color.rgb;
}

fn shade(
    in: ShadeInput,
) -> vec4<f32> {
    var base_color: vec4<f32> = in.material.base_color;
    let alpha = mtoon_functions::alpha_discard(in.material, base_color).a;
    var output_color: vec4<f32> = vec4<f32>(0.0, 0.0, 0.0, alpha);

    let view_z = dot(vec4<f32>(
        view_bindings::view.inverse_view[0].z,
        view_bindings::view.inverse_view[1].z,
        view_bindings::view.inverse_view[2].z,
        view_bindings::view.inverse_view[3].z
    ), in.world_position);
    let cluster_index = clustering::fragment_cluster_index(in.frag_coord.xy, view_z, in.is_orthographic);
    let offset_and_counts = clustering::unpack_offset_and_counts(cluster_index);

    // Point lights (direct)
    for (var i: u32 = offset_and_counts[0]; i < offset_and_counts[0] + offset_and_counts[1]; i = i + 1u) {
        let light_id = clustering::get_light_id(i);
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::point_lights.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_point_shadow(light_id, in.world_position, in.world_normal);
        }
        let light_contrib = point_light(in.world_position.xyz, light_id, in.N);
        output_color = max(output_color, vec4(light_contrib * shadow, 0.0));
    }

    // Spot lights (direct)
    for (var i: u32 = offset_and_counts[0] + offset_and_counts[1]; i < offset_and_counts[0] + offset_and_counts[1] + offset_and_counts[2]; i = i + 1u) {
        let light_id = clustering::get_light_id(i);

        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::point_lights.data[light_id].flags & mesh_view_types::POINT_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_spot_shadow(light_id, in.world_position, in.world_normal);
        }
        let light_contrib = spot_light(in.world_position.xyz, light_id, in.N);
        output_color = max(output_color, vec4(light_contrib * shadow, 0.0));
    }

    // directional lights (direct)
    let n_directional_lights = view_bindings::lights.n_directional_lights;
    for (var i: u32 = 0u; i < n_directional_lights; i = i + 1u) {
        var shadow: f32 = 1.0;
        if ((in.flags & MESH_FLAGS_SHADOW_RECEIVER_BIT) != 0u
                && (view_bindings::lights.directional_lights[i].flags & mesh_view_types::DIRECTIONAL_LIGHT_FLAGS_SHADOWS_ENABLED_BIT) != 0u) {
            shadow = shadows::fetch_directional_shadow(i, in.world_position, in.world_normal, view_z);
        }
        var light_contrib = directional_light(i, in.N);
#ifdef DIRECTIONAL_LIGHT_SHADOW_MAP_DEBUG_CASCADES
        light_contrib = shadows::cascade_debug_visualization(light_contrib, i, view_z);
#endif
        output_color = max(output_color, vec4(light_contrib * shadow, 0.0));
    }

    output_color += vec4(in.material.emissive.rgb, 0.0);
    output_color += vec4(view_bindings::lights.ambient_color.rgb, 0.0);
    return saturate(output_color);
}
