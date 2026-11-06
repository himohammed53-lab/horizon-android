// A calm, slightly cool sky: vanilla's own colour, a little less bright and blue-shifted.
override coolness: f32 = 0.5;

fn sky(input: SkyInput) -> vec4<f32> {
    let c = input.color.rgb;
    let grey = dot(c, vec3<f32>(0.2126, 0.7152, 0.0722));
    let calm = mix(vec3<f32>(grey), c, 1.0 - 0.15 * coolness);
    let cool = calm * mix(vec3<f32>(1.0), vec3<f32>(0.93, 0.98, 1.06), coolness);
    return vec4<f32>(cool, input.color.a);
}
