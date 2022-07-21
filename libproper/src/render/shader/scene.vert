#version 450

layout(location = 0) in vec3 v_position;

layout(location = 0) out vec3 m_position;

layout(set = 0, binding = 0) uniform Scene_Data {
    mat4 projection;
    mat4 view;
} u_scene;

layout(set = 1, binding = 0) uniform Model_Data {
    mat4 transform;
} u_model;

void main() {
    gl_Position = u_scene.projection * u_scene.view * u_model.transform * vec4(v_position, 1.0);
    m_position = v_position;
}