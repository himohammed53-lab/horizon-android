// Helpers shared by the stages: the soft tone mapping and colour grade.

fn luma(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
}

// An extended Reinhard curve: bright colours roll off softly instead of clipping.
fn tonemap_soft(c: vec3<f32>) -> vec3<f32> {
    return (c * (1.0 + c * 0.063)) / (1.0 + c);
}

// The grade: `tone` says how much of the result replaces the input.
fn grade(c: vec3<f32>, tone: f32, saturation: f32, contrast: f32) -> vec3<f32> {
    // The soft curve darkens, so the light is raised first, more the stronger the grade.
    let exposure = 1.0 + 0.6 * tone;
    let mapped = pow(max(tonemap_soft(c * exposure), vec3<f32>(0.0)), vec3<f32>(contrast));
    let colourful = mix(vec3<f32>(luma(mapped)), mapped, saturation);
    return mix(c, colourful, tone);
}

// 1 by day in the Overworld, fading out as the sun sets; 0 elsewhere.
fn day_amount() -> f32 {
    return select(0.0, smoothstep(-0.12, 0.2, frame.sun_direction.y), frame.dimension == 0);
}

// 1 when the sun is low (sunrise and sunset), 0 once it is well up.
fn sun_low_amount() -> f32 {
    return 1.0 - smoothstep(0.0, 0.4, abs(frame.sun_direction.y));
}
