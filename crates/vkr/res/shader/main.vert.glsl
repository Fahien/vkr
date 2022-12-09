#version 450
// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

layout(binding = 0) uniform Mvp {
    mat4 model;
} mvp;

layout(location = 0) in vec3 in_pos;
layout(location = 1) in vec4 in_color;

out gl_PerVertex {
    vec4 gl_Position;
};

layout(location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
    gl_Position = mvp.model * vec4(in_pos, 1.0);
}
