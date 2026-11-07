// The End's sky: black, with white stars and a bright swirling galaxy overhead (and a glow from the void
// below). The End has no sky stage, so this runs as a pass over the sky's pixels (depth 0) and does
// nothing anywhere else. It is the sky's own look, written for Horizon (the swirl is a
// logarithmic spiral of two arms that turns slowly).
override saturation: f32 = 1.1;
override contrast: f32 = 0.85;
override nether_end: bool = true;
override end_stars: f32 = 1.0;

fn star_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

fn post(input: PostInput) -> vec4<f32> {
    if (!nether_end || frame.dimension != 1 || input.depth > 0.0) {
        return input.color;
    }
    let d = normalize(view_position(input.uv, 0.5));
    let end_glow = grade(vec3<f32>(0.16, 0.06, 0.2), 1.0, saturation, contrast);

    // A black sky, with only a faint purple haze at the horizon and the void's glow below it.
    var grad = 1.0 - max(d.y, 0.0);
    grad = grad * grad;
    var col = end_glow * smoothstep(0.55, 1.0, grad) * 0.3;
    let glow = max((-d.y - 0.5) * 2.0, 0.0);
    col += end_glow * glow * glow * glow * 0.6;

    // White stars: a few bright points, each a small round dot that twinkles a little.
    let p = d * 150.0;
    let cell = floor(p);
    let h = star_hash(cell);
    let r = length(fract(p) - 0.5);
    let twinkle = 0.75 + 0.25 * sin(frame.time * 2.0 + h * 60.0);
    let star = step(0.982, h) * smoothstep(0.38, 0.0, r) * twinkle;
    col += vec3<f32>(1.0, 0.97, 1.0) * star * end_stars * (1.0 - 0.6 * grad);

    // The galaxy: two spiral arms around the point straight overhead, with a bright core.
    if (d.y > 0.0) {
        let r2 = length(d.xz);
        let a = atan2(d.z, d.x) - frame.time * 0.1;
        let arm = 0.5 + 0.5 * cos(2.0 * (a - 5.5 * log(r2 + 0.08)));
        let falloff = exp(-r2 * 2.6) * smoothstep(1.0, 0.5, r2);
        let core = exp(-r2 * r2 * 40.0);
        col += vec3<f32>(0.42, 0.2, 0.85) * pow(arm, 2.5) * falloff * 2.4;
        col += vec3<f32>(1.0, 0.82, 1.0) * core * 1.5;
    }
    return vec4<f32>(col, 1.0);
}
