// The End's sky: a purple gradient with a bright swirling galaxy overhead and a glow from the void
// below. The End has no sky stage, so this runs as a pass over the sky's pixels (depth 0) and does
// nothing anywhere else. It is the sky's own look, written for Horizon (the swirl is a
// logarithmic spiral of two arms that turns slowly).
override saturation: f32 = 1.1;
override contrast: f32 = 0.85;
override nether_end: bool = true;

fn post(input: PostInput) -> vec4<f32> {
    if (!nether_end || frame.dimension != 1 || input.depth > 0.0) {
        return input.color;
    }
    let d = normalize(view_position(input.uv, 0.5));
    let end_glow = grade(vec3<f32>(0.16, 0.06, 0.2), 1.0, saturation, contrast);

    // Gradient from the horizon up, then the stars of vanilla's own End sky texture.
    var grad = 1.0 - max(d.y, 0.0);
    grad = grad * grad;
    var col = end_glow * smoothstep(0.0, 1.0, grad);
    col += input.color.rgb * 5.0 * (1.0 - grad * grad);

    // The void's glow below the horizon.
    let glow = max((-d.y - 0.5) * 2.0, 0.0);
    col += end_glow * glow * glow * glow;

    // The galaxy: two spiral arms around the point straight overhead, with a bright core.
    if (d.y > 0.0) {
        let r = length(d.xz);
        let a = atan2(d.z, d.x) - frame.time * 0.1;
        let arm = 0.5 + 0.5 * cos(2.0 * (a - 5.5 * log(r + 0.08)));
        let falloff = exp(-r * 3.2) * smoothstep(1.0, 0.55, r);
        let core = exp(-r * r * 40.0);
        col += vec3<f32>(0.42, 0.2, 0.85) * pow(arm, 2.5) * falloff * 2.2;
        col += vec3<f32>(1.0, 0.82, 1.0) * core * 1.4;
    }
    return vec4<f32>(col, 1.0);
}
