#version 150

uniform sampler2D font_tex;

in vec2 f_tex_pos;

out vec4 out_color;

void main() {
    out_color = texture(font_tex, f_tex_pos);
}
