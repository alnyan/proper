#version 450

layout(location = 0) out vec4 f_color;

layout(input_attachment_index = 0, set = 0, binding = 0) uniform subpassInputMS u_color;

void main() {
    vec4 color_out = vec4(0.0);
    for (int i = 0; i < 4; ++i) {
        color_out += subpassLoad(u_color, i);
    }
    color_out /= vec4(4.0);
    f_color = color_out;
}
