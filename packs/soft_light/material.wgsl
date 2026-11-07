// Soft shade under cover, warm sunsets and torches, and water with waves. It starts from
// vanilla's own colour (`albedo * tint * lightmap`) and changes it a little, so it is cheap.
override tone: f32 = 0.5;
override saturation: f32 = 1.1;
override contrast: f32 = 0.85;
override shade: f32 = 0.5;
override warmth: f32 = 0.6;
override torch: f32 = 0.6;
override waves: f32 = 1.0;
override water_opacity: f32 = 0.6;
override nether_end: bool = true;

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
    var rgb = input.albedo.rgb * input.tint.rgb * input.lightmap;
    var alpha = input.albedo.a * input.tint.a;

    let day = day_amount();
    let rain = frame.rain;
    // 1 where the sky's light reaches the point fully, 0 under cover.
    let open_sky = smoothstep(0.78, 0.92, input.light.y);
    let covered = (1.0 - open_sky) * smoothstep(0.1, 0.5, input.light.y);

    // Soft shade under trees, roofs and overhangs, by day only.
    rgb *= 1.0 - shade * 0.45 * covered * day * (1.0 - input.light.x * 0.5);

    // Golden sunlight at sunrise and sunset, only where the sun reaches.
    let golden = vec3<f32>(1.0, 0.76, 0.52);
    let sun_tint = mix(vec3<f32>(1.0), golden, sun_low_amount() * warmth);
    rgb *= mix(vec3<f32>(1.0), sun_tint, open_sky * day * (1.0 - rain));

    // Cool moonlight: the open night sky is a little blue.
    let night = select(0.0, 1.0 - day, frame.dimension == 0);
    rgb *= mix(vec3<f32>(1.0), vec3<f32>(0.86, 0.93, 1.1), night * open_sky);

    // Warm torch light that flickers a little, strongest away from the sun.
    let world = input.position + frame.camera_position;
    let flicker = 0.92 + 0.08 * sin(frame.time * 7.0 + world.x * 3.1 + world.z * 2.3);
    let glow = pow(clamp(input.light.x, 0.0, 1.0), 3.0) * (1.0 - open_sky * day);
    rgb += input.albedo.rgb * vec3<f32>(1.0, 0.62, 0.3) * glow * torch * 0.6 * flicker;

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
