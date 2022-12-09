#version 450
// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

layout(location = 0) in vec4 in_color;

layout(location = 0) out vec4 out_color;

void main() {
    out_color = in_color;
}
