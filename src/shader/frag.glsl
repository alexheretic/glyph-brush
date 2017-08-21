#version 130

uniform sampler2D font_tex;

in vec2 f_tex_pos;
in vec4 f_color;

out vec4 Target0;

void main() {
    Target0 = f_color * vec4(1.0, 1.0, 1.0, texture(font_tex, f_tex_pos).r);
}
