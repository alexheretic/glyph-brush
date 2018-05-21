#version 150

layout(points) in;
layout(triangle_strip, max_vertices = 4) out;

in vec2 g_tex_left_top[];
in vec2 g_tex_right_bottom[];
in vec4 g_color[];
in vec4 g_right_top[];
in vec4 g_left_bottom[];
in vec4 g_right_bottom[];

out vec2 f_tex_pos;
out vec4 f_color;

// Generate 4 vertex corners of the glyph rectangle
void main() {
    f_color = g_color[0];

    gl_Position = gl_in[0].gl_Position;
    f_tex_pos = g_tex_left_top[0];
    EmitVertex();

    gl_Position = g_right_top[0];
    f_tex_pos = vec2(g_tex_right_bottom[0].x, g_tex_left_top[0].y);
    EmitVertex();

    gl_Position = g_left_bottom[0];
    f_tex_pos = vec2(g_tex_left_top[0].x, g_tex_right_bottom[0].y);
    EmitVertex();

    gl_Position = g_right_bottom[0];
    f_tex_pos = g_tex_right_bottom[0];
    EmitVertex();

    EndPrimitive();
}
