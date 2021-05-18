struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] mode: f32;
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] a_pos: vec2<f32>,
    [[location(1)]] a_uv: vec2<f32>,
    [[location(2)]] a_color: vec4<f32>,
    [[location(3)]] a_mode: f32,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = a_uv;
    out.color = a_color;
    out.mode = a_mode;
    out.pos = vec4<f32>(a_pos.x, -a_pos.y, 0.0, 1.0);
    return out;
}

[[group(0), binding(0)]]
var u_color_texture: texture_2d<f32>;
[[group(0), binding(1)]]
var u_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var tex_color: vec4<f32> = textureSample(u_color_texture, u_sampler, in.uv);
    tex_color.x = mix(tex_color.x, 1.0, in.mode);
    tex_color.y = mix(tex_color.y, 1.0, in.mode);
    tex_color.z = mix(tex_color.z, 1.0, in.mode);
    tex_color.w = mix(tex_color.w, 1.0, in.mode);
    return in.color * tex_color;
}
