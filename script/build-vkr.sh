#!/bin/sh
# Copyright © 2021-2022
# Author: Antonio Caggiano <info@antoniocaggiano.eu>
# SPDX-License-Identifier: MIT

BRANCH="v0.4.0-alpha.12"
REPO=https://raw.githubusercontent.com/EmbarkStudios/rust-gpu

curl $REPO/$BRANCH/rust-toolchain --output rust-toolchain
cargo build

# Run tests before the application to generate test PNG file
cargo test
