#version 450

layout(set = 0, binding = 0) uniform texture2D t_Color;
layout(set = 0, binding = 1) uniform sampler s_Color;

layout(location = 0) in vec2 v_Uv;
layout(location = 1) in vec4 v_Color;
layout(location = 2) flat in uint v_Mode;

layout(location = 0) out vec4 Target0;

void main() {
    // Text
    if (v_Mode == uint(0)) {
        Target0 = v_Color * vec4(1.0, 1.0, 1.0, texture(t_Color, v_Uv).a);

    // Image
    } else if (v_Mode == uint(1)) {
        Target0 = v_Color * texture(t_Color, v_Uv);

    // 2D Geometry
    } else if (v_Mode == uint(2)) {
        Target0 = v_Color;
    }
}