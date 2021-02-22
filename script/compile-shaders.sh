#!/bin/sh
# Copyright © 2021
# Author: Antonio Caggiano <info@antoniocaggiano.eu>
# SPDX-License-Identifier: MIT

glslangValidator -S vert res/shader/vert.glsl -V -o res/shader/vert.spv
glslangValidator -S frag res/shader/frag.glsl -V -o res/shader/frag.spv
