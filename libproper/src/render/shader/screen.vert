#version 450

layout(location = 0) in vec3 v_position;
layout(location = 1) in vec3 v_normal;

void main() {
    gl_Position = vec4(v_position, 1.0);
}
