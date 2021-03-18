#!/bin/sh
# Copyright © 2021
# Author: Antonio Caggiano <info@antoniocaggiano.eu>
# SPDX-License-Identifier: MIT

git submodule update --init
cp dep/rust-gpu/rust-toolchain .
cargo build

# Run tests before the application to generate test PNG file
cargo test
