// Real directional light: the sun (or the moon) lights each face by its angle, block shadows come
// from a shadow map, and the open sky lights the shade a cool blue. Torches, sunsets and water
// are added on top, and the result goes through a soft tone mapping.
override tone: f32 = 0.6;
override saturation: f32 = 1.1;
override contrast: f32 = 0.85;
override shade: f32 = 0.5;
override sun_strength: f32 = 0.95;
override warmth: f32 = 0.35;
override torch: f32 = 0.6;
override waves: f32 = 1.0;
override water_opacity: f32 = 0.6;
override nether_end: bool = true;

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

// The height of the water at a point of the world: three slow waves crossing.
fn wave_height(xz: vec2<f32>, t: f32) -> f32 {
    let a = sin(dot(xz, vec2<f32>(0.9, 0.4)) * 1.6 + t * 1.3);
    let b = sin(dot(xz, vec2<f32>(-0.3, 1.0)) * 2.3 - t * 1.7);
    let c = sin(dot(xz, vec2<f32>(0.7, -0.8)) * 3.7 + t * 2.1);
    return a * 0.5 + b * 0.3 + c * 0.2;
}

// The Nether and the End: a constant warm or purple ambient light (the End's is the same everywhere),
// plus torch light in the Nether, run through the soft tone mapping.
fn other_dimension(input: MaterialInput) -> Surface {
    let alpha = input.albedo.a * input.tint.a;
    let base = input.albedo.rgb * input.tint.rgb;
    let lm = mix(0.5, 1.0, clamp(luma(input.lightmap), 0.0, 1.0));
    var light: vec3<f32>;
    if (frame.dimension == 1) {
        // End: ambient plus a purple glow that reaches everywhere.
        light = (vec3<f32>(1.98, 1.25, 2.3) + 5.5 * vec3<f32>(0.4, 0.1, 0.6) * 1.1) * 0.27;
    } else {
        // Nether: ambient plus warm light that is strongest next to a torch or lava.
        let lx = clamp(input.light.x, 0.0, 1.0);
        let torch_strength = min(lx / (0.5 - 0.45 * lx * lx), 10.0);
        light = (vec3<f32>(1.98, 1.44, 1.26) + vec3<f32>(0.49, 0.2, 0.1) * torch_strength * 0.55) * 0.5;
    }
    return Surface(vec4<f32>(grade(base * light * lm, 1.0, saturation, contrast), alpha));
}

fn material(input: MaterialInput) -> Surface {
    if (nether_end && frame.dimension != 0) {
        return other_dimension(input);
    }
    let base = input.albedo.rgb * input.tint.rgb;
    let vanilla = base * input.lightmap;
    var alpha = input.albedo.a * input.tint.a;

    let day = day_amount();
    let rain = frame.rain;
    // 1 where the sky's light reaches the point, 0 under cover.
    let sky_vis = smoothstep(0.7, 0.95, input.light.y);
    let covered = (1.0 - sky_vis) * smoothstep(0.1, 0.5, input.light.y);

    // The light's own colour: white-gold sun (golden when low), pale blue moon.
    let golden = vec3<f32>(1.0, 0.72, 0.45);
    let sun_col = mix(vec3<f32>(1.0, 0.97, 0.92), golden, sun_low_amount() * warmth);
    let source = mix(vec3<f32>(0.16, 0.2, 0.34), sun_col, day);
    let ndl = max(dot(input.normal, frame.light_direction), 0.0);
    // No shadow map (it costs the phone too much): a face is lit where the sky's light reaches it.
    let visible = lit_amount(input.position, input.normal);
    let strength = mix(0.35, 1.0, day) * (1.0 - 0.75 * rain) * sun_strength;
    let direct = source * (ndl * visible * sky_vis * strength);

    // The shade is vanilla's light, a little darker and cooler where the sky is open.
    let openness = sky_vis * mix(0.35, 1.0, day) * sun_strength;
    let shade_tint = mix(vec3<f32>(1.0), vec3<f32>(0.8, 0.88, 1.0), openness * 0.7);
    var rgb = vanilla * shade_tint + base * direct * 1.1;
    // A bright open day: the air itself glows a little, so places in the open feel lit.
    rgb += base * source * (0.05 * openness * (1.0 - rain));

    // Extra soft shade under trees, roofs and overhangs, by day only.
    rgb *= 1.0 - shade * 0.3 * covered * day * (1.0 - input.light.x * 0.5);

    // Warm torch light that flickers a little, strongest away from the sun.
    let world = input.position + frame.camera_position;
    let flicker = 0.92 + 0.08 * sin(frame.time * 7.0 + world.x * 3.1 + world.z * 2.3);
    let glow = pow(clamp(input.light.x, 0.0, 1.0), 3.0) * (1.0 - sky_vis * day);
    rgb += base * vec3<f32>(1.0, 0.62, 0.3) * glow * torch * 0.6 * flicker;

    // Water: waves tilt the surface, the sky is mirrored in it and the sun glints on it.
    if (input.fluid == 1u && waves > 0.0) {
        let t = frame.time;
        let e = 0.08;
        let h = wave_height(world.xz, t);
        let hx = wave_height(world.xz + vec2<f32>(e, 0.0), t);
        let hz = wave_height(world.xz + vec2<f32>(0.0, e), t);
        let slope = vec2<f32>(hx - h, hz - h) / e * 0.12 * waves;
        var n = normalize(vec3<f32>(-slope.x, 1.0, -slope.y));
        if (input.face != 1u) {
            n = input.normal;
        }
        let view = normalize(input.position);
        let fresnel = pow(1.0 - clamp(dot(-view, n), 0.0, 1.0), 3.0);
        let reflected = frame.sky_color * vec3<f32>(0.9, 0.97, 1.05);
        rgb = mix(rgb, reflected, fresnel * 0.55);
        rgb *= 0.93 + 0.14 * (h * 0.5 + 0.5) * waves;
        let sun_up = smoothstep(0.02, 0.25, frame.light_direction.y);
        let glint = pow(max(dot(reflect(view, n), frame.light_direction), 0.0), 120.0);
        rgb += vec3<f32>(1.0, 0.97, 0.9) * glint * 0.7 * sun_up * (1.0 - rain) * waves;
        alpha = min(mix(alpha * water_opacity / 0.6, 1.0, fresnel * 0.5), 0.97);
    }

    return Surface(vec4<f32>(grade(rgb, tone, saturation, contrast), alpha));
}
