#version 450

layout(location = 0) in vec3 v_position;

layout(location = 0) out vec3 m_position;

void main() {
    gl_Position = vec4(v_position, 1.0);
    m_position = v_position;
}
