#version 450

layout(location = 0) in vec3 m_normal;
layout(location = 1) in vec2 m_tex_coord;

layout(set = 1, binding = 0) uniform Material_Data {
    vec4 diffuse_color;
} mat;
// layout(set = 1, binding = 1) uniform sampler2D u_diffuse_map;

layout(location = 0) out vec4 f_color;

const vec3 c_light_direction = normalize(vec3(-1.0, -1.0, -1.0));

void main() {
    vec3 color_in = mat.diffuse_color.xyz; // * texture(u_diffuse_map, m_tex_coord).rgb;

    float cos_theta = clamp(dot(m_normal, -c_light_direction), 0, 1);

    vec3 color_out = color_in * cos_theta;

    f_color = vec4(color_out, mat.diffuse_color.a);
}
