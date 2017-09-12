#version 130

uniform mat4 transform;

in vec3 pos;
in vec2 tex_pos;
in vec4 color;

out vec2 f_tex_pos;
out vec4 f_color;

void main() {
    f_color = color;
    f_tex_pos = tex_pos;
    gl_Position = transform * vec4(pos, 1.0);
}
