#version 450

layout(location = 0) out vec4 f_color;

layout(set = 1, binding = 0) uniform Material_Data {
    vec4 diffuse_color;
} mat;

void main() {
    f_color = mat.diffuse_color;
}
