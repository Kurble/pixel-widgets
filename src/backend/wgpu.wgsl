struct VertexOutput {
    [[location(0)]] uv: vec2<f32>;
    [[location(1)]] color: vec4<f32>;
    [[location(2)]] mode: vec4<f32>;
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn vs_main(
    [[location(0)]] a_pos: vec2<f32>,
    [[location(1)]] a_uv: vec2<f32>,
    [[location(2)]] a_color: vec4<f32>,
    [[location(3)]] a_mode: vec4<f32>,
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
[[group(0), binding(2)]]
var u_linear_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    var tex: vec4<f32> = textureSample(u_color_texture, u_sampler, in.uv);
    var font: vec4<f32> = textureSample(u_color_texture, u_linear_sampler, in.uv);

    switch (u32(in.mode.x)) {
        case 1: {
            tex = vec4<f32>(1.0);
            break;
        }
        case 2: {
            let sd = max(min(font.r, font.g), min(max(font.r, font.g), font.b));
            let px_range = 1.0;
            let screen_px_dist = in.mode.y * (sd - 0.5);
            let opacity = clamp(screen_px_dist + 0.5, 0.0, 1.0);
            tex = vec4<f32>(1.0, 1.0, 1.0, opacity);
            break;
        }
    }
    
    return in.color * tex;
}
