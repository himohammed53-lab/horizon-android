// A clear three-colour sky gradient (zenith, horizon, red edge) driven by vanilla's fog colour,
// blended with vanilla's own sky so it meets the distance fog without a seam.
override sky_style: f32 = 0.6;
override tone: f32 = 0.5;
override nether_end: bool = true;
override end_stars: f32 = 1.0;
override saturation: f32 = 1.12;
override contrast: f32 = 0.93;

const NIGHT_SKY: vec3<f32> = vec3<f32>(0.02, 0.02, 0.19);
const SKY_BASE: vec3<f32> = vec3<f32>(0.15, 0.6, 1.4);
const HORIZON_BASE: vec3<f32> = vec3<f32>(1.0, 0.4, 0.3);
const HORIZON_EDGE: vec3<f32> = vec3<f32>(1.0, 0.4, 0.2);
const DAY_CLARITY: f32 = 0.3;

fn zenith_colour(fog: vec3<f32>, rain: f32) -> vec3<f32> {
    let val = max(fog.r * 0.6, max(fog.g, fog.b));
    var c = (0.77 * val * val + 0.33 * val) * SKY_BASE;
    c += NIGHT_SKY * (0.4 - 0.4 * fog.b);
    let b = min(fog.g, 0.26);
    let grey = vec3<f32>(0.85, 0.9, 1.0) * (b * b * 13.2);
    return mix(c * (1.0 + 0.5 * rain), grey, rain);
}

fn sky_horizon_colour(fog: vec3<f32>, rain: f32) -> vec3<f32> {
    let val = max(fog.r * 0.65, max(fog.g * 1.1, fog.b));
    let sun = max(fog.r - fog.b, 0.0);
    var c = HORIZON_BASE * ((0.7 * val * val + 0.4 * val + sun) * 2.4);
    c += NIGHT_SKY;
    c = mix(c, 2.0 * val * mix(vec3<f32>(0.7, 1.0, 0.9), SKY_BASE, DAY_CLARITY), val * val);
    let b = min(fog.g, 0.26);
    return mix(c, vec3<f32>(b * b * 19.6), rain);
}

fn star_hash(p: vec3<f32>) -> f32 {
    return fract(sin(dot(p, vec3<f32>(12.9898, 78.233, 37.719))) * 43758.5453);
}

// The End's sky: black, with white stars and a bright swirling galaxy overhead (and a glow from the
// void below). It turns with the view like any sky. The swirl is a logarithmic spiral of two arms.
fn end_sky(d: vec3<f32>) -> vec4<f32> {
    let end_glow = grade(vec3<f32>(0.16, 0.06, 0.2), 1.0, saturation, contrast);
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

fn sky(input: SkyInput) -> vec4<f32> {
    if (frame.dimension == 1 && nether_end) {
        return end_sky(normalize(input.direction));
    }
    // The Overworld has the clear gradient sky; the Nether keeps vanilla's.
    if (frame.dimension != 0 || sky_style <= 0.0) {
        return input.color;
    }
    let fog = frame.fog_color;
    let rain = frame.rain;
    let zenith = zenith_colour(fog, rain);
    var horizon = sky_horizon_colour(fog, rain);

    let edge_amount = (1.1 - fog.b) * fog.g * 2.1 * (1.0 - rain);
    let edge = horizon * (vec3<f32>(1.0) - edge_amount * (vec3<f32>(1.0) - HORIZON_EDGE));

    let up = clamp(abs(input.direction.y), 0.0, 1.0);
    let h = 1.0 - up * up;
    let hsq = h * h;
    var g1 = hsq * hsq * hsq * hsq;
    let g2 = 0.6 * g1 + 0.4 * hsq;
    g1 = g1 * g1;
    horizon = mix(horizon, edge, g1);
    let gradient = mix(zenith, horizon, g2);

    let styled = grade(gradient, tone, saturation, contrast);
    // At the very horizon go back toward vanilla's colour, which the fog fades to.
    let colour = mix(styled, input.color.rgb, g1 * 0.6);
    return vec4<f32>(mix(input.color.rgb, colour, clamp(sky_style, 0.0, 1.0)), input.color.a);
}
