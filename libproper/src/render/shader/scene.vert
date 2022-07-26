#version 450

layout(location = 0) in vec3 v_position;
layout(location = 1) in vec3 v_normal;
layout(location = 2) in vec2 v_tex_coord;

layout(set = 0, binding = 0) uniform Scene_Data {
    mat4 projection;
    mat4 view;
} u_scene;

layout(set = 2, binding = 0) uniform Model_Data {
    mat4 transform;
} u_model;

layout(location = 0) out vec3 m_normal;
layout(location = 1) out vec2 m_tex_coord;

void main() {
    gl_Position = u_scene.projection * u_scene.view * u_model.transform * vec4(v_position, 1.0);

    m_tex_coord = v_tex_coord;
    m_normal = v_normal;
}
