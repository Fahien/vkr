#version 450
// Copyright © 2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

layout(location = 0) in vec3 in_pos;

out gl_PerVertex {
    vec4 gl_Position;
};

void main() {
    gl_Position = vec4(in_pos, 1.0);
}
