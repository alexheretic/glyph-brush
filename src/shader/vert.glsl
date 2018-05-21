#version 150

uniform mat4 transform;

in vec3 left_top;
in vec2 right_bottom;
in vec2 tex_left_top;
in vec2 tex_right_bottom;
in vec4 color;

out vec2 g_tex_left_top;
out vec2 g_tex_right_bottom;
out vec4 g_color;
out vec4 g_right_top;
out vec4 g_left_bottom;
out vec4 g_right_bottom;

// forward on color and 4-positions & texture coords -> geometry
void main() {
    g_color = color;
    g_tex_left_top = tex_left_top;
    g_tex_right_bottom = tex_right_bottom;

    float left = left_top.x;
    float right = right_bottom.x;
    float top = left_top.y;
    float bottom = right_bottom.y;
    vec2 zw = vec2(left_top.z, 1.0);

    g_right_top = transform * vec4(right, top, zw);
    g_left_bottom = transform * vec4(left, bottom, zw);
    g_right_bottom = transform * vec4(right, bottom, zw);
    gl_Position = transform * vec4(left, top, zw);
}
