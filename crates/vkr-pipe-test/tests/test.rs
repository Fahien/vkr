// Copyright © 2021-2022
// Author: Antonio Caggiano <info@antoniocaggiano.eu>
// SPDX-License-Identifier: MIT

use vkr_pipe::*;

pipewriter!("crates/vkr-pipe-test/shader/simple/src/simple.rs");

#[test]
fn build_simple_shader() {
    let _main_pipeline = PipelineMain {};
    let _secondary_pipeline = PipelineSecondary {};
    assert!(1 == 1);
}