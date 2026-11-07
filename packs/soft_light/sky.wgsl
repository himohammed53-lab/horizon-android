// A clear three-colour sky gradient (zenith, horizon, red edge) driven by vanilla's fog colour,
// blended with vanilla's own sky so it meets the distance fog without a seam.
override sky_style: f32 = 0.6;
override tone: f32 = 0.5;
override saturation: f32 = 1.1;
override contrast: f32 = 0.85;

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

fn horizon_colour(fog: vec3<f32>, rain: f32) -> vec3<f32> {
    let val = max(fog.r * 0.65, max(fog.g * 1.1, fog.b));
    let sun = max(fog.r - fog.b, 0.0);
    var c = HORIZON_BASE * ((0.7 * val * val + 0.4 * val + sun) * 2.4);
    c += NIGHT_SKY;
    c = mix(c, 2.0 * val * mix(vec3<f32>(0.7, 1.0, 0.9), SKY_BASE, DAY_CLARITY), val * val);
    let b = min(fog.g, 0.26);
    return mix(c, vec3<f32>(b * b * 19.6), rain);
}

fn sky(input: SkyInput) -> vec4<f32> {
    // Only the Overworld has this sky; the Nether and the End keep vanilla's.
    if (frame.dimension != 0 || sky_style <= 0.0) {
        return input.color;
    }
    let fog = frame.fog_color;
    let rain = frame.rain;
    let zenith = zenith_colour(fog, rain);
    var horizon = horizon_colour(fog, rain);

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
