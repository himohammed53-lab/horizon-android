// Block shadows from a shadow map, cool calm colours, and water with waves.
override shadow_strength: f32 = 0.5;
override waves: f32 = 1.0;
override coolness: f32 = 0.5;

// 1 where the point is in the light, 0 in the shadow of a block (4 taps, softened).
fn lit_amount(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if (frame.has_shadow == 0u) {
        return 1.0;
    }
    let texel = max(frame.shadow_center_texel, 0.05);
    // Move the point off the surface a little, so a face does not shadow itself.
    let moved = position + normal * (texel * 1.5);
    let p = shadow_position(moved);
    if (p.x < 0.0 || p.x > 1.0 || p.y < 0.0 || p.y > 1.0 || p.z > 1.0) {
        return 1.0;
    }
    let d = 1.0 / max(frame.shadow_resolution, 1.0);
    let z = p.z - 0.0015;
    var sum = 0.0;
    sum += textureSampleCompareLevel(shadow_map, shadow_sampler, p.xy + vec2<f32>(-d, -d), z);
    sum += textureSampleCompareLevel(shadow_map, shadow_sampler, p.xy + vec2<f32>(d, -d), z);
    sum += textureSampleCompareLevel(shadow_map, shadow_sampler, p.xy + vec2<f32>(-d, d), z);
    sum += textureSampleCompareLevel(shadow_map, shadow_sampler, p.xy + vec2<f32>(d, d), z);
    return sum * 0.25;
}

// The height of the water at a point of the world (two slow crossing waves).
fn wave_height(xz: vec2<f32>, t: f32) -> f32 {
    let a = sin(dot(xz, vec2<f32>(0.9, 0.4)) * 1.6 + t * 1.3);
    let b = sin(dot(xz, vec2<f32>(-0.3, 1.0)) * 2.3 - t * 1.7);
    let c = sin(dot(xz, vec2<f32>(0.7, -0.8)) * 3.7 + t * 2.1);
    return a * 0.5 + b * 0.3 + c * 0.2;
}

fn material(input: MaterialInput) -> Surface {
    var color = input.albedo * input.tint * vec4<f32>(input.lightmap, 1.0);

    // Shadows: only where the sky's light reaches, and fading as the light gets low.
    let light_up = smoothstep(0.02, 0.25, frame.light_direction.y);
    let sky_light = smoothstep(0.55, 0.95, input.light.y);
    let lit = lit_amount(input.position, input.normal);
    let dark = (1.0 - lit) * shadow_strength * light_up * sky_light;
    let shade = mix(vec3<f32>(1.0), vec3<f32>(0.66, 0.77, 1.0), clamp(dark * 1.6, 0.0, 1.0));
    color = vec4<f32>(color.rgb * (1.0 - dark) * shade, color.a);

    // Water: waves tilt the surface; sky and sun shine on it.
    if (input.fluid == 1u && waves > 0.0) {
        let world_xz = input.position.xz + frame.camera_position.xz;
        let t = frame.time;
        let e = 0.08;
        let h = wave_height(world_xz, t);
        let hx = wave_height(world_xz + vec2<f32>(e, 0.0), t);
        let hz = wave_height(world_xz + vec2<f32>(0.0, e), t);
        let slope = vec2<f32>(hx - h, hz - h) / e * 0.12 * waves;
        var n = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
        if (input.face != 1u) {
            n = input.normal;
        }
        let view = normalize(input.position);
        let fresnel = pow(1.0 - clamp(dot(-view, n), 0.0, 1.0), 3.0);
        let deep = color.rgb * vec3<f32>(0.78, 0.9, 1.05);
        let reflected = frame.sky_color * vec3<f32>(0.9, 0.97, 1.05);
        var rgb = mix(deep, reflected, fresnel * 0.6);
        rgb *= 0.92 + 0.16 * (h * 0.5 + 0.5) * waves;
        let glint = pow(max(dot(reflect(view, n), frame.light_direction), 0.0), 120.0);
        rgb += vec3<f32>(1.0, 0.97, 0.9) * glint * 0.8 * light_up * (1.0 - frame.rain) * waves;
        color = vec4<f32>(rgb, min(color.a + fresnel * 0.2, 0.95));
    }

    // Calm cool colours for everything.
    let grey = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let calm = mix(vec3<f32>(grey), color.rgb, 1.0 - 0.1 * coolness);
    let cool = calm * mix(vec3<f32>(1.0), vec3<f32>(0.96, 0.99, 1.04), coolness);
    return Surface(vec4<f32>(cool, color.a));
}
