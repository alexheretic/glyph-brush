#version 150

const mat4 INVERT_Y_AXIS = mat4(
    vec4(1.0, 0.0, 0.0, 0.0),
    vec4(0.0, -1.0, 0.0, 0.0),
    vec4(0.0, 0.0, 1.0, 0.0),
    vec4(0.0, 0.0, 0.0, 1.0)
);

uniform mat4 transform;

in vec3 left_top;
in vec2 right_bottom;
in vec4 color;

out vec2 f_tex_pos;

// generate positional data based on vertex ID
void main() {
    vec2 pos = vec2(0.0);
    float left = left_top.x;
    float right = right_bottom.x;
    float top = left_top.y;
    float bottom = right_bottom.y;

    switch (gl_VertexID) {
        case 0:
            pos = vec2(left, top);
            f_tex_pos = vec2(0.0, 1.0);
            break;
        case 1:
            pos = vec2(right, top);
            f_tex_pos = vec2(1.0, 1.0);
            break;
        case 2:
            pos = vec2(left, bottom);
            f_tex_pos = vec2(0.0, 0.0);
            break;
        case 3:
            pos = vec2(right, bottom);
            f_tex_pos = vec2(1.0, 0.0);
            break;
    }

    gl_Position = INVERT_Y_AXIS * transform * vec4(pos, left_top.z, 1.0);
}
